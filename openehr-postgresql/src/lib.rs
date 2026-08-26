//! openEHR persistence for **`PostgreSQL` 18**.
//!
//! This crate supplies one [`Dialect`]. Everything else — the storage model,
//! the projection from openEHR objects onto rows, the commit rules, the
//! conformance suite — lives in [`openehr_store`], which all five engine crates
//! share. A dialect owns four things and no more: type spellings, identifier
//! quoting, placeholder style, and how the engine enforces append-only.
//!
//! # Trademarks
//!
//! openEHR® is the registered trademark of the openEHR Foundation. Use of
//! the trademark does not constitute endorsement of this product by openEHR
//! International or openEHR Foundation.
//!
//! # Conformance level: **Schema**
//!
//! This crate emits DDL, and that DDL has been executed against
//! **`PostgreSQL` 18**: five tables and seven indexes created, the script
//! re-applied as a no-op, foreign keys enforced, and both append-only tables
//! observed refusing `UPDATE` and `DELETE` with a row present and unchanged
//! afterwards. `openehr-store/scripts/verify-schema.sh postgresql` reproduces
//! it from a fresh container.
//!
//! It does **not** contain a store: there is no driver dependency, no
//! connection handling, and no implementation of [`openehr_store::Store`].
//!
//! That is stated plainly because the sibling FHIR monorepo in this repository
//! carries an audit finding (**F-01**) for six READMEs that claimed a working
//! store, a CLI, and 7,399 round-tripped resources in ports where none of it
//! existed. See `spec/conformance.md` for what each level means.
//!
//! ```
//! use openehr_postgresql::PostgresqlDialect;
//! use openehr_store::{ColTy, Dialect, ddl_script};
//!
//! let sql = ddl_script(&PostgresqlDialect);
//! assert!(sql.contains("CREATE TABLE IF NOT EXISTS \"openehr_version\""));
//! // Canonical JSON is text, not `jsonb` — see `col_sql` (`M3.43`, `D-08`).
//! assert_eq!(PostgresqlDialect.col_sql(ColTy::Json), "text");
//! assert_eq!(PostgresqlDialect.col_sql(ColTy::InstantUtc), "timestamptz");
//! // …while the authoritative one is text, so the lexical form survives.
//! assert_eq!(PostgresqlDialect.col_sql(ColTy::Instant), "text");
//! ```

#![forbid(unsafe_code)]

use openehr_store::schema::Table;
use openehr_store::{ColTy, Dialect, Placeholder};

/// The `PostgreSQL` dialect.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PostgresqlDialect;

impl Dialect for PostgresqlDialect {
    fn name(&self) -> &'static str {
        "PostgreSQL"
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
            // `text`, not `varchar(n)`. PostgreSQL stores both identically and
            // `varchar(n)` only adds a length check that would reject a long
            // but legal `ARCHETYPE_ID` — and rejecting conformant data is a
            // worse failure than storing a few extra bytes.
            ColTy::Id(_) | ColTy::Text(_) | ColTy::LongText | ColTy::Instant => "text",
            // `text`, and emphatically **not** `jsonb` (`M3.43`).
            //
            // This said `jsonb`, reasoning that the byte form never had to come
            // back out because the canonical form was regenerated from the
            // parsed object. That stopped being true when `D-07` wired the
            // chain: the content digest is SHA-256 over the canonical bytes, so
            // those bytes must be reproducible *from storage*.
            //
            // `jsonb` reorders keys and inserts whitespace — measured on
            // PostgreSQL 18, not assumed — so what came back was a different
            // document that happened to be equivalent, and the digest could not
            // be recomputed. Canonical means these bytes in this order
            // (`lib:J9.12`).
            //
            // What this gives up is `jsonb` operators and GIN indexing over
            // content. Real, and unused: the relational columns are this
            // schema's index and the JSON is never queried as structure
            // (`M3.20`).
            ColTy::Json => "text",
            ColTy::InstantUtc => "timestamptz",
            ColTy::Int => "bigint",
            ColTy::Bool => "boolean",
            // `bytea`, PostgreSQL's only binary type. Fixed width is not
            // expressible, so length is enforced in Rust (`M3.41` departure).
            ColTy::Digest => "bytea",
        }
        .to_owned()
    }

    fn quote(&self, identifier: &str) -> String {
        format!("\"{}\"", identifier.replace('"', "\"\""))
    }

    fn placeholder(&self) -> Placeholder {
        Placeholder::Dollar
    }

    fn append_only_sql(&self, table: &Table) -> Vec<String> {
        // Enforced in the database, not only in the application. A guarantee
        // that lives in application code ends the first time somebody opens
        // psql, and openEHR's whole change-control model rests on this one
        // (`V8.10`).
        let name = self.quote(table.name);
        let function = format!("openehr_refuse_mutation_{}", table.name);
        vec![
            format!(
                "CREATE OR REPLACE FUNCTION {}() RETURNS trigger AS $$\n\
                 BEGIN\n  \
                 RAISE EXCEPTION '{} is append-only (openEHR V8.10)';\n\
                 END;\n$$ LANGUAGE plpgsql",
                self.quote(&function),
                table.name
            ),
            format!(
                "CREATE OR REPLACE TRIGGER {} BEFORE UPDATE OR DELETE ON {} \
                 FOR EACH ROW EXECUTE FUNCTION {}()",
                self.quote(&format!("trg_append_only_{}", table.name)),
                name,
                self.quote(&function)
            ),
        ]
    }
}
