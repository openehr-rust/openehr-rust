//! A restore drill (`db:O10.6`, `db:O10.19`, `db:O10.20`, `tasks.md` triage
//! item 5): back up a live store, keep a witness outside it, go on writing,
//! restore the backup in isolation, and check it against the witness.
//!
//! What this establishes, and only this:
//!
//! - SQLite's own online snapshot (`VACUUM INTO`, a consistent copy taken while
//!   the source stays open for writes; <https://www.sqlite.org/lang_vacuum.html>)
//!   is a usable backup of this store. `O10.6`: the engine's own mechanism,
//!   no bespoke format.
//! - The restored copy opens read-only, without being installed into, and
//!   answers a historical read with the version that was current then.
//! - Its chain re-verifies from the stored content (`Unkeyed`, intact), and
//!   its checkpoint is byte-identical to the witness taken at backup time.
//! - The commit made *after* the backup is absent, so the backup is a
//!   **prefix** of the history (`O10.19`). "Last recoverable commit" is the
//!   witness's `last_version`.
//!
//! It measures elapsed time for the record and asserts nothing about it
//! (`W0.35`). Not covered: a large database, PostgreSQL's `pg_dump`/PITR, or
//! a restore across schema versions (refused by `O10.15`).

use openehr::base::HierObjectId;
use openehr::rm::data_types::DvDateTime;
use openehr_sqlite::SqliteStore;
use openehr_store::{Store as _, conformance, integrity};
use std::path::PathBuf;
use std::time::Instant;

const CONTRIBUTION: &str = "22222222-3333-4444-5555-666666666666";

fn temp(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after the epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("openehr-restore-{tag}-{nanos}.sqlite3"));
    let _ = std::fs::remove_file(&path);
    path
}

#[test]
fn a_backup_restores_in_isolation_as_a_verifiable_prefix_of_the_history() {
    let live_path = temp("live");
    let backup_path = temp("backup");
    let container = HierObjectId::from_uid_str(conformance::RECORD).expect("literal");

    // A live store with three versions of one composition.
    let mut live = SqliteStore::open(&live_path).expect("open");
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

    // The witness, kept outside the database (here: a variable; in operation,
    // somewhere the database administrator does not control, `db:M3.16c`).
    let witness = live.chain_checkpoint(&container).expect("checkpoint");

    // The backup, taken while the store stays open, then a commit after it.
    let started = Instant::now();
    live.connection()
        .execute(
            "VACUUM INTO ?1",
            [backup_path.to_str().expect("utf-8 temp path")],
        )
        .expect("online snapshot");
    let backup_elapsed = started.elapsed();
    live.commit_composition(
        &ehr_id,
        &conformance::sample_version(4, Some(3), 20),
        CONTRIBUTION,
    )
    .expect("a commit after the backup");
    drop(live);
    std::fs::remove_file(&live_path).expect("the live database is gone");

    // Restore in isolation: the backup alone, read-only, nothing installed.
    let started = Instant::now();
    let before = std::fs::read(&backup_path).expect("backup bytes");
    let restored = SqliteStore::open_read_only(&backup_path).expect("restore opens read-only");
    let rows = restored.all_versions(&container).expect("history");
    let verdict = integrity::verify_versions(&rows, &[]);
    let checkpoint = restored.chain_checkpoint(&container).expect("checkpoint");
    let at_ten = restored
        .version_at_time(
            &container,
            &DvDateTime::new("2026-08-01T09:12:00Z").expect("literal"),
        )
        .expect("historical read");
    let restore_elapsed = started.elapsed();
    drop(restored);

    // The prefix: exactly the three versions the witness saw.
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
    assert!(at_ten.uid.ends_with("::2"), "{}", at_ten.uid);
    assert!(
        at_ten
            .data_json
            .as_deref()
            .is_some_and(|j| j.contains("Encounter")),
        "the historical version carries its content"
    );
    // Inspecting the backup did not change it.
    assert_eq!(std::fs::read(&backup_path).expect("backup bytes"), before);

    // For the record, not a gate (`W0.35`).
    eprintln!(
        "restore drill: backup {backup_elapsed:?}, restore and verify {restore_elapsed:?}, \
         last recoverable commit {}",
        witness.rsplit(' ').next().unwrap_or_default()
    );
    let _ = std::fs::remove_file(&backup_path);
}
