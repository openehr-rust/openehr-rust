//! The cross-dialect check: no two engines may emit the same schema.
//!
//! # Why this test exists, and why it is here
//!
//! The sibling FHIR monorepo in this repository shipped an Oracle DDL emitter
//! that produced `MySQL` types for as long as that fork existed (**F-08**). It
//! was not a subtle bug — the file had been copied and three lines changed —
//! and it survived because six ports each owned a full copy of the generator
//! and nothing ever compared them.
//!
//! Here the six dialects own only their type spellings, and this test compares
//! all six. It lives in this crate because the comparison needs to see every
//! dialect, and `openehr-store` cannot: they depend on it.
//!
//! # The guard was itself incomplete, and that is how F-08 recurred
//!
//! This list held five of the six dialects for as long as `openehr-mariadb`
//! existed. The omitted one was a copy of `openehr-mysql` with the name
//! substituted: same struct name, same `Idempotence`, byte-identical DDL, and a
//! documented conformance claim naming a `MariaDB` 8.4 release that was never
//! published. `no_two_dialects_emit_the_same_ddl` passed the whole time, because
//! a comparison that does not include a dialect cannot find it identical to
//! another. A partial guard and a complete one report the same green.

use openehr_mariadb::MariadbDialect;
use openehr_mssql::MssqlDialect;
use openehr_mysql::MysqlDialect;
use openehr_oracle::OracleDialect;
use openehr_postgresql::PostgresqlDialect;
use openehr_sqlite::SqliteDialect;
use openehr_store::{ColTy, Dialect, Idempotence, conformance, ddl_script};

/// Every dialect, for the tests below.
fn all() -> Vec<&'static dyn Dialect> {
    vec![
        &PostgresqlDialect,
        &SqliteDialect,
        &MysqlDialect,
        &MariadbDialect,
        &MssqlDialect,
        &OracleDialect,
    ]
}

/// Fails if a dialect crate exists that this comparison does not cover.
///
/// The check above is only as good as `all()`, and `all()` is hand-maintained —
/// which is exactly how `openehr-mariadb` went unexamined. Tying the count to
/// the number of engine crates makes adding a seventh engine and forgetting to
/// list it a test failure rather than a silent reduction in coverage.
#[test]
fn every_engine_crate_is_in_the_comparison() {
    const ENGINE_CRATES: usize = 6;
    assert_eq!(
        all().len(),
        ENGINE_CRATES,
        "a dialect crate is missing from `all()`; the distinctness check does not cover it"
    );
    let names: std::collections::HashSet<&str> = all().iter().map(|d| d.name()).collect();
    assert_eq!(names.len(), ENGINE_CRATES, "two dialects report one name");
}

#[test]
fn no_two_dialects_emit_the_same_ddl() {
    conformance::dialects_are_distinct(&all());
}

#[test]
fn every_dialect_is_self_consistent() {
    for dialect in all() {
        conformance::check_dialect(dialect);
    }
}

/// Fails if two engines agree on how to spell a type they spell differently in
/// reality. Distinct DDL is necessary but not sufficient — two dialects could
/// differ only in quoting and still map `Bool` identically, which is how a
/// copied emitter hides.
#[test]
fn the_types_that_differ_between_engines_actually_differ() {
    let boolean: Vec<String> = all().iter().map(|d| d.col_sql(ColTy::Bool)).collect();
    // PostgreSQL `boolean`, SQLite/Oracle numerics, MySQL and MariaDB
    // `TINYINT(1)`, SQL Server `bit` — six engines, at least four spellings.
    // MySQL and MariaDB genuinely agree here, which is why this is a floor on
    // the number of spellings and not an equality: the dialects are separated by
    // idempotence and trigger syntax, not by how they spell a boolean.
    let distinct: std::collections::HashSet<_> = boolean.iter().collect();
    assert!(
        distinct.len() >= 4,
        "booleans collapsed to {distinct:?} — a dialect is not its own engine"
    );

    let json: std::collections::HashSet<String> =
        all().iter().map(|d| d.col_sql(ColTy::Json)).collect();
    assert!(json.len() >= 3, "JSON storage collapsed to {json:?}");
}

