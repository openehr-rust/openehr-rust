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

/// `M3.16` / `D-03`: the chain links successive versions in a container, and a
/// mutation to stored content is detectable.
///
/// The chain is per container, in version-tree order. That detects a version
/// altered in place, removed from the middle, or reordered. It does **not**
/// detect deleting the newest version or dropping a whole container — that is
/// what an external checkpoint is for (`M3.16c`, unimplemented).
#[test]
fn the_chain_links_versions_and_notices_a_rewrite() {
    use openehr::security::{Chain, Digest256};

    let mut store = SqliteStore::in_memory().expect("open");
    store.install().expect("install");
    let ehr = conformance::sample_ehr();
    store.create_ehr(&ehr).expect("ehr");
    store
        .create_contribution(ehr.ehr_id(), &conformance::sample_contribution("c1", &[1, 2, 3]))
        .expect("contribution");

    for n in 1..=3u32 {
        let preceding = (n > 1).then(|| n - 1);
        store
            .commit_composition(ehr.ehr_id(), &conformance::sample_version(n, preceding, n), "c1")
            .unwrap_or_else(|e| panic!("commit {n}: {e}"));
    }

    let container = ehr.ehr_id().clone();
    let all = store.all_versions(&container).expect("all_versions");
    assert_eq!(all.len(), 3);

    // Each entry links to the one before it, and the first to genesis.
    assert_eq!(all[0].chain.previous, [0u8; 32], "first links to genesis");
    for pair in all.windows(2) {
        assert_eq!(
            pair[1].chain.previous, pair[0].chain.digest,
            "version {} does not link to {}",
            pair[1].uid, pair[0].uid
        );
    }

    // Recompute the chain from the stored content. This is the check an auditor
    // runs: it uses only what the database holds, so a rewritten row fails it.
    let mut recomputed = Chain::new();
    for row in &all {
        let content: serde_json::Value = row
            .data_json
            .as_deref()
            .map(|j| serde_json::from_str(j).expect("stored JSON parses"))
            .expect("a committed version has content");
        recomputed
            .append(row.uid.clone(), &Some(content), None)
            .expect("append");
    }
    assert_eq!(
        recomputed.head(),
        Digest256::from_bytes(all[2].chain.digest),
        "the chain recomputed from stored rows must reach the stored head"
    );

    // Now rewrite a row behind the store's back and show the chain notices.
    // The append-only trigger blocks UPDATE, so this goes around it the way a
    // determined operator would: drop the trigger first. That is precisely the
    // attacker the unkeyed chain is documented as detecting but not stopping.
    let connection = store.connection();
    connection
        .execute_batch("DROP TRIGGER trg_openehr_version_no_update;")
        .expect("drop trigger");
    connection
        .execute(
            "UPDATE openehr_version SET data_json = replace(data_json, 'Encounter 2', 'Tampered') \
             WHERE trunk_version = 2",
            [],
        )
        .expect("rewrite");

    let after = store.all_versions(&container).expect("all_versions");
    let mut recheck = Chain::new();
    for row in &after {
        let content: serde_json::Value =
            serde_json::from_str(row.data_json.as_deref().expect("content")).expect("parses");
        recheck.append(row.uid.clone(), &Some(content), None).expect("append");
    }
    assert_ne!(
        recheck.head(),
        Digest256::from_bytes(after[2].chain.digest),
        "a rewritten version must not recompute to the stored head — \
         the chain would be evidence of nothing"
    );
}

/// `O10.14`: a database installed under a different schema version is refused,
/// not half-upgraded.
///
/// There is deliberately no migration path before 1.0 — the schema is expected
/// to change, and building an upgrade route for a shape that is still moving
/// would be machinery maintained against a moving target. What a deployment is
/// owed in the meantime is to be **told**, at `install()`, rather than to
/// discover it when the first commit fails on a column that is not there.
#[test]
fn a_database_from_another_schema_version_is_refused() {
    use openehr_store::StoreError;

    let mut store = SqliteStore::in_memory().expect("open");
    store.install().expect("fresh install");
    // Idempotent: installing again over our own version is fine.
    store.install().expect("re-install is a no-op");

    // Pretend this database was written by a different build.
    store
        .connection()
        .execute("UPDATE openehr_schema_version SET version = 99", [])
        .expect("rewrite version");

    match store.install() {
        Err(StoreError::SchemaVersionMismatch { found, expected }) => {
            assert_eq!(found, 99);
            assert_eq!(expected, openehr_store::SCHEMA_VERSION);
        }
        other => panic!("expected a version mismatch, got {other:?}"),
    }
}

