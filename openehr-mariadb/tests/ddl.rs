//! The dialect's DDL, checked without a database.
//!
//! These are golden tests: they assert the SQL this crate emits, so a change to
//! a type mapping shows up as a diff in a test rather than as a migration that
//! fails in someone's staging environment.

use openehr_mariadb::MariadbDialect;
use openehr_store::{Dialect, conformance, ddl_script};

#[test]
fn the_dialect_is_self_consistent() {
    conformance::check_dialect(&MariadbDialect);
}

/// Fails if this crate starts emitting another engine's types.
///
/// This is the sibling FHIR monorepo's finding **F-08** stated as a test: that
/// port's Oracle DDL emitted `MySQL` types for as long as the fork existed,
/// because nothing ever looked.
#[test]
fn the_ddl_is_this_engine_and_not_another() {
    let sql = ddl_script(&MariadbDialect);
    assert!(sql.contains("VARCHAR(255)"));
    assert!(sql.contains("TINYINT(1)"), "MariaDB has no boolean");
    assert!(sql.contains("DATETIME(6)"));
    assert!(!sql.contains("jsonb"), "that is PostgreSQL");
    assert!(!sql.contains("VARCHAR2"), "that is Oracle");
    assert!(!sql.contains("nvarchar"), "that is SQL Server");
}

/// Fails if this crate drifts back into being `openehr-mysql` under another
/// name.
///
/// It was exactly that for its whole existence before this test: byte-identical
/// DDL, a struct still called `MysqlDialect`, and a conformance claim naming a
/// `MariaDB` 8.4 release that has never existed. The two assertions below are
/// the engine facts that separate the dialects, so a copy-paste regression
/// fails here rather than being discovered by an operator whose script died at
/// the first index.
#[test]
fn the_two_differences_from_mysql_are_present() {
    let sql = ddl_script(&MariadbDialect);
    // MariaDB has accepted `CREATE INDEX IF NOT EXISTS` since 10.0.5; MySQL
    // rejects it and must fold indexes into `CREATE TABLE` instead.
    assert!(
        sql.contains("INDEX IF NOT EXISTS"),
        "MariaDB supports CREATE INDEX IF NOT EXISTS — emitting inline keys is the MySQL workaround"
    );
    // MariaDB has had `CREATE OR REPLACE TRIGGER` since 10.1.4, so the
    // append-only guarantee is never dropped and recreated.
    assert!(
        sql.contains("CREATE OR REPLACE TRIGGER"),
        "MariaDB replaces triggers in one statement"
    );
    assert!(
        !sql.contains("DROP TRIGGER"),
        "dropping first would leave an interval in which an append-only table accepts UPDATE"
    );
}

/// Fails if an identifier stops being quoted this engine's way — the single
/// most common source of a script that parses everywhere except its target.
#[test]
fn identifiers_are_quoted_for_this_engine() {
    assert_eq!(MariadbDialect.quote("openehr_version"), "`openehr_version`");
    let sql = ddl_script(&MariadbDialect);
    assert!(sql.contains(&MariadbDialect.quote("openehr_composition_index")));
}

/// Fails if the schema stops covering every table, which would leave a store
/// that compiles and cannot write.
#[test]
fn every_table_and_index_is_emitted() {
    let sql = ddl_script(&MariadbDialect);
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
/// actually refuses `UPDATE` and `DELETE`. Engine detail: two triggers, like `MySQL`, but `CREATE OR REPLACE` rather than DROP-then-CREATE.
#[test]
fn the_dialect_names_itself_and_the_trigger_refuses_both_mutations() {
    assert_eq!(MariadbDialect.name(), "MariaDB");

    let sql = MariadbDialect.append_only_sql(&openehr_store::schema::VERSION).join("\n");
    assert!(sql.contains("BEFORE UPDATE"), "{sql}");
    assert!(sql.contains("BEFORE DELETE"), "{sql}");
    assert!(sql.contains("SIGNAL SQLSTATE '45000'"), "{sql}");
    assert!(sql.contains(openehr_store::schema::VERSION.name), "{sql}");
    assert!(sql.contains("openEHR V8.10"), "{sql}");
}

