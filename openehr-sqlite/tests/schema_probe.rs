//! The schema-version probes treat only "no such table" as absent (RCA-IN-6).
//! Any other failed read is an error, never a silent "fresh database".

use openehr_sqlite::SqliteStore;
use openehr_store::{Store as _, StoreError};
use rusqlite::Connection;

fn install_on(setup: &str) -> Result<(), StoreError> {
    let connection = Connection::open_in_memory().expect("open");
    connection.execute_batch(setup).expect("setup");
    SqliteStore::from_connection(connection)?.install()
}

#[test]
fn fresh_database_installs() {
    install_on("").expect("a database with no tables is fresh");
}

#[test]
fn version_table_with_wrong_shape_is_an_error_not_fresh() {
    // The table exists but lacks the `version` column: "no such column".
    let err = install_on("CREATE TABLE openehr_schema_version (other INTEGER);")
        .expect_err("a broken version table must not read as absent");
    assert!(matches!(err, StoreError::Engine { .. }), "{err:?}");
}

#[test]
fn legacy_probe_failure_other_than_missing_table_is_an_error() {
    // `openehr_ehr` is a view over a column that does not exist, so the
    // probe fails with "no such column", not "no such table".
    let err =
        install_on("CREATE TABLE t (x INTEGER); CREATE VIEW openehr_ehr AS SELECT nope FROM t;")
            .expect_err("a probe that fails for another reason must propagate");
    assert!(matches!(err, StoreError::Engine { .. }), "{err:?}");
}