/// A database predating the version table is refused too.
///
/// The absence of a version row is ambiguous — it means "fresh" or "older than
/// versioning" — and the two are told apart by whether the database holds data.
/// Guessing "fresh" would run the DDL over an unknown shape.
#[test]
fn a_database_predating_the_version_table_is_refused() {
    use openehr_store::StoreError;

    let mut store = SqliteStore::in_memory().expect("open");
    store.install().expect("install");
    let ehr = conformance::sample_ehr();
    store.create_ehr(&ehr).expect("ehr");

    // Erase the version row, leaving data behind: exactly what an older
    // database looks like.
    store
        .connection()
        .execute("DELETE FROM openehr_schema_version", [])
        .expect("clear version");

    assert!(
        matches!(
            store.install(),
            Err(StoreError::SchemaVersionMismatch { found: 0, .. })
        ),
        "a populated database with no recorded version must be refused"
    );
}

/// `M3.16c` / `T11.8`: a **truncated** chain still verifies clean, and only the
/// checkpoint reveals it.
///
/// This is the checkpoint's whole reason for existing, and a test that only
/// checked the checkpoint had moved would not show it. Removing the newest
/// version leaves a shorter history whose every link is intact — the chain has
/// no way to know how long it was supposed to be. Something outside the
/// database has to remember.
#[test]
fn a_truncated_chain_verifies_clean_and_only_the_checkpoint_notices() {
    use openehr::security::{Chain, Digest256};

    let mut store = SqliteStore::in_memory().expect("open");
    store.install().expect("install");
    let ehr = conformance::sample_ehr();
    store.create_ehr(&ehr).expect("ehr");
    store
        .create_contribution(ehr.ehr_id(), &conformance::sample_contribution("c1", &[1, 2, 3]))
        .expect("contribution");
    for n in 1..=3u32 {
        store
            .commit_composition(
                ehr.ehr_id(),
                &conformance::sample_version(n, (n > 1).then(|| n - 1), n),
                "c1",
            )
            .expect("commit");
    }

    let container = ehr.ehr_id().clone();
    let before = store.chain_checkpoint(&container).expect("checkpoint");
    assert!(before.starts_with("entries=3 "), "{before}");
    assert!(
        !before.contains("Encounter"),
        "a checkpoint must carry no clinical content: {before}"
    );

    // Truncate: remove the newest version, as an operator with write access
    // would. The append-only trigger blocks DELETE, so it goes first — which is
    // the attacker this design says it detects but does not stop.
    let connection = store.connection();
    connection
        .execute_batch("DROP TRIGGER trg_openehr_version_no_delete;")
        .expect("drop trigger");
    // The index row references the version, so a foreign key blocks deleting
    // the version alone. That is a small, real obstacle — it makes casual
    // truncation fail — and no obstacle at all to someone who reads the error
    // and removes both, which is what this does.
    connection
        .execute(
            "DELETE FROM openehr_composition_index WHERE version_uid IN \
             (SELECT uid FROM openehr_version WHERE trunk_version = 3)",
            [],
        )
        .expect("remove index row");
    connection
        .execute("DELETE FROM openehr_version WHERE trunk_version = 3", [])
        .expect("truncate");

    // The remaining chain is *perfectly consistent*. Every link still holds.
    let after = store.all_versions(&container).expect("all_versions");
    assert_eq!(after.len(), 2);
    let mut rebuilt = Chain::new();
    for row in &after {
        let content: serde_json::Value =
            serde_json::from_str(row.data_json.as_deref().expect("content")).expect("parses");
        rebuilt.append(row.uid.clone(), &Some(content), None).expect("append");
    }
    assert_eq!(
        rebuilt.head(),
        Digest256::from_bytes(after[1].chain.digest),
        "the truncated history verifies against itself — which is exactly the \
         problem, and why a checkpoint is not optional"
    );

    // Only the checkpoint, held elsewhere, shows the loss.
    let now = store.chain_checkpoint(&container).expect("checkpoint");
    assert!(now.starts_with("entries=2 "), "{now}");
    assert_ne!(
        before, now,
        "a checkpoint published before the truncation must not match after it"
    );
}
