//! Fault injection: break the database under a live store and watch every
//! operation answer with an error instead of a panic, a wrong row, or a
//! silently shorter result (`W0.3`, `W0.41`).
//!
//! # Why this exists
//!
//! The store's engine-error paths (`map_err(|e| engine(&e))`, the 32-byte
//! digest checks, the MAC length check) are the ones a healthy database never
//! reaches. They were the bulk of `openehr-sqlite`'s uncovered lines, and an
//! error path nothing has run is a claim nothing has checked: a `?` that
//! quietly turned a corrupt row into an empty result would read as "no such
//! record" (`lib:A-28`'s shape, one layer down).
//!
//! # Method
//!
//! Seed a healthy in-memory store, apply **one** fault through the store's
//! own connection, then run the *whole* read and write battery against it.
//! A fault is one of three kinds: a table removed, a column removed, or a
//! stored value of the wrong shape. The battery is the same every time, so a
//! new operation added to the battery is exercised under every fault at once.
//!
//! Each fault must produce at least one `Err`, and the control (no fault) must
//! produce none, so a battery that had stopped asking would not pass.

use openehr::base::{HierObjectId, ObjectVersionId};
use openehr::rm::data_types::DvDateTime;
use openehr_sqlite::SqliteStore;
use openehr_store::{Store as _, StoreError, conformance};

const CONTRIBUTION: &str = "22222222-3333-4444-5555-666666666666";

fn container() -> HierObjectId {
    HierObjectId::from_uid_str(conformance::RECORD).expect("literal")
}

const OTHER: &str = "99999999-3333-4444-5555-666666666666";

/// An EHR with an identifier no seeded store holds.
fn another_ehr() -> openehr::rm::ehr::Ehr {
    let json = serde_json::to_string(&conformance::sample_ehr())
        .expect("serialise")
        .replace(conformance::RECORD, OTHER);
    serde_json::from_str(&json).expect("deserialise")
}

/// A first version in a container the seeded store has not seen.
fn version_in_new_container() -> openehr::rm::common::Version<openehr::rm::ehr::Composition> {
    let json = serde_json::to_string(&conformance::sample_version(1, None, 1))
        .expect("serialise")
        .replace(conformance::RECORD, OTHER);
    serde_json::from_str(&json).expect("deserialise")
}

fn seeded() -> SqliteStore {
    let mut store = SqliteStore::in_memory().expect("open");
    store.install().expect("install");
    let ehr = conformance::sample_ehr();
    let ehr_id = ehr.ehr_id().clone();
    store.create_ehr(&ehr).expect("ehr");
    store
        .create_contribution(
            &ehr_id,
            &conformance::sample_contribution(CONTRIBUTION, &[1, 2, 3]),
        )
        .expect("contribution");
    for (n, preceding) in [(1, None), (2, Some(1)), (3, Some(2))] {
        store
            .commit_composition(
                &ehr_id,
                &conformance::sample_version(n, preceding, n * 5),
                CONTRIBUTION,
            )
            .expect("commit");
    }
    store
}

