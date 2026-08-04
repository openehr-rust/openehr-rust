//! The dialect's DDL, checked without a database.
//!
//! These are golden tests: they assert the SQL this crate emits, so a change to
//! a type mapping shows up as a diff in a test rather than as a migration that
//! fails in someone's staging environment.

use openehr_postgresql::PostgresqlDialect;
use openehr_store::{Dialect, conformance, ddl_script};

#[test]
fn the_dialect_is_self_consistent() {
    conformance::check_dialect(&PostgresqlDialect);
}

/// Fails if this crate starts emitting another engine's types.
///
/// This is the sibling FHIR monorepo's finding **F-08** stated as a test: that
/// port's Oracle DDL emitted `MySQL` types for as long as the fork existed,
/// because nothing ever looked.
#[test]
fn the_ddl_is_this_engine_and_not_another() {
    let sql = ddl_script(&PostgresqlDialect);
    // Was `assert!(sql.contains("jsonb"))` until `D-08`. `jsonb` reorders keys
    // and reinserts whitespace — measured on PostgreSQL 18 — so the bytes the
    // content digest was taken over cannot be read back out (`M3.43`).
    assert!(
        !sql.contains("jsonb"),
        "jsonb normalises canonical JSON, so the chain digest cannot be \
         recomputed from storage (M3.43, D-08)"
    );
    assert!(
        sql.lines()
            .any(|line| line.contains("data_json") && line.contains("text")),
        "canonical JSON must be held in a byte-preserving text column"
    );
    assert!(sql.contains("timestamptz"));
    assert!(!sql.contains("VARCHAR2"), "that is Oracle");
    assert!(!sql.contains("nvarchar"), "that is SQL Server");
    assert!(!sql.contains("TINYINT"), "that is MySQL");
    assert!(!sql.contains('`'), "backticks are MySQL quoting");
}

/// Fails if an identifier stops being quoted this engine's way — the single
/// most common source of a script that parses everywhere except its target.
#[test]
fn identifiers_are_quoted_for_this_engine() {
    assert_eq!(
        PostgresqlDialect.quote("openehr_version"),
        "\"openehr_version\""
    );
    let sql = ddl_script(&PostgresqlDialect);
    assert!(sql.contains(&PostgresqlDialect.quote("openehr_composition_index")));
}

/// Fails if the schema stops covering every table, which would leave a store
/// that compiles and cannot write.
#[test]
fn every_table_and_index_is_emitted() {
    let sql = ddl_script(&PostgresqlDialect);
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
/// actually refuses `UPDATE` and `DELETE`. Engine detail: one trigger, both operations in one clause.
#[test]
fn the_dialect_names_itself_and_the_trigger_refuses_both_mutations() {
    assert_eq!(PostgresqlDialect.name(), "PostgreSQL");

    let sql = PostgresqlDialect.append_only_sql(&openehr_store::schema::VERSION).join("\n");
    assert!(sql.contains("BEFORE UPDATE OR DELETE"), "{sql}");
    assert!(sql.contains("RAISE EXCEPTION"), "{sql}");
    assert!(sql.contains(openehr_store::schema::VERSION.name), "{sql}");
    assert!(sql.contains("openEHR V8.10"), "{sql}");
}

