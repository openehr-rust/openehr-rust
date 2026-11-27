//! Paths of `SqliteStore` that the shared conformance stories never take
//! (`W0.41`): an indexed composition that carries an `EVENT_CONTEXT`
//! with both times, a database file that cannot be opened, and a schema-version
//! record that cannot be written, and a commit the database refuses only at
//! `COMMIT`.
//!
//! The sample composition of `openehr_store::conformance` has no context, so
//! before this file no test of this crate had ever written or read back a
//! `context_start_*` or `context_end_*` column: a store that dropped both
//! would have passed every other suite here.

use openehr::rm::common::Version;
use openehr::rm::data_types::DvDateTime;
use openehr::rm::ehr::{Composition, EventContext};
use openehr::terminology::setting;
use openehr_sqlite::SqliteStore;
use openehr_store::{Store as _, StoreError, conformance};

const CONTRIBUTION: &str = "22222222-3333-4444-5555-666666666666";

/// The conformance sample version 1, with an `EVENT_CONTEXT` from 09:00Z to
/// 10:45+01:00 (09:45Z) put into its composition.
fn contextual_version() -> Version<Composition> {
    let context = EventContext::new(
        DvDateTime::new("2026-08-01T09:00:00Z").expect("literal"),
        setting::PRIMARY_MEDICAL_CARE,
    )
    .expect("literal")
    .with_end_time(DvDateTime::new("2026-08-01T10:45:00+01:00").expect("literal"))
    .expect("literal");
    let mut json =
        serde_json::to_value(conformance::sample_version(1, None, 1)).expect("serialise");
    json["data"]["context"] = serde_json::to_value(context).expect("serialise");
    serde_json::from_value(json).expect("deserialise")
}

#[test]
fn an_event_context_is_indexed_and_read_back_with_both_instants() {
    let mut store = SqliteStore::in_memory().expect("open");
    store.install().expect("install");
    let ehr = conformance::sample_ehr();
    let ehr_id = ehr.ehr_id().clone();
    store.create_ehr(&ehr).expect("ehr");
    store
        .create_contribution(
            &ehr_id,
            &conformance::sample_contribution(CONTRIBUTION, &[1]),
        )
        .expect("contribution");
    store
        .commit_composition(&ehr_id, &contextual_version(), CONTRIBUTION)
        .expect("commit");

    let rows = store
        .find_compositions_by_archetype(&ehr_id, "openEHR-EHR-COMPOSITION.encounter.v1")
        .expect("search");
    assert_eq!(rows.len(), 1, "{rows:?}");
    let start = rows[0]
        .context_start
        .as_ref()
        .expect("context start indexed");
    let end = rows[0].context_end.as_ref().expect("context end indexed");
    // The text is authoritative and kept exactly as written, offset included;
    // the UTC column is derived from it.
    assert_eq!(start.text, "2026-08-01T09:00:00Z");
    assert_eq!(end.text, "2026-08-01T10:45:00+01:00");
    assert!(start.utc_seconds.is_some());
    assert_eq!(
        end.utc_seconds.expect("derived") - start.utc_seconds.expect("derived"),
        45 * 60,
        "the end's offset must be applied, not ignored"
    );
}

#[test]
fn a_database_path_that_is_a_directory_is_an_engine_error() {
    let directory = std::env::temp_dir();
    let error = SqliteStore::open(&directory)
        .and_then(|mut store| store.install())
        .expect_err("a directory is not a database");
    assert!(matches!(error, StoreError::Engine { .. }), "{error:?}");
}

