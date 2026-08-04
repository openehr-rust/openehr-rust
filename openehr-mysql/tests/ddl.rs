//! The dialect's DDL, checked without a database.
//!
//! These are golden tests: they assert the SQL this crate emits, so a change to
//! a type mapping shows up as a diff in a test rather than as a migration that
//! fails in someone's staging environment.

use openehr_mysql::MysqlDialect;
use openehr_store::{Dialect, conformance, ddl_script};

#[test]
fn the_dialect_is_self_consistent() {
    conformance::check_dialect(&MysqlDialect);
}

/// Fails if this crate starts emitting another engine's types.
///
/// This is the sibling FHIR monorepo's finding **F-08** stated as a test: that
/// port's Oracle DDL emitted `MySQL` types for as long as the fork existed,
/// because nothing ever looked.
#[test]
fn the_ddl_is_this_engine_and_not_another() {
    let sql = ddl_script(&MysqlDialect);
    assert!(sql.contains("VARCHAR(255)"));
    assert!(sql.contains("TINYINT(1)"), "MySQL has no boolean");
    assert!(sql.contains("DATETIME(6)"));
    assert!(!sql.contains("jsonb"), "that is PostgreSQL");
    assert!(!sql.contains("VARCHAR2"), "that is Oracle");
    assert!(!sql.contains("nvarchar"), "that is SQL Server");
}

/// Fails if an identifier stops being quoted this engine's way — the single
/// most common source of a script that parses everywhere except its target.
#[test]
fn identifiers_are_quoted_for_this_engine() {
    assert_eq!(MysqlDialect.quote("openehr_version"), "`openehr_version`");
    let sql = ddl_script(&MysqlDialect);
    assert!(sql.contains(&MysqlDialect.quote("openehr_composition_index")));
}

/// Fails if the schema stops covering every table, which would leave a store
/// that compiles and cannot write.
#[test]
fn every_table_and_index_is_emitted() {
    let sql = ddl_script(&MysqlDialect);
    for table in openehr_store::TABLES {
        assert!(sql.contains(table.name), "missing table {}", table.name);
        for index in table.indexes {
            assert!(sql.contains(index.name), "missing index {}", index.name);
        }
    }
}

/// The dialect names itself, and its append-only trigger names both the
/// mutation it refuses and the table it protects.
///
/// `name` could return `""` or a wrong constant, and `append_only_sql`
/// could return an empty or nonsense string, with nothing failing
/// (`lib:A-09`) — the generated SQL is asserted against structurally
/// elsewhere in this file, but never against its own stated purpose: that it
/// actually refuses `UPDATE` and `DELETE`. Engine detail: two triggers, one per operation — SIGNAL cannot name both at once.
#[test]
fn the_dialect_names_itself_and_the_trigger_refuses_both_mutations() {
    assert_eq!(MysqlDialect.name(), "MySQL");

    let sql = MysqlDialect.append_only_sql(&openehr_store::schema::VERSION).join("\n");
    assert!(sql.contains("BEFORE UPDATE"), "{sql}");
    assert!(sql.contains("BEFORE DELETE"), "{sql}");
    assert!(sql.contains("SIGNAL SQLSTATE '45000'"), "{sql}");
    assert!(sql.contains(openehr_store::schema::VERSION.name), "{sql}");
    assert!(sql.contains("openEHR V8.10"), "{sql}");
}