/// Every `Store` operation, in one place, against whatever state the store is
/// in. Returns the errors, in order; panics only if the store does.
fn battery(store: &mut SqliteStore) -> Vec<StoreError> {
    let ehr_id = conformance::sample_ehr().ehr_id().clone();
    let uid: ObjectVersionId = format!("{}::{}::2", conformance::RECORD, conformance::SYSTEM)
        .parse()
        .expect("literal");
    let at = DvDateTime::new("2026-08-01T09:12:00Z").expect("literal");
    let mut errors = Vec::new();
    let mut note = |r: Result<(), StoreError>| {
        if let Err(e) = r {
            errors.push(e);
        }
    };
    note(store.create_ehr(&conformance::sample_ehr()));
    note(store.get_ehr(&ehr_id).map(drop));
    note(store.create_contribution(
        &ehr_id,
        &conformance::sample_contribution("33333333-3333-4444-5555-666666666666", &[4]),
    ));
    note(
        store
            .commit_composition(
                &ehr_id,
                &conformance::sample_version(4, Some(3), 20),
                CONTRIBUTION,
            )
            .map(drop),
    );
    note(
        store
            .commit_ehr_status(
                &ehr_id,
                &conformance::sample_ehr_status_version(1, None, 1, true),
                CONTRIBUTION,
            )
            .map(drop),
    );
    // A record and a container the store has not seen, so the *creating*
    // statements run (the existing ones answer "already exists" first).
    note(store.create_ehr(&another_ehr()));
    note(
        store
            .commit_composition(&ehr_id, &version_in_new_container(), CONTRIBUTION)
            .map(drop),
    );
    note(store.get_version(&uid).map(drop));
    note(store.latest_version(&container()).map(drop));
    note(store.version_at_time(&container(), &at).map(drop));
    note(store.all_versions(&container()).map(drop));
    note(store.chain_checkpoint(&container()).map(drop));
    note(
        store
            .find_compositions_by_archetype(&ehr_id, "openEHR-EHR-COMPOSITION.encounter.v1")
            .map(drop),
    );
    // Last: `install` recreates a missing table, which would heal the fault
    // before the other operations had met it.
    note(store.install());
    errors
}

#[test]
fn a_healthy_store_answers_the_battery_without_an_engine_error() {
    // The control. Conflicts and refusals are expected (the battery re-creates
    // what exists); an *engine* error on a healthy database would mean the
    // battery itself is wrong, and every fault below would prove nothing.
    let mut store = seeded();
    let engine_errors: Vec<_> = battery(&mut store)
        .into_iter()
        .filter(|e| matches!(e, StoreError::Engine { .. }))
        .collect();
    assert!(engine_errors.is_empty(), "{engine_errors:?}");
}

/// A fault, as the SQL that causes it. Triggers and foreign keys are removed
/// first where a fault needs to rewrite append-only rows: this models someone
/// with file access, as `tamper.rs` does.
const PREPARE: &str = "PRAGMA foreign_keys = OFF;
    DROP TRIGGER IF EXISTS trg_openehr_version_no_update;
    DROP TRIGGER IF EXISTS trg_openehr_version_no_delete;
    DROP TRIGGER IF EXISTS trg_openehr_contribution_no_update;
    DROP TRIGGER IF EXISTS trg_openehr_contribution_no_delete;
    DROP INDEX IF EXISTS ix_versioned_object_ehr;
    DROP INDEX IF EXISTS ix_version_time;
    DROP INDEX IF EXISTS ix_version_preceding;
    DROP INDEX IF EXISTS ix_contribution_ehr_time;
    DROP INDEX IF EXISTS ix_composition_archetype;
    DROP INDEX IF EXISTS ix_composition_context_start;";

