//! The operator tasks, run.
//!
//! `Checkpoint::run` was an empty body returning `Ok(())` for as long as the
//! crate existed, while its doc comment and the README described a checkpoint
//! being printed. Nothing caught it, because nothing ran it — the same shape as
//! the missing binary next door in `src/main.rs`.
//!
//! These tests exist so that "the task prints a checkpoint" and "the task fails
//! on a tampered history" are statements about behaviour rather than about
//! intent.

use loco_rs::{
    app::AppContext,
    environment::Environment,
    task::{Task as _, Vars},
};
use openehr_loco::{
    app::open_store_at,
    tasks::{Checkpoint, Verify, checkpoint_line, verify_line},
};
use openehr_sqlite::rusqlite::Connection;
use openehr_store::{Store as _, conformance};
use std::path::{Path, PathBuf};

const CONTRIBUTION: &str = "22222222-3333-4444-5555-666666666666";

fn temp_db(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after the epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("openehr-loco-task-{tag}-{nanos}.sqlite3"));
    let _ = std::fs::remove_file(&path);
    path
}

/// Two committed versions of one composition.
fn seed(path: &Path) {
    let mut store = open_store_at(path).expect("open");
    let ehr = conformance::sample_ehr();
    let ehr_id = ehr.ehr_id().clone();
    store.create_ehr(&ehr).expect("ehr");
    store
        .create_contribution(
            &ehr_id,
            &conformance::sample_contribution(CONTRIBUTION, &[1, 2]),
        )
        .expect("contribution");
    for (n, preceding) in [(1, None), (2, Some(1))] {
        store
            .commit_composition(
                &ehr_id,
                &conformance::sample_version(n, preceding, n * 5),
                CONTRIBUTION,
            )
            .expect("commit");
    }
}

fn vars(path: &Path) -> Vars {
    Vars::from_cli_args(vec![
        ("container".to_owned(), conformance::RECORD.to_owned()),
        ("path".to_owned(), path.display().to_string()),
    ])
}

/// A context with nothing in its shared store.
///
/// Deliberately empty: it is what `cli::main` hands a task, because
/// `before_run` is not on that path. A task that reached into `shared_store`
/// would fail here, which is the point of building the context this way rather
/// than the way the HTTP tests do.
fn bare_context() -> AppContext {
    AppContext::builder(Environment::Test, loco_rs::tests_cfg::config::test_config()).build()
}

