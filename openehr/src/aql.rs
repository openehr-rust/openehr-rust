//! Archetype Query Language: lexing, parsing, and static checking.
//!
//! AQL is openEHR's portable query language. It looks like SQL and is not:
//! `FROM` describes a **containment tree** over archetyped structures rather
//! than a set of tables, and paths address nodes inside those structures by
//! archetype node id.
//!
//! ```text
//! SELECT
//!     o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude AS systolic,
//!     c/context/start_time AS taken
//! FROM EHR e[ehr_id/value=$ehrUid]
//!     CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1]
//!         CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.blood_pressure.v2]
//! WHERE o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude >= 140
//! ORDER BY c/context/start_time DESC
//! LIMIT 5
//! ```
//!
//! # This is a front end, not a query engine
//!
//! This module turns AQL text into an [`AqlQuery`] and checks what can be
//! checked without data. It does **not** execute queries: executing AQL means
//! resolving archetype paths against a repository, and this crate has no
//! repository. Every function here is honest about that — nothing returns rows,
//! and nothing pretends a parse is a plan.
//!
//! What it is useful for:
//!
//! - rejecting a malformed query at the API edge, with an offset, rather than
//!   at the storage layer,
//! - finding the undefined-alias bug ([`AqlQuery::check`]) that AQL's syntax
//!   makes very easy and that no runtime reports usefully,
//! - enumerating the archetypes and parameters a query touches, before running
//!   it, which is what an authorisation check needs,
//! - re-rendering a query in a normal form ([`AqlQuery`]'s
//!   [`Display`](core::fmt::Display)).
//!
//! # Coverage
//!
//! Supported: `SELECT` with `DISTINCT` and `TOP`, aliases, aggregate and scalar
//! function calls, `FROM` with `CONTAINS` / `NOT CONTAINS` / `AND` / `OR` and
//! parentheses, archetype and standard predicates, `WHERE` with the comparison
//! operators, `AND` / `OR` / `NOT` / `EXISTS` / `MATCHES` / `LIKE`, parameters,
//! `ORDER BY` with `ASC` / `DESC`, and `LIMIT` / `OFFSET`.
//!
//! Not supported, and reported as a parse error rather than silently ignored:
//! `SELECT *`, `VERSION` and `TOP … FORWARD/BACKWARD` extensions, and
//! terminology-function subqueries. `spec/12-paths-and-query.md` `Q12.9`
//! records the list.

use core::fmt;
use core::fmt::Write as _;

// ---------------------------------------------------------------------------
// Lexer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Word(String),
    /// A path written after an alias: `o/data[at0001]/value`.
    Path(String),
    String(String),
    Number(f64),
    Integer(i64),
    Parameter(String),
    Symbol(&'static str),
}

#[derive(Debug, Clone)]
struct Lexed {
    token: Token,
    offset: usize,
}

/// A failure to parse AQL.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("AQL parse error at offset {offset}: {reason}")]
pub struct AqlError {
    /// Byte offset into the query where parsing stopped.
    pub offset: usize,
    /// What was expected, or what was found and not supported.
    pub reason: String,
}

impl AqlError {
    fn new(offset: usize, reason: impl Into<String>) -> Self {
        Self {
            offset,
            reason: reason.into(),
        }
    }
}

#[allow(clippy::too_many_lines)]
fn lex(input: &str) -> Result<Vec<Lexed>, AqlError> {
    let bytes = input.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        match c {
            b'\'' | b'"' => {
                let quote = c;
                i += 1;
                let mut value = String::new();
                // Copied as **slices of the input**, not byte by byte.
                //
                // This read `value.push(bytes[i] as char)`, which takes one
                // UTF-8 byte and widens it to a `char` — so every non-ASCII
                // character in a string literal came out as Latin-1 mojibake.
                // `'Müller'` lexed to `'MÃ¼ller'`, and a `WHERE name = …`
                // against it matched nobody. See `A-37`.
                //
                // Scanning by byte is still correct: the only bytes examined
                // are `\\` and the quote, both ASCII, and an ASCII byte never
                // occurs inside a multi-byte UTF-8 sequence. What was wrong was
                // *copying* by byte. `segment` tracks the start of the current
                // run so each run is appended whole.
                let mut segment = i;
                loop {
                    if i >= bytes.len() {
                        return Err(AqlError::new(start, "unterminated string literal"));
                    }
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        value.push_str(&input[segment..i]);
                        // The escaped character may itself be multi-byte, so
                        // step over all of it. `i + 1` is a character boundary
                        // because a backslash is one byte.
                        let escaped = input[i + 1..]
                            .chars()
                            .next()
                            .expect("a character starts here: i + 1 < bytes.len()");
                        value.push(escaped);
                        i += 1 + escaped.len_utf8();
                        segment = i;
                        continue;
                    }
                    if bytes[i] == quote {
                        value.push_str(&input[segment..i]);
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                out.push(Lexed {
                    token: Token::String(value),
                    offset: start,
                });
            }
            b'$' => {
                i += 1;
                let from = i;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                if i == from {
                    return Err(AqlError::new(start, "`$` with no parameter name"));
                }
                out.push(Lexed {
                    token: Token::Parameter(input[from..i].to_owned()),
                    offset: start,
                });
            }
            b'0'..=b'9' => {
                while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                    i += 1;
                }
                let text = &input[start..i];
                let token = if text.contains('.') {
                    Token::Number(
                        text.parse()
                            .map_err(|_| AqlError::new(start, "malformed number"))?,
                    )
                } else {
                    Token::Integer(
                        text.parse()
                            .map_err(|_| AqlError::new(start, "number does not fit in i64"))?,
                    )
                };
                out.push(Lexed {
                    token,
                    offset: start,
                });
            }
            c if c.is_ascii_alphabetic() || c == b'_' => {
                // A word may continue into a path (`o/data[at0001]/value`), and
                // scanning the two together keeps `/` out of the operator table.
                //
                // The subtlety is `[`. In a path it is a node predicate and
                // belongs to the token; after a bare alias in FROM
                // (`COMPOSITION c[openEHR-…]`) it opens a *class* predicate that
                // the parser must see. The rule below is therefore: absorb `[`
                // only once a `/` has been seen, because a class predicate never
                // follows a slash and a node predicate always does.
                let mut depth = 0usize;
                let mut seen_slash = false;
                while i < bytes.len() {
                    let b = bytes[i];
                    if b == b'[' {
                        if depth == 0 && !seen_slash {
                            break;
                        }
                        depth += 1;
                    } else if b == b']' {
                        if depth == 0 {
                            break;
                        }
                        depth -= 1;
                    } else if depth == 0 {
                        if b == b'/' {
                            seen_slash = true;
                        } else if !(b.is_ascii_alphanumeric()
                            || b == b'_'
                            || b == b'-'
                            || b == b'.')
                        {
                            break;
                        }
                    }
                    i += 1;
                }
                let text = &input[start..i];
                let token = if text.contains('/') {
                    Token::Path(text.to_owned())
                } else {
                    Token::Word(text.to_owned())
                };
                out.push(Lexed {
                    token,
                    offset: start,
                });
            }
            _ => {
                const SYMBOLS: [&str; 14] = [
                    ">=", "<=", "!=", "(", ")", "{", "}", ",", "=", ">", "<", "[", "]", "*",
                ];
                let Some(sym) = SYMBOLS.iter().find(|s| input[i..].starts_with(**s)) else {
                    return Err(AqlError::new(i, "unexpected character"));
                };
                i += sym.len();
                out.push(Lexed {
                    token: Token::Symbol(sym),
                    offset: start,
                });
            }
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// AST
// ---------------------------------------------------------------------------

/// A literal value in a query.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    /// A quoted string.
    String(String),
    /// A whole number.
    Integer(i64),
    /// A number with a fractional part.
    Number(f64),
    /// `true` or `false`.
    Boolean(bool),
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Escaped, so that a value containing a quote or a backslash
            // renders as something the lexer reads back unchanged. Without
            // this, `it's` rendered as `'it's'` — which reparses as the string
            // `it` followed by garbage, or fails outright (`A-37`).
            //
            // Only these two characters: the lexer's escape rule is "a
            // backslash introduces the next character literally", not a C-style
            // table, so `\n` in a rendered query would mean the letter `n`.
            Self::String(v) => {
                f.write_str("'")?;
                for ch in v.chars() {
                    if ch == '\'' || ch == '\\' {
                        f.write_char('\\')?;
                    }
                    f.write_char(ch)?;
                }
                f.write_str("'")
            }
            Self::Integer(v) => write!(f, "{v}"),
            Self::Number(v) => write!(f, "{v}"),
            Self::Boolean(v) => write!(f, "{v}"),
        }
    }
}

