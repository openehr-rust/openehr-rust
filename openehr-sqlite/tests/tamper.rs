//! Altering a stored row, and watching the store notice.
//!
//! # Why this test is the point
//!
//! `PR12.12` has said since it was written that no crate may describe its
//! storage as tamper-evident until **a test alters a row and shows the store
//! rejects it**. Everything before this demonstrated that the chain is written
//! correctly — genesis is zero, each link matches, no digest is empty. That is
//! a claim about the writer. A well-formed chain over rows nobody has tried to
//! corrupt says nothing about whether anything would notice if they had.
//!
//! # The attacker model, and why the triggers are dropped
//!
//! Append-only enforcement (`M3.17`) is not tamper evidence and `PR12.11` says
//! so. The triggers stop the database's own `UPDATE` and `DELETE` paths. They
//! say nothing about someone with file access — and in `SQLite` a trigger is a
//! row in `sqlite_master`, not a law of physics.
//!
//! So these tests open a **second connection to the file**, drop the triggers,
//! and edit the row. That is not cheating past the defence; it is the threat
//! the chain exists for. A test that only tried `UPDATE` through the store
//! would be testing `M3.17` again under a new name.
//!
//! # Engine-specific on purpose
//!
//! The corruption is spelled in `SQLite`, so it lives here rather than in
//! `openehr_store::conformance`. The *judgement* — `verify_versions` — is
//! shared, so a second engine writing its own corruption gets the same verdict
//! logic rather than a second opinion.

use openehr::base::HierObjectId;
use openehr_sqlite::SqliteStore;
use openehr_store::{
    Store as _, conformance,
    integrity::{Breach, Integrity, verify_versions},
};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// A private database file for one test.
fn temp_db(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after the epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("openehr-tamper-{tag}-{nanos}.sqlite3"));
    let _ = std::fs::remove_file(&path);
    path
}

fn container() -> HierObjectId {
    HierObjectId::from_uid_str(conformance::RECORD).expect("literal")
}

/// Three committed versions of one composition, on disk.
fn seed(path: &Path) {
    let mut store = SqliteStore::open(path).expect("open");
    store.install().expect("install");
    let ehr = conformance::sample_ehr();
    let ehr_id = ehr.ehr_id().clone();
    store.create_ehr(&ehr).expect("ehr");
    store
        .create_contribution(
            &ehr_id,
            &conformance::sample_contribution("22222222-3333-4444-5555-666666666666", &[1, 2, 3]),
        )
        .expect("contribution");
    for (n, preceding) in [(1, None), (2, Some(1)), (3, Some(2))] {
        store
            .commit_composition(
                &ehr_id,
                &conformance::sample_version(n, preceding, n * 5),
                "22222222-3333-4444-5555-666666666666",
            )
            .expect("commit");
    }
}

/// Edits the database behind the store's back.
///
/// Drops the append-only triggers first, because that is what someone with
/// file access does and because leaving them up would test `M3.17` rather than
/// the chain.
fn tamper(path: &Path, sql: &str) {
    let connection = Connection::open(path).expect("second connection");
    connection
        .execute_batch(
            "DROP TRIGGER IF EXISTS trg_openehr_version_no_update;
             DROP TRIGGER IF EXISTS trg_openehr_version_no_delete;",
        )
        .expect("drop triggers");
    connection.execute_batch(sql).expect("tamper");
}

/// Removes a version and the index row that points at it.
///
/// The first attempt deleted the version alone and hit a foreign key from
/// `openehr_composition_index`. Turning the constraint off would have been
/// easier and would have modelled a *worse* attacker: one who leaves the
/// database inconsistent, so that the next query complains and the whole thing
/// is discovered by accident rather than by the chain.
///
/// This models the careful one, who leaves referential integrity intact and no
/// other trace. What catches them has to be the chain, because nothing else is
/// left to notice.
fn excise(path: &Path, suffix: &str) {
    tamper(
        path,
        &format!(
            "DELETE FROM openehr_composition_index WHERE version_uid LIKE '%::{suffix}';
             DELETE FROM openehr_version WHERE uid LIKE '%::{suffix}';"
        ),
    );
}

