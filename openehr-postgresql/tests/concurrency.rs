//! Concurrency, tested adversarially rather than assumed (`db:T11.6`).
//!
//! Ported from `openehr-sqlite/tests/concurrency.rs`, whose own module doc
//! explains why: every other test in this crate drives one store from one
//! thread, which cannot distinguish "the commit rules hold" from "the commit
//! rules hold when nothing else is happening." Found worth porting rather
//! than assuming the two engines' commit-conflict paths behave alike: `cargo
//! mutants` against `PostgresqlStore::commit_conflict` reported every one of
//! its mutants — including deleting the match arm that turns a unique-index
//! violation into `CommitError::NotLatest` — as `MISSED`, because nothing in
//! this crate's own single-threaded tests ever drives two commits into a
//! real database-level race. This file is what closes that.
//!
//! `#[ignore]`d for the same reason `tests/store.rs` is — see that file's own
//! module doc for how to provision a server.
//!
//! One difference from the `SQLite` original: no file path to make unique
//! per test. `SQLite`'s own file-locking model needs a database per test, so
//! two connections can share one; `PostgreSQL` connections share a server
//! naturally, and `reset` (this file's own, matching `tests/store.rs`'s)
//! clears the one shared database between tests instead.

use openehr::base::HierObjectId;
use openehr_postgresql::PostgresqlStore;
use openehr_store::{Store, StoreError, conformance};
use std::sync::{Arc, Barrier};

fn connect() -> PostgresqlStore {
    let params = std::env::var("OPENEHR_POSTGRESQL_URL").expect(
        "set OPENEHR_POSTGRESQL_URL to a postgres connection string, or run \
         sh openehr-postgresql/scripts/verify-store.sh, which does both",
    );
    PostgresqlStore::connect(&params).expect("connect")
}

fn reset(store: &mut PostgresqlStore) {
    store
        .client()
        .batch_execute(
            "DROP TABLE IF EXISTS openehr_composition_index, openehr_version, \
             openehr_versioned_object, openehr_contribution, openehr_ehr, \
             openehr_schema_version CASCADE",
        )
        .expect("reset");
}

fn container() -> HierObjectId {
    HierObjectId::from_uid_str(conformance::RECORD).expect("literal")
}

/// Installs the schema, an EHR, a contribution, and version 1.
fn seeded() {
    let mut store = connect();
    reset(&mut store);
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
/// not engine errors. See the `SQLite` original for why the distinction
/// matters; the assertion is identical.
#[test]
#[ignore = "needs a real PostgreSQL server, see tests/store.rs's own module doc"]
fn racing_commits_to_one_position_produce_one_winner() {
    const WRITERS: usize = 8;
    seeded();

    let barrier = Arc::new(Barrier::new(WRITERS));
    let outcomes: Vec<_> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..WRITERS)
            .map(|i| {
                let barrier = Arc::clone(&barrier);
                let minute = u32::try_from(i).expect("writer count fits");
                scope.spawn(move || {
                    let mut store = connect();
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
    let store = connect();
    let all = store.all_versions(&container()).expect("all_versions");
    assert_eq!(all.len(), 2, "the version tree gained a duplicate position");
}

/// `R4.5`: a reader looping against a writer never observes a torn commit.
///
/// A commit writes the version row and its index row in one transaction
/// (`db:R4.4`). If a reader can see the first without the second, the record
/// is briefly present and unfindable — which reads as data loss to anyone
/// querying by archetype.
#[test]
#[ignore = "needs a real PostgreSQL server, see tests/store.rs's own module doc"]
fn a_reader_never_observes_a_torn_commit() {
    const COMMITS: u32 = 24;
    seeded();
    let ehr_id = container();

    std::thread::scope(|scope| {
        let writer = scope.spawn(move || {
            let mut store = connect();
            let id = container();
            for n in 2..=COMMITS {
                let version = conformance::sample_version(n, Some(n - 1), n);
                store
                    .commit_composition(&id, &version, "c1")
                    .unwrap_or_else(|e| panic!("writer failed at version {n}: {e}"));
            }
        });

        let reader = scope.spawn(move || {
            let store = connect();
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

    let store = connect();
    let all = store.all_versions(&container()).expect("all_versions");
    assert_eq!(all.len(), COMMITS as usize, "a commit was lost");
}