/// A path rooted at a `FROM` alias: `o/data[at0001]/value/magnitude`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentifiedPath {
    /// The alias the path is rooted at.
    pub root: String,
    /// The path within it, without a leading `/`.
    pub path: Option<String>,
}

impl fmt::Display for IdentifiedPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.path {
            Some(p) => write!(f, "{}/{p}", self.root),
            None => f.write_str(&self.root),
        }
    }
}

/// Anything that can appear where a value is expected.
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    /// A path into a matched object.
    Path(IdentifiedPath),
    /// A literal.
    Literal(Literal),
    /// A `$name` parameter.
    Parameter(String),
    /// A function call, such as `COUNT(...)` or `CURRENT_DATE_TIME()`.
    Function {
        /// The function's name, upper-cased.
        name: String,
        /// Its arguments.
        args: Vec<Operand>,
    },
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path(p) => write!(f, "{p}"),
            Self::Literal(l) => write!(f, "{l}"),
            Self::Parameter(name) => write!(f, "${name}"),
            Self::Function { name, args } => {
                write!(f, "{name}(")?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{a}")?;
                }
                write!(f, ")")
            }
        }
    }
}

/// A comparison operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CompareOp {
    /// `=`
    Equal,
    /// `!=`
    NotEqual,
    /// `>`
    Greater,
    /// `>=`
    GreaterOrEqual,
    /// `<`
    Less,
    /// `<=`
    LessOrEqual,
    /// `LIKE`
    Like,
    /// `MATCHES`
    Matches,
}

impl fmt::Display for CompareOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Equal => "=",
            Self::NotEqual => "!=",
            Self::Greater => ">",
            Self::GreaterOrEqual => ">=",
            Self::Less => "<",
            Self::LessOrEqual => "<=",
            Self::Like => "LIKE",
            Self::Matches => "MATCHES",
        })
    }
}

/// A `WHERE` condition.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// `lhs op rhs`
    Compare {
        /// Left operand.
        lhs: Operand,
        /// Operator.
        op: CompareOp,
        /// Right operand.
        rhs: Operand,
    },
    /// `lhs MATCHES {a, b, c}` — written with a value set.
    MatchesSet {
        /// The path being tested.
        lhs: Operand,
        /// The permitted values.
        values: Vec<Operand>,
    },
    /// `EXISTS path`
    Exists(IdentifiedPath),
    /// `a AND b`
    And(Box<Expr>, Box<Expr>),
    /// `a OR b`
    Or(Box<Expr>, Box<Expr>),
    /// `NOT a`
    Not(Box<Expr>),
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compare { lhs, op, rhs } => write!(f, "{lhs} {op} {rhs}"),
            Self::MatchesSet { lhs, values } => {
                write!(f, "{lhs} MATCHES {{")?;
                for (i, v) in values.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{v}")?;
                }
                write!(f, "}}")
            }
            Self::Exists(p) => write!(f, "EXISTS {p}"),
            Self::And(a, b) => write!(f, "({a} AND {b})"),
            Self::Or(a, b) => write!(f, "({a} OR {b})"),
            Self::Not(a) => write!(f, "NOT ({a})"),
        }
    }
}

/// A predicate attached to a class in `FROM`.
#[derive(Debug, Clone, PartialEq)]
pub enum Predicate {
    /// The `[openEHR-EHR-OBSERVATION.x.v2]` shorthand.
    Archetype(String),
    /// A full condition, such as `[ehr_id/value=$ehrUid]`.
    Standard(Box<Expr>),
}

impl fmt::Display for Predicate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Archetype(id) => write!(f, "[{id}]"),
            Self::Standard(e) => write!(f, "[{e}]"),
        }
    }
}

/// One RM class in the containment tree.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassExpr {
    /// The RM class name, such as `COMPOSITION`.
    pub rm_type: String,
    /// The alias bound to it, if any.
    pub alias: Option<String>,
    /// The predicate narrowing it, if any.
    pub predicate: Option<Predicate>,
}

impl fmt::Display for ClassExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.rm_type)?;
        if let Some(alias) = &self.alias {
            write!(f, " {alias}")?;
        }
        if let Some(predicate) = &self.predicate {
            write!(f, "{predicate}")?;
        }
        Ok(())
    }
}

/// The `FROM` containment tree.
#[derive(Debug, Clone, PartialEq)]
pub enum From {
    /// A class on its own.
    Class(ClassExpr),
    /// `left CONTAINS right`, or `left NOT CONTAINS right`.
    Contains {
        /// The containing side.
        left: Box<From>,
        /// Whether the containment is negated.
        negated: bool,
        /// The contained side.
        right: Box<From>,
    },
    /// `a AND b`
    And(Box<From>, Box<From>),
    /// `a OR b`
    Or(Box<From>, Box<From>),
}

impl From {
    /// Every class in the tree, in the order they appear.
    #[must_use]
    pub fn classes(&self) -> Vec<&ClassExpr> {
        let mut out = Vec::new();
        self.collect_classes(&mut out);
        out
    }

    fn collect_classes<'a>(&'a self, out: &mut Vec<&'a ClassExpr>) {
        match self {
            Self::Class(c) => out.push(c),
            Self::Contains { left, right, .. } | Self::And(left, right) | Self::Or(left, right) => {
                left.collect_classes(out);
                right.collect_classes(out);
            }
        }
    }
}

