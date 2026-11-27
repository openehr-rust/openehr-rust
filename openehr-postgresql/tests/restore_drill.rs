//! A restore drill for PostgreSQL (`db:O10.6`, `db:O10.19`, `tasks.md` triage
//! item 5), the same drill `openehr-sqlite/tests/restore_drill.rs` runs: back
//! up a live store, keep a witness outside it, go on writing, restore the
//! backup in isolation, and check it against the witness.
//!
//! The backup is the engine's own logical dump, `pg_dump -Fc`, taken while the
//! store stays connected (`pg_dump` reads one consistent snapshot;
//! <https://www.postgresql.org/docs/18/app-pgdump.html>). `O10.6`: the
//! engine's own mechanism, no bespoke format. The restore is `pg_restore`
//! into a database that did not exist before, after the source database is
//! dropped, and it is opened with `default_transaction_read_only`, so the
//! checks cannot have written to it.
//!
//! What it establishes: the restored history is exactly the three versions
//! the witness saw (a **prefix**, `O10.19`); its chain re-verifies from the
//! stored content; its checkpoint is byte-identical to the witness; an as-of
//! read returns the version current then; the recorded schema version is
//! this build's. Not covered: point-in-time recovery from WAL, a large
//! database, a restore across schema versions (refused by `O10.15`). Elapsed
//! times are printed, never asserted (`W0.35`).
//!
//! `#[ignore]`d: it needs the server `scripts/verify-store.sh` provisions, and
//! that server's container, through which `pg_dump` and `pg_restore` run.

use openehr::base::HierObjectId;
use openehr::rm::data_types::DvDateTime;
use openehr_postgresql::PostgresqlStore;
use openehr_store::{Store as _, conformance, integrity};
use std::process::Command;
use std::time::Instant;

const CONTRIBUTION: &str = "22222222-3333-4444-5555-666666666666";
const SOURCE: &str = "openehr_drill_source";
const RESTORED: &str = "openehr_drill_restored";
const DUMP: &str = "/tmp/openehr-drill.dump";

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("set {name} or run scripts/verify-store.sh"))
}

/// The connection string for `database`, from the one `verify-store.sh` sets,
/// with `extra` appended.
fn params(database: &str, extra: &str) -> String {
    let base = env("OPENEHR_POSTGRESQL_URL");
    let replaced = base.replace("dbname=openehr", &format!("dbname={database}"));
    assert_ne!(replaced, base, "the URL names dbname=openehr");
    format!("{replaced} {extra}")
}

fn connect(database: &str, extra: &str) -> PostgresqlStore {
    let ca = std::fs::read(env("OPENEHR_POSTGRESQL_CA")).expect("read test CA");
    PostgresqlStore::connect_with_ca(&params(database, extra), &ca).expect("verified TLS connect")
}

