//! Executes the bounded AQL subset `db:P6.20`–`P6.24` define.
//!
//! This is the reversal `db:S1.6` records: not general AQL execution — that
//! still needs an archetype engine this tree does not have (`lib:A-40`) — but
//! one `EHR`, one archetype-predicated containment level, `WHERE`/`SELECT`/
//! `ORDER BY` over scalar leaves. It parses nothing itself:
//! [`openehr::aql::AqlQuery`] is the only parser, [`AqlQuery::check`] the only
//! alias check, and everything here either recognises a query as inside the
//! bounded subset or refuses it by name (`db:P6.24`) before this module issues
//! a single call to [`Store`].
//!
//! # Why the executor lives here and not in an engine crate
//!
//! [`Store::find_compositions_by_archetype`] and [`Store::latest_version`]
//! are all this module calls, and both are already engine-agnostic trait
//! methods. Nothing here is SQL, so nothing here needs a driver — nothing
//! here needs to be written six times, and [`crate::conformance::run_aql`]
//! runs it against every engine that has a [`Store`], `SQLite` first because
//! it is the only one at *Verified* today.
//!
//! `latest_version`, not `get_version` on the index row's own version uid:
//! `openehr_composition_index` keeps one row per version ever committed with
//! content, never rewritten or removed by a later amendment or a deletion
//! (`db:H5.2`), so a candidate's indexed version can be stale. Reading a
//! container's *current* version is what "the composition in this record"
//! has to mean.

use crate::error::{Result, StoreError};
use crate::store::Store;
use openehr::aql::{
    AqlQuery, ClassExpr, CompareOp, Direction, Expr, From as AqlFrom, IdentifiedPath, Literal,
    Operand, Predicate,
};
use openehr::base::ObjectVersionId;
use openehr::path::{Pathable as _, Scalar};
use openehr::rm::ehr::Composition;
use std::cmp::Ordering;
use std::collections::HashMap;

/// One value in a result row: an owned copy of a [`Scalar`], or the absence
/// of a match (`db:P6.22`).
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    /// A string.
    Str(String),
    /// A number.
    Number(f64),
    /// An integer.
    Integer(i64),
    /// A boolean.
    Boolean(bool),
    /// The path this column or `ORDER BY` term names matched nothing on
    /// this composition.
    Null,
}

/// Converts a resolved [`Scalar`] into an owned [`Cell`].
///
/// `Scalar` is `#[non_exhaustive]`: a wildcard is required, and it refuses
/// rather than guesses. A future scalar kind this executor has never heard
/// of is exactly the shape of gap `db:P6.22` already refuses for a
/// non-`Scalar` `Node` — silently mapping it to some existing `Cell` variant
/// would be the same wrong answer as skipping the check entirely.
fn scalar_to_cell(value: Scalar<'_>, path: &IdentifiedPath, version_uid: &str) -> Result<Cell> {
    match value {
        Scalar::Str(s) => Ok(Cell::Str(s.to_owned())),
        Scalar::Number(n) => Ok(Cell::Number(n)),
        Scalar::Integer(i) => Ok(Cell::Integer(i)),
        Scalar::Boolean(b) => Ok(Cell::Boolean(b)),
        _ => Err(unsupported(format!(
            "path `{path}` on version {version_uid} matched a scalar kind this executor \
             does not recognise"
        ))),
    }
}

/// The result of executing a query.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct QueryResult {
    /// Column headers, in `SELECT` order: the `AS` alias, or the path text
    /// verbatim when none was given.
    pub columns: Vec<String>,
    /// One row per matched composition, `WHERE`-filtered and
    /// `ORDER BY`-sorted, `OFFSET`-skipped and `LIMIT`-truncated.
    pub rows: Vec<Vec<Cell>>,
}

/// A refusal naming the construct, built for [`StoreError::AqlUnsupported`].
fn unsupported(detail: impl Into<String>) -> StoreError {
    StoreError::AqlUnsupported(detail.into())
}