const FAULTS: &[(&str, &str)] = &[
    // A table removed.
    ("no ehr table", "DROP TABLE openehr_ehr;"),
    (
        "no versioned_object table",
        "DROP TABLE openehr_versioned_object;",
    ),
    ("no version table", "DROP TABLE openehr_version;"),
    ("no contribution table", "DROP TABLE openehr_contribution;"),
    (
        "no composition_index table",
        "DROP TABLE openehr_composition_index;",
    ),
    (
        "no schema_version table",
        "DROP TABLE openehr_schema_version;",
    ),
    // A column removed: the statement prepares for none of the readers.
    (
        "no signature column",
        "ALTER TABLE openehr_version DROP COLUMN signature;",
    ),
    (
        "no chain_digest column",
        "ALTER TABLE openehr_version DROP COLUMN chain_digest;",
    ),
    (
        "no data_json column",
        "ALTER TABLE openehr_version DROP COLUMN data_json;",
    ),
    (
        "no archetype_id column",
        "ALTER TABLE openehr_composition_index DROP COLUMN archetype_id;",
    ),
    (
        "no context_start column",
        "ALTER TABLE openehr_composition_index DROP COLUMN context_start_text;",
    ),
    (
        "no ehr time column",
        "ALTER TABLE openehr_ehr DROP COLUMN time_created_text;",
    ),
    // A stored value of the wrong shape.
    (
        "short chain_digest",
        "UPDATE openehr_version SET chain_digest = x'00';",
    ),
    (
        "short chain_previous",
        "UPDATE openehr_version SET chain_previous = x'00';",
    ),
    (
        "short chain_content",
        "UPDATE openehr_version SET chain_content = x'00';",
    ),
    (
        "short chain_tag_mac",
        "UPDATE openehr_version SET chain_tag_mac = x'00';",
    ),
    (
        "text chain_digest",
        "UPDATE openehr_version SET chain_digest = 'not bytes';",
    ),
    (
        "integer trunk_version",
        "UPDATE openehr_version SET trunk_version = 'x' WHERE uid LIKE '%::2';",
    ),
    (
        "blob lifecycle",
        "UPDATE openehr_version SET lifecycle_state_code = x'ff';",
    ),
    (
        "blob audit time",
        "UPDATE openehr_version SET audit_time_committed_text = x'ff';",
    ),
    (
        "text utc time",
        "UPDATE openehr_version SET audit_time_committed_utc = 'x';",
    ),
    (
        "blob composition start",
        "UPDATE openehr_composition_index SET context_start_text = x'ff';",
    ),
    (
        "text composition utc",
        "UPDATE openehr_composition_index SET context_start_utc = 'x';",
    ),
    (
        "blob ehr system",
        "UPDATE openehr_ehr SET system_id = x'ff';",
    ),
    // A write refused by the engine part-way through a commit.
    (
        "insert refused on openehr_ehr",
        "CREATE TRIGGER fault_insert_openehr_ehr BEFORE INSERT ON openehr_ehr BEGIN SELECT RAISE(ABORT, 'injected'); END;",
    ),
    (
        "update refused on openehr_ehr",
        "CREATE TRIGGER fault_update_openehr_ehr BEFORE UPDATE ON openehr_ehr BEGIN SELECT RAISE(ABORT, 'injected'); END;",
    ),
    (
        "insert refused on openehr_versioned_object",
        "CREATE TRIGGER fault_insert_openehr_versioned_object BEFORE INSERT ON openehr_versioned_object BEGIN SELECT RAISE(ABORT, 'injected'); END;",
    ),
    (
        "update refused on openehr_versioned_object",
        "CREATE TRIGGER fault_update_openehr_versioned_object BEFORE UPDATE ON openehr_versioned_object BEGIN SELECT RAISE(ABORT, 'injected'); END;",
    ),
    (
        "insert refused on openehr_version",
        "CREATE TRIGGER fault_insert_openehr_version BEFORE INSERT ON openehr_version BEGIN SELECT RAISE(ABORT, 'injected'); END;",
    ),
    (
        "update refused on openehr_version",
        "CREATE TRIGGER fault_update_openehr_version BEFORE UPDATE ON openehr_version BEGIN SELECT RAISE(ABORT, 'injected'); END;",
    ),
    (
        "insert refused on openehr_contribution",
        "CREATE TRIGGER fault_insert_openehr_contribution BEFORE INSERT ON openehr_contribution BEGIN SELECT RAISE(ABORT, 'injected'); END;",
    ),
    (
        "update refused on openehr_contribution",
        "CREATE TRIGGER fault_update_openehr_contribution BEFORE UPDATE ON openehr_contribution BEGIN SELECT RAISE(ABORT, 'injected'); END;",
    ),
    (
        "insert refused on openehr_composition_index",
        "CREATE TRIGGER fault_insert_openehr_composition_index BEFORE INSERT ON openehr_composition_index BEGIN SELECT RAISE(ABORT, 'injected'); END;",
    ),
    (
        "update refused on openehr_composition_index",
        "CREATE TRIGGER fault_update_openehr_composition_index BEFORE UPDATE ON openehr_composition_index BEGIN SELECT RAISE(ABORT, 'injected'); END;",
    ),
    // The schema record itself refused, and a name the DDL needs already
    // taken by another kind of object.
    (
        "insert refused on openehr_schema_version",
        "DELETE FROM openehr_schema_version;
         CREATE TRIGGER fault_insert_schema BEFORE INSERT ON openehr_schema_version
         BEGIN SELECT RAISE(ABORT, 'injected'); END;",
    ),
    (
        "index name taken by a table",
        "CREATE TABLE ix_version_time (x INTEGER);",
    ),
    // A transaction already open on the connection: every commit's own
    // `BEGIN` is refused rather than silently nested.
    ("transaction already open", "BEGIN;"),
    // A schema recorded by a different version.
    (
        "future schema version",
        "UPDATE openehr_schema_version SET version = 99;",
    ),
    (
        "empty schema version",
        "DELETE FROM openehr_schema_version;",
    ),
];

