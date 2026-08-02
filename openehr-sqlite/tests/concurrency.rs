//! Concurrency, tested adversarially rather than assumed (`db:T11.6`).
//!
//! Every other test in this crate drives one store from one thread, which
//! cannot distinguish "the commit rules hold" from "the commit rules hold when
//! nothing else is happening". `db:D-02` recorded both concurrency requirements
//! as unverified for exactly that reason; this file is what closes it.
//!
//! These tests use a **file** database with a connection per thread. An
//! in-memory database is private to its connection, so a concurrent test
//! against `in_memory()` would run N independent databases and pass without
//! testing anything — the same shape as a guard whose list is incomplete.

use openehr::base::HierObjectId;
use openehr_sqlite::SqliteStore;
use openehr_store::{Store, StoreError, conformance};
use std::path::PathBuf;
use std::sync::{Arc, Barrier};

/// A unique database path for one test.
fn scratch(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "openehr-concurrency-{}-{name}.sqlite3",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&p);
    p
}

fn container() -> HierObjectId {
    HierObjectId::from_uid_str(conformance::RECORD).expect("literal")
}

/// Installs the schema, an EHR, a contribution, and version 1.
fn seeded(path: &std::path::Path) {
    let mut store = SqliteStore::open(path).expect("open");
    store.install().expect("install");
    let ehr = conformance::sample_ehr();
    store.create_ehr(&ehr).expect("ehr");
    store
        .create_contribution(ehr.ehr_id(), &conformance::sample_contribution("c1", &[1]))
        .expect("contribution");
    store
        .commit_composition(ehr.ehr_id(), &conformance::sample_version(1, None, 0), "c1")
        .expect("version 1");
}

/// `H5.4`: N racing commits to one position in a version tree produce exactly
/// one success and N−1 refusals, and the refusals are **commit refusals** —
/// not engine errors.
///
/// The distinction matters. A caller that gets `Commit` knows another writer
/// won and can re-read and retry against the new head. A caller that gets
/// `Engine("database is locked")` knows only that something went wrong, and the
/// version tree is exactly the place where guessing is not allowed.
#[test]
fn racing_commits_to_one_position_produce_one_winner() {
    const WRITERS: usize = 8;
    let path = scratch("race");
    seeded(&path);

    let barrier = Arc::new(Barrier::new(WRITERS));
    let outcomes: Vec<_> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..WRITERS)
            .map(|i| {
                let path = path.clone();
                let barrier = Arc::clone(&barrier);
                let minute = u32::try_from(i).expect("writer count fits");
                scope.spawn(move || {
                    let mut store = SqliteStore::open(&path).expect("open");
                    let ehr_id = container();
                    let version = conformance::sample_version(2, Some(1), 10 + minute);
                    barrier.wait();
                    store.commit_composition(&ehr_id, &version, "c1")
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("thread"))
            .collect()
    });

    let winners = outcomes.iter().filter(|r| r.is_ok()).count();
    assert_eq!(
        winners, 1,
        "exactly one writer may take a position in a version tree; {winners} did"
    );

    for outcome in &outcomes {
        if let Err(error) = outcome {
            assert!(
                matches!(error, StoreError::Commit(_)),
                "a losing writer must be refused by the commit rules, \
                 not by the engine: {error}"
            );
        }
    }

    // The tree must read back intact: two versions, no more.
    let store = SqliteStore::open(&path).expect("open");
    let all = store.all_versions(&container()).expect("all_versions");
    assert_eq!(all.len(), 2, "the version tree gained a duplicate position");
    let _ = std::fs::remove_file(&path);
}

/// `R4.5`: a reader looping against a writer never observes a torn commit.
///
/// A commit writes the version row and its index row in one transaction
/// (`db:R4.4`). If a reader can see the first without the second, the record is
/// briefly present and unfindable — which reads as data loss to anyone querying
/// by archetype.
#[test]
fn a_reader_never_observes_a_torn_commit() {
    const COMMITS: u32 = 24;
    let path = scratch("torn");
    seeded(&path);
    let ehr_id = container();

    std::thread::scope(|scope| {
        let writer_path = path.clone();
        let writer = scope.spawn(move || {
            let mut store = SqliteStore::open(&writer_path).expect("open");
            let id = container();
            for n in 2..=COMMITS {
                let version = conformance::sample_version(n, Some(n - 1), n);
                store
                    .commit_composition(&id, &version, "c1")
                    .unwrap_or_else(|e| panic!("writer failed at version {n}: {e}"));
            }
        });

        let reader_path = path.clone();
        let reader = scope.spawn(move || {
            let store = SqliteStore::open(&reader_path).expect("open");
            let id = container();
            for _ in 0..400 {
                let Ok(head) = store.latest_version(&id) else {
                    continue;
                };
                // Every version the reader can see must have its index row.
                let indexed = store
                    .find_compositions_by_archetype(&ehr_id, "openEHR-EHR-COMPOSITION.encounter.v1")
                    .expect("archetype query");
                assert!(
                    indexed.iter().any(|row| row.version_uid == head.uid),
                    "read a version ({}) whose index row was not visible — a torn commit",
                    head.uid
                );
            }
        });

        writer.join().expect("writer");
        reader.join().expect("reader");
    });

    let store = SqliteStore::open(&path).expect("open");
    let all = store.all_versions(&container()).expect("all_versions");
    assert_eq!(all.len(), COMMITS as usize, "a commit was lost");
    let _ = std::fs::remove_file(&path);
}