/// Runs a PostgreSQL client tool inside the server's container, as an
/// operator's backup job would run it beside the server.
fn tool(args: &[&str]) {
    let container = env("OPENEHR_POSTGRESQL_CONTAINER");
    let runtime = std::env::var("CONTAINER").unwrap_or_else(|_| "podman".to_owned());
    let out = Command::new(runtime)
        .args([
            "exec",
            "-e",
            &format!("PGPASSWORD={}", env("OPENEHR_POSTGRESQL_PASSWORD")),
        ])
        .arg(container)
        .args(args)
        .output()
        .expect("the container runtime runs");
    assert!(
        out.status.success(),
        "{args:?}: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn psql(sql: &str) {
    tool(&[
        "psql",
        "-h",
        "127.0.0.1",
        "-U",
        "postgres",
        "-d",
        "openehr",
        "-v",
        "ON_ERROR_STOP=1",
        "-c",
        sql,
    ]);
}

// Long on purpose, like the SQLite drill: one drill, in the order an operator
// runs it, and splitting it would split the witness from what it witnesses.
#[allow(clippy::too_many_lines)]
#[test]
#[ignore = "needs the server and container scripts/verify-store.sh provisions"]
fn a_dump_restores_in_isolation_as_a_verifiable_prefix_of_the_history() {
    let container = HierObjectId::from_uid_str(conformance::RECORD).expect("literal");
    psql(&format!("DROP DATABASE IF EXISTS {SOURCE} WITH (FORCE)"));
    psql(&format!("DROP DATABASE IF EXISTS {RESTORED} WITH (FORCE)"));
    psql(&format!("CREATE DATABASE {SOURCE}"));

    // A live store with three versions of one composition.
    let mut live = connect(SOURCE, "");
    live.install().expect("install");
    let ehr = conformance::sample_ehr();
    let ehr_id = ehr.ehr_id().clone();
    live.create_ehr(&ehr).expect("ehr");
    live.create_contribution(
        &ehr_id,
        &conformance::sample_contribution(CONTRIBUTION, &[1, 2, 3, 4]),
    )
    .expect("contribution");
    for (n, preceding) in [(1, None), (2, Some(1)), (3, Some(2))] {
        live.commit_composition(
            &ehr_id,
            &conformance::sample_version(n, preceding, n * 5),
            CONTRIBUTION,
        )
        .expect("commit");
    }

    // The witness, kept outside the database (`db:M3.16c`).
    let witness = live.chain_checkpoint(&container).expect("checkpoint");

    // The dump, taken while the store stays connected, then a commit after it.
    let started = Instant::now();
    tool(&[
        "pg_dump",
        "-h",
        "127.0.0.1",
        "-U",
        "postgres",
        "-Fc",
        "-d",
        SOURCE,
        "-f",
        DUMP,
    ]);
    let backup_elapsed = started.elapsed();
    live.commit_composition(
        &ehr_id,
        &conformance::sample_version(4, Some(3), 20),
        CONTRIBUTION,
    )
    .expect("a commit after the backup");
    assert_eq!(live.all_versions(&container).expect("history").len(), 4);
    drop(live);
    psql(&format!("DROP DATABASE {SOURCE} WITH (FORCE)"));

    // Restore in isolation: a new database, the dump alone, opened read-only.
    let started = Instant::now();
    psql(&format!("CREATE DATABASE {RESTORED}"));
    tool(&[
        "pg_restore",
        "-h",
        "127.0.0.1",
        "-U",
        "postgres",
        "--exit-on-error",
        "-d",
        RESTORED,
        DUMP,
    ]);
    let mut restored = connect(RESTORED, "options='-c default_transaction_read_only=on'");
    let rows = restored.all_versions(&container).expect("history");
    let verdict = integrity::verify_versions(&rows, &[]);
    let checkpoint = restored.chain_checkpoint(&container).expect("checkpoint");
    let at_twelve = restored
        .version_at_time(
            &container,
            &DvDateTime::new("2026-08-01T09:12:00Z").expect("literal"),
        )
        .expect("historical read");
    let recorded: i64 = restored
        .client()
        .query_one("SELECT max(version)::bigint FROM openehr_schema_version", &[])
        .expect("schema version")
        .get(0);
    // The session really is read-only: a write is refused by the server.
    let write = restored
        .client()
        .batch_execute("CREATE TABLE drill_probe (x int)");
    let restore_elapsed = started.elapsed();
    drop(restored);
    psql(&format!("DROP DATABASE {RESTORED} WITH (FORCE)"));

    assert_eq!(rows.len(), 3, "the post-backup commit must be absent");
    assert!(verdict.is_intact(), "{verdict:?}");
    assert_eq!(
        checkpoint, witness,
        "the restored chain must match the witness byte for byte"
    );
    assert!(
        witness.ends_with(&format!(
            "{}::{}::3",
            conformance::RECORD,
            conformance::SYSTEM
        )),
        "last recoverable commit: {witness}"
    );
    // 09:12 lies between version 2 (09:10) and version 3 (09:15).
    assert!(at_twelve.uid.ends_with("::2"), "{}", at_twelve.uid);
    assert!(
        at_twelve
            .data_json
            .as_deref()
            .is_some_and(|j| j.contains("Encounter")),
        "the historical version carries its content"
    );
    assert_eq!(
        recorded,
        openehr_store::schema::SCHEMA_VERSION,
        "the restored database records this build's schema version"
    );
    assert!(write.is_err(), "the restored session must be read-only");

    eprintln!(
        "restore drill (PostgreSQL): pg_dump {backup_elapsed:?}, restore and verify \
         {restore_elapsed:?}, last recoverable commit {}",
        witness.rsplit(' ').next().unwrap_or_default()
    );
}
