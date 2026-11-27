//! Contention measurements for `openehr-axa` (`db:P6.7`, `tasks.md` triage
//! item 7): request latency, throughput, health responsiveness and resident
//! memory at increasing chart sizes and client counts.
//!
//! **Run, never gated** (`W0.35`): it prints a table and asserts only that
//! every request succeeded. Numbers belong to the machine and the build that
//! produced them, and are recorded with both in `spec/conformance-runs.md`.
//!
//! ```sh
//! cargo run --release --example contention
//! ```
//!
//! The router is driven in process (`tower::ServiceExt::oneshot`), as
//! `benches/http.rs` does, on a multi-thread runtime with four workers. One
//! chart holds one composition amended `size - 1` times, so its history has
//! `size` versions, its archetype search `size` index rows, and an AQL query
//! `size` candidates. Each client sends the same mix in turn: a history page
//! (`_count=20`, from the middle), a search page, and an AQL query
//! (`LIMIT 10`). Beside them a probe asks `GET /metadata` every 5 ms. It is
//! the one route that takes no store lock, so its latency shows whether store
//! work holding the runtime's worker threads delays requests that need no store.
//!
//! \`health*\` is the probe's **lateness**: from when it was due to its answer.
//!
//! Not measured: rows read inside the engine, a network socket, a file-backed
//! database (the store is `SqliteStore::in_memory`), or read auditing.

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, header},
};
use openehr_axa::{
    access::{AccessLog, SharedAccessLog},
    app::SharedOpenehrStore,
    auth::{PasetoVerifier, SharedVerifier},
};
use openehr_sqlite::SqliteStore;
use openehr_store::{Store as _, conformance};
use pasetors::{
    claims::Claims,
    keys::{AsymmetricKeyPair, Generate as _},
    paserk::FormatAsPaserk as _,
    public,
    version4::V4,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tower::ServiceExt as _;

const CONTRIBUTION: &str = "22222222-3333-4444-5555-666666666666";
const REQUESTS_PER_CLIENT: usize = 30;

fn served(size: u32) -> (Router, String, String) {
    let pair = AsymmetricKeyPair::<V4>::generate().expect("keypair");
    let mut claims = Claims::new().expect("claims");
    claims.subject("clinician-4417").expect("subject");
    claims
        .add_additional(
            openehr_axa::authz::GRANTS_CLAIM,
            serde_json::json!([{"ehr": "*", "actions": ["read"]}]),
        )
        .expect("claim");
    let token = public::sign(&pair.secret, &claims, None, None).expect("signs");

    let mut store = SqliteStore::in_memory().expect("store");
    store.install().expect("install");
    let ehr = conformance::sample_ehr();
    store.create_ehr(&ehr).expect("ehr");
    let ehr_id = ehr.ehr_id().clone();
    let versions: Vec<u32> = (1..=size).collect();
    store
        .create_contribution(
            &ehr_id,
            &conformance::sample_contribution(CONTRIBUTION, &versions),
        )
        .expect("contribution");
    for n in 1..=size {
        store
            .commit_composition(
                &ehr_id,
                &conformance::sample_version(n, (n > 1).then(|| n - 1), n % 60),
                CONTRIBUTION,
            )
            .expect("commit");
    }

    let ctx = axa::AppContext::for_test(()).expect("test context");
    let mut paserk = String::new();
    pair.public.fmt(&mut paserk).expect("PASERK");
    ctx.shared_store.insert::<SharedVerifier>(Arc::new(
        PasetoVerifier::new(&paserk, None, None, None).expect("verifier"),
    ));
    ctx.shared_store
        .insert::<SharedOpenehrStore>(Arc::new(Mutex::new(store)));
    ctx.shared_store
        .insert::<SharedAccessLog>(Arc::new(AccessLog::off()));
    let router = openehr_axa::app::routes()
        .into_app(ctx.clone())
        .expect("router");
    (router, format!("Bearer {token}"), ehr_id.to_string())
}

#[derive(Clone, Copy)]
enum Kind {
    History,
    Search,
    Aql,
}

fn request(kind: Kind, size: u32, ehr_id: &str, authorization: &str) -> Request<Body> {
    let base = format!("/openehr/v1/ehr/{ehr_id}/composition");
    let builder = Request::builder().header(header::AUTHORIZATION, authorization);
    match kind {
        Kind::History => builder
            .uri(format!(
                "{base}/{}/_history?_count=20&_offset={}",
                conformance::RECORD,
                size / 2
            ))
            .body(Body::empty()),
        Kind::Search => builder
            .uri(format!(
                "{base}?archetype_id=openEHR-EHR-COMPOSITION.encounter.v1&_count=20&_offset={}",
                size / 2
            ))
            .body(Body::empty()),
        Kind::Aql => builder
            .method(Method::POST)
            .uri("/openehr/v1/query/aql")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::json!({
                    "q": "SELECT c/name/value FROM EHR e[ehr_id/value=$ehrUid] \
                          CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1] LIMIT 10",
                    "query_parameters": {"ehrUid": ehr_id},
                })
                .to_string(),
            )),
    }
    .expect("request")
}