/// What the store makes of the history now.
fn verdict(path: &Path) -> Integrity {
    let store = SqliteStore::open(path).expect("reopen");
    let rows = store.all_versions(&container()).expect("read back");
    verify_versions(&rows, &[])
}

#[test]
fn an_untampered_history_verifies() {
    let path = temp_db("clean");
    seed(&path);

    // Unkeyed, not Verified: nothing signed these entries. Asserted here so the
    // tests below are known to start from a history that passes, which is what
    // makes their failures mean something.
    assert_eq!(verdict(&path), Integrity::Unkeyed);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn editing_a_stored_document_is_detected() {
    let path = temp_db("content");
    seed(&path);

    // **The one the chain cannot find by itself.** Every chain column is left
    // exactly as written, so the links match, every digest recomputes, and
    // `Chain::verify` alone would report this history as sound. Only
    // recomputing the content digest from the stored bytes catches it.
    //
    // A name is edited because it is visible in the fixture; it stands for any
    // content change at all — a dose, a laterality, a diagnosis.
    tamper(
        &path,
        "UPDATE openehr_version
            SET data_json = replace(data_json, 'Encounter 2', 'Encounter X')
          WHERE uid LIKE '%::2'",
    );

    match verdict(&path) {
        Integrity::Broken { at, uid, reason } => {
            assert_eq!(reason, Breach::ContentAltered);
            assert_eq!(at, 1, "the second version was the one edited");
            assert!(uid.ends_with("::2"), "{uid}");
        }
        other => panic!("an edited clinical document was not detected: {other:?}"),
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn removing_a_version_from_the_middle_is_detected() {
    let path = temp_db("middle");
    seed(&path);

    excise(&path, "2");

    match verdict(&path) {
        Integrity::Broken { at, reason, .. } => {
            // The third version still names the second as its predecessor, and
            // the second is gone. This is what the links are for.
            assert_eq!(reason, Breach::PreviousMismatch);
            assert_eq!(at, 1, "the break is at the row that no longer follows");
        }
        other => panic!("a version removed from the middle was not detected: {other:?}"),
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rewriting_one_chain_column_is_detected() {
    let path = temp_db("digest");
    seed(&path);

    // Someone who realised the content digest would give them away, and tried
    // to move it instead.
    tamper(
        &path,
        "UPDATE openehr_version SET chain_digest = zeroblob(32) WHERE uid LIKE '%::1'",
    );

    match verdict(&path) {
        Integrity::Broken { at, reason, .. } => {
            assert_eq!(reason, Breach::DigestMismatch);
            assert_eq!(at, 0);
        }
        other => panic!("a rewritten chain digest was not detected: {other:?}"),
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn truncating_the_newest_version_is_not_detected_and_that_is_why_checkpoints_exist() {
    let path = temp_db("truncate");
    seed(&path);

    // Delete the newest version and what remains is a shorter history that
    // verifies perfectly. There is nothing inside the chain that could catch
    // this: a chain of two is indistinguishable from a chain that was only ever
    // two.
    excise(&path, "3");

    assert!(
        verdict(&path).is_intact(),
        "if this ever fails, the chain gained a property it is not documented \
         to have, and M3.16c should be revisited rather than this test deleted"
    );

    // This test asserts a **limit**, and asserting a limit is only useful if the
    // thing that closes it is named. The checkpoint counts the entries it
    // covers, so a witness holding `entries=3` sees `entries=2` and knows —
    // but only because it is held somewhere the database administrator cannot
    // reach (`M3.16c`).
    let store = SqliteStore::open(&path).expect("reopen");
    let checkpoint = store.chain_checkpoint(&container()).expect("checkpoint");
    assert!(
        checkpoint.starts_with("entries=2 "),
        "the checkpoint must report the count it can actually see: {checkpoint}"
    );
    let _ = std::fs::remove_file(&path);
}
