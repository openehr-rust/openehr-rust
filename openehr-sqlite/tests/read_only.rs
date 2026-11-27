//! `SqliteStore::open_read_only`, the opener for inspection (`db:O10.20`).
//!
//! It exists because the inspection tasks of `openehr-axa` used the server's
//! open-and-install path: a mistyped backup path became a new six-table
//! database and a checkpoint of nothing exited 0 (`db:D-27`). Each test here
//! checks the bytes on disk, not only the answer, since "read-only" is a claim
//! about the file.

use openehr::base::HierObjectId;
use openehr_sqlite::SqliteStore;
use openehr_store::{Store as _, StoreError, conformance};
use std::path::{Path, PathBuf};

const CONTRIBUTION: &str = "22222222-3333-4444-5555-666666666666";

fn temp_path(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after the epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("openehr-sqlite-ro-{tag}-{nanos}.db"));
    let _ = std::fs::remove_file(&path);
    path
}

/// The error `open_read_only` answers with; a store returned instead fails the
/// test. (`SqliteStore` is not `Debug`, so `expect_err` is unavailable.)
fn refused(path: &Path, why: &str) -> StoreError {
    match SqliteStore::open_read_only(path) {
        Ok(_) => panic!("{why}: {} opened", path.display()),
        Err(error) => error,
    }
}

fn seed(path: &Path) {
    let mut store = SqliteStore::open(path).expect("open");
    store.install().expect("install");
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

#[test]
fn a_missing_path_is_refused_and_stays_absent() {
    let path = temp_path("missing");
    let error = refused(&path, "no such database");
    assert!(matches!(error, StoreError::Engine { .. }), "{error:?}");
    assert!(!path.exists(), "inspection created {}", path.display());
}

#[test]
fn an_empty_file_is_not_a_store_and_is_left_empty() {
    let path = temp_path("empty");
    std::fs::write(&path, b"").expect("create an empty file");
    let error = refused(&path, "no schema recorded");
    assert!(
        matches!(error, StoreError::SchemaVersionMismatch { found: 0, .. }),
        "{error:?}"
    );
    assert_eq!(
        std::fs::read(&path).expect("read").len(),
        0,
        "the file was written"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_database_of_another_schema_version_is_refused() {
    let path = temp_path("other-version");
    seed(&path);
    {
        let store = SqliteStore::open(&path).expect("open");
        store
            .connection()
            .execute(
                "UPDATE openehr_schema_version SET version = version + 1",
                [],
            )
            .expect("pretend a later build installed it");
    }
    let error = refused(&path, "not this build's schema");
    assert!(
        matches!(error, StoreError::SchemaVersionMismatch { found, expected } if found == expected + 1),
        "{error:?}"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_populated_store_reads_without_a_byte_changing_and_refuses_writes() {
    let path = temp_path("populated");
    seed(&path);
    let before = std::fs::read(&path).expect("read");

    let mut store = SqliteStore::open_read_only(&path).expect("open read-only");
    let container = HierObjectId::from_uid_str(conformance::RECORD).expect("literal");
    assert_eq!(store.all_versions(&container).expect("history").len(), 2);
    assert!(
        store
            .chain_checkpoint(&container)
            .expect("checkpoint")
            .starts_with("entries=2 ")
    );
    let error = store
        .install()
        .expect_err("a read-only store installs nothing");
    assert!(matches!(error, StoreError::Engine { .. }), "{error:?}");
    let error = store
        .create_ehr(&conformance::sample_ehr())
        .expect_err("nor writes a record");
    assert!(
        matches!(
            error,
            StoreError::Engine { .. } | StoreError::Conflict { .. }
        ),
        "{error:?}"
    );
    drop(store);

    assert_eq!(
        std::fs::read(&path).expect("read"),
        before,
        "inspection changed the file"
    );
    let _ = std::fs::remove_file(&path);
}
