//! The `SQLite` dialect.

use openehr_store::schema::Table;
use openehr_store::{ColTy, Dialect, Placeholder};

/// The `SQLite` dialect.
///
/// ```
/// use openehr_sqlite::SqliteDialect;
/// use openehr_store::{ColTy, Dialect};
///
/// // `SQLite` has one string type and one integer type, so the *distinction*
/// // between an authoritative instant and its derived partner has to be
/// // carried by the choice of storage class rather than by two date types.
/// assert_eq!(SqliteDialect.col_sql(ColTy::Instant), "TEXT");
/// assert_eq!(SqliteDialect.col_sql(ColTy::InstantUtc), "INTEGER");
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SqliteDialect;

impl Dialect for SqliteDialect {
    fn name(&self) -> &'static str {
        "SQLite"
    }

    // The arms genuinely coincide: `SQLite` has one string storage class and one
    // integer one, so several logical types must map to the same SQL. Merging
    // them into one arm would lose which logical types this dialect was asked
    // about, which is what the next reader needs.
    #[allow(clippy::match_same_arms)]
    fn col_sql(&self, ty: ColTy) -> String {
        match ty {
            // SQLite's declared types are advisory — it applies type affinity
            // rather than enforcement — so lengths would be documentation that
            // nothing checks. Better to say TEXT and mean it.
            ColTy::Id(_) | ColTy::Text(_) | ColTy::LongText | ColTy::Instant => "TEXT",
            // No JSON type. The JSON1 extension operates on TEXT, so TEXT is
            // both the honest declaration and the working one.
            ColTy::Json => "TEXT",
            // Seconds from the Unix epoch, not a string. SQLite has no date
            // type at all, and storing the derived instant as text would make
            // it sort identically to the authoritative column — collapsing the
            // very distinction the two columns exist to keep (`D3.10`).
            ColTy::InstantUtc | ColTy::Int => "INTEGER",
            // SQLite has no boolean; 0 and 1 in an INTEGER column is the
            // documented convention.
            ColTy::Bool => "INTEGER",
            // SQLite has one binary type and no length enforcement — affinity
            // again, so 32 bytes is a Rust-side rule here (`M3.41`).
            ColTy::Digest => "BLOB",
        }
        .to_owned()
    }

    fn quote(&self, identifier: &str) -> String {
        format!("\"{}\"", identifier.replace('"', "\"\""))
    }

    fn placeholder(&self) -> Placeholder {
        Placeholder::Question
    }

    fn append_only_sql(&self, table: &Table) -> Vec<String> {
        // SQLite has triggers, so the guarantee lives in the database rather
        // than in application code — where it would end the first time somebody
        // opened the file with the `sqlite3` CLI.
        let name = self.quote(table.name);
        ["UPDATE", "DELETE"]
            .into_iter()
            .map(|op| {
                format!(
                    "CREATE TRIGGER IF NOT EXISTS {} BEFORE {op} ON {name} BEGIN \
                     SELECT RAISE(ABORT, '{} is append-only (openEHR V8.10)'); END",
                    self.quote(&format!("trg_{}_no_{}", table.name, op.to_lowercase())),
                    table.name
                )
            })
            .collect()
    }
}
