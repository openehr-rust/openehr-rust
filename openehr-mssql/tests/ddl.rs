//! The dialect's DDL, checked without a database.
//!
//! These are golden tests: they assert the SQL this crate emits, so a change to
//! a type mapping shows up as a diff in a test rather than as a migration that
//! fails in someone's staging environment.

use openehr_mssql::MssqlDialect;
use openehr_store::{Dialect, conformance, ddl_script};

#[test]
fn the_dialect_is_self_consistent() {
    conformance::check_dialect(&MssqlDialect);
}

/// Fails if this crate starts emitting another engine's types.
///
/// This is the sibling FHIR monorepo's finding **F-08** stated as a test: that
/// port's Oracle DDL emitted `MySQL` types for as long as the fork existed,
/// because nothing ever looked.
#[test]
fn the_ddl_is_this_engine_and_not_another() {
    let sql = ddl_script(&MssqlDialect);
    assert!(sql.contains("nvarchar(max)"));
    assert!(sql.contains("datetimeoffset(7)"));
    assert!(sql.contains("bit"));
    assert!(
        !sql.contains("CREATE TABLE IF NOT EXISTS") && !sql.contains("INDEX IF NOT EXISTS"),
        "SQL Server has no such clause"
    );
    // It reaches idempotence by a catalogue guard instead, and that guard must
    // actually be emitted — declaring one and inheriting the no-op default is
    // how this crate shipped non-idempotent DDL.
    assert!(sql.contains("IF NOT EXISTS (SELECT 1 FROM sys.objects"));
    assert!(sql.contains("IF NOT EXISTS (SELECT 1 FROM sys.indexes"));
    assert!(!sql.contains("jsonb"), "that is PostgreSQL");
    assert!(!sql.contains("VARCHAR2"), "that is Oracle");
}

/// Fails if an identifier stops being quoted this engine's way — the single
/// most common source of a script that parses everywhere except its target.
#[test]
fn identifiers_are_quoted_for_this_engine() {
    assert_eq!(MssqlDialect.quote("openehr_version"), "[openehr_version]");
    let sql = ddl_script(&MssqlDialect);
    assert!(sql.contains(&MssqlDialect.quote("openehr_composition_index")));
}

/// Fails if the schema stops covering every table, which would leave a store
/// that compiles and cannot write.
#[test]
fn every_table_and_index_is_emitted() {
    let sql = ddl_script(&MssqlDialect);
    for table in openehr_store::TABLES {
        assert!(sql.contains(table.name), "missing table {}", table.name);
        for index in table.indexes {
            assert!(sql.contains(index.name), "missing index {}", index.name);
        }
    }
}
