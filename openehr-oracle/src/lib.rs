//! openEHR persistence for **Oracle Database 23ai**.
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
//! against an Oracle instance. See `spec/conformance.md`.
//!
//! The sibling FHIR monorepo's Oracle port shipped a DDL emitter that produced
//! **`MySQL`** types for as long as the fork existed (**F-08**), because each
//! port owned a full copy of the generator. That cannot happen here: this
//! crate owns only the type spellings below, and
//! [`openehr_store::conformance::dialects_are_distinct`] fails the build if any
//! two dialects agree.
//!
//! ```
//! use openehr_oracle::OracleDialect;
//! use openehr_store::{ColTy, Dialect};
//!
//! assert_eq!(OracleDialect.col_sql(ColTy::Id(255)), "VARCHAR2(255 CHAR)");
//! assert_eq!(OracleDialect.col_sql(ColTy::Bool), "NUMBER(1)");
//! ```

#![forbid(unsafe_code)]

use openehr_store::{ColTy, Dialect, Idempotence, ObjectKind, Placeholder, Table};

/// The Oracle dialect.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OracleDialect;

impl OracleDialect {
    /// The longest identifier Oracle accepts, from 12.2 onward.
    ///
    /// Checked by this crate's tests against every name in the shared schema,
    /// because an over-long identifier fails at `CREATE TABLE` time on Oracle
    /// and nowhere else.
    pub const MAX_IDENTIFIER: usize = 128;
}

impl Dialect for OracleDialect {
    fn name(&self) -> &'static str {
        "Oracle"
    }

    fn col_sql(&self, ty: ColTy) -> String {
        match ty {
            // `CHAR` semantics, not the default `BYTE`: `VARCHAR2(255)` means
            // 255 *bytes* on Oracle, so a name in a non-Latin script would be
            // rejected at about a third of its length.
            ColTy::Id(n) | ColTy::Text(n) => return format!("VARCHAR2({n} CHAR)"),
            // Oracle's `VARCHAR2` maxes out at 4000 bytes, so anything
            // unbounded must be a LOB.
            ColTy::LongText | ColTy::Json => "CLOB",
            ColTy::Instant => "VARCHAR2(64 CHAR)",
            ColTy::InstantUtc => "TIMESTAMP WITH TIME ZONE",
            // Oracle has no integer type; `NUMBER(19)` is the range of an
            // `i64`, which is what the row types use.
            ColTy::Int => "NUMBER(19)",
            // …and no boolean in SQL, whatever PL/SQL offers.
            ColTy::Bool => "NUMBER(1)",
            // `RAW(32)` is Oracle's fixed-width binary type and is exactly the
            // size of a SHA-256 digest (`M3.41`).
            ColTy::Digest => "RAW(32)",
        }
        .to_owned()
    }

    fn quote(&self, identifier: &str) -> String {
        // Quoted, and therefore case-sensitive: unquoted Oracle identifiers
        // fold to upper case, which would make this schema's lower-case names
        // disagree with every other engine's.
        format!("\"{}\"", identifier.replace('"', "\"\""))
    }

    fn placeholder(&self) -> Placeholder {
        Placeholder::Colon
    }

    // Oracle has no `IF NOT EXISTS` for tables or indexes before 23ai, and this
    // crate targets a script that runs on 19c as well.
    fn table_idempotence(&self) -> Idempotence {
        Idempotence::Guard
    }

    fn index_idempotence(&self) -> Idempotence {
        Idempotence::Guard
    }

    fn guard(&self, _kind: ObjectKind, _name: &str, statement: &str) -> String {
        // Catch-and-inspect rather than query-then-create: checking
        // `user_tables` first is a race, and Oracle's own idiom is to attempt
        // the DDL and swallow the one error that means "already there".
        // ORA-00955 covers tables and indexes alike, so the kind is not needed;
        // every other SQLCODE is re-raised, because a guard that swallows
        // unrelated failures turns a broken schema into a silent one.
        format!(
            "BEGIN\n  EXECUTE IMMEDIATE '{}';\nEXCEPTION WHEN OTHERS THEN\n  \
             IF SQLCODE != -955 THEN RAISE; END IF;\nEND;",
            statement.replace('\'', "''")
        )
    }

    fn terminator(&self) -> &'static str {
        // Every statement this dialect emits is a PL/SQL block, and SQL*Plus
        // ends a block with `/` on its own line — a bare `;` would submit only
        // as far as the block's first inner semicolon.
        "\n/"
    }

    fn append_only_sql(&self, table: &Table) -> Vec<String> {
        let name = self.quote(table.name);
        vec![format!(
            "CREATE OR REPLACE TRIGGER {} BEFORE UPDATE OR DELETE ON {name} \
             FOR EACH ROW BEGIN raise_application_error(-20001, \
             '{} is append-only (openEHR V8.10)'); END;",
            self.quote(&format!("trg_{}_append_only", table.name)),
            table.name
        )]
    }
}
