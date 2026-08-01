//! The SQL dialect trait, and DDL generation from [`crate::schema`].
//!
//! # What a dialect is allowed to change
//!
//! Exactly four things: how it spells a type, how it quotes an identifier, how
//! it writes a placeholder, and how it enforces append-only. Everything else —
//! which tables exist, which columns, which indexes, what order they are
//! emitted in — comes from the shared schema and is identical across engines by
//! construction.
//!
//! That boundary is the whole point. The sibling FHIR monorepo in this
//! repository has an audit finding (**F-08**) for an Oracle DDL emitter that
//! silently emitted `MySQL` types, because each port owned a full copy of the
//! generator. Here a dialect cannot emit another engine's schema, because it
//! does not own the schema — only the spellings. [`crate::conformance`]
//! includes a test that asserts no two dialects agree on all of them.

use crate::schema::{ColTy, Column, TABLES, Table};
use core::fmt::Write as _;

/// How a dialect writes a bind placeholder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Placeholder {
    /// `?` — `SQLite`, `MySQL`, `MariaDB`.
    Question,
    /// `$1`, `$2` — `PostgreSQL`.
    Dollar,
    /// `@p1`, `@p2` — SQL Server.
    AtP,
    /// `:1`, `:2` — Oracle.
    Colon,
}

impl Placeholder {
    /// Renders the placeholder for a one-based parameter position.
    ///
    /// ```
    /// use openehr_store::Placeholder;
    ///
    /// assert_eq!(Placeholder::Question.render(1), "?");
    /// assert_eq!(Placeholder::Dollar.render(2), "$2");
    /// assert_eq!(Placeholder::AtP.render(3), "@p3");
    /// assert_eq!(Placeholder::Colon.render(4), ":4");
    /// ```
    #[must_use]
    pub fn render(self, position: usize) -> String {
        match self {
            Self::Question => "?".to_owned(),
            Self::Dollar => format!("${position}"),
            Self::AtP => format!("@p{position}"),
            Self::Colon => format!(":{position}"),
        }
    }
}

/// A schema object a `CREATE` statement can bring into being.
///
/// Passed to [`Dialect::guard`] so an engine that checks a catalogue before
/// creating knows which catalogue to check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    /// A table.
    Table,
    /// An index.
    Index,
}

/// How an engine makes a `CREATE` statement safe to re-run.
///
/// `install()` must be idempotent — an operator who runs it twice, or a
/// deployment that retries, must not get a hard error the second time. The
/// three engines differ enough that a single boolean gets it wrong, which is
/// how the first live run against `MySQL` failed: `MySQL` accepts
/// `CREATE TABLE IF NOT EXISTS` and **rejects** `CREATE INDEX IF NOT EXISTS`,
/// so one flag covering both statement kinds emitted a script that created
/// every table and then failed on the first index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Idempotence {
    /// The statement accepts an inline `IF NOT EXISTS` clause.
    IfNotExists,
    /// The statement must be wrapped by [`Dialect::guard`].
    Guard,
    /// No separate statement exists — the object is declared inside its
    /// table, and inherits that table's idempotence.
    Inline,
}

/// One SQL engine's spellings.
///
/// # Implementing one
///
/// Implement [`Dialect::name`], [`Dialect::col_sql`], [`Dialect::quote`], and
/// [`Dialect::placeholder`]. The default methods build the DDL from those and
/// from the shared schema; override one only where the engine genuinely cannot
/// do what the default emits, and say so in the crate's dialect annex.
pub trait Dialect {
    /// The engine's name, as it appears in documentation and error messages.
    fn name(&self) -> &'static str;

    /// The SQL type for a logical column type.
    ///
    /// This is the one function that names engine-specific types, and it is
    /// where every dialect difference that matters actually lives.
    fn col_sql(&self, ty: ColTy) -> String;

    /// Quotes an identifier.
    fn quote(&self, identifier: &str) -> String;

    /// The engine's placeholder style.
    fn placeholder(&self) -> Placeholder;

    /// How `CREATE TABLE` is made re-runnable.
    fn table_idempotence(&self) -> Idempotence {
        Idempotence::IfNotExists
    }

    /// How `CREATE INDEX` is made re-runnable.
    ///
    /// Separate from [`Dialect::table_idempotence`] because `MySQL` treats the
    /// two statements differently; see [`Idempotence`].
    fn index_idempotence(&self) -> Idempotence {
        Idempotence::IfNotExists
    }

    /// Wraps a statement so that re-running it is a no-op.
    ///
    /// Called only for object kinds whose idempotence is
    /// [`Idempotence::Guard`]. The default returns the statement unchanged,
    /// which is correct only when no kind declares `Guard` —
    /// [`crate::conformance::check_dialect`] fails a dialect that declares
    /// `Guard` and then does not actually wrap, because a guard that is
    /// documented but not emitted is worse than none.
    fn guard(&self, _kind: ObjectKind, _name: &str, statement: &str) -> String {
        statement.to_owned()
    }