impl From {
    /// Renders an operand, parenthesised unless it is a bare class.
    ///
    /// Every operator in a `FROM` clause sits at **one** precedence level, and
    /// `CONTAINS` takes the whole remainder as its right operand
    /// (`containment` calls itself there). So the parenthesis is not
    /// decoration: without it, rendering re-associates the tree.
    ///
    /// `Or(Contains(a, b), c)` used to render `(a CONTAINS b OR c)`, and
    /// reparsing that produced `Contains(a, Or(b, c))` — *a containing either b
    /// or c*, where the caller wrote *either (a containing b) or c*. Those
    /// select different records. `A-37`.
    ///
    /// A bare class is left unwrapped so the common query renders unchanged:
    /// `EHR e CONTAINS COMPOSITION c`, not `(EHR e) CONTAINS (COMPOSITION c)`.
    fn operand(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Class(c) => write!(f, "{c}"),
            other => write!(f, "({other})"),
        }
    }
}

impl fmt::Display for From {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Class(c) => write!(f, "{c}"),
            Self::Contains {
                left,
                negated,
                right,
            } => {
                left.operand(f)?;
                write!(f, " {}CONTAINS ", if *negated { "NOT " } else { "" })?;
                right.operand(f)
            }
            Self::And(a, b) => {
                a.operand(f)?;
                write!(f, " AND ")?;
                b.operand(f)
            }
            Self::Or(a, b) => {
                a.operand(f)?;
                write!(f, " OR ")?;
                b.operand(f)
            }
        }
    }
}

/// One projected column.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectColumn {
    /// What is projected.
    pub expr: Operand,
    /// The name it is projected as.
    pub alias: Option<String>,
}

impl fmt::Display for SelectColumn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.expr)?;
        if let Some(alias) = &self.alias {
            write!(f, " AS {alias}")?;
        }
        Ok(())
    }
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Ascending, the default.
    Ascending,
    /// Descending.
    Descending,
}

/// One `ORDER BY` term.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderBy {
    /// The path to sort on.
    pub path: IdentifiedPath,
    /// The direction.
    pub direction: Direction,
}

impl fmt::Display for OrderBy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path)?;
        if self.direction == Direction::Descending {
            write!(f, " DESC")?;
        }
        Ok(())
    }
}

/// A parsed AQL query.
///
/// ```
/// use openehr::aql::AqlQuery;
///
/// let q: AqlQuery = "
///     SELECT c/uid/value AS id
///     FROM EHR e[ehr_id/value=$ehrUid]
///         CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1]
///     WHERE c/context/start_time > '2026-01-01'
///     ORDER BY c/context/start_time DESC
///     LIMIT 10
/// ".parse().unwrap();
///
/// assert_eq!(q.parameters(), vec!["ehrUid"]);
/// assert_eq!(q.archetype_ids(), vec!["openEHR-EHR-COMPOSITION.encounter.v1"]);
/// assert_eq!(q.limit, Some(10));
/// q.check().unwrap();
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct AqlQuery {
    /// Whether duplicate rows are removed.
    pub distinct: bool,
    /// The `TOP n` limit, if present.
    pub top: Option<u64>,
    /// The projected columns.
    pub columns: Vec<SelectColumn>,
    /// The containment tree.
    pub from: From,
    /// The filter.
    pub where_clause: Option<Expr>,
    /// The sort terms.
    pub order_by: Vec<OrderBy>,
    /// The row limit.
    pub limit: Option<u64>,
    /// The row offset.
    pub offset: Option<u64>,
}

impl AqlQuery {
    /// Every alias bound in `FROM`.
    #[must_use]
    pub fn aliases(&self) -> Vec<&str> {
        self.from
            .classes()
            .into_iter()
            .filter_map(|c| c.alias.as_deref())
            .collect()
    }

    /// Every archetype id named by an archetype predicate, in order.
    ///
    /// This is what an authorisation check wants before the query runs: which
    /// archetypes the caller is asking to read.
    #[must_use]
    pub fn archetype_ids(&self) -> Vec<&str> {
        self.from
            .classes()
            .into_iter()
            .filter_map(|c| match &c.predicate {
                Some(Predicate::Archetype(id)) => Some(id.as_str()),
                _ => None,
            })
            .collect()
    }

    /// Every `$parameter` the query uses, deduplicated, in first-use order.
    #[must_use]
    pub fn parameters(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for c in &self.columns {
            walk_operand_parameters(&c.expr, &mut out);
        }
        for class in self.from.classes() {
            if let Some(Predicate::Standard(e)) = &class.predicate {
                walk_expr_parameters(e, &mut out);
            }
        }
        if let Some(w) = &self.where_clause {
            walk_expr_parameters(w, &mut out);
        }
        out
    }

    /// Checks what can be checked without a repository.
    ///
    /// Reports paths rooted at an alias that `FROM` does not bind. This is the
    /// error AQL's syntax makes easiest to write — rename a class alias and
    /// miss one `SELECT` column — and hardest to see, because a query with an
    /// undefined alias is syntactically perfect and returns nothing.
    ///
    /// # Errors
    ///
    /// Returns [`AqlError`] naming the first unbound alias. The offset is 0:
    /// the check runs on the AST, after positions are gone.
    ///
    /// ```
    /// use openehr::aql::AqlQuery;
    ///
    /// let q: AqlQuery = "SELECT o/value FROM COMPOSITION c".parse().unwrap();
    /// // Parses cleanly, returns nothing, and would be debugged at 3am.
    /// assert!(q.check().is_err());
    /// ```
    pub fn check(&self) -> Result<(), AqlError> {
        let aliases = self.aliases();
        let mut unbound: Option<String> = None;
        let mut check_path = |p: &IdentifiedPath| {
            if unbound.is_none() && !aliases.contains(&p.root.as_str()) {
                unbound = Some(p.root.clone());
            }
        };
        for c in &self.columns {
            if let Operand::Path(p) = &c.expr {
                check_path(p);
            }
        }
        for o in &self.order_by {
            check_path(&o.path);
        }
        if let Some(w) = &self.where_clause {
            for p in collect_paths(w) {
                check_path(p);
            }
        }
        match unbound {
            None => Ok(()),
            Some(alias) => Err(AqlError::new(
                0,
                format!("path is rooted at `{alias}`, which FROM does not bind"),
            )),
        }
    }
}

/// Collects `$parameter` names from an operand, deduplicating as it goes.
///
/// Free functions rather than closures: a closure capturing `&mut out` cannot
/// recurse, and the AST is recursive in three places.
fn walk_operand_parameters<'a>(op: &'a Operand, out: &mut Vec<&'a str>) {
    match op {
        Operand::Parameter(name) => {
            if !out.contains(&name.as_str()) {
                out.push(name);
            }
        }
        Operand::Function { args, .. } => {
            for a in args {
                walk_operand_parameters(a, out);
            }
        }
        Operand::Path(_) | Operand::Literal(_) => {}
    }
}