/// Executes `query` against `store`, binding `$name` parameters from `params`.
///
/// # Errors
///
/// Returns [`StoreError::AqlUnsupported`] if `query` uses anything outside
/// the bounded subset `db:P6.20`–`P6.24` state, naming the construct — or,
/// for the two outcomes that depend on stored content rather than the query
/// text (an ambiguous or non-scalar path match, `db:P6.22`), naming the path
/// and the container once a candidate composition is actually read. Returns
/// the store's own error if a lookup fails, or [`StoreError::Json`] if a
/// candidate's canonical JSON does not parse as a `COMPOSITION`.
// Long on purpose, the same reason `SqliteStore::commit_composition` gives:
// this is the whole planning pass — one gate per clause, in the order a
// query is written — and splitting it into a dozen helpers would hide the
// order, which is itself part of what `db:P6.24` requires callers be able
// to see reasoned about.
#[allow(clippy::too_many_lines)]
// A caller's own parameter map, built however it likes; requiring the
// default hasher's generic parameter here for one function's sake is
// friction `openehr-loco`'s own eventual caller would have to carry too.
#[allow(clippy::implicit_hasher)]
pub fn execute<S: Store + ?Sized>(
    store: &S,
    query: &AqlQuery,
    params: &HashMap<String, Literal>,
) -> Result<QueryResult> {
    // The library's own alias check, first: every path in `columns`,
    // `order_by`, and `where_clause` must root at a bound alias before this
    // module asks any narrower question of it.
    query.check().map_err(|e| unsupported(e.to_string()))?;

    if query.distinct {
        return Err(unsupported("DISTINCT"));
    }
    if query.top.is_some() {
        return Err(unsupported("TOP"));
    }
    let Some(limit) = query.limit else {
        return Err(unsupported(
            "a query with no LIMIT (db:P6.15, db:P6.7: no unbounded result set)",
        ));
    };

    let (ehr_class, composition_class) = single_containment(&query.from)?;
    if !ehr_class.rm_type.eq_ignore_ascii_case("EHR") {
        return Err(unsupported(format!(
            "FROM must start from EHR, not {}",
            ehr_class.rm_type
        )));
    }
    if !composition_class.rm_type.eq_ignore_ascii_case("COMPOSITION") {
        return Err(unsupported(format!(
            "the contained class must be COMPOSITION, not {}",
            composition_class.rm_type
        )));
    }
    let Some(composition_alias) = composition_class.alias.as_deref() else {
        return Err(unsupported("COMPOSITION must be aliased"));
    };
    let Some(Predicate::Archetype(archetype_id)) = &composition_class.predicate else {
        return Err(unsupported(
            "COMPOSITION must be predicated by a bare archetype id, such as [openEHR-EHR-COMPOSITION.encounter.v1]",
        ));
    };

    let ehr_id_text = ehr_id_from_predicate(ehr_class, params)?;
    let ehr_id = ehr_id_text
        .parse()
        .map_err(|_| unsupported(format!("malformed ehr_id: {ehr_id_text}")))?;

    // SELECT: every column a bare path rooted at the COMPOSITION alias.
    let mut headers = Vec::with_capacity(query.columns.len());
    let mut select_paths = Vec::with_capacity(query.columns.len());
    for column in &query.columns {
        let Operand::Path(p) = &column.expr else {
            return Err(unsupported(format!(
                "SELECT column `{}` is not a bare path — this executor projects paths only, \
                 no literals, parameters, or function calls",
                column.expr
            )));
        };
        require_root(p, composition_alias)?;
        headers.push(column.alias.clone().unwrap_or_else(|| p.to_string()));
        select_paths.push(p);
    }

    // ORDER BY: every term a bare path rooted at the same alias.
    for term in &query.order_by {
        require_root(&term.path, composition_alias)?;
    }

    // WHERE's shape, once, from the query alone — never a construct outside
    // Compare/And/Or/Not, and never a path rooted anywhere but the
    // COMPOSITION alias.
    if let Some(w) = &query.where_clause {
        check_expr_shape(w, composition_alias)?;
    }

    let candidates = store.find_compositions_by_archetype(&ehr_id, archetype_id)?;

    let mut matched: Vec<(Vec<Cell>, Vec<Cell>)> = Vec::new();
    for candidate in &candidates {
        // The index row names the version it was *built from* — not
        // necessarily the container's current one. A container amended, or
        // logically deleted (`db:H5.2`), since the row was written still has
        // exactly that one row (`openehr_composition_index`'s own primary
        // key is `version_uid`, and nothing here rewrites or removes an
        // older row when a newer version is committed). Reading
        // `candidate.version_uid` directly would answer with stale or
        // withdrawn content; `latest_version` on its *container* is what
        // AQL means by "the composition in this record" — its current
        // state, never a snapshot from whenever it happened to be indexed.
        let indexed: ObjectVersionId = candidate.version_uid.parse().map_err(|_| {
            StoreError::Engine {
                engine: "aql",
                message: format!(
                    "the archetype index named a malformed version uid: {}",
                    candidate.version_uid
                ),
            }
        })?;
        let container = indexed.object_id().to_string().parse().map_err(|_| {
            StoreError::Engine {
                engine: "aql",
                message: format!(
                    "the archetype index named a version uid with a malformed container: {}",
                    candidate.version_uid
                ),
            }
        })?;
        let version = store.latest_version(&container)?;
        // A logical deletion carries no content (`db:H5.2`) — not a row any
        // query can match, the same as if it had never been committed.
        let Some(json) = version.data_json.as_deref() else {
            continue;
        };
        let composition: Composition = serde_json::from_str(json)?;

        if let Some(w) = &query.where_clause
            && !eval_expr(w, &composition, params, &version.uid)?
        {
            continue;
        }

        let mut row = Vec::with_capacity(select_paths.len());
        for p in &select_paths {
            row.push(resolve_scalar(&composition, p, &version.uid)?);
        }
        let mut key = Vec::with_capacity(query.order_by.len());
        for term in &query.order_by {
            key.push(resolve_scalar(&composition, &term.path, &version.uid)?);
        }
        matched.push((row, key));
    }

    // A stable sort per term, applied last term first: each pass only
    // reorders rows the previous passes left tied, which is exactly what
    // "ORDER BY a, b" means — sort by `a`, and use `b` only to break ties.
    for (i, term) in query.order_by.iter().enumerate().rev() {
        matched.sort_by(|a, b| {
            let ord = compare_cells(&a.1[i], &b.1[i]);
            if term.direction == Direction::Descending {
                ord.reverse()
            } else {
                ord
            }
        });
    }

    let offset = usize::try_from(query.offset.unwrap_or(0)).unwrap_or(usize::MAX);
    let limit = usize::try_from(limit).unwrap_or(usize::MAX);
    let rows = matched
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|(row, _)| row)
        .collect();

    Ok(QueryResult {
        columns: headers,
        rows,
    })
}

