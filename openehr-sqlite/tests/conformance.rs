//! The shared conformance suite, run against a real `SQLite` database.
//!
//! `SQLite` is the only one of the five engines that can be verified without
//! provisioning a server, so it is where the shared store logic is actually
//! exercised. The suite itself lives in `openehr-store` and is written once
//! (see that crate's `conformance` module for why).

use openehr_sqlite::{SqliteDialect, SqliteStore};
use openehr_store::{Store, conformance, ddl_script};

#[test]
fn the_shared_suite_passes_against_a_real_database() {
    let mut store = SqliteStore::in_memory().expect("open");
    conformance::run(&mut store).expect("conformance suite");
}

#[test]
fn the_dialect_is_self_consistent() {
    conformance::check_dialect(&SqliteDialect);
}

/// Fails if the append-only trigger stops firing. openEHR's entire
/// change-control model rests on `version` being append-only (`V8.10`), and a
/// guarantee enforced only in application code ends the first time somebody
/// opens the database with the `sqlite3` CLI.
#[test]
fn the_database_itself_refuses_to_mutate_a_version() {
    let mut store = SqliteStore::in_memory().expect("open");
    store.install().expect("install");
    let ehr = conformance::sample_ehr();
    store.create_ehr(&ehr).expect("create ehr");
    store
        .create_contribution(
            ehr.ehr_id(),
            &conformance::sample_contribution("22222222-3333-4444-5555-666666666666", &[1]),
        )
        .expect("contribution");
    store
        .commit_composition(
            ehr.ehr_id(),
            &conformance::sample_version(1, None, 5),
            "22222222-3333-4444-5555-666666666666",
        )
        .expect("commit");

    // Go around the store entirely, as an operator with a SQL console would.
    let update = store.connection().execute(
        "UPDATE openehr_version SET lifecycle_state_code = '523'",
        [],
    );
    assert!(update.is_err(), "an UPDATE on openehr_version succeeded");
    let delete = store
        .connection()
        .execute("DELETE FROM openehr_version", []);
    assert!(delete.is_err(), "a DELETE on openehr_version succeeded");

    // …and the row is still there, unchanged.
    let row = store
        .latest_version(&"87284370-2D4B-4E3D-A3F3-F303D2F4F34B".parse().unwrap())
        .expect("still readable");
    assert_eq!(row.lifecycle_state_code, "532");
}

/// Fails if an invalid composition can reach storage. A store that accepted one
/// would make every later reader's `validate()` fail on data it cannot fix.
#[test]
fn an_invalid_composition_is_refused_before_it_is_written() {
    use openehr::rm::common::{AuditDetails, LocatableAttrs, OriginalVersion, PartyIdentified};
    use openehr::rm::data_types::{CodePhrase, DvDateTime};
    use openehr::rm::ehr::Composition;
    use openehr::terminology::{audit_change_type, composition_category, version_lifecycle_state};

    let mut store = SqliteStore::in_memory().expect("open");
    store.install().expect("install");
    let ehr = conformance::sample_ehr();
    store.create_ehr(&ehr).expect("create ehr");
    store
        .create_contribution(
            ehr.ehr_id(),
            &conformance::sample_contribution("22222222-3333-4444-5555-666666666666", &[1]),
        )
        .expect("contribution");

    // No `archetype_details`, so not an archetype root (`E6.6a`).
    let rootless = Composition::new(
        LocatableAttrs::named("Encounter", "openEHR-EHR-COMPOSITION.encounter.v1").unwrap(),
        composition_category::EVENT,
        PartyIdentified::named("Dr A Nurse").unwrap().into(),
        CodePhrase::new("ISO_639-1", "en").unwrap(),
        CodePhrase::new("ISO_3166-1", "GB").unwrap(),
    )
    .unwrap();
    let owner = openehr::base::ObjectRef::new(
        "local",
        "EHR",
        openehr::base::ObjectId::HierObjectId(ehr.ehr_id().clone()),
    )
    .unwrap();
    let version = OriginalVersion::new(
        format!("{}::{}::1", conformance::RECORD, conformance::SYSTEM)
            .parse()
            .unwrap(),
        None,
        version_lifecycle_state::COMPLETE,
        Some(rootless),
        AuditDetails::new(
            conformance::SYSTEM,
            DvDateTime::new("2026-08-01T09:05:00Z").unwrap(),
            audit_change_type::CREATION,
            PartyIdentified::named("Dr A Nurse").unwrap().into(),
        )
        .unwrap(),
        owner,
    )
    .unwrap();

    let result = store.commit_composition(
        ehr.ehr_id(),
        &version.into(),
        "22222222-3333-4444-5555-666666666666",
    );
    assert!(
        matches!(result, Err(openehr_store::StoreError::Invalid(_))),
        "an invalid composition was accepted"
    );

    // And nothing was written — the refusal is before the transaction, not
    // inside a rolled-back one that left a container behind.
    let count: i64 = store
        .connection()
        .query_row("SELECT COUNT(*) FROM openehr_versioned_object", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(count, 0, "a refused commit left a container behind");
}

/// Fails if a stored instant loses its lexical form. The whole two-column design
/// exists for this (`D3.10`).
#[test]
fn a_partial_or_offset_instant_survives_storage_verbatim() {
    let mut store = SqliteStore::in_memory().expect("open");
    store.install().expect("install");
    let ehr = conformance::sample_ehr();
    store.create_ehr(&ehr).expect("create");

    let read = store.get_ehr(ehr.ehr_id()).expect("read");
    assert_eq!(
        read.time_created().as_str(),
        ehr.time_created().as_str(),
        "the authoritative lexical form was altered by a round trip"
    );
}

/// Fails if the DDL stops being runnable. A script that does not execute is the
/// defect the sibling FHIR monorepo records as **F-25**/**F-26** — a migration
/// path that could never have run, in a port with no store to notice.
#[test]
fn the_generated_ddl_executes_and_is_idempotent() {
    let script = ddl_script(&SqliteDialect);
    let connection = rusqlite::Connection::open_in_memory().expect("open");
    connection.execute_batch(&script).expect("first install");
    connection.execute_batch(&script).expect("second install");
}