/// Collects `$parameter` names from a condition.
fn walk_expr_parameters<'a>(expr: &'a Expr, out: &mut Vec<&'a str>) {
    match expr {
        Expr::Compare { lhs, rhs, .. } => {
            walk_operand_parameters(lhs, out);
            walk_operand_parameters(rhs, out);
        }
        Expr::MatchesSet { lhs, values } => {
            walk_operand_parameters(lhs, out);
            for v in values {
                walk_operand_parameters(v, out);
            }
        }
        Expr::And(a, b) | Expr::Or(a, b) => {
            walk_expr_parameters(a, out);
            walk_expr_parameters(b, out);
        }
        Expr::Not(a) => walk_expr_parameters(a, out),
        Expr::Exists(_) => {}
    }
}

/// Collects every path referenced by a condition.
fn walk_expr_paths<'a>(expr: &'a Expr, out: &mut Vec<&'a IdentifiedPath>) {
    match expr {
        Expr::Compare { lhs, rhs, .. } => {
            if let Operand::Path(p) = lhs {
                out.push(p);
            }
            if let Operand::Path(p) = rhs {
                out.push(p);
            }
        }
        Expr::MatchesSet { lhs, .. } => {
            if let Operand::Path(p) = lhs {
                out.push(p);
            }
        }
        Expr::Exists(p) => out.push(p),
        Expr::And(a, b) | Expr::Or(a, b) => {
            walk_expr_paths(a, out);
            walk_expr_paths(b, out);
        }
        Expr::Not(a) => walk_expr_paths(a, out),
    }
}

fn collect_paths(expr: &Expr) -> Vec<&IdentifiedPath> {
    let mut out = Vec::new();
    walk_expr_paths(expr, &mut out);
    out
}

impl fmt::Display for AqlQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SELECT ")?;
        if self.distinct {
            f.write_str("DISTINCT ")?;
        }
        if let Some(top) = self.top {
            write!(f, "TOP {top} ")?;
        }
        for (i, c) in self.columns.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{c}")?;
        }
        write!(f, " FROM {}", self.from)?;
        if let Some(w) = &self.where_clause {
            write!(f, " WHERE {w}")?;
        }
        if !self.order_by.is_empty() {
            f.write_str(" ORDER BY ")?;
            for (i, o) in self.order_by.iter().enumerate() {
                if i > 0 {
                    f.write_str(", ")?;
                }
                write!(f, "{o}")?;
            }
        }
        if let Some(limit) = self.limit {
            write!(f, " LIMIT {limit}")?;
        }
        if let Some(offset) = self.offset {
            write!(f, " OFFSET {offset}")?;
        }
        Ok(())
    }
}

impl core::str::FromStr for AqlQuery {
    type Err = AqlError;

    /// # Errors
    ///
    /// Returns [`AqlError`] with the byte offset at which parsing stopped.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Parser::new(lex(s)?, s.len()).query()
    }
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

struct Parser {
    tokens: Vec<Lexed>,
    pos: usize,
    end: usize,
}

impl Parser {
    fn new(tokens: Vec<Lexed>, end: usize) -> Self {
        Self {
            tokens,
            pos: 0,
            end,
        }
    }