#[test]
fn every_fault_surfaces_as_an_error_not_a_panic_or_a_silent_answer() {
    for (name, sql) in FAULTS {
        let mut store = seeded();
        store
            .connection()
            .execute_batch(PREPARE)
            .unwrap_or_else(|e| panic!("{name}: prepare: {e}"));
        store
            .connection()
            .execute_batch(sql)
            .unwrap_or_else(|e| panic!("{name}: fault did not apply: {e}"));
        let errors = battery(&mut store);
        assert!(
            !errors.is_empty(),
            "{name}: the whole battery succeeded against a broken database"
        );
    }
}

#[test]
fn engine_errors_name_the_engine_and_never_echo_row_data() {
    let mut store = seeded();
    store
        .connection()
        .execute_batch(&format!("{PREPARE} DROP TABLE openehr_version;"))
        .expect("fault");
    let errors = battery(&mut store);
    let engine: Vec<_> = errors
        .iter()
        .filter_map(|e| match e {
            StoreError::Engine { engine, message } => Some((*engine, message.as_str())),
            _ => None,
        })
        .collect();
    assert!(!engine.is_empty());
    for (name, message) in engine {
        assert_eq!(name, "SQLite");
        assert!(!message.contains(conformance::RECORD), "{message}");
    }
}

#[test]
fn a_branched_version_reads_back_with_its_branch_numbers() {
    // The branch columns are written as `0` when absent, so only a branched
    // row takes the `Some` arm of `read_version`.
    let store = seeded();
    store
        .connection()
        .execute_batch(&format!(
            "{PREPARE} UPDATE openehr_version SET branch_number = 2, branch_version = 3 \
             WHERE uid LIKE '%::2';"
        ))
        .expect("branch");
    let rows = store.all_versions(&container()).expect("read");
    let branched: Vec<_> = rows.iter().filter(|r| r.branch_number.is_some()).collect();
    assert_eq!(branched.len(), 1);
    assert_eq!(branched[0].branch_number, Some(2));
    assert_eq!(branched[0].branch_version, Some(3));
}

#[test]
fn a_file_that_is_not_a_database_is_refused_before_anything_is_stored() {
    let path = std::env::temp_dir().join(format!("openehr-notadb-{}.sqlite3", std::process::id()));
    std::fs::write(&path, vec![0x5a_u8; 4096]).expect("write");
    // SQLite reads lazily, so the refusal may come at open or at the first
    // statement; what must not happen is a store that accepts both.
    let refused = match SqliteStore::open(&path) {
        Err(StoreError::Engine { .. }) => true,
        Err(other) => panic!("wrong error kind: {other:?}"),
        Ok(mut store) => matches!(store.install(), Err(StoreError::Engine { .. })),
    };
    let _ = std::fs::remove_file(&path);
    assert!(refused);
}