#[tokio::test]
async fn the_checkpoint_task_runs_without_before_run_having_run() {
    let path = temp_db("checkpoint");
    seed(&path);

    // Both: that `run` completes, and that what it would print is a real
    // checkpoint. Asserting only the first would have passed against the empty
    // body this task had until now — which is the defect, not a check for it.
    Checkpoint
        .run(&bare_context(), &vars(&path))
        .await
        .expect("checkpoint");

    let line = checkpoint_line(&vars(&path)).expect("checkpoint");
    assert!(line.starts_with("entries=2 "), "{line}");
    assert!(
        line.contains("::2"),
        "the head version must be named: {line}"
    );
    // A checkpoint goes somewhere clinical data may not (`db:M3.16c`), so it
    // must carry none.
    assert!(!line.contains("Encounter"), "{line}");

    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn the_verify_task_passes_on_an_untouched_history() {
    let path = temp_db("verify-ok");
    seed(&path);

    Verify
        .run(&bare_context(), &vars(&path))
        .await
        .expect("verify");

    let line = verify_line(&vars(&path)).expect("verify");
    // Unkeyed, never Verified, and the line must say why that is weaker.
    assert!(line.contains("Unkeyed"), "{line}");
    assert!(line.contains("unsigned"), "{line}");

    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn the_verify_task_fails_on_a_tampered_history() {
    let path = temp_db("verify-bad");
    seed(&path);

    // The threat, not a defeat of it: append-only triggers stop the database's
    // own UPDATE path and say nothing about file access (`db:PR12.11`).
    let connection = Connection::open(&path).expect("second connection");
    connection
        .execute_batch(
            "DROP TRIGGER IF EXISTS trg_openehr_version_no_update;
             UPDATE openehr_version
                SET data_json = replace(data_json, 'Encounter 2', 'Encounter X')
              WHERE uid LIKE '%::2';",
        )
        .expect("tamper");
    drop(connection);

    let error = Verify
        .run(&bare_context(), &vars(&path))
        .await
        .expect_err("a tampered history must fail the task");

    // The message has to name the breach, or a cron job's mail is a mystery.
    let message = error.to_string();
    assert!(message.contains("NOT intact"), "{message}");
    assert!(message.contains("ContentAltered"), "{message}");

    let _ = std::fs::remove_file(&path);
}

/// Runs the **real binary**, with the real CLI, and returns (success, output).
///
/// The in-process tests below call `run` and the extracted line functions.
/// Neither catches a `run` whose body was emptied — which is precisely the
/// defect this file was written for, and a first attempt at these tests missed
/// it: gutting `Checkpoint::run` left every one of them green, because they
/// asserted on `checkpoint_line` rather than on what the task printed.
///
/// Only executing the binary and reading its stdout closes that, and it covers
/// the rest of the path nothing else touches: `src/main.rs`, `config/`,
/// argument parsing, and task registration.
fn run_binary(args: &[&str]) -> (bool, String) {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_openehr-loco"))
        .args(args)
        .output()
        .expect("the binary runs");
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.success(), text)
}

#[test]
fn the_binary_prints_a_checkpoint() {
    let path = temp_db("bin-checkpoint");
    seed(&path);

    let (ok, out) = run_binary(&[
        "task",
        "checkpoint",
        &format!("container:{}", conformance::RECORD),
        &format!("path:{}", path.display()),
    ]);

    assert!(ok, "the task failed: {out}");
    assert!(out.contains("entries=2 "), "nothing was printed: {out}");
    assert!(
        !out.contains("Encounter"),
        "clinical content in a checkpoint: {out}"
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_binary_exits_non_zero_on_a_tampered_history() {
    let path = temp_db("bin-verify");
    seed(&path);
    let connection = Connection::open(&path).expect("second connection");
    connection
        .execute_batch(
            "DROP TRIGGER IF EXISTS trg_openehr_version_no_update;
             UPDATE openehr_version
                SET data_json = replace(data_json, 'Encounter 2', 'Encounter X')
              WHERE uid LIKE '%::2';",
        )
        .expect("tamper");
    drop(connection);

    let (ok, out) = run_binary(&[
        "task",
        "verify",
        &format!("container:{}", conformance::RECORD),
        &format!("path:{}", path.display()),
    ]);

    // A non-zero exit is the whole point. A nightly sweep reads the status,
    // not the prose.
    assert!(!ok, "a tampered history exited zero: {out}");
    assert!(out.contains("ContentAltered"), "{out}");

    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn verifying_a_container_that_does_not_exist_is_not_a_pass() {
    let path = temp_db("empty");
    seed(&path);

    // `Empty` is not a breach and it is not a pass. A sweep that verified the
    // wrong identifier — a typo, a decommissioned record — must not report
    // success for having checked nothing.
    let error = Verify
        .run(
            &bare_context(),
            &Vars::from_cli_args(vec![
                (
                    "container".to_owned(),
                    "6BA7B810-9DAD-11D1-80B4-00C04FD430C8".to_owned(),
                ),
                ("path".to_owned(), path.display().to_string()),
            ]),
        )
        .await
        .expect_err("checking nothing must not report success");
    assert!(error.to_string().contains("Empty"), "{error}");

    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn a_task_without_a_container_says_what_it_wanted() {
    let path = temp_db("no-container");
    seed(&path);

    let error = Verify
        .run(
            &bare_context(),
            &Vars::from_cli_args(vec![("path".to_owned(), path.display().to_string())]),
        )
        .await
        .expect_err("no container");
    assert!(error.to_string().contains("container:"), "{error}");

    let _ = std::fs::remove_file(&path);
}
