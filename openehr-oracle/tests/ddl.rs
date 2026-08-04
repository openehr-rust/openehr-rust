//! The dialect's DDL, checked without a database.
//!
//! These are golden tests: they assert the SQL this crate emits, so a change to
//! a type mapping shows up as a diff in a test rather than as a migration that
//! fails in someone's staging environment.

use openehr_oracle::OracleDialect;
use openehr_store::{Dialect, conformance, ddl_script};

#[test]
fn the_dialect_is_self_consistent() {
    conformance::check_dialect(&OracleDialect);
}

/// Fails if this crate starts emitting another engine's types.
///
/// This is the sibling FHIR monorepo's finding **F-08** stated as a test: that
/// port's Oracle DDL emitted `MySQL` types for as long as the fork existed,
/// because nothing ever looked.
#[test]
fn the_ddl_is_this_engine_and_not_another() {
    let sql = ddl_script(&OracleDialect);
    assert!(sql.contains("VARCHAR2(255 CHAR)"));
    assert!(sql.contains("NUMBER(19)"), "Oracle has no integer type");
    assert!(sql.contains("CLOB"));
    assert!(sql.contains("TIMESTAMP WITH TIME ZONE"));
    assert!(!sql.contains("IF NOT EXISTS"), "Oracle has no such clause");
    assert!(!sql.contains("jsonb"), "that is PostgreSQL");
    assert!(
        !sql.contains("TINYINT"),
        "that is MySQL — the exact F-08 defect"
    );
}

/// Fails if an identifier stops being quoted this engine's way — the single
/// most common source of a script that parses everywhere except its target.
#[test]
fn identifiers_are_quoted_for_this_engine() {
    assert_eq!(
        OracleDialect.quote("openehr_version"),
        "\"openehr_version\""
    );
    let sql = ddl_script(&OracleDialect);
    assert!(sql.contains(&OracleDialect.quote("openehr_composition_index")));
}

/// Fails if the schema stops covering every table, which would leave a store
/// that compiles and cannot write.
#[test]
fn every_table_and_index_is_emitted() {
    let sql = ddl_script(&OracleDialect);
    for table in openehr_store::TABLES {
        assert!(sql.contains(table.name), "missing table {}", table.name);
        for index in table.indexes {
            assert!(sql.contains(index.name), "missing index {}", index.name);
        }
    }
}

/// Fails if any identifier in the shared schema exceeds Oracle's limit.
///
/// Oracle is the only one of the five with a limit short enough to hit, and an
/// over-long name fails at `CREATE TABLE` time and nowhere else — so a schema
/// change made against another engine would break here and only here.
#[test]
fn no_identifier_is_too_long_for_oracle() {
    let mut checked = 0usize;
    for table in openehr_store::TABLES {
        let check = |name: &str, what: &str| {
            assert!(
                name.len() <= OracleDialect::MAX_IDENTIFIER,
                "{what} `{name}` is {} characters, over Oracle's {}",
                name.len(),
                OracleDialect::MAX_IDENTIFIER
            );
        };
        check(table.name, "table");
        checked += 1;
        for column in table.columns {
            check(column.name, "column");
            checked += 1;
        }
        for index in table.indexes {
            check(index.name, "index");
            checked += 1;
        }
    }
    assert!(
        checked > 40,
        "the schema shrank unexpectedly; only {checked} names checked"
    );
}

/// The dialect names itself, and its append-only trigger names both the
/// mutation it refuses and the table it protects.
///
/// `name` could return `""` or a wrong constant, and `append_only_sql`
/// could return an empty or nonsense string, with nothing failing
/// (`lib:A-09`) — the generated SQL is asserted against structurally
/// elsewhere in this file, but never against its own stated purpose: that it
/// actually refuses `UPDATE` and `DELETE`. Engine detail: one trigger, PL/SQL `raise_application_error`.
#[test]
fn the_dialect_names_itself_and_the_trigger_refuses_both_mutations() {
    assert_eq!(OracleDialect.name(), "Oracle");

    let sql = OracleDialect.append_only_sql(&openehr_store::schema::VERSION).join("\n");
    assert!(sql.contains("BEFORE UPDATE OR DELETE"), "{sql}");
    assert!(sql.contains("raise_application_error"), "{sql}");
    assert!(sql.contains(openehr_store::schema::VERSION.name), "{sql}");
    assert!(sql.contains("openEHR V8.10"), "{sql}");
}

/// Every statement ends `\n/`, the SQL*Plus block terminator, not `;`.
///
/// `terminator` could return `""` or nonsense with nothing failing
/// (`lib:A-09`): every statement here is a PL/SQL block, and a bare `;`
/// submits only as far as the block's first inner semicolon.
#[test]
fn statements_terminate_with_the_sqlplus_block_marker() {
    assert_eq!(OracleDialect.terminator(), "\n/");
    let sql = ddl_script(&OracleDialect);
    assert!(sql.contains("\n/"), "{sql}");
}

