//! Startup refuses a protection setting it cannot honour (`db:PR12.24`,
//! `db:D-30`). This runs the real binary, as a deployment does, because
//! "refuses to start" is a claim about the process: its exit status, and that
//! it never reaches the point of serving.

use pasetors::{
    keys::{AsymmetricKeyPair, Generate as _},
    paserk::FormatAsPaserk as _,
    version4::V4,
};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn paserk() -> String {
    let pair = AsymmetricKeyPair::<V4>::generate().expect("keypair");
    let mut out = String::new();
    pair.public.fmt(&mut out).expect("PASERK");
    out
}

/// Starts the service with `env` added and waits up to 20 s. Returns the exit
/// status and output, or `None` if it was still running (and so serving), in
/// which case it is killed.
fn start_with(env: &[(&str, std::ffi::OsString)]) -> Option<(bool, String)> {
    let db = std::env::temp_dir().join(format!(
        "openehr-axa-startup-{}-{}.sqlite3",
        std::process::id(),
        env.len()
    ));
    let mut command = Command::new(env!("CARGO_BIN_EXE_openehr-axa"));
    command
        .args(["start", "--port", "0"])
        .env("OPENEHR_SQLITE_PATH", &db)
        .env("OPENEHR_PASETO_PUBLIC_KEYS", paserk())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (name, value) in env {
        command.env(name, value);
    }
    let mut child = command.spawn().expect("the binary runs");
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(status) = child.try_wait().expect("wait") {
            let output = child.wait_with_output().expect("output");
            let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
            text.push_str(&String::from_utf8_lossy(&output.stderr));
            let _ = std::fs::remove_file(&db);
            return Some((status.success(), text));
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_file(&db);
            return None;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn an_empty_access_log_setting_refuses_startup() {
    // Every read error used to mean "auditing off".
    let (ok, out) = start_with(&[("OPENEHR_ACCESS_LOG", "".into())])
        .expect("an empty access log must refuse startup, not serve");
    assert!(!ok, "{out}");
    assert!(out.contains("OPENEHR_ACCESS_LOG"), "{out}");
}

#[cfg(unix)]
#[test]
fn a_claim_setting_that_is_not_unicode_refuses_startup() {
    use std::os::unix::ffi::OsStringExt as _;
    let (ok, out) = start_with(&[(
        "OPENEHR_PASETO_AUDIENCE",
        std::ffi::OsString::from_vec(vec![0x61, 0xff]),
    )])
    .expect("an unreadable audience must refuse startup, not serve without one");
    assert!(!ok, "{out}");
    assert!(out.contains("OPENEHR_PASETO_AUDIENCE"), "{out}");
}

#[test]
fn an_unusable_system_id_refuses_startup() {
    // It would be minted into every new identifier, and one that does not
    // parse back cannot be read again (`db:S1.22`).
    let (ok, out) = start_with(&[("OPENEHR_SYSTEM_ID", "two words".into())])
        .expect("an unusable system id must refuse startup, not serve");
    assert!(!ok, "{out}");
    assert!(out.contains("OPENEHR_SYSTEM_ID"), "{out}");
}

#[test]
fn an_access_log_that_cannot_be_appended_to_refuses_startup() {
    // A directory: configured, and impossible to write. Not "auditing off".
    let (ok, out) = start_with(&[("OPENEHR_ACCESS_LOG", std::env::temp_dir().into())])
        .expect("an unwritable access log must refuse startup, not serve");
    assert!(!ok, "{out}");
    assert!(out.contains("cannot be appended to"), "{out}");
}

#[test]
fn a_database_file_that_is_not_a_database_refuses_startup() {
    let path = std::env::temp_dir().join(format!(
        "openehr-axa-startup-{}-not-a-database",
        std::process::id()
    ));
    std::fs::write(
        &path,
        b"this is not an SQLite database, and is long enough to say so",
    )
    .expect("fixture");
    let outcome = start_with(&[("OPENEHR_SQLITE_PATH", path.clone().into())]);
    let bytes = std::fs::read(&path).expect("still there");
    let _ = std::fs::remove_file(&path);
    let (ok, out) = outcome.expect("an unreadable database must refuse startup, not serve");
    assert!(!ok, "{out}");
    assert_eq!(
        bytes, b"this is not an SQLite database, and is long enough to say so",
        "the file it refused is left as it was"
    );
}

#[cfg(unix)]
#[test]
fn a_database_path_that_is_not_unicode_refuses_startup() {
    use std::os::unix::ffi::OsStringExt as _;
    let (ok, out) = start_with(&[(
        "OPENEHR_SQLITE_PATH",
        std::ffi::OsString::from_vec(vec![0x61, 0xff]),
    )])
    .expect("an unreadable database path must refuse startup, not open another");
    assert!(!ok, "{out}");
    assert!(out.contains("OPENEHR_SQLITE_PATH"), "{out}");
}

/// The positive twin: correctly configured, it starts, serves, and stops
/// cleanly when asked (`SIGTERM`, which is what a container runtime sends).
#[cfg(unix)]
#[test]
fn a_configured_service_starts_and_stops_on_sigterm() {
    let db = std::env::temp_dir().join(format!(
        "openehr-axa-startup-{}-serving.sqlite3",
        std::process::id()
    ));
    let mut child = Command::new(env!("CARGO_BIN_EXE_openehr-axa"))
        .args(["start", "--port", "0"])
        .env("OPENEHR_SQLITE_PATH", &db)
        .env("OPENEHR_PASETO_PUBLIC_KEYS", paserk())
        .env_remove("OPENEHR_ACCESS_LOG")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary runs");
    // Serving once the store exists: `prepare` installs it before listening.
    let deadline = Instant::now() + Duration::from_secs(20);
    while !db.exists() {
        assert!(
            child.try_wait().expect("wait").is_none(),
            "it exited instead of serving"
        );
        assert!(Instant::now() < deadline, "it never opened its store");
        std::thread::sleep(Duration::from_millis(50));
    }
    std::thread::sleep(Duration::from_millis(500));
    assert!(
        child.try_wait().expect("wait").is_none(),
        "it stopped by itself"
    );
    let killed = Command::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .status()
        .expect("kill runs");
    assert!(killed.success());
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        assert!(Instant::now() < deadline, "SIGTERM did not stop it");
        std::thread::sleep(Duration::from_millis(50));
    };
    let output = child.wait_with_output().expect("output");
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", db.display()));
    }
    assert!(
        status.success(),
        "{status}: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Axa's own database commands have nothing to act on here, and must say so
/// rather than succeed: `db reset` is "drop everything and reapply", and an
/// operator who typed it against a clinical store has to be told it did
/// nothing, by exit status as well as text (`db:O10.14`).
#[test]
fn axas_database_commands_refuse_and_leave_the_store_alone() {
    let db = std::env::temp_dir().join(format!(
        "openehr-axa-startup-{}-db-commands.sqlite3",
        std::process::id()
    ));
    for command in [["db", "migrate"], ["db", "reset"], ["db", "truncate"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_openehr-axa"))
            .args(command)
            .env("OPENEHR_SQLITE_PATH", &db)
            .env("OPENEHR_PASETO_PUBLIC_KEYS", paserk())
            // The case that would get past Axa's own guard on destructive
            // commands: a development shell that has opted in. This service
            // must still refuse, since the store is not Axa's to drop.
            .env("APP_ENV", "development")
            .env("AXA_DESTROY_DATA", "true")
            .output()
            .expect("the binary runs");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!output.status.success(), "{command:?} succeeded: {text}");
        assert!(text.contains("no database of Axa's"), "{command:?}: {text}");
        assert!(!db.exists(), "{command:?} created the store it refused");
    }
}

/// `check` is answered before anything is opened, so it works with no store
/// and creates none.
#[test]
fn check_runs_without_opening_the_store() {
    let db = std::env::temp_dir().join(format!(
        "openehr-axa-startup-{}-check.sqlite3",
        std::process::id()
    ));
    let output = Command::new(env!("CARGO_BIN_EXE_openehr-axa"))
        .arg("check")
        .env("OPENEHR_SQLITE_PATH", &db)
        .output()
        .expect("the binary runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!db.exists(), "check created a store");
}
