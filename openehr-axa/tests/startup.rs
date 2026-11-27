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