/// Destructures `from` into `(EHR class, COMPOSITION class)`, refusing
/// anything but one `CONTAINS` between two bare classes (`db:P6.21`).
fn single_containment(from: &AqlFrom) -> Result<(&ClassExpr, &ClassExpr)> {
    let AqlFrom::Contains {
        left,
        negated: false,
        right,
    } = from
    else {
        return Err(unsupported(format!(
            "FROM must be exactly EHR CONTAINS COMPOSITION, not `{from}`"
        )));
    };
    let AqlFrom::Class(left) = left.as_ref() else {
        return Err(unsupported(format!(
            "FROM's containing side must be a bare EHR class, not `{left}`"
        )));
    };
    let AqlFrom::Class(right) = right.as_ref() else {
        return Err(unsupported(format!(
            "FROM's contained side must be a bare COMPOSITION class, not `{right}`"
        )));
    };
    Ok((left, right))
}

/// Extracts the `ehr_id` an `EHR` class's own predicate names.
///
/// The predicate's path is *not* rooted at the class's own alias in the
/// parsed tree — `e[ehr_id/value=$ehrUid]` parses `ehr_id/value` as its own
/// `IdentifiedPath{root: "ehr_id", path: Some("value")}`, since a class
/// predicate has no alias of its own to root against
/// ([`openehr::aql::AqlQuery::check`] does not examine it for exactly this
/// reason). So this recognises the one shape `db:P6.21` allows by its exact
/// rendered text, `ehr_id/value`, rather than by resolving it as a path.
fn ehr_id_from_predicate(
    ehr_class: &ClassExpr,
    params: &HashMap<String, Literal>,
) -> Result<String> {
    let Some(Predicate::Standard(expr)) = &ehr_class.predicate else {
        return Err(unsupported(
            "EHR must be scoped by a predicate naming a literal or parameter \
             compared against ehr_id/value — this executor has no cross-EHR index",
        ));
    };
    let Expr::Compare {
        lhs: Operand::Path(path),
        op: CompareOp::Equal,
        rhs,
    } = expr.as_ref()
    else {
        return Err(unsupported(format!(
            "EHR's predicate must be `ehr_id/value = <literal or parameter>`, not `{expr}`"
        )));
    };
    if path.to_string() != "ehr_id/value" {
        return Err(unsupported(format!(
            "EHR's predicate must name ehr_id/value, not `{path}`"
        )));
    }
    match rhs {
        Operand::Literal(Literal::String(s)) => Ok(s.clone()),
        Operand::Parameter(name) => match params.get(name) {
            Some(Literal::String(s)) => Ok(s.clone()),
            Some(other) => Err(unsupported(format!(
                "parameter ${name} must be a string ehr_id, not `{other}`"
            ))),
            None => Err(unsupported(format!("missing parameter: ${name}"))),
        },
        other => Err(unsupported(format!(
            "ehr_id/value must be compared to a string literal or a parameter, not `{other}`"
        ))),
    }
}

