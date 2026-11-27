//! The authorization boundary through the real ingress (`db:PR12.18`, triage
//! item 6): the image `Containerfile` builds, started by `compose.yaml` as a
//! deployment would start it, reached over its published port, with read
//! auditing on.
//!
//! The in-process tests in `http.rs` drive the router directly. This drives
//! the container: the binary, its environment, the port mapping and the data
//! volume. A route that only refuses when called in-process, or an access log
//! that is configured and never written inside the image, fails here.
//!
//! `#[ignore]`d: it needs podman and builds an image. Run it with
//!
//! ```sh
//! cargo test --test ingress -- --ignored --nocapture
//! ```
//!
//! It mints its own throwaway keypair, starts the stack with that public key,
//! stops and restarts it on the same volume (the record, its chain checkpoint
//! and the access log must all survive), and tears the stack and its volume
//! down afterwards.

use openehr_store::conformance;
use pasetors::{
    claims::Claims,
    keys::{AsymmetricKeyPair, Generate as _},
    paserk::FormatAsPaserk as _,
    public,
    version4::V4,
};
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

const PROJECT: &str = "openehr-ingress-check";
const BASE: &str = "http://127.0.0.1:5150/openehr/v1";
const ELSEWHERE: &str = "6BA7B810-9DAD-11D1-80B4-00C04FD430C8";

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the crate sits in the repository")
        .to_path_buf()
}

fn compose(public_key: &str, args: &[&str]) -> std::process::Output {
    Command::new("podman")
        .arg("compose")
        .args(["-p", PROJECT])
        .args(args)
        .current_dir(repository_root())
        .env("OPENEHR_PASETO_PUBLIC_KEYS", public_key)
        .output()
        .expect("podman runs")
}

fn token(pair: &AsymmetricKeyPair<V4>, grants: Option<serde_json::Value>) -> String {
    let mut claims = Claims::new().expect("claims");
    claims.subject("clinician-4417").expect("subject");
    if let Some(grants) = grants {
        claims
            .add_additional(openehr_axa::authz::GRANTS_CLAIM, grants)
            .expect("claim");
    }
    public::sign(&pair.secret, &claims, None, None).expect("signs")
}

/// `curl`, returning the HTTP status. The body goes to a scratch file and is
/// returned too.
fn call(method: &str, path: &str, token: &str, body: Option<&str>) -> (u16, String) {
    let out = std::env::temp_dir().join(format!("openehr-ingress-{}.body", std::process::id()));
    let mut command = Command::new("curl");
    command
        .args(["-s", "-o"])
        .arg(&out)
        .args(["-w", "%{http_code}", "-X", method])
        .args(["-H", &format!("Authorization: Bearer {token}")]);
    if let Some(body) = body {
        command.args([
            "-H",
            "Content-Type: application/json",
            "--data-binary",
            body,
        ]);
    }
    let status = command
        .arg(format!("{BASE}{path}"))
        .output()
        .expect("curl runs");
    let code = String::from_utf8_lossy(&status.stdout)
        .trim()
        .parse()
        .unwrap_or(0);
    let text = std::fs::read_to_string(&out).unwrap_or_default();
    let _ = std::fs::remove_file(&out);
    (code, text)
}

struct Teardown<'a>(&'a str);

impl Drop for Teardown<'_> {
    fn drop(&mut self) {
        let _ = compose(self.0, &["down", "-v"]);
    }
}