#[test]
fn install_on_a_read_only_database_is_an_engine_error_not_a_silent_success() {
    let path = std::env::temp_dir().join(format!(
        "openehr-sqlite-read-only-{}.db",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    {
        // A fresh file with the schema but no recorded version, so a later
        // install has to write the version row and cannot.
        let mut store = SqliteStore::open(&path).expect("open");
        store.install().expect("install");
        store
            .connection()
            .execute("DELETE FROM openehr_schema_version", [])
            .expect("clear the version row");
    }
    let read_only =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("open read-only");
    let mut store = SqliteStore::from_connection(read_only).expect("wrap");
    let error = store
        .install()
        .expect_err("a read-only database cannot record its version");
    let _ = std::fs::remove_file(&path);
    assert!(matches!(error, StoreError::Engine { .. }), "{error:?}");
}

/// A violation the database only reports at `COMMIT`: a deferred foreign key,
/// written by a temporary trigger on every version insert. Every statement of
/// the commit succeeds and the transaction's own `COMMIT` is refused, which is
/// the one failure a store must not report as success after it has already run
/// its inserts.
const FAIL_AT_COMMIT: &str = "CREATE TABLE fault_parent (id INTEGER PRIMARY KEY);
    CREATE TABLE fault_child (
        parent INTEGER REFERENCES fault_parent (id) DEFERRABLE INITIALLY DEFERRED
    );
    CREATE TEMP TRIGGER fault_on_version AFTER INSERT ON main.openehr_version
    BEGIN INSERT INTO fault_child (parent) VALUES (1); END;";

fn seeded_failing_at_commit() -> (SqliteStore, openehr::base::HierObjectId) {
    let mut store = SqliteStore::in_memory().expect("open");
    store.install().expect("install");
    let ehr = conformance::sample_ehr();
    let ehr_id = ehr.ehr_id().clone();
    store.create_ehr(&ehr).expect("ehr");
    store
        .create_contribution(
            &ehr_id,
            &conformance::sample_contribution_of(CONTRIBUTION, &[1], &[1]),
        )
        .expect("contribution");
    store
        .connection()
        .execute_batch(FAIL_AT_COMMIT)
        .expect("install the fault");
    (store, ehr_id)
}

#[test]
fn a_composition_commit_refused_at_commit_is_an_engine_error_and_writes_nothing() {
    let (mut store, ehr_id) = seeded_failing_at_commit();
    let error = store
        .commit_composition(&ehr_id, &contextual_version(), CONTRIBUTION)
        .expect_err("the deferred violation refuses the commit");
    assert!(matches!(error, StoreError::Engine { .. }), "{error:?}");
    let container =
        openehr::base::HierObjectId::from_uid_str(conformance::RECORD).expect("literal");
    assert!(
        matches!(
            store.latest_version(&container),
            Err(StoreError::NotFound { .. })
        ),
        "a refused COMMIT must leave no version behind"
    );
    assert!(
        store
            .find_compositions_by_archetype(&ehr_id, "openEHR-EHR-COMPOSITION.encounter.v1")
            .expect("search")
            .is_empty(),
        "nor an index row"
    );
}

#[test]
fn an_ehr_status_commit_refused_at_commit_is_an_engine_error_and_writes_nothing() {
    let (mut store, ehr_id) = seeded_failing_at_commit();
    let version = conformance::sample_ehr_status_version(1, None, 1, true);
    let error = store
        .commit_ehr_status(&ehr_id, &version, CONTRIBUTION)
        .expect_err("the deferred violation refuses the commit");
    assert!(matches!(error, StoreError::Engine { .. }), "{error:?}");
    let container = openehr::base::HierObjectId::new(version.uid().object_id().clone(), None)
        .expect("container uid");
    assert!(
        matches!(
            store.latest_version(&container),
            Err(StoreError::NotFound { .. })
        ),
        "a refused COMMIT must leave no status version behind"
    );
}

/// A rival row inserted by a temporary trigger in the instant before the
/// store's own insert, which is what a second writer that passed the same
/// commit-rule check does under a real race. `concurrency.rs` races real
/// threads and reaches this path only when the two inserts actually overlap,
/// so line coverage of `commit_conflict` changed from run to run and the exact
/// ratchet (`W0.41a`) flickered. Here the overlap is forced, every run.
///
/// `uid` and `trunk` are SQL expressions over `NEW`: the rival takes either
/// the same tree position under another identity, or the same identity at
/// another position.
fn rival_trigger(uid: &str, trunk: &str) -> String {
    format!(
        "CREATE TEMP TRIGGER fault_rival BEFORE INSERT ON main.openehr_version \
         WHEN NEW.uid NOT LIKE 'rival-%' AND NEW.trunk_version = 2 BEGIN \
         INSERT INTO openehr_version (uid, versioned_object_uid, creating_system_id, trunk_version, branch_number, branch_version, preceding_version_uid, lifecycle_state_code, is_deleted, contribution_uid, audit_system_id, audit_change_type_code, audit_committer_name, audit_committer_json, audit_time_committed_text, audit_time_committed_utc, data_json, audit_description, signature, attestations_json, other_input_version_uids_json, chain_previous, chain_content, chain_digest, chain_tag_key_id, chain_tag_mac) \
         VALUES ({uid}, NEW.versioned_object_uid, NEW.creating_system_id, {trunk}, \
         NEW.branch_number, NEW.branch_version, NEW.preceding_version_uid, NEW.lifecycle_state_code, NEW.is_deleted, NEW.contribution_uid, NEW.audit_system_id, NEW.audit_change_type_code, NEW.audit_committer_name, NEW.audit_committer_json, NEW.audit_time_committed_text, NEW.audit_time_committed_utc, NEW.data_json, NEW.audit_description, NEW.signature, NEW.attestations_json, NEW.other_input_version_uids_json, NEW.chain_previous, NEW.chain_content, NEW.chain_digest, NEW.chain_tag_key_id, NEW.chain_tag_mac); END;"
    )
}

fn seeded_with_one_version() -> (SqliteStore, openehr::base::HierObjectId) {
    let mut store = SqliteStore::in_memory().expect("open");
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
    store
        .commit_composition(
            &ehr_id,
            &conformance::sample_version(1, None, 1),
            CONTRIBUTION,
        )
        .expect("first version");
    (store, ehr_id)
}

#[test]
fn a_rival_at_the_same_tree_position_is_not_latest_not_an_engine_error() {
    let (mut store, ehr_id) = seeded_with_one_version();
    store
        .connection()
        .execute_batch(&rival_trigger("'rival-' || NEW.uid", "NEW.trunk_version"))
        .expect("install the rival");
    let error = store
        .commit_composition(
            &ehr_id,
            &conformance::sample_version(2, Some(1), 2),
            CONTRIBUTION,
        )
        .expect_err("the position is taken");
    // `db:H5.9`: a caller told `NotLatest` can re-read the head and retry; one
    // told "UNIQUE constraint failed" cannot tell a lost race from a fault.
    assert!(
        matches!(
            error,
            StoreError::Commit(openehr::rm::common::CommitError::NotLatest)
        ),
        "{error:?}"
    );
}

#[test]
fn a_rival_with_the_same_identity_is_a_duplicate_version() {
    let (mut store, ehr_id) = seeded_with_one_version();
    store
        .connection()
        .execute_batch(&rival_trigger("NEW.uid", "NEW.trunk_version + 100"))
        .expect("install the rival");
    let error = store
        .commit_composition(
            &ehr_id,
            &conformance::sample_version(2, Some(1), 2),
            CONTRIBUTION,
        )
        .expect_err("the identity is taken");
    assert!(
        matches!(
            error,
            StoreError::Commit(openehr::rm::common::CommitError::DuplicateVersion)
        ),
        "{error:?}"
    );
}

/// `db:D-33`'s aside. A trigger's `RAISE(ABORT)` is a `ConstraintViolation`
/// like a duplicate key, and was reported as "contribution already exists". It
/// is an engine failure, and only the primary key's collision is a conflict.
#[test]
fn a_refused_contribution_insert_is_an_engine_error_unless_the_id_is_taken() {
    let mut store = SqliteStore::in_memory().expect("open");
    store.install().expect("install");
    let ehr = conformance::sample_ehr();
    store.create_ehr(&ehr).expect("ehr");
    let contribution = conformance::sample_contribution_of(CONTRIBUTION, &[1], &[]);

    store
        .connection()
        .execute_batch(
            "CREATE TRIGGER fault_contribution BEFORE INSERT ON openehr_contribution \
             BEGIN SELECT RAISE(ABORT, 'injected'); END;",
        )
        .expect("trigger");
    let refused = store.create_contribution(ehr.ehr_id(), &contribution);
    assert!(
        matches!(refused, Err(StoreError::Engine { .. })),
        "a trigger's refusal is the engine's, got {refused:?}"
    );

    store
        .connection()
        .execute_batch("DROP TRIGGER fault_contribution;")
        .expect("drop");
    store
        .create_contribution(ehr.ehr_id(), &contribution)
        .expect("the id was never taken");
    let again = store.create_contribution(ehr.ehr_id(), &contribution);
    assert!(
        matches!(
            again,
            Err(StoreError::Conflict {
                kind: "contribution",
                ..
            })
        ),
        "a taken id is a conflict, got {again:?}"
    );
}