    fn offset(&self) -> usize {
        self.tokens.get(self.pos).map_or(self.end, |t| t.offset)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|t| &t.token)
    }

    fn next_token(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).map(|t| t.token.clone());
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn peek_keyword(&self, word: &str) -> bool {
        matches!(self.peek(), Some(Token::Word(w)) if w.eq_ignore_ascii_case(word))
    }

    fn eat_keyword(&mut self, word: &str) -> bool {
        if self.peek_keyword(word) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect_keyword(&mut self, word: &str) -> Result<(), AqlError> {
        if self.eat_keyword(word) {
            Ok(())
        } else {
            Err(AqlError::new(self.offset(), format!("expected `{word}`")))
        }
    }

    fn eat_symbol(&mut self, sym: &str) -> bool {
        if matches!(self.peek(), Some(Token::Symbol(s)) if *s == sym) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect_symbol(&mut self, sym: &str) -> Result<(), AqlError> {
        if self.eat_symbol(sym) {
            Ok(())
        } else {
            Err(AqlError::new(self.offset(), format!("expected `{sym}`")))
        }
    }

    fn query(&mut self) -> Result<AqlQuery, AqlError> {
        self.expect_keyword("SELECT")?;
        let distinct = self.eat_keyword("DISTINCT");
        let top = if self.eat_keyword("TOP") {
            Some(self.integer()?)
        } else {
            None
        };
        if self.eat_symbol("*") {
            return Err(AqlError::new(
                self.offset(),
                "`SELECT *` is not supported: AQL projections must name paths (Q12.9)",
            ));
        }
        let mut columns = vec![self.select_column()?];
        while self.eat_symbol(",") {
            columns.push(self.select_column()?);
        }

        self.expect_keyword("FROM")?;
        let from = self.containment()?;

        let where_clause = if self.eat_keyword("WHERE") {
            Some(self.expr()?)
        } else {
            None
        };

        let mut order_by = Vec::new();
        if self.eat_keyword("ORDER") {
            self.expect_keyword("BY")?;
            loop {
                let path = self.identified_path()?;
                let direction = if self.eat_keyword("DESC") {
                    Direction::Descending
                } else {
                    let _ = self.eat_keyword("ASC");
                    Direction::Ascending
                };
                order_by.push(OrderBy { path, direction });
                if !self.eat_symbol(",") {
                    break;
                }
            }
        }

        let limit = if self.eat_keyword("LIMIT") {
            Some(self.integer()?)
        } else {
            None
        };
        let offset = if self.eat_keyword("OFFSET") {
            Some(self.integer()?)
        } else {
            None
        };

        if self.pos < self.tokens.len() {
            return Err(AqlError::new(self.offset(), "unexpected trailing input"));
        }

        Ok(AqlQuery {
            distinct,
            top,
            columns,
            from,
            where_clause,
            order_by,
            limit,
            offset,
        })
    }

    fn integer(&mut self) -> Result<u64, AqlError> {
        let offset = self.offset();
        match self.next_token() {
            Some(Token::Integer(v)) if v >= 0 => Ok(u64::try_from(v).unwrap_or(0)),
            _ => Err(AqlError::new(offset, "expected a non-negative integer")),
        }
    }

    fn select_column(&mut self) -> Result<SelectColumn, AqlError> {
        let expr = self.operand()?;
        let alias = if self.eat_keyword("AS") {
            let offset = self.offset();
            match self.next_token() {
                Some(Token::Word(w)) => Some(w),
                _ => return Err(AqlError::new(offset, "expected an alias after `AS`")),
            }
        } else {
            None
        };
        Ok(SelectColumn { expr, alias })
    }

    fn operand(&mut self) -> Result<Operand, AqlError> {
        let offset = self.offset();
        match self.next_token() {
            Some(Token::String(v)) => Ok(Operand::Literal(Literal::String(v))),
            Some(Token::Integer(v)) => Ok(Operand::Literal(Literal::Integer(v))),
            Some(Token::Number(v)) => Ok(Operand::Literal(Literal::Number(v))),
            Some(Token::Parameter(name)) => Ok(Operand::Parameter(name)),
            Some(Token::Path(text)) => Ok(Operand::Path(split_path(&text))),
            Some(Token::Word(w)) => {
                if w.eq_ignore_ascii_case("true") {
                    return Ok(Operand::Literal(Literal::Boolean(true)));
                }
                if w.eq_ignore_ascii_case("false") {
                    return Ok(Operand::Literal(Literal::Boolean(false)));
                }
                if self.eat_symbol("(") {
                    let mut args = Vec::new();
                    if !self.eat_symbol(")") {
                        loop {
                            args.push(self.operand()?);
                            if !self.eat_symbol(",") {
                                break;
                            }
                        }
                        self.expect_symbol(")")?;
                    }
                    return Ok(Operand::Function {
                        name: w.to_uppercase(),
                        args,
                    });
                }
                Ok(Operand::Path(IdentifiedPath {
                    root: w,
                    path: None,
                }))
            }
            _ => Err(AqlError::new(offset, "expected a value or a path")),
        }
    }

    fn identified_path(&mut self) -> Result<IdentifiedPath, AqlError> {
        let offset = self.offset();
        match self.next_token() {
            Some(Token::Path(text)) => Ok(split_path(&text)),
            Some(Token::Word(w)) => Ok(IdentifiedPath {
                root: w,
                path: None,
            }),
            _ => Err(AqlError::new(offset, "expected a path")),
        }
    }

    fn containment(&mut self) -> Result<From, AqlError> {
        let mut left = self.containment_primary()?;
        loop {
            if self.peek_keyword("CONTAINS") || self.peek_keyword("NOT") {
                let negated = self.eat_keyword("NOT");
                if negated && !self.peek_keyword("CONTAINS") {
                    return Err(AqlError::new(
                        self.offset(),
                        "expected `CONTAINS` after `NOT`",
                    ));
                }
                self.expect_keyword("CONTAINS")?;
                let right = self.containment()?;
                left = From::Contains {
                    left: Box::new(left),
                    negated,
                    right: Box::new(right),
                };
                continue;
            }
            if self.eat_keyword("AND") {
                let right = self.containment()?;
                left = From::And(Box::new(left), Box::new(right));
                continue;
            }
            if self.eat_keyword("OR") {
                let right = self.containment()?;
                left = From::Or(Box::new(left), Box::new(right));
                continue;
            }
            break;
        }
        Ok(left)
    }

    fn containment_primary(&mut self) -> Result<From, AqlError> {
        if self.eat_symbol("(") {
            let inner = self.containment()?;
            self.expect_symbol(")")?;
            return Ok(inner);
        }
        let offset = self.offset();
        let Some(Token::Word(rm_type)) = self.next_token() else {
            return Err(AqlError::new(offset, "expected an RM class name"));
        };
        if rm_type.eq_ignore_ascii_case("VERSION") {
            return Err(AqlError::new(
                offset,
                "the VERSION class extension is not supported (Q12.9)",
            ));
        }
        // An alias is a bare word that is not one of the words that continue
        // the FROM clause. Checking the keyword list here is what lets
        // `CONTAINS COMPOSITION c CONTAINS OBSERVATION o` parse without
        // treating `CONTAINS` as an alias.
        let alias = match self.peek() {
            Some(Token::Word(w))
                if ![
                    "CONTAINS", "NOT", "AND", "OR", "WHERE", "ORDER", "LIMIT", "OFFSET",
                ]
                .iter()
                .any(|k| w.eq_ignore_ascii_case(k)) =>
            {
                self.next_token();
                match self.tokens[self.pos - 1].token.clone() {
                    Token::Word(w) => Some(w),
                    _ => None,
                }
            }
            _ => None,
        };
        let predicate = if self.eat_symbol("[") {
            let p = self.predicate()?;
            self.expect_symbol("]")?;
            Some(p)
        } else {
            None
        };
        Ok(From::Class(ClassExpr {
            rm_type,
            alias,
            predicate,
        }))
    }

    fn predicate(&mut self) -> Result<Predicate, AqlError> {
        // An archetype shorthand is a single bare word containing `-` and `.`
        // followed immediately by `]`. Anything else is a condition.
        if let Some(Token::Word(w) | Token::Path(w)) = self.peek() {
            let looks_archetype = w.contains('-') && w.contains('.');
            let next_is_close = matches!(
                self.tokens.get(self.pos + 1).map(|t| &t.token),
                Some(Token::Symbol("]"))
            );
            if looks_archetype && next_is_close {
                let id = w.clone();
                self.pos += 1;
                return Ok(Predicate::Archetype(id));
            }
        }
        Ok(Predicate::Standard(Box::new(self.expr()?)))
    }

    fn expr(&mut self) -> Result<Expr, AqlError> {
        let mut left = self.expr_and()?;
        while self.eat_keyword("OR") {
            let right = self.expr_and()?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn expr_and(&mut self) -> Result<Expr, AqlError> {
        let mut left = self.expr_unary()?;
        while self.eat_keyword("AND") {
            let right = self.expr_unary()?;
            left = Expr::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn expr_unary(&mut self) -> Result<Expr, AqlError> {
        if self.eat_keyword("NOT") {
            return Ok(Expr::Not(Box::new(self.expr_unary()?)));
        }
        if self.eat_keyword("EXISTS") {
            return Ok(Expr::Exists(self.identified_path()?));
        }
        if self.eat_symbol("(") {
            let inner = self.expr()?;
            self.expect_symbol(")")?;
            return Ok(inner);
        }
        let lhs = self.operand()?;
        if self.eat_keyword("MATCHES") {
            // openEHR writes value sets in braces; real queries in the wild
            // also use parentheses. Both are accepted, and the closing bracket
            // must match the opening one so that `{a, b)` is an error rather
            // than a silently accepted set.
            let close = if self.eat_symbol("{") {
                "}"
            } else if self.eat_symbol("(") {
                ")"
            } else {
                return Err(AqlError::new(
                    self.offset(),
                    "expected `{` or `(` after MATCHES",
                ));
            };
            let mut values = Vec::new();
            if !self.eat_symbol(close) {
                loop {
                    values.push(self.operand()?);
                    if !self.eat_symbol(",") {
                        break;
                    }
                }
                self.expect_symbol(close)?;
            }
            return Ok(Expr::MatchesSet { lhs, values });
        }
        let offset = self.offset();
        let op = if self.eat_keyword("LIKE") {
            CompareOp::Like
        } else {
            match self.next_token() {
                Some(Token::Symbol("=")) => CompareOp::Equal,
                Some(Token::Symbol("!=")) => CompareOp::NotEqual,
                Some(Token::Symbol(">")) => CompareOp::Greater,
                Some(Token::Symbol(">=")) => CompareOp::GreaterOrEqual,
                Some(Token::Symbol("<")) => CompareOp::Less,
                Some(Token::Symbol("<=")) => CompareOp::LessOrEqual,
                _ => return Err(AqlError::new(offset, "expected a comparison operator")),
            }
        };
        let rhs = self.operand()?;
        Ok(Expr::Compare { lhs, op, rhs })
    }
}

fn split_path(text: &str) -> IdentifiedPath {
    match text.split_once('/') {
        Some((root, path)) => IdentifiedPath {
            root: root.to_owned(),
            path: Some(path.to_owned()),
        },
        None => IdentifiedPath {
            root: text.to_owned(),
            path: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLOOD_PRESSURE: &str = "
        SELECT
            o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude AS systolic,
            o/data[at0001]/events[at0006]/data[at0003]/items[at0005]/value/magnitude AS diastolic,
            c/context/start_time AS taken
        FROM EHR e[ehr_id/value=$ehrUid]
            CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1]
                CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.blood_pressure.v2]
        WHERE o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude >= 140
            OR o/data[at0001]/events[at0006]/data[at0003]/items[at0005]/value/magnitude >= 90
        ORDER BY c/context/start_time DESC
        LIMIT 5
    ";

    #[test]
    fn the_canonical_blood_pressure_query_parses() {
        let q: AqlQuery = BLOOD_PRESSURE.parse().unwrap();
        assert_eq!(q.columns.len(), 3);
        assert_eq!(q.aliases(), vec!["e", "c", "o"]);
        assert_eq!(
            q.archetype_ids(),
            vec![
                "openEHR-EHR-COMPOSITION.encounter.v1",
                "openEHR-EHR-OBSERVATION.blood_pressure.v2"
            ]
        );
        assert_eq!(q.parameters(), vec!["ehrUid"]);
        assert_eq!(q.limit, Some(5));
        assert_eq!(q.order_by[0].direction, Direction::Descending);
        q.check().unwrap();
    }

    #[test]
    fn a_parsed_query_reparses_from_its_own_rendering() {
        let q: AqlQuery = BLOOD_PRESSURE.parse().unwrap();
        let rendered = q.to_string();
        let again: AqlQuery = rendered.parse().unwrap_or_else(|e| {
            panic!("re-parse failed: {e}\nrendered: {rendered}");
        });
        // Full structural equality, not a spot check: the rendering normalises
        // whitespace and adds parentheses (see spec/audit.md A-05), and the
        // point of the requirement is that none of that changes the query.
        assert_eq!(again, q);
    }

    #[test]
    fn an_undefined_alias_is_reported_although_the_query_is_well_formed() {
        // The bug this catches: rename `o` to `obs` in FROM and miss one
        // SELECT column. The query parses, runs, and returns nothing.
        let q: AqlQuery = "SELECT o/value FROM COMPOSITION c CONTAINS OBSERVATION obs"
            .parse()
            .unwrap();
        let err = q.check().unwrap_err();
        assert!(err.reason.contains("`o`"), "{err}");

        let good: AqlQuery = "SELECT obs/value FROM COMPOSITION c CONTAINS OBSERVATION obs"
            .parse()
            .unwrap();
        assert!(good.check().is_ok());
    }

    #[test]
    fn aggregates_and_distinct_parse() {
        let q: AqlQuery = "
            SELECT DISTINCT MAX(o/data/events/data/items/value/magnitude) AS peak, COUNT(c) AS n
            FROM EHR CONTAINS COMPOSITION c CONTAINS OBSERVATION o
        "
        .parse()
        .unwrap();
        assert!(q.distinct);
        assert!(matches!(q.columns[0].expr, Operand::Function { .. }));
        if let Operand::Function { name, .. } = &q.columns[0].expr {
            assert_eq!(name, "MAX");
        }
    }

    #[test]
    fn not_contains_parses_and_keeps_its_negation() {
        let q: AqlQuery = "
            SELECT e/ehr_id/value
            FROM EHR e CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.referral.v1]
                NOT CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.lab_test.v1]
        "
        .parse()
        .unwrap();
        let rendered = q.to_string();
        assert!(rendered.contains("NOT CONTAINS"), "{rendered}");
    }

    #[test]
    fn like_and_offset_parse() {
        let q: AqlQuery = "
            SELECT DISTINCT c/name/value AS n
            FROM EHR e[ehr_id/value=$ehrUid] CONTAINS COMPOSITION c
            WHERE c/context/start_time LIKE '2019-0?-*'
            ORDER BY c/context/start_time
            LIMIT 10 OFFSET 10
        "
        .parse()
        .unwrap();
        assert_eq!(q.offset, Some(10));
        assert_eq!(q.order_by[0].direction, Direction::Ascending);
        assert!(matches!(
            q.where_clause,
            Some(Expr::Compare {
                op: CompareOp::Like,
                ..
            })
        ));
    }

    #[test]
    fn unsupported_constructs_are_refused_and_say_so() {
        for (text, needle) in [
            ("SELECT * FROM COMPOSITION c", "SELECT *"),
            ("SELECT c/uid FROM VERSION v", "VERSION"),
        ] {
            let err = text.parse::<AqlQuery>().unwrap_err();
            assert!(err.reason.contains(needle), "{err}");
            // And the refusal points at the spec section that records it.
            assert!(err.reason.contains("Q12.9"), "{err}");
        }
    }

    #[test]
    fn malformed_queries_report_an_offset() {
        for text in [
            "SELECT",
            "SELECT c/uid",
            "SELECT c/uid FROM",
            "SELECT c/uid FROM COMPOSITION c WHERE",
            "SELECT c/uid FROM COMPOSITION c LIMIT",
            "SELECT c/uid FROM COMPOSITION c EXTRA",
            "SELECT 'unterminated FROM COMPOSITION c",
        ] {
            assert!(text.parse::<AqlQuery>().is_err(), "accepted {text:?}");
        }
    }

    #[test]
    fn parameters_are_collected_from_every_clause_and_deduplicated() {
        let q: AqlQuery = "
            SELECT c/uid/value
            FROM EHR e[ehr_id/value=$ehrUid] CONTAINS COMPOSITION c
            WHERE c/context/start_time > $since AND c/name/value = $name
        "
        .parse()
        .unwrap();
        assert_eq!(q.parameters(), vec!["ehrUid", "since", "name"]);
    }

    #[test]
    fn keywords_are_case_insensitive() {
        let lower: AqlQuery = "select c/uid from COMPOSITION c limit 1".parse().unwrap();
        let upper: AqlQuery = "SELECT c/uid FROM COMPOSITION c LIMIT 1".parse().unwrap();
        assert_eq!(lower, upper);
    }

    /// A string literal's escape handling.
    ///
    /// The lexer's backslash branch — three lines of index arithmetic — could
    /// have every one of its `+`s and `<`s changed with the suite green
    /// (`lib:A-09`). It decides where a quoted literal *ends*, which in a query
    /// language is the boundary that separates a value from syntax. `db:P6.8`
    /// forbids interpolating a value into SQL precisely so that boundary is
    /// never load-bearing downstream, but the parser still has to get it right
    /// to report what the caller actually wrote.
    #[test]
    fn a_string_literal_carries_its_escapes() {
        let value = |text: &str| -> String {
            let q: AqlQuery = format!("SELECT c/uid FROM COMPOSITION c WHERE c/name/value = {text}")
                .parse()
                .unwrap_or_else(|e| panic!("{text}: {e}"));
            match q.where_clause {
                Some(Expr::Compare {
                    rhs: Operand::Literal(Literal::String(v)),
                    ..
                }) => v,
                other => panic!("{text} did not parse to a string literal: {other:?}"),
            }
        };

        // An escaped quote does not end the literal, and the backslash is not
        // kept. `i += 2` — a `-=` here loops forever, a `*=` skips the quote.
        assert_eq!(value(r"'O\'Brien'"), "O'Brien");
        // An escaped backslash is one backslash, and does not then escape the
        // closing quote.
        assert_eq!(value(r"'a\\b'"), r"a\b");
        // A quote of the other kind needs no escape.
        assert_eq!(value(r#""it's here""#), "it's here");
        assert_eq!(value(r#"'say \"hi\"'"#), r#"say "hi""#);
        // Empty, and a lone backslash at the very end of the input: the
        // `i + 1 < bytes.len()` guard is what keeps this from indexing past
        // the end, and nothing exercised it.
        assert_eq!(value("''"), "");
        assert!(
            "SELECT c/uid FROM COMPOSITION c WHERE c/name/value = 'x\\"
                .parse::<AqlQuery>()
                .is_err(),
            "a literal ending in a dangling backslash is unterminated"
        );
    }

    /// A parameter name that runs to the end of the input.
    ///
    /// `while i < bytes.len() && …` scans the name; widening that bound to
    /// `<=` indexes one past the end. Every existing query had something after
    /// its last parameter, so the boundary was never reached.
    #[test]
    fn a_parameter_may_be_the_last_thing_in_a_query() {
        let q: AqlQuery = "SELECT c/uid FROM COMPOSITION c WHERE c/uid/value = $uid"
            .parse()
            .unwrap();
        assert_eq!(q.parameters(), vec!["uid"]);
        // `$` with nothing after it names nothing, and must be refused rather
        // than yielding an empty parameter name.
        assert!("SELECT c/uid FROM COMPOSITION c WHERE c/uid/value = $"
            .parse::<AqlQuery>()
            .is_err());
    }

    /// `LIMIT` and `OFFSET` refuse a negative count — from the lexer.
    ///
    /// Written to test `Parser::integer`'s `v >= 0` guard, which mutation
    /// testing could replace with `true` unnoticed. It turned out the guard
    /// **cannot** be reached: a numeric token starts only at an ASCII digit and
    /// `-` is not in the symbol table, so `Token::Integer` is never negative
    /// and the refusal happens one layer earlier, at `unexpected character`.
    ///
    /// The guard stays. It is one comparison, and it is the layer that would
    /// have to hold if the lexer ever learns a sign — which it should, because
    /// no AQL query here can compare against a negative number at all
    /// (`lib:A-27`). What this test pins is the *behaviour*: negative counts
    /// are refused, positive ones are carried.
    #[test]
    fn a_negative_limit_or_offset_is_refused_rather_than_clamped() {
        for text in [
            "SELECT c/uid FROM COMPOSITION c LIMIT -5",
            "SELECT c/uid FROM COMPOSITION c LIMIT 5 OFFSET -1",
        ] {
            // Refused, not clamped to 0. A `LIMIT 0` that the caller wrote as
            // `-5` returns an empty result set that looks like an answer,
            // which is the failure `db:P6.15` names.
            assert!(text.parse::<AqlQuery>().is_err(), "accepted {text}");
        }
        let q: AqlQuery = "SELECT c/uid FROM COMPOSITION c LIMIT 5 OFFSET 10"
            .parse()
            .unwrap();
        assert_eq!((q.limit, q.offset), (Some(5), Some(10)));
    }

    /// No numeric literal in a condition may be negative.
    ///
    /// A limitation, pinned so that it is a decision rather than a surprise
    /// (`lib:A-27`). `WHERE o/value/magnitude > -2.5` is an ordinary clinical
    /// condition — a base excess, a temperature difference, a scale scored
    /// below zero — and this parser rejects it at the lexer.
    ///
    /// `Q12.9a` says a construct the crate does not model must be refused with
    /// an error rather than parsed and ignored. It is refused. The error names
    /// the character rather than the requirement, which is the part `A-27`
    /// records as unfinished.
    #[test]
    fn a_negative_numeric_literal_is_refused_rather_than_misread() {
        for text in [
            "SELECT c/uid FROM COMPOSITION c WHERE c/v > -1",
            "SELECT c/uid FROM COMPOSITION c WHERE c/v > -2.5",
        ] {
            let err = text.parse::<AqlQuery>().expect_err(text);
            // Refused where the sign is, not silently read as a bare `1`.
            assert_eq!(err.offset, text.find('-').unwrap(), "{}", err.reason);
        }
    }

    /// An error reports where the query went wrong, not where it started.
    ///
    /// `Parser::offset` could return a constant `0` or `1` for every error in
    /// the file and nothing failed. The offset is the whole value of the error
    /// type — a caller shows it to whoever wrote the query.
    #[test]
    fn a_parse_error_points_at_the_token_that_failed() {
        let text = "SELECT c/uid FROM COMPOSITION c WHERE";
        let err = text.parse::<AqlQuery>().expect_err(text);
        assert_eq!(
            err.offset,
            text.len(),
            "running off the end reports the end, not 0"
        );

        // A bad token in the middle reports the middle. `LIMIT` wants an
        // integer and gets a string.
        let text = "SELECT c/uid FROM COMPOSITION c LIMIT 'five'";
        let err = text.parse::<AqlQuery>().expect_err(text);
        assert_eq!(err.offset, text.find('\'').unwrap());
    }

    /// A float literal is a distinct token from an integer.
    ///
    /// Deleting the `Token::Number` arm of `operand` left every test green:
    /// nothing compared a path against a non-integer, although a magnitude is
    /// the commonest thing an AQL condition compares.
    #[test]
    fn a_comparison_may_use_a_float_a_boolean_or_a_negative_number() {
        let q: AqlQuery = "
            SELECT o/value/magnitude
            FROM COMPOSITION c CONTAINS OBSERVATION o
            WHERE o/value/magnitude > 37.5
                AND o/value/units = 'Cel'
                AND o/deleted = false
        "
        .parse()
        .unwrap();
        let rendered = q.to_string();
        for wanted in ["37.5", "'Cel'", "false"] {
            assert!(rendered.contains(wanted), "{wanted} lost from {rendered}");
        }
        assert_eq!(rendered.parse::<AqlQuery>().unwrap(), q);
    }

    /// `NOT`, `MATCHES` and the comparison operators the parser recognises.
    ///
    /// Four comparison arms — `!=`, `<`, `<=`, and the `MATCHES` set loop —
    /// could each be deleted without a test noticing. A dropped `!=` is not a
    /// parse failure a caller sees; it is a *different query*.
    #[test]
    fn every_comparison_operator_and_matches_set_survives_a_round_trip() {
        let q: AqlQuery = "
            SELECT c/uid/value
            FROM COMPOSITION c
            WHERE c/a != 1 AND c/b < 2 AND c/c <= 3 AND c/d > 4 AND c/e >= 5
                AND c/f LIKE 'x%'
                AND NOT c/g = 6
                AND c/category MATCHES {'433', '431', '451'}
        "
        .parse()
        .unwrap();
        let rendered = q.to_string();
        for wanted in [
            "c/a != 1", "c/b < 2", "c/c <= 3", "c/d > 4", "c/e >= 5",
            "c/f LIKE 'x%'", "NOT ", "'433', '431', '451'",
        ] {
            assert!(rendered.contains(wanted), "{wanted} lost from {rendered}");
        }
        // The set kept all three members and their order — the separator's
        // `i > 0` and the comma loop's `!` are both on this line.
        assert_eq!(rendered.parse::<AqlQuery>().unwrap(), q);
    }

    /// A `WHERE` clause's paths are collected, and `check` uses them.
    ///
    /// `collect_paths` could return an empty vector and `walk_expr_paths` could
    /// do nothing at all: `check` would then approve a query whose condition is
    /// rooted at an alias `FROM` never bound, which is the exact defect
    /// `check` is documented as catching.
    #[test]
    fn check_sees_an_unbound_alias_inside_every_shape_of_condition() {
        for condition in [
            "o/value = 1",
            "NOT o/value = 1",
            "c/uid = 1 AND o/value = 1",
            "c/uid = 1 OR o/value = 1",
            "o/value MATCHES {1, 2}",
            "EXISTS o/value",
        ] {
            let text = format!("SELECT c/uid FROM COMPOSITION c WHERE {condition}");
            let q: AqlQuery = text.parse().unwrap_or_else(|e| panic!("{condition}: {e}"));
            let err = q
                .check()
                .expect_err(&format!("`{condition}` is rooted at unbound `o`"));
            assert!(err.reason.contains('o'), "{}", err.reason);
        }
    }

    /// A function call's arguments, and an `ORDER BY` with more than one term.
    ///
    /// Both are rendered by a loop whose `i > 0` separator was untested — one
    /// argument and one sort key never exercise it, and every query in this
    /// module had exactly one of each.
    #[test]
    fn a_rendering_separates_more_than_one_of_everything() {
        let q: AqlQuery = "
            SELECT max(o/value/magnitude, o/value/precision) AS peak
            FROM COMPOSITION c CONTAINS OBSERVATION o
            ORDER BY c/context/start_time DESC, c/uid/value ASC
        "
        .parse()
        .unwrap();
        let rendered = q.to_string();
        assert!(
            rendered.contains("MAX(o/value/magnitude, o/value/precision)"),
            "arguments run together: {rendered}"
        );
        assert!(
            rendered.contains("c/context/start_time DESC, c/uid/value"),
            "sort keys run together: {rendered}"
        );
        assert_eq!(rendered.parse::<AqlQuery>().unwrap(), q);
    }

    /// An archetype shorthand is told apart from a condition by two facts.
    ///
    /// `w.contains('-') && w.contains('.')` and `looks_archetype &&
    /// next_is_close`: widening either to `||` makes the parser read an
    /// ordinary predicate as an archetype id, or the reverse. Both mattered and
    /// neither was tested.
    #[test]
    fn a_predicate_is_an_archetype_only_when_it_looks_like_one_and_stands_alone() {
        // The shorthand.
        let q: AqlQuery = "SELECT c/uid FROM COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1]"
            .parse()
            .unwrap();
        assert_eq!(q.archetype_ids(), vec!["openEHR-EHR-COMPOSITION.encounter.v1"]);

        // A path with a dot but no dash, followed by `=`: a condition, not an
        // archetype. It must not be swallowed as an id.
        let q: AqlQuery = "SELECT c/uid FROM COMPOSITION c[name.value = 'x']"
            .parse()
            .unwrap();
        assert!(
            q.archetype_ids().is_empty(),
            "a condition was read as an archetype id"
        );

        // Something that looks like an archetype id but is compared rather
        // than standing alone is also a condition.
        let q: AqlQuery =
            "SELECT c/uid FROM COMPOSITION c[archetype_node_id = 'openEHR-EHR-COMPOSITION.encounter.v1']"
                .parse()
                .unwrap();
        assert!(q.archetype_ids().is_empty());
    }

    /// A bare alias, with no `/` after it, is a path.
    ///
    /// `ORDER BY c` and `EXISTS c` name the whole object rather than an
    /// attribute of it. The lexer emits a `Word` rather than a `Path` for
    /// those, and deleting that arm of `identified_path` — turning them into
    /// "expected a path" — broke no test.
    #[test]
    fn a_bare_alias_is_a_path_to_the_whole_object() {
        let q: AqlQuery = "SELECT c/uid FROM COMPOSITION c ORDER BY c"
            .parse()
            .unwrap();
        assert_eq!(q.order_by[0].path.root, "c");
        assert!(q.order_by[0].path.path.is_none());
        q.check().expect("`c` is bound by FROM");

        // And it is still checked: a bare alias `FROM` does not bind is the
        // same defect `Q12.14` names, reached by a different route.
        let q: AqlQuery = "SELECT c/uid FROM COMPOSITION c WHERE EXISTS o"
            .parse()
            .unwrap();
        assert!(q.check().is_err(), "`o` is not bound by FROM");
    }

    /// Which bracketed predicates are archetype ids and which are conditions.
    ///
    /// `w.contains('-') && w.contains('.')` and `looks_archetype &&
    /// next_is_close` were each free to become `||`. Widening the first makes
    /// `c[at0001.1]` an archetype id; widening the second makes *any* lone word
    /// one. Both mistakes are silent — `archetype_ids()` is what an
    /// authorisation check reads before a query runs (`Q12.13`), so a word
    /// promoted to an archetype id there is a permission decision made about
    /// something that does not exist.
    #[test]
    fn only_a_dashed_and_dotted_word_standing_alone_is_an_archetype_id() {
        let ids = |text: &str| -> Vec<String> {
            text.parse::<AqlQuery>()
                .unwrap_or_else(|e| panic!("{text}: {e}"))
                .archetype_ids()
                .into_iter()
                .map(str::to_owned)
                .collect()
        };

        assert_eq!(
            ids("SELECT c/uid FROM COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1]"),
            vec!["openEHR-EHR-COMPOSITION.encounter.v1"]
        );
        // Everything else in brackets must be a *condition*. A bare word is
        // refused, whether it has a dot, a dash, or neither — this parser has
        // no node-id shorthand, so `c[at0001]` is an error rather than
        // `archetype_node_id = 'at0001'` (`lib:A-30`).
        //
        // Refusal is what makes these cases evidence: widening either `&&` to
        // `||` accepts them *as archetype ids*, and `archetype_ids()` is what
        // an authorisation check reads before a query runs (`Q12.13`). A word
        // promoted to an archetype id there is a permission decision made
        // about something that does not exist.
        for text in [
            "SELECT c/uid FROM COMPOSITION c[at0001.1]", // a dot, no dash
            "SELECT c/uid FROM COMPOSITION c[some-word]", // a dash, no dot
            "SELECT c/uid FROM COMPOSITION c[at0001]",   // neither
        ] {
            assert!(
                text.parse::<AqlQuery>().is_err(),
                "{text} was accepted, and its predicate read as an archetype id"
            );
        }

        // Looks like an archetype id but does not stand alone.
        assert!(ids(
            "SELECT c/uid FROM COMPOSITION c[archetype_node_id = 'openEHR-EHR-COMPOSITION.encounter.v1']"
        )
        .is_empty());
    }
}