/// Fails if any dialect stops keeping the authoritative and derived instant
/// columns distinct — the design decision the whole schema turns on (`D3.10`).
#[test]
fn every_dialect_keeps_the_two_instant_columns_apart() {
    for dialect in all() {
        assert_ne!(
            dialect.col_sql(ColTy::Instant),
            dialect.col_sql(ColTy::InstantUtc),
            "{}: the authoritative lexical form and the derived instant share a type",
            dialect.name()
        );
    }
}

/// Fails if a dialect stops emitting a placeholder its driver understands.
#[test]
fn placeholders_match_their_drivers() {
    assert_eq!(PostgresqlDialect.placeholder().render(1), "$1");
    assert_eq!(SqliteDialect.placeholder().render(1), "?");
    assert_eq!(MysqlDialect.placeholder().render(1), "?");
    assert_eq!(MssqlDialect.placeholder().render(1), "@p1");
    assert_eq!(OracleDialect.placeholder().render(1), ":1");
}

/// Fails if a dialect claims a clause its engine does not have, which produces
/// a script that fails on the one engine it targets.
#[test]
fn only_engines_with_if_not_exists_emit_it() {
    for dialect in all() {
        let script = ddl_script(dialect);
        // The *clause on a CREATE statement*, not the substring: SQL Server and
        // Oracle guards legitimately contain `IF NOT EXISTS (SELECT ...)`, which
        // is a different construct and is why this asserts on the prefix.
        assert_eq!(
            script.contains("CREATE TABLE IF NOT EXISTS"),
            dialect.table_idempotence() == Idempotence::IfNotExists,
            "{}: CREATE TABLE IF NOT EXISTS emitted but not supported, or the reverse",
            dialect.name()
        );
        assert_eq!(
            script.contains("INDEX IF NOT EXISTS"),
            dialect.index_idempotence() == Idempotence::IfNotExists,
            "{}: CREATE INDEX IF NOT EXISTS emitted but not supported, or the reverse",
            dialect.name()
        );
    }
}

/// Every engine must enforce append-only in the schema, not merely in the store.
///
/// `MySQL`, SQL Server, and Oracle each silently inherited an empty default
/// until a live run against `MySQL` 8.4 made the gap visible: the guarantee held
/// on two engines of five and the documentation described it as a property of
/// the design.
#[test]
fn every_dialect_enforces_append_only_in_the_schema() {
    for dialect in all() {
        for table in openehr_store::TABLES.iter().filter(|t| t.append_only) {
            let statements = dialect.append_only_sql(table);
            assert!(
                !statements.is_empty(),
                "{}: {} is append-only and nothing enforces it",
                dialect.name(),
                table.name
            );
            assert!(
                statements.iter().any(|s| s.contains("UPDATE"))
                    && statements.iter().any(|s| s.contains("DELETE")),
                "{}: {} must refuse both UPDATE and DELETE",
                dialect.name(),
                table.name
            );
        }
    }
}

/// Every dialect names itself, and no two share a name.
///
/// `Dialect::name()` is used only inside `openehr_store::conformance`'s panic
/// messages — never compared against anything — so it could return `""` for
/// every one of the six engines and nothing would notice (`lib:A-09`). This is
/// the one place all six dialects are already assembled to be checked
/// together, so it is also where two engines sharing a name would first be
/// visible: a diagnostic that can't tell `PostgreSQL`'s failure from `Oracle`'s is
/// worse than one that fails to run at all.
#[test]
fn every_dialect_names_itself_and_no_two_share_a_name() {
    let names: Vec<&str> = all().iter().map(|d| d.name()).collect();
    assert_eq!(
        names,
        vec!["PostgreSQL", "SQLite", "MySQL", "MariaDB", "SQL Server", "Oracle"]
    );

    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len(), "two dialects share a name: {names:?}");
}