async fn timed(router: &Router, request: Request<Body>) -> Duration {
    let started = Instant::now();
    let response = router.clone().oneshot(request).await.expect("infallible");
    let status = response.status();
    let body = to_bytes(response.into_body(), 1 << 24).await.expect("body");
    assert!(
        status.is_success(),
        "{status}: {}",
        String::from_utf8_lossy(&body)
    );
    started.elapsed()
}

fn percentile(sorted: &[Duration], p: f64) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let i = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[i]
}

fn row(label: &str, mut samples: Vec<Duration>) {
    samples.sort();
    println!(
        "  {label:<8} n={:<5} p50={:>9.2?} p99={:>9.2?} max={:>9.2?}",
        samples.len(),
        percentile(&samples, 0.50),
        percentile(&samples, 0.99),
        samples.last().copied().unwrap_or_default()
    );
}

fn resident_kib() -> String {
    std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .map_or_else(
            |_| String::new(),
            |o| String::from_utf8_lossy(&o.stdout).trim().to_owned(),
        )
}

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .expect("runtime");
    println!(
        "contention: {REQUESTS_PER_CLIENT} requests per client, 4 runtime workers, in-memory SQLite"
    );
    for size in [100_u32, 1_000, 5_000] {
        let seeded = Instant::now();
        let (router, authorization, ehr_id) = served(size);
        println!(
            "\nchart of {size} versions (seeded in {:.2?}; RSS {} KiB)",
            seeded.elapsed(),
            resident_kib()
        );
        for clients in [1_usize, 4, 16] {
            let (per_kind, health, wall) = runtime.block_on(async {
                let done = Arc::new(AtomicBool::new(false));
                let probe = {
                    let router = router.clone();
                    let done = done.clone();
                    tokio::spawn(async move {
                        // Lateness, not latency: from the moment the probe was
                        // due (5 ms after its last answer) to its answer. A
                        // probe that is never scheduled has no latency to
                        // measure, and is exactly what a health check misses.
                        let mut samples = Vec::new();
                        let mut due = Instant::now();
                        while !done.load(Ordering::Relaxed) {
                            let request = Request::builder()
                                .uri("/openehr/v1/metadata")
                                .body(Body::empty())
                                .expect("request");
                            timed(&router, request).await;
                            samples.push(due.elapsed());
                            due = Instant::now() + Duration::from_millis(5);
                            tokio::time::sleep_until(due.into()).await;
                        }
                        samples
                    })
                };
                let started = Instant::now();
                let workers: Vec<_> = (0..clients)
                    .map(|c| {
                        let router = router.clone();
                        let authorization = authorization.clone();
                        let ehr_id = ehr_id.clone();
                        tokio::spawn(async move {
                            let mut samples: [Vec<Duration>; 3] = Default::default();
                            for i in 0..REQUESTS_PER_CLIENT {
                                let k = (c + i) % 3;
                                let kind = [Kind::History, Kind::Search, Kind::Aql][k];
                                samples[k].push(
                                    timed(&router, request(kind, size, &ehr_id, &authorization))
                                        .await,
                                );
                            }
                            samples
                        })
                    })
                    .collect();
                let mut per_kind: [Vec<Duration>; 3] = Default::default();
                for worker in workers {
                    for (all, mine) in per_kind.iter_mut().zip(worker.await.expect("client")) {
                        all.extend(mine);
                    }
                }
                let wall = started.elapsed();
                done.store(true, Ordering::Relaxed);
                (per_kind, probe.await.expect("probe"), wall)
            });
            let total = per_kind
                .iter()
                .try_fold(0_usize, |n, v| n.checked_add(v.len()))
                .expect("a request count fits usize");
            #[allow(clippy::cast_precision_loss)]
            let rate = total as f64 / wall.as_secs_f64();
            println!(
                " {clients:>2} clients: {total} requests in {wall:.2?} ({rate:.0}/s), RSS {} KiB",
                resident_kib()
            );
            let [history, search, aql] = per_kind;
            row("history", history);
            row("search", search);
            row("aql", aql);
            row("health*", health);
        }
    }
}
