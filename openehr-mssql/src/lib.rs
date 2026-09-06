//! openEHR persistence for **Microsoft SQL Server 2022**.
//!
//! One [`Dialect`]; the model, projection, commit rules, and conformance suite
//! are in [`openehr_store`].
//!
//! # Trademarks
//!
//! openEHR® is the registered trademark of the openEHR Foundation and is used
//! with the permission of openEHR International. Use of the trademark does not
//! constitute endorsement of this product by openEHR International or openEHR
//! Foundation.
//!
//! # Conformance level: **Schema**
//!
//! This crate emits DDL, and that DDL has been executed against a real SQL
//! Server: `mcr.microsoft.com/mssql/server:2022-latest`. Tables and indexes
//! were created, the script re-applied as a no-op, a seed row's canonical
//! JSON round-tripped byte for byte, and both append-only tables refused
//! `UPDATE` and `DELETE` with a row present and unchanged afterwards.
//! `openehr-store/scripts/verify-schema.sh mssql` reproduces it from a fresh
//! container, and runs in CI on every push.
//!
//! The first real run found a genuine defect, not only an evidence gap:
//! `append_only_sql`'s `CREATE TRIGGER` shared a batch with every statement
//! before it, which SQL Server refuses outright (`Msg 111`). Fixed by giving
//! this dialect its own statement terminator, `\nGO`, so every statement —
//! the trigger included — is the sole content of its own batch. Full account:
//! `spec/databases/audit.md` **D-12**.
//!
//! It does **not** contain a store: there is no driver dependency and no
//! implementation of [`openehr_store::Store`]. See `spec/databases/
//! conformance-matrix.md` — the one file that owns a level (`W0.40`).
//!
//! ```
//! use openehr_mssql::MssqlDialect;
//! use openehr_store::{ColTy, Dialect, ddl_script};
//!
//! assert_eq!(MssqlDialect.quote("openehr_version"), "[openehr_version]");
//! // No `CREATE TABLE IF NOT EXISTS` on this engine, so the DDL does not
//! // claim it; re-runnability comes from a `sys.objects` guard instead.
//! let sql = ddl_script(&MssqlDialect);
//! assert!(!sql.contains("CREATE TABLE IF NOT EXISTS"));
//! assert!(sql.contains("IF NOT EXISTS (SELECT 1 FROM sys.objects"));
//! ```

#![forbid(unsafe_code)]

use openehr_store::{ColTy, Dialect, Idempotence, ObjectKind, Placeholder, Table};

/// The SQL Server dialect.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MssqlDialect;

impl Dialect for MssqlDialect {
    fn name(&self) -> &'static str {
        "SQL Server"
    }

    fn col_sql(&self, ty: ColTy) -> String {
        match ty {
            // `nvarchar`, not `varchar`: openEHR content is Unicode by
            // construction — `DV_TEXT` carries an encoding attribute and
            // clinical names are not ASCII — and a `varchar` column silently
            // substitutes `?` for anything outside the collation's code page.
            ColTy::Id(n) | ColTy::Text(n) => return format!("nvarchar({n})"),
            ColTy::LongText | ColTy::Json => "nvarchar(max)",
            ColTy::Instant => "nvarchar(64)",
            // `datetimeoffset`, not `datetime2`: openEHR instants carry a UTC
            // offset, and `datetime2` would drop it, making two records from
            // different zones compare as though they were the same moment.
            ColTy::InstantUtc => "datetimeoffset(7)",
            ColTy::Int => "bigint",
            ColTy::Bool => "bit",
            // `binary(32)`, fixed width. T-SQL pads a shorter value rather than
            // rejecting it, so length is also checked in Rust (`M3.41`).
            ColTy::Digest => "binary(32)",
        }
        .to_owned()
    }

    fn quote(&self, identifier: &str) -> String {
        format!("[{}]", identifier.replace(']', "]]"))
    }

    fn placeholder(&self) -> Placeholder {
        Placeholder::AtP
    }

    // SQL Server has neither `CREATE TABLE IF NOT EXISTS` nor
    // `CREATE INDEX IF NOT EXISTS`. Emitting either anyway would produce a
    // script that fails on the engine it targets — exactly the class of defect
    // the sibling FHIR monorepo records as **F-25** and **F-26**.
    fn table_idempotence(&self) -> Idempotence {
        Idempotence::Guard
    }

    fn index_idempotence(&self) -> Idempotence {
        Idempotence::Guard
    }

    fn guard(&self, kind: ObjectKind, name: &str, statement: &str) -> String {
        // The catalogue view differs by object kind, so the guard cannot be one
        // shared string. `sys.objects` rather than `sys.tables` for tables so
        // that a *name collision with a non-table* still fails loudly instead of
        // being created alongside.
        let test = match kind {
            ObjectKind::Table => {
                format!("SELECT 1 FROM sys.objects WHERE name = N'{name}' AND type = 'U'")
            }
            ObjectKind::Index => format!("SELECT 1 FROM sys.indexes WHERE name = N'{name}'"),
        };
        // EXEC with a quoted string: `CREATE TABLE` must be the first statement
        // in its batch, so it cannot appear directly inside `IF ... BEGIN`.
        format!(
            "IF NOT EXISTS ({test})\n  EXEC('{}')",
            statement.replace('\'', "''")
        )
    }

    fn append_only_sql(&self, table: &Table) -> Vec<String> {
        // INSTEAD OF rather than AFTER: an AFTER trigger would have to roll the
        // transaction back, which aborts work the caller had already done and
        // succeeded at. INSTEAD OF refuses before anything is written.
        let name = self.quote(table.name);
        vec![format!(
            "CREATE OR ALTER TRIGGER {} ON {name} INSTEAD OF UPDATE, DELETE AS \
             BEGIN THROW 50000, '{} is append-only (openEHR V8.10)', 1; END",
            self.quote(&format!("trg_{}_append_only", table.name)),
            table.name
        )]
    }

    // `CREATE [OR ALTER] TRIGGER` must be the first statement in its batch
    // (`Msg 111`) — the same rule `guard`'s own comment names for `CREATE
    // TABLE`, worked around there by wrapping the statement inside `EXEC('…')`
    // so it is never a literal first-class statement in the outer batch at
    // all. The trigger in `append_only_sql` has no such wrapping, so without a
    // real batch boundary between it and whatever precedes it in the script,
    // the engine refuses it outright — found by running the generated script
    // against a real server (`M14.6`), not by reading the rule.
    //
    // `GO` after every statement, not only before the trigger: it is a
    // `sqlcmd`/SSMS client directive rather than T-SQL proper, harmless
    // between two statements that did not need separating, and simpler than
    // tracking which statement kinds do.
    fn terminator(&self) -> &'static str {
        "\nGO"
    }
}