// Long on purpose: one deployment, brought up once (an image build takes
// minutes), and every grant case asked of it in the order a reviewer reads
// them. Splitting it would mean building the stack once per test.
#[allow(clippy::too_many_lines)]
#[test]
#[ignore = "needs podman; builds and starts the compose stack"]
fn the_deployed_service_enforces_grants_records_denials_and_survives_a_restart() {
    let pair = AsymmetricKeyPair::<V4>::generate().expect("keypair");
    let mut paserk = String::new();
    pair.public.fmt(&mut paserk).expect("PASERK");

    let up = compose(&paserk, &["up", "--build", "-d"]);
    assert!(
        up.status.success(),
        "compose up failed: {}",
        String::from_utf8_lossy(&up.stderr)
    );
    let _teardown = Teardown(&paserk);

    wait_ready();

    let ehr = conformance::sample_ehr();
    let ehr_id = ehr.ehr_id().to_string();
    let writer = token(
        &pair,
        Some(serde_json::json!([{"ehr": ehr_id, "actions": ["write"]}])),
    );
    let reader = token(
        &pair,
        Some(serde_json::json!([{"ehr": ehr_id, "actions": ["read"]}])),
    );
    let other_reader = token(
        &pair,
        Some(serde_json::json!([{"ehr": ELSEWHERE, "actions": ["read"]}])),
    );
    let none = token(&pair, None);

    // Creating the record needs write on it (`PUT`, a named id, `db:S1.22`).
    // `POST`, which mints an id, needs a "*" grant, which no token here has.
    let path = format!("/ehr/{ehr_id}");
    assert_eq!(
        call("PUT", &path, &reader, Some("")).0,
        403,
        "read cannot create"
    );
    assert_eq!(
        call("POST", "/ehr", &writer, Some("")).0,
        403,
        "POST needs \"*\""
    );
    let (created, text) = call("PUT", &path, &writer, Some(""));
    assert_eq!(created, 201, "{text}");

    // Reading it needs read on it, and nothing else will do.
    let (ok, text) = call("GET", &path, &reader, None);
    assert_eq!(ok, 200, "{text}");
    assert!(text.contains(&ehr_id), "{text}");
    assert_eq!(
        call("GET", &path, &writer, None).0,
        403,
        "write is not read"
    );
    assert_eq!(
        call("GET", &path, &other_reader, None).0,
        403,
        "another record's grant"
    );
    assert_eq!(call("GET", &path, &none, None).0, 403, "no grants");
    // No oracle: the absent record answers the same as the present one.
    assert_eq!(
        call("GET", &format!("/ehr/{ELSEWHERE}"), &reader, None).0,
        403
    );
    // A query reads the record it resolves to.
    let aql = serde_json::json!({
        "q": "SELECT c/name/value FROM EHR e[ehr_id/value=$ehrUid] \
              CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1] LIMIT 10",
        "query_parameters": {"ehrUid": ehr_id},
    })
    .to_string();
    assert_eq!(call("POST", "/query/aql", &other_reader, Some(&aql)).0, 403);
    assert_eq!(call("POST", "/query/aql", &reader, Some(&aql)).0, 200);

    // The deployment audits reads (`compose.yaml` sets the path): the denials
    // are in the log on the data volume, as `denied`, and so are the reads.
    let log = access_log(&paserk);
    let outcomes: Vec<String> = log
        .lines()
        .map(|line| {
            serde_json::from_str::<serde_json::Value>(line).expect("one JSON record per line")["outcome"]
                .as_str()
                .unwrap_or_default()
                .to_owned()
        })
        .collect();
    let count = |o: &str| outcomes.iter().filter(|x| *x == o).count();
    assert_eq!(count("ok"), 2, "{log}");
    assert_eq!(count("denied"), 5, "{log}");
    assert!(
        !log.contains("Encounter"),
        "the log quotes no content: {log}"
    );
    eprintln!(
        "ingress: {} access records, {} denied",
        outcomes.len(),
        count("denied")
    );

    // --- Restart on the same volume (release evidence: install and restart
    // the packaged service on a disposable volume). What was committed before
    // the stop is there after it, its chain unchanged, and the access log
    // keeps its earlier records rather than starting over.
    let (_, text) = call("GET", &path, &reader, None);
    let body: serde_json::Value = serde_json::from_str(&text).expect("the EHR is JSON");
    let status_id = body["ehr_status"]["id"]["value"]
        .as_str()
        .unwrap_or_else(|| panic!("the EHR names its status: {text}"))
        .to_owned();
    let before = operator_task(&paserk, "checkpoint", &status_id);
    assert!(before.status.success(), "{}", stderr_of(&before));
    let before = String::from_utf8_lossy(&before.stdout).trim().to_owned();
    assert!(before.starts_with("entries=1 "), "{before}");

    // The log as the stop finds it: the read above, for the status id, is in it.
    let at_stop = access_log(&paserk);
    let down = compose(&paserk, &["down"]);
    assert!(down.status.success(), "compose down: {}", stderr_of(&down));
    let up = compose(&paserk, &["up", "-d"]);
    assert!(up.status.success(), "compose up again: {}", stderr_of(&up));
    wait_ready();

    let (ok, text) = call("GET", &path, &reader, None);
    assert_eq!(ok, 200, "the record survives the restart: {text}");
    let (ok, text) = call(
        "GET",
        &format!("{path}/versioned_ehr_status/revision_history"),
        &reader,
        None,
    );
    assert_eq!(ok, 200, "{text}");
    let history: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    assert_eq!(history["items"].as_array().map(Vec::len), Some(1), "{text}");

    let after = operator_task(&paserk, "checkpoint", &status_id);
    assert!(after.status.success(), "{}", stderr_of(&after));
    assert_eq!(
        String::from_utf8_lossy(&after.stdout).trim(),
        before,
        "the chain is the one committed before the stop"
    );
    let verified = operator_task(&paserk, "verify", &status_id);
    assert!(
        verified.status.success(),
        "verify after restart: {}",
        stderr_of(&verified)
    );

    let kept = access_log(&paserk);
    assert!(
        kept.starts_with(at_stop.as_str()),
        "the log was appended to, not replaced"
    );
    assert_eq!(
        kept.lines().count(),
        at_stop.lines().count() + 2,
        "plus the two reads after the restart: {kept}"
    );
    eprintln!("ingress: restart kept the record, its chain ({before}) and the log");
}

/// Runs an operator task inside the running container, as an operator would
/// (`podman compose exec`), against the deployment's own database file.
fn operator_task(public_key: &str, task: &str, container: &str) -> std::process::Output {
    Command::new("podman")
        .arg("compose")
        .args([
            "-p",
            PROJECT,
            "exec",
            "-T",
            "openehr-axa",
            "openehr-axa",
            "task",
            task,
        ])
        .arg(format!("container:{container}"))
        .arg("path:/home/openehr/data/openehr.sqlite3")
        .current_dir(repository_root())
        .env("OPENEHR_PASETO_PUBLIC_KEYS", public_key)
        .output()
        .expect("podman runs")
}

fn access_log(public_key: &str) -> String {
    let out = Command::new("podman")
        .arg("compose")
        .args([
            "-p",
            PROJECT,
            "exec",
            "-T",
            "openehr-axa",
            "cat",
            "/home/openehr/data/access.jsonl",
        ])
        .current_dir(repository_root())
        .env("OPENEHR_PASETO_PUBLIC_KEYS", public_key)
        .output()
        .expect("podman runs");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr_of(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// Ready when metadata answers. It is the one route served without a token.
fn wait_ready() {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let (status, _) = call("GET", "/metadata", "", None);
        if status == 200 {
            break;
        }
        assert!(Instant::now() < deadline, "the service never became ready");
        std::thread::sleep(Duration::from_millis(500));
    }
}