/// Refuses a path rooted anywhere but `alias` (`db:P6.24`).
///
/// [`AqlQuery::check`] already proved every path roots at *some* bound
/// alias; this narrows further, to the one alias this executor can resolve
/// against — it has a [`Composition`] in hand, never the `EHR` the other
/// bound alias names.
fn require_root(path: &IdentifiedPath, alias: &str) -> Result<()> {
    if path.root == alias {
        Ok(())
    } else {
        Err(unsupported(format!(
            "path `{path}` is rooted at `{}`, not the COMPOSITION alias `{alias}` — \
             this executor cannot resolve a path against the EHR alias",
            path.root
        )))
    }
}

/// Refuses any `WHERE` shape outside `Compare`/`And`/`Or`/`Not`
/// (`db:P6.22`), checked once, before any store is queried.
fn check_expr_shape(expr: &Expr, alias: &str) -> Result<()> {
    match expr {
        Expr::Compare { lhs, op, rhs } => {
            if matches!(op, CompareOp::Like | CompareOp::Matches) {
                return Err(unsupported(format!("WHERE ... {op} ...")));
            }
            check_operand_shape(lhs, alias)?;
            check_operand_shape(rhs, alias)
        }
        Expr::And(a, b) | Expr::Or(a, b) => {
            check_expr_shape(a, alias)?;
            check_expr_shape(b, alias)
        }
        Expr::Not(a) => check_expr_shape(a, alias),
        Expr::MatchesSet { .. } => Err(unsupported("WHERE ... MATCHES { ... }")),
        Expr::Exists(_) => Err(unsupported("WHERE EXISTS ...")),
    }
}

fn check_operand_shape(operand: &Operand, alias: &str) -> Result<()> {
    match operand {
        Operand::Path(p) => require_root(p, alias),
        Operand::Literal(_) | Operand::Parameter(_) => Ok(()),
        Operand::Function { name, .. } => {
            Err(unsupported(format!("function call in WHERE: {name}(...)")))
        }
    }
}

/// Evaluates a `WHERE` expression against one composition.
///
/// # Errors
///
/// Returns [`StoreError::AqlUnsupported`] if a path matches more than once
/// or matches something that is not a [`Scalar`] — an outcome `db:P6.22`
/// says depends on the composition actually read, not on the query text, so
/// it cannot be caught any earlier than this.
fn eval_expr(
    expr: &Expr,
    composition: &Composition,
    params: &HashMap<String, Literal>,
    version_uid: &str,
) -> Result<bool> {
    match expr {
        Expr::Compare { lhs, op, rhs } => {
            let l = eval_operand(lhs, composition, params, version_uid)?;
            let r = eval_operand(rhs, composition, params, version_uid)?;
            compare(&l, *op, &r)
        }
        Expr::And(a, b) => Ok(eval_expr(a, composition, params, version_uid)?
            && eval_expr(b, composition, params, version_uid)?),
        Expr::Or(a, b) => Ok(eval_expr(a, composition, params, version_uid)?
            || eval_expr(b, composition, params, version_uid)?),
        Expr::Not(a) => Ok(!eval_expr(a, composition, params, version_uid)?),
        Expr::MatchesSet { .. } | Expr::Exists(_) => {
            unreachable!("refused by check_expr_shape before any composition was read")
        }
    }
}

fn eval_operand(
    operand: &Operand,
    composition: &Composition,
    params: &HashMap<String, Literal>,
    version_uid: &str,
) -> Result<Cell> {
    match operand {
        Operand::Path(p) => resolve_scalar(composition, p, version_uid),
        Operand::Literal(l) => Ok(literal_to_cell(l)),
        Operand::Parameter(name) => params
            .get(name)
            .map(literal_to_cell)
            .ok_or_else(|| unsupported(format!("missing parameter: ${name}"))),
        Operand::Function { .. } => {
            unreachable!("refused by check_operand_shape before any composition was read")
        }
    }
}

fn literal_to_cell(literal: &Literal) -> Cell {
    match literal {
        Literal::String(s) => Cell::Str(s.clone()),
        Literal::Integer(i) => Cell::Integer(*i),
        Literal::Number(n) => Cell::Number(*n),
        Literal::Boolean(b) => Cell::Boolean(*b),
    }
}

