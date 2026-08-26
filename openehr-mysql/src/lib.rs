//! openEHR persistence for **`MySQL` 8.4**.
//!
//! One [`Dialect`]; the model, projection, commit rules, and conformance suite
//! are in [`openehr_store`].
//!
//! # Trademarks
//!
//! openEHR® is the registered trademark of the openEHR Foundation. Use of
//! the trademark does not constitute endorsement of this product by openEHR
//! International or openEHR Foundation.
//!
//! # Conformance level: **Schema**
//!
//! No driver and no [`openehr_store::Store`] — but the DDL has been executed
//! against **`MySQL` 8.4**, re-applied as a no-op, and its append-only triggers
//! observed refusing `UPDATE` and `DELETE`. Reproduce with
//! `openehr-store/scripts/verify-schema.sh mysql`.
//!
//! That run found two defects the golden tests could not (**A-13**, **A-15**):
//! `MySQL` rejects `CREATE INDEX IF NOT EXISTS`, and this dialect enforced
//! append-only nowhere. Both are fixed above.
//!
//! ```
//! use openehr_mysql::MysqlDialect;
//! use openehr_store::{ColTy, Dialect};
//!
//! // Lengths are mandatory here, unlike `PostgreSQL`: an unbounded column
//! // cannot be indexed.
//! assert_eq!(MysqlDialect.col_sql(ColTy::Id(255)), "VARCHAR(255)");
//! assert_eq!(MysqlDialect.quote("openehr_version"), "`openehr_version`");
//! ```

#![forbid(unsafe_code)]

use openehr_store::{ColTy, Dialect, Idempotence, Placeholder, Table};

/// The `MySQL` dialect.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MysqlDialect;

impl Dialect for MysqlDialect {
    fn name(&self) -> &'static str {
        "MySQL"
    }

    // `ColTy::Json` and the general text arm now spell the same type, and they
    // stay separate arms (`M3.43`). They coincide for unrelated reasons: one is
    // "this engine has no reason to bound these", the other is "canonical JSON
    // must return the bytes it was given". Merging them would delete the second
    // reason and silently couple the two, so that changing the text arm would
    // move the JSON column with it — which is how `D-08` happened in reverse.
    #[allow(clippy::match_same_arms)]
    fn col_sql(&self, ty: ColTy) -> String {
        match ty {
            // Bounded, and the bound is load-bearing: InnoDB cannot index an
            // unbounded column, and every `Id` column in the schema is either a
            // key or part of one.
            ColTy::Id(n) | ColTy::Text(n) => return format!("VARCHAR({n})"),
            ColTy::LongText => "LONGTEXT",
            // `LONGTEXT`, not `JSON` (`M3.43`). MySQL's `JSON` is a binary
            // type that normalises on the way in: it reorders keys and — the
            // reason this is a defect and not a preference — **rewrote a
            // magnitude of `1.10` as `1.1`**, measured on MySQL 8.4.
            //
            // In openEHR that trailing zero is precision. `1.10 mg` asserts two
            // decimal places and `1.1 mg` asserts one, so the column was
            // changing what the record claimed about itself, whether or not
            // anything hashed it. `lib:J9.13` already forbade it: numbers MUST
            // NOT be renormalised, measured precision is data.
            //
            // Note that `openehr-mariadb` spells this differently and always
            // round-tripped, because MariaDB's `JSON` is an alias for
            // `LONGTEXT`. Two engines, one type name, opposite behaviour — see
            // `D-08`.
            ColTy::Json => "LONGTEXT",
            // 64 characters: the longest ISO 8601 form this crate accepts is a
            // date-time with fractional seconds and an offset, well under half
            // that. Bounded rather than TEXT so it can be indexed alongside the
            // derived column.
            ColTy::Instant => "VARCHAR(64)",
            // Microsecond precision, which is MySQL's maximum. openEHR permits
            // finer fractional seconds in the lexical form — which is why that
            // form is stored separately and authoritatively (`D3.10`).
            ColTy::InstantUtc => "DATETIME(6)",
            ColTy::Int => "BIGINT",
            // MySQL has no boolean; TINYINT(1) is the conventional spelling and
            // is what every driver maps to `bool`.
            ColTy::Bool => "TINYINT(1)",
            // `BINARY(32)`, not `VARBINARY`: the width is exact and fixed, so
            // the engine rejects a wrong-length digest rather than storing it
            // (`M3.41`).
            ColTy::Digest => "BINARY(32)",
        }
        .to_owned()
    }

    fn quote(&self, identifier: &str) -> String {
        format!("`{}`", identifier.replace('`', "``"))
    }

    fn placeholder(&self) -> Placeholder {
        Placeholder::Question
    }

    /// Indexes are declared inside `CREATE TABLE`.
    ///
    /// `MySQL` accepts `CREATE TABLE IF NOT EXISTS` and rejects
    /// `CREATE INDEX IF NOT EXISTS` — verified against `MySQL` 8.4, which failed
    /// the generated script at the first index with `ERROR 1064`. Declaring
    /// the index inside the table makes it inherit the table's idempotence.
    fn index_idempotence(&self) -> Idempotence {
        Idempotence::Inline
    }

    fn append_only_sql(&self, table: &Table) -> Vec<String> {
        // MySQL has no `RAISE`; `SIGNAL SQLSTATE '45000'` is the documented way
        // for a trigger to refuse, and is what a driver surfaces as an error.
        // Without this the append-only guarantee would hold in application code
        // only — and a guarantee that a SQL console can walk around is not one.
        let name = self.quote(table.name);
        ["UPDATE", "DELETE"]
            .into_iter()
            .flat_map(|op| {
                let trigger = self.quote(&format!("trg_{}_no_{}", table.name, op.to_lowercase()));
                // DROP-then-CREATE because MySQL 8 has neither
                // `CREATE TRIGGER IF NOT EXISTS` nor `CREATE OR REPLACE
                // TRIGGER` — verified against 8.4, where re-running the script
                // failed with `ERROR 1359`. This leaves a window in which the
                // table is unprotected, which is tolerable only because it is
                // confined to `install()`; do not reuse the idiom at run time.
                [
                    format!("DROP TRIGGER IF EXISTS {trigger}"),
                    format!(
                        "CREATE TRIGGER {trigger} BEFORE {op} ON {name} FOR EACH ROW \
                         SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = \
                         '{} is append-only (openEHR V8.10)'",
                        table.name
                    ),
                ]
            })
            .collect()
    }
}