    /// The statement terminator used when joining statements into a script.
    fn terminator(&self) -> &'static str {
        ";"
    }

    /// Statements enforcing append-only on a table.
    ///
    /// All six engines this workspace targets can do it with a trigger, and
    /// all six must, because a guarantee enforced only in application code is
    /// a guarantee that ends the first time somebody opens a SQL console.
    ///
    /// The empty default is retained only so that a *new* dialect compiles
    /// before it is finished. It is not a permissible resting state:
    /// [`crate::conformance::check_dialect`] fails any dialect that leaves an
    /// append-only table unenforced. Three dialects inherited this default
    /// silently for as long as they existed (**A-15**) while the shared
    /// documentation described append-only as a property of the design — which
    /// is why the check exists rather than another sentence here.
    fn append_only_sql(&self, _table: &Table) -> Vec<String> {
        Vec::new()
    }

    /// The full DDL script for the shared schema.
    ///
    /// Tables in dependency order, then indexes, then append-only enforcement.
    /// Indexes come after all tables so that a partial failure leaves a
    /// readable schema rather than a half-indexed one.
    fn ddl(&self) -> Vec<String> {
        let mut out = Vec::new();
        for table in TABLES {
            out.push(self.create_table(table));
        }
        if self.index_idempotence() != Idempotence::Inline {
            for table in TABLES {
                for index in table.indexes {
                    out.push(self.create_index(table, index));
                }
            }
        }
        for table in TABLES {
            if table.append_only {
                out.extend(self.append_only_sql(table));
            }
        }
        out
    }

    /// One `CREATE TABLE` statement.
    fn create_table(&self, table: &Table) -> String {
        let mut sql = String::new();
        let exists = if self.table_idempotence() == Idempotence::IfNotExists {
            "IF NOT EXISTS "
        } else {
            ""
        };
        let _ = writeln!(sql, "CREATE TABLE {exists}{} (", self.quote(table.name));
        let mut parts: Vec<String> = Vec::new();
        for column in table.columns {
            parts.push(format!(
                "  {} {}{}",
                self.quote(column.name),
                self.col_sql(column.ty),
                if column.nullable { "" } else { " NOT NULL" }
            ));
        }
        if !table.primary_key.is_empty() {
            let keys: Vec<String> = table.primary_key.iter().map(|k| self.quote(k)).collect();
            parts.push(format!("  PRIMARY KEY ({})", keys.join(", ")));
        }
        for fk in table.foreign_keys {
            parts.push(format!(
                "  FOREIGN KEY ({}) REFERENCES {} ({})",
                self.quote(fk.column),
                self.quote(fk.table),
                self.quote(fk.references)
            ));
        }
        // MySQL cannot say `CREATE INDEX IF NOT EXISTS`, but it can declare the
        // index inside the table, where it inherits the table's own
        // `IF NOT EXISTS`. That is the idiomatic answer rather than a
        // workaround: one statement, one object, one idempotence rule.
        if self.index_idempotence() == Idempotence::Inline {
            for index in table.indexes {
                let columns: Vec<String> = index.columns.iter().map(|c| self.quote(c)).collect();
                parts.push(format!(
                    "  {}KEY {} ({})",
                    if index.unique { "UNIQUE " } else { "" },
                    self.quote(index.name),
                    columns.join(", ")
                ));
            }
        }
        let _ = write!(sql, "{}", parts.join(",\n"));
        let _ = write!(sql, "\n)");
        if self.table_idempotence() == Idempotence::Guard {
            return self.guard(ObjectKind::Table, table.name, &sql);
        }
        sql
    }

    /// One `CREATE INDEX` statement.
    fn create_index(&self, table: &Table, index: &crate::schema::Index) -> String {
        let unique = if index.unique { "UNIQUE " } else { "" };
        let exists = if self.index_idempotence() == Idempotence::IfNotExists {
            "IF NOT EXISTS "
        } else {
            ""
        };
        let columns: Vec<String> = index.columns.iter().map(|c| self.quote(c)).collect();
        let sql = format!(
            "CREATE {unique}INDEX {exists}{} ON {} ({})",
            self.quote(index.name),
            self.quote(table.name),
            columns.join(", ")
        );
        if self.index_idempotence() == Idempotence::Guard {
            return self.guard(ObjectKind::Index, index.name, &sql);
        }
        sql
    }
}

/// Renders a dialect's DDL as one script.
///
/// # Errors
///
/// Never fails; the signature is infallible and this returns a `String`
/// directly. Present as a free function so callers do not have to import the
/// trait to get a script.
#[must_use]
pub fn ddl_script<D: Dialect + ?Sized>(dialect: &D) -> String {
    let terminator = dialect.terminator();
    dialect
        .ddl()
        .into_iter()
        .map(|statement| format!("{statement}{terminator}\n"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Checks that a column type maps to something plausible.
///
/// Used by [`crate::conformance::check_dialect`]; exposed so a new dialect's
/// own tests can call it directly.
#[must_use]
pub fn column_sql<D: Dialect + ?Sized>(dialect: &D, column: &Column) -> String {
    dialect.col_sql(column.ty)
}
