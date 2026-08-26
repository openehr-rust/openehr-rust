//! openEHR persistence for **`MariaDB` 11.4**.
//!
//! One [`Dialect`]; the model, projection, commit rules, and conformance suite
//! are in [`openehr_store`].
//!
//! # Trademarks
//!
//! openEHR® is a registered trademark of openEHR International (the openEHR
//! Foundation). This project is an independent implementation: it is not
//! affiliated with, endorsed by, or certified by openEHR International.
//!
//! # Conformance level: **Schema**
//!
//! No driver and no [`openehr_store::Store`] — but the DDL has been executed
//! against **`MariaDB` 11.4**, re-applied as a no-op, and its append-only
//! triggers observed refusing `UPDATE` and `DELETE` with a row present.
//! Reproduce with `openehr-store/scripts/verify-schema.sh mariadb`.
//!
//! # `MariaDB` is not `MySQL`, and this dialect is where that is decided
//!
//! This crate began as a copy of `openehr-mysql` with the name substituted, and
//! for as long as that lasted it emitted **byte-identical** DDL to `MySQL` while
//! its documentation claimed a `MariaDB` server had accepted it — against a
//! version, "`MariaDB` 8.4", that has never existed. That is the sibling FHIR
//! monorepo's **F-08**, an Oracle emitter shipping `MySQL` types, reproduced
//! here. It survived because the cross-dialect comparison in
//! `openehr-sqlite/tests/dialects.rs` compared five dialects and this was the
//! sixth. Both holes are closed: the comparison now covers all six, and the two
//! differences below are real engine facts rather than cosmetic edits.
//!
//! | `MariaDB` 11.4 | `MySQL` 8.4 | Consequence here |
//! | --- | --- | --- |
//! | `CREATE INDEX IF NOT EXISTS` (since 10.0.5) | not supported | indexes are their own statements, not folded into `CREATE TABLE` |
//! | `CREATE OR REPLACE TRIGGER` (since 10.1.4) | not supported | no drop-then-create window where the table is unprotected |
//!
//! The second is the one that matters. `MySQL` must `DROP TRIGGER` before
//! recreating it, which leaves an interval — short, but real — in which an
//! append-only table would accept an `UPDATE`. `MariaDB` replaces the trigger in
//! one statement, so the guarantee never lapses.
//!
//! ```
//! use openehr_mariadb::MariadbDialect;
//! use openehr_store::{ColTy, Dialect};
//!
//! // Lengths are mandatory here, unlike `PostgreSQL`: an unbounded column
//! // cannot be indexed.
//! assert_eq!(MariadbDialect.col_sql(ColTy::Id(255)), "VARCHAR(255)");
//! assert_eq!(MariadbDialect.quote("openehr_version"), "`openehr_version`");
//! ```

#![forbid(unsafe_code)]

use openehr_store::{ColTy, Dialect, Placeholder, Table};

/// The `MariaDB` dialect.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MariadbDialect;

impl Dialect for MariadbDialect {
    fn name(&self) -> &'static str {
        "MariaDB"
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
            // `LONGTEXT`, spelled out rather than written `JSON` (`M3.43`).
            //
            // `MariaDB`'s `JSON` is an alias for `LONGTEXT`, so this column
            // always round-tripped canonical bytes and this change alters no
            // stored value. The spelling changes because the old one was right
            // *by inheritance*: it relied on a property of today's MariaDB
            // rather than on the column's declared contract, and the identical
            // spelling in `openehr-mysql` meant the opposite thing (`D-08`).
            //
            // Canonical JSON must come back as the bytes it went in as, because
            // the chain's content digest is taken over them (`M3.16`). A type
            // whose contract permits normalisation must not hold it, even where
            // this release of this engine happens not to normalise.
            ColTy::Json => "LONGTEXT",
            // 64 characters: the longest ISO 8601 form this crate accepts is a
            // date-time with fractional seconds and an offset, well under half
            // that. Bounded rather than TEXT so it can be indexed alongside the
            // derived column.
            ColTy::Instant => "VARCHAR(64)",
            // Microsecond precision, which is MariaDB's maximum. openEHR permits
            // finer fractional seconds in the lexical form — which is why that
            // form is stored separately and authoritatively (`D3.10`).
            ColTy::InstantUtc => "DATETIME(6)",
            ColTy::Int => "BIGINT",
            // MariaDB has no boolean; TINYINT(1) is the conventional spelling and
            // is what every driver maps to `bool`.
            ColTy::Bool => "TINYINT(1)",
            // As MySQL: fixed width, so a wrong-length digest is rejected by
            // the column rather than discovered during comparison (`M3.41`).
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

    // `index_idempotence` is deliberately left at the default,
    // `Idempotence::IfNotExists`. This is the documented split from
    // `openehr-mysql`, which must declare `Inline` because `MySQL` rejects
    // `CREATE INDEX IF NOT EXISTS`. `MariaDB` has accepted it since 10.0.5, so
    // an index here is its own statement and reads the way the schema declares
    // it.

    fn append_only_sql(&self, table: &Table) -> Vec<String> {
        // `CREATE OR REPLACE TRIGGER` is the difference that matters against
        // `MySQL`, which has neither it nor `CREATE TRIGGER IF NOT EXISTS` and
        // so must drop first — leaving a window in which the table is
        // unprotected. Here re-running `install()` never lapses the guarantee.
        //
        // `SIGNAL SQLSTATE '45000'` is the documented way for a trigger to
        // refuse, and is what a driver surfaces as an error. Without it the
        // append-only guarantee would hold in application code only, and a
        // guarantee that a SQL console can walk around is not one.
        let name = self.quote(table.name);
        ["UPDATE", "DELETE"]
            .into_iter()
            .map(|op| {
                let trigger = self.quote(&format!("trg_{}_no_{}", table.name, op.to_lowercase()));
                format!(
                    "CREATE OR REPLACE TRIGGER {trigger} BEFORE {op} ON {name} FOR EACH ROW \
                     SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = \
                     '{} is append-only (openEHR V8.10)'",
                    table.name
                )
            })
            .collect()
    }
}