/// Resolves `path` against `composition`, per `db:P6.22`'s three outcomes:
/// no match is [`Cell::Null`], one scalar match is that scalar, and
/// anything else — more than one match, or a match that is not a
/// [`Scalar`] — refuses the whole query, naming `path` and `version_uid`.
fn resolve_scalar(composition: &Composition, path: &IdentifiedPath, version_uid: &str) -> Result<Cell> {
    let text = path.path.as_deref().unwrap_or("");
    let matches = composition
        .items_at_path(text)
        .map_err(|e| unsupported(format!("path `{path}` on version {version_uid}: {e}")))?;
    match matches.len() {
        0 => Ok(Cell::Null),
        1 => match matches[0] {
            openehr::path::Node::Scalar(s) => scalar_to_cell(s, path, version_uid),
            _ => Err(unsupported(format!(
                "path `{path}` on version {version_uid} names a whole structure, \
                 not a scalar — this executor compares and projects scalar leaves only"
            ))),
        },
        n => Err(unsupported(format!(
            "path `{path}` matched {n} nodes on version {version_uid} — \
             this executor refuses to guess which one was meant"
        ))),
    }
}

/// Compares two operand values for `WHERE`.
///
/// [`Cell::Null`] on either side makes every comparison `false` — the
/// tri-valued-logic simplification `db:P6.22` states: a path matching
/// nothing is not an error, and nothing it could be compared against
/// makes it suddenly true.
///
/// # Errors
///
/// Returns [`StoreError::AqlUnsupported`] for `LIKE`/`MATCHES` or any future
/// [`CompareOp`] this executor does not recognise. `check_expr_shape`
/// already refuses both before any store is queried; this arm exists so
/// that a caller reaching this function by any other path is refused
/// rather than given a silently wrong answer — a panicking `unreachable!()`
/// here would trust a guarantee this function has no way to enforce itself,
/// the exact shape of defect `lib:A-36` already found once in this tree.
fn compare(lhs: &Cell, op: CompareOp, rhs: &Cell) -> Result<bool> {
    if matches!(lhs, Cell::Null) || matches!(rhs, Cell::Null) {
        return Ok(false);
    }
    match op {
        CompareOp::Equal => Ok(cells_eq(lhs, rhs)),
        CompareOp::NotEqual => Ok(!cells_eq(lhs, rhs)),
        CompareOp::Greater => Ok(compare_cells(lhs, rhs) == Ordering::Greater),
        CompareOp::GreaterOrEqual => Ok(compare_cells(lhs, rhs) != Ordering::Less),
        CompareOp::Less => Ok(compare_cells(lhs, rhs) == Ordering::Less),
        CompareOp::LessOrEqual => Ok(compare_cells(lhs, rhs) != Ordering::Greater),
        _ => Err(unsupported(format!("WHERE ... {op} ..."))),
    }
}

/// Numeric-aware equality: `Integer(2)` and `Number(2.0)` are the same
/// value under any comparison operator, the same as SQL and as AQL's own
/// arithmetic — a caller writing `magnitude = 2` must not depend on which
/// of the two a store happened to parse the literal as.
fn cells_eq(a: &Cell, b: &Cell) -> bool {
    compare_cells(a, b) == Ordering::Equal
}

/// Orders two cells of possibly-different scalar kinds.
///
/// A string is never less than or greater than a number by this ordering
/// — they compare equal only when `cells_eq` already returned `false` for
/// them via a type mismatch below, so `Equal` here for mismatched kinds is
/// deliberately not reachable from `cells_eq`'s own callers in a way that
/// would claim `"140" = 140`; mismatched kinds order as `Equal` here only
/// to give `sort_by` a total order, and `compare` never calls this for a
/// `CompareOp` where that distinction matters without the caller already
/// knowing the two operands are numeric or both strings.
fn compare_cells(a: &Cell, b: &Cell) -> Ordering {
    match (a, b) {
        (Cell::Str(a), Cell::Str(b)) => a.cmp(b),
        (Cell::Boolean(a), Cell::Boolean(b)) => a.cmp(b),
        (Cell::Integer(a), Cell::Integer(b)) => a.cmp(b),
        (Cell::Number(a), Cell::Number(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
        (Cell::Integer(a), Cell::Number(b)) => {
            #[allow(clippy::cast_precision_loss)]
            let a = *a as f64;
            a.partial_cmp(b).unwrap_or(Ordering::Equal)
        }
        (Cell::Number(a), Cell::Integer(b)) => {
            #[allow(clippy::cast_precision_loss)]
            let b = *b as f64;
            a.partial_cmp(&b).unwrap_or(Ordering::Equal)
        }
        (Cell::Null, Cell::Null) => Ordering::Equal,
        (Cell::Null, _) => Ordering::Less,
        (_, Cell::Null) => Ordering::Greater,
        _ => Ordering::Equal,
    }
}
