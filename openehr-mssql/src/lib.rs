//! openEHR persistence for **Microsoft SQL Server 2022**.
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
//! # Conformance level: **Dialect**
//!
//! DDL only. No driver, no [`openehr_store::Store`], and nothing here has run
//! against a SQL Server instance. See `spec/conformance.md`.
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
}
