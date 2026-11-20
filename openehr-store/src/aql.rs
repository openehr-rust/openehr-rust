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
use openehr::base::{HierObjectId, ObjectVersionId};
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
fn unsupported(detail: String) -> StoreError {
    StoreError::AqlUnsupported(detail)
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
///
/// # Panics
///
/// Never: once an `OBJECT_VERSION_ID` parses, its object component is already
/// a `UID`; wrapping that UID as a bare `HIER_OBJECT_ID` adds no extension that
/// could fail validation.
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
pub fn execute(
    store: &dyn Store,
    query: &AqlQuery,
    params: &HashMap<String, Literal>,
) -> Result<QueryResult> {
    // The library's own alias check, first: every path in `columns`,
    // `order_by`, and `where_clause` must root at a bound alias before this
    // module asks any narrower question of it.
    query.check().map_err(|e| unsupported(e.to_string()))?;

    if query.distinct {
        return Err(unsupported("DISTINCT".to_owned()));
    }
    if query.top.is_some() {
        return Err(unsupported("TOP".to_owned()));
    }
    let Some(limit) = query.limit else {
        return Err(unsupported(
            "a query with no LIMIT (db:P6.15, db:P6.7: no unbounded result set)".to_owned(),
        ));
    };

    let (ehr_class, composition_class) = single_containment(&query.from)?;
    if !ehr_class.rm_type.eq_ignore_ascii_case("EHR") {
        return Err(unsupported(format!(
            "FROM must start from EHR, not {}",
            ehr_class.rm_type
        )));
    }
    if !composition_class
        .rm_type
        .eq_ignore_ascii_case("COMPOSITION")
    {
        return Err(unsupported(format!(
            "the contained class must be COMPOSITION, not {}",
            composition_class.rm_type
        )));
    }
    let Some(composition_alias) = composition_class.alias.as_deref() else {
        return Err(unsupported("COMPOSITION must be aliased".to_owned()));
    };
    let Some(Predicate::Archetype(archetype_id)) = &composition_class.predicate else {
        return Err(unsupported(
            "COMPOSITION must be predicated by a bare archetype id, such as [openEHR-EHR-COMPOSITION.encounter.v1]".to_owned(),
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
        let indexed: ObjectVersionId =
            candidate
                .version_uid
                .parse()
                .map_err(|_| StoreError::Engine {
                    engine: "aql",
                    message: format!(
                        "the archetype index named a malformed version uid: {}",
                        candidate.version_uid
                    ),
                })?;
        // Parsing the OBJECT_VERSION_ID above already proved its object part is
        // a UID. A bare HIER_OBJECT_ID adds no fallible extension of its own.
        let container = HierObjectId::new(indexed.object_id().clone(), None)
            .expect("a bare UID is always a valid HIER_OBJECT_ID");
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
             compared against ehr_id/value — this executor has no cross-EHR index"
                .to_owned(),
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
        Expr::MatchesSet { .. } => Err(unsupported("WHERE ... MATCHES { ... }".to_owned())),
        Expr::Exists(_) => Err(unsupported("WHERE EXISTS ...".to_owned())),
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
fn resolve_scalar(
    composition: &Composition,
    path: &IdentifiedPath,
    version_uid: &str,
) -> Result<Cell> {
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
    let comparable = matches!(
        (lhs, rhs),
        (Cell::Str(_), Cell::Str(_))
            | (Cell::Boolean(_), Cell::Boolean(_))
            | (
                Cell::Integer(_) | Cell::Number(_),
                Cell::Integer(_) | Cell::Number(_),
            )
    );
    if !comparable {
        return Ok(op == CompareOp::NotEqual);
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::{
        Cell, compare, compare_cells, eval_expr, eval_operand, execute, literal_to_cell,
        resolve_scalar, scalar_to_cell, single_containment,
    };
    use crate::conformance::{self, RECORD};
    use crate::record::{CompositionIndexRow, VersionRow};
    use crate::{CommitOutcome, Result, Store};
    use openehr::aql::{
        AqlQuery, CompareOp, Expr, From as AqlFrom, IdentifiedPath, Literal, Operand,
    };
    use openehr::base::{HierObjectId, ObjectVersionId};
    use openehr::path::Scalar;
    use openehr::rm::common::{Contribution, Version};
    use openehr::rm::data_types::DvDateTime;
    use openehr::rm::ehr::{Composition, Ehr, EhrStatus};
    use std::collections::HashMap;

    /// Read-only owner-crate fixture for the executor. Engine conformance tests
    /// remain the evidence that real stores supply these rows; this fixture
    /// makes mutations in the engine-agnostic executor visible to this crate's
    /// own tests rather than only to a downstream workspace (`W0.41`).
    struct ReadStore {
        index: Vec<CompositionIndexRow>,
        version: VersionRow,
    }

    impl ReadStore {
        fn one() -> Self {
            let version = conformance::sample_version(1, None, 5);
            let row = VersionRow::project(&version, "contribution", None, None).expect("project");
            let index =
                CompositionIndexRow::project(&row.uid, RECORD, version.data().expect("content"))
                    .expect("index");
            Self {
                index: vec![index],
                version: row,
            }
        }
    }

    impl Store for ReadStore {
        fn engine(&self) -> &'static str {
            "read fixture"
        }

        fn install(&mut self) -> Result<()> {
            unreachable!("the AQL executor does not install a store")
        }

        fn create_ehr(&mut self, _: &Ehr) -> Result<()> {
            unreachable!("the AQL executor does not create an EHR")
        }

        fn get_ehr(&self, _: &HierObjectId) -> Result<Ehr> {
            unreachable!("the AQL executor does not read the EHR row")
        }

        fn create_contribution(&mut self, _: &HierObjectId, _: &Contribution) -> Result<()> {
            unreachable!("the AQL executor does not create a contribution")
        }

        fn commit_composition(
            &mut self,
            _: &HierObjectId,
            _: &Version<Composition>,
            _: &str,
        ) -> Result<CommitOutcome> {
            unreachable!("the AQL executor does not commit")
        }

        fn commit_ehr_status(
            &mut self,
            _: &HierObjectId,
            _: &Version<EhrStatus>,
            _: &str,
        ) -> Result<CommitOutcome> {
            unreachable!("the AQL executor does not commit")
        }

        fn get_version(&self, _: &ObjectVersionId) -> Result<VersionRow> {
            unreachable!("the executor resolves candidates through latest_version")
        }

        fn latest_version(&self, _: &HierObjectId) -> Result<VersionRow> {
            Ok(self.version.clone())
        }

        fn version_at_time(&self, _: &HierObjectId, _: &DvDateTime) -> Result<VersionRow> {
            unreachable!("the AQL executor does not perform historical reads")
        }

        fn all_versions(&self, _: &HierObjectId) -> Result<Vec<VersionRow>> {
            unreachable!("the AQL executor does not enumerate history")
        }

        fn chain_checkpoint(&self, _: &HierObjectId) -> Result<String> {
            unreachable!("the AQL executor does not inspect integrity metadata")
        }

        fn find_compositions_by_archetype(
            &self,
            _: &HierObjectId,
            _: &str,
        ) -> Result<Vec<CompositionIndexRow>> {
            Ok(self.index.clone())
        }
    }

    fn params() -> HashMap<String, Literal> {
        HashMap::from([("ehrUid".to_owned(), Literal::String(RECORD.to_owned()))])
    }

    #[test]
    fn the_owner_crate_executes_projection_filter_sort_and_pagination() {
        let query: AqlQuery = "
            SELECT c/name/value AS name, c/context/location AS location
            FROM EHR e[ehr_id/value=$ehrUid]
                CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1]
            WHERE c/name/value = 'Encounter 1'
            ORDER BY c/name/value DESC
            LIMIT 1 OFFSET 0
        "
        .parse()
        .expect("query");
        let result = execute(&ReadStore::one(), &query, &params()).expect("execute");

        assert_eq!(result.columns, ["name", "location"]);
        assert_eq!(
            result.rows,
            [vec![Cell::Str("Encounter 1".to_owned()), Cell::Null]]
        );
    }

    #[test]
    fn an_unbounded_query_is_refused_before_the_store_is_called() {
        let query: AqlQuery = "
            SELECT c/name/value
            FROM EHR e[ehr_id/value=$ehrUid]
                CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1]
        "
        .parse()
        .expect("query");
        let error = execute(&ReadStore::one(), &query, &params()).expect_err("must refuse");
        assert!(error.to_string().contains("no LIMIT"), "{error}");
    }

    #[test]
    fn every_unsupported_query_shape_is_refused_by_name_in_the_owner_crate() {
        let cases = [
            (
                "SELECT missing/name/value FROM EHR e[ehr_id/value=$ehrUid] CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1] LIMIT 10",
                "FROM does not bind",
            ),
            (
                "SELECT DISTINCT c/name/value FROM EHR e[ehr_id/value=$ehrUid] CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1] LIMIT 10",
                "DISTINCT",
            ),
            (
                "SELECT TOP 1 c/name/value FROM EHR e[ehr_id/value=$ehrUid] CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1] LIMIT 10",
                "TOP",
            ),
            (
                "SELECT c/name/value FROM EHR_STATUS e[ehr_id/value=$ehrUid] CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1] LIMIT 10",
                "FROM must start from EHR",
            ),
            (
                "SELECT c/name/value FROM EHR e[ehr_id/value=$ehrUid] CONTAINS OBSERVATION c[openEHR-EHR-OBSERVATION.test.v1] LIMIT 10",
                "contained class must be COMPOSITION",
            ),
            (
                "SELECT 1 FROM EHR e[ehr_id/value=$ehrUid] CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1] LIMIT 10",
                "not a bare path",
            ),
            (
                "SELECT e/ehr_id/value FROM EHR e[ehr_id/value=$ehrUid] CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1] LIMIT 10",
                "not the COMPOSITION alias",
            ),
            (
                "SELECT c/name/value FROM EHR e[ehr_id/value=$ehrUid] CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1] WHERE c/name/value LIKE 'Encounter%' LIMIT 10",
                "WHERE ... LIKE ...",
            ),
            (
                "SELECT c/name/value FROM EHR e[ehr_id/value=$ehrUid] CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1] WHERE EXISTS c/context LIMIT 10",
                "WHERE EXISTS",
            ),
            (
                "SELECT c/name/value FROM EHR e[ehr_id/value=$ehrUid] CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1] WHERE c/name/value MATCHES {'a', 'b'} LIMIT 10",
                "MATCHES",
            ),
            (
                "SELECT c/name/value FROM EHR e[ehr_id/value=$ehrUid] CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1] WHERE LENGTH(c/name/value) = 1 LIMIT 10",
                "function call in WHERE",
            ),
            (
                "SELECT c/name/value FROM EHR e[ehr_id/value=$ehrUid] CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1] ORDER BY e/ehr_id/value LIMIT 10",
                "not the COMPOSITION alias",
            ),
            (
                "SELECT o/name/value FROM EHR e[ehr_id/value=$ehrUid] CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1] CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.test.v1] LIMIT 10",
                "bare COMPOSITION class",
            ),
        ];

        for (text, expected) in cases {
            let query: AqlQuery = text
                .parse()
                .unwrap_or_else(|error| panic!("{text}: {error}"));
            let error = execute(&ReadStore::one(), &query, &params()).unwrap_err();
            assert!(
                error.to_string().contains(expected),
                "expected {expected:?} from {text}, got {error}"
            );
        }
    }

    #[test]
    fn required_scope_alias_archetype_and_parameter_are_each_checked() {
        let cases = [
            (
                "SELECT 1 FROM EHR e[ehr_id/value=$ehrUid] CONTAINS COMPOSITION[openEHR-EHR-COMPOSITION.encounter.v1] LIMIT 10",
                "COMPOSITION must be aliased",
            ),
            (
                "SELECT c/name/value FROM EHR e[ehr_id/value=$ehrUid] CONTAINS COMPOSITION c LIMIT 10",
                "must be predicated by a bare archetype id",
            ),
            (
                "SELECT c/name/value FROM EHR e CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1] LIMIT 10",
                "EHR must be scoped",
            ),
            (
                "SELECT c/name/value FROM EHR e[name/value=$ehrUid] CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1] LIMIT 10",
                "must name ehr_id/value",
            ),
            (
                "SELECT c/name/value FROM EHR e[ehr_id/value!=$ehrUid] CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1] LIMIT 10",
                "ehr_id/value =",
            ),
            (
                "SELECT c/name/value FROM EHR e[ehr_id/value=17] CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1] LIMIT 10",
                "string literal or a parameter",
            ),
        ];
        for (text, expected) in cases {
            let query: AqlQuery = text
                .parse()
                .unwrap_or_else(|error| panic!("{text}: {error}"));
            let error = execute(&ReadStore::one(), &query, &params()).unwrap_err();
            assert!(
                error.to_string().contains(expected),
                "expected {expected:?} from {text}, got {error}"
            );
        }

        let query: AqlQuery = "
            SELECT c/name/value
            FROM EHR e[ehr_id/value=$ehrUid]
                CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1]
            LIMIT 10
        "
        .parse()
        .unwrap();
        let missing = execute(&ReadStore::one(), &query, &HashMap::new()).unwrap_err();
        assert!(missing.to_string().contains("missing parameter: $ehrUid"));

        let wrong_type = HashMap::from([("ehrUid".to_owned(), Literal::Integer(1))]);
        let error = execute(&ReadStore::one(), &query, &wrong_type).unwrap_err();
        assert!(error.to_string().contains("must be a string ehr_id"));

        let malformed = HashMap::from([("ehrUid".to_owned(), Literal::String(String::new()))]);
        let error = execute(&ReadStore::one(), &query, &malformed).unwrap_err();
        assert!(error.to_string().contains("malformed ehr_id"));
    }

    #[test]
    fn malformed_deleted_and_invalid_index_candidates_have_distinct_outcomes() {
        let query: AqlQuery = "
            SELECT c/name/value
            FROM EHR e[ehr_id/value=$ehrUid]
                CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1]
            LIMIT 10
        "
        .parse()
        .unwrap();

        let mut malformed = ReadStore::one();
        malformed.index[0].version_uid = "not-a-version-uid".to_owned();
        let error = execute(&malformed, &query, &params()).unwrap_err();
        assert!(error.to_string().contains("malformed version uid"));

        let mut deleted = ReadStore::one();
        deleted.version.data_json = None;
        assert!(
            execute(&deleted, &query, &params())
                .unwrap()
                .rows
                .is_empty()
        );

        let mut invalid = ReadStore::one();
        invalid.version.data_json = Some("{".to_owned());
        assert!(execute(&invalid, &query, &params()).is_err());
    }

    #[test]
    fn every_scalar_and_literal_kind_becomes_the_matching_cell() {
        let path = IdentifiedPath {
            root: "c".to_owned(),
            path: Some("value".to_owned()),
        };
        assert_eq!(
            scalar_to_cell(Scalar::Str("text"), &path, "uid").unwrap(),
            Cell::Str("text".to_owned())
        );
        assert_eq!(
            scalar_to_cell(Scalar::Number(1.5), &path, "uid").unwrap(),
            Cell::Number(1.5)
        );
        assert_eq!(
            scalar_to_cell(Scalar::Integer(2), &path, "uid").unwrap(),
            Cell::Integer(2)
        );
        assert_eq!(
            scalar_to_cell(Scalar::Boolean(true), &path, "uid").unwrap(),
            Cell::Boolean(true)
        );

        for (literal, expected) in [
            (
                Literal::String("text".to_owned()),
                Cell::Str("text".to_owned()),
            ),
            (Literal::Integer(2), Cell::Integer(2)),
            (Literal::Number(1.5), Cell::Number(1.5)),
            (Literal::Boolean(true), Cell::Boolean(true)),
        ] {
            assert_eq!(literal_to_cell(&literal), expected);
        }
    }

    #[test]
    fn comparisons_cover_every_operator_without_cross_type_false_positives() {
        for (op, expected) in [
            (CompareOp::Equal, false),
            (CompareOp::NotEqual, true),
            (CompareOp::Greater, false),
            (CompareOp::GreaterOrEqual, false),
            (CompareOp::Less, true),
            (CompareOp::LessOrEqual, true),
        ] {
            assert_eq!(
                compare(&Cell::Integer(1), op, &Cell::Number(2.0)).unwrap(),
                expected
            );
        }
        for (op, expected) in [
            (CompareOp::Equal, false),
            (CompareOp::NotEqual, true),
            (CompareOp::Greater, false),
            (CompareOp::GreaterOrEqual, false),
            (CompareOp::Less, false),
            (CompareOp::LessOrEqual, false),
        ] {
            assert_eq!(
                compare(&Cell::Str("1".to_owned()), op, &Cell::Integer(1)).unwrap(),
                expected,
                "{op}"
            );
        }
        assert!(!compare(&Cell::Null, CompareOp::NotEqual, &Cell::Integer(1)).unwrap());
        assert!(compare(&Cell::Boolean(false), CompareOp::Less, &Cell::Boolean(true)).unwrap());
        assert!(
            compare(
                &Cell::Str("a".to_owned()),
                CompareOp::Less,
                &Cell::Str("b".to_owned())
            )
            .unwrap()
        );
        assert!(compare(&Cell::Integer(2), CompareOp::Equal, &Cell::Number(2.0)).unwrap());
        assert!(compare(&Cell::Number(2.0), CompareOp::Equal, &Cell::Integer(2)).unwrap());
        assert!(compare(&Cell::Integer(2), CompareOp::Equal, &Cell::Integer(2)).unwrap());
        assert!(compare(&Cell::Number(2.0), CompareOp::Equal, &Cell::Number(2.0)).unwrap());
        assert!(compare(&Cell::Integer(1), CompareOp::Like, &Cell::Integer(1)).is_err());

        assert_eq!(
            compare_cells(&Cell::Null, &Cell::Null),
            std::cmp::Ordering::Equal
        );
        assert_eq!(
            compare_cells(&Cell::Null, &Cell::Integer(1)),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_cells(&Cell::Integer(1), &Cell::Null),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_cells(&Cell::Number(f64::NAN), &Cell::Number(1.0)),
            std::cmp::Ordering::Equal
        );
        assert_eq!(
            compare_cells(&Cell::Str("1".to_owned()), &Cell::Boolean(true)),
            std::cmp::Ordering::Equal
        );
    }

    #[test]
    fn literal_scope_runtime_parameters_paths_and_descending_sort_are_driven() {
        let literal_scope: AqlQuery = format!(
            "SELECT c/name/value \
             FROM EHR e[ehr_id/value='{RECORD}'] \
                 CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1] \
             LIMIT 10"
        )
        .parse()
        .expect("query");
        assert_eq!(
            execute(&ReadStore::one(), &literal_scope, &HashMap::new())
                .expect("literal scope")
                .rows
                .len(),
            1
        );

        let runtime_parameter: AqlQuery = "
            SELECT c/name/value
            FROM EHR e[ehr_id/value=$ehrUid]
                CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1]
            WHERE c/name/value = $wanted
            LIMIT 10
        "
        .parse()
        .expect("query");
        let error = execute(&ReadStore::one(), &runtime_parameter, &params()).unwrap_err();
        assert!(error.to_string().contains("missing parameter: $wanted"));

        let composition = conformance::sample_composition("Encounter");
        let structure = IdentifiedPath {
            root: "c".to_owned(),
            path: Some("name".to_owned()),
        };
        assert!(
            resolve_scalar(&composition, &structure, "uid")
                .unwrap_err()
                .to_string()
                .contains("whole structure")
        );
        let malformed = IdentifiedPath {
            root: "c".to_owned(),
            path: Some("[".to_owned()),
        };
        assert!(resolve_scalar(&composition, &malformed, "uid").is_err());

        let repeated = composition
            .clone()
            .with_content(
                openehr::rm::ehr::Section::new(
                    openehr::rm::common::LocatableAttrs::named("One", "at0001").expect("fixture"),
                    Vec::new(),
                )
                .into(),
            )
            .with_content(
                openehr::rm::ehr::Section::new(
                    openehr::rm::common::LocatableAttrs::named("Two", "at0002").expect("fixture"),
                    Vec::new(),
                )
                .into(),
            );
        let many = IdentifiedPath {
            root: "c".to_owned(),
            path: Some("/content/name/value".to_owned()),
        };
        assert!(
            resolve_scalar(&repeated, &many, "uid")
                .unwrap_err()
                .to_string()
                .contains("matched 2 nodes")
        );

        let mut two = ReadStore::one();
        two.index.push(two.index[0].clone());
        let descending: AqlQuery = "
            SELECT c/name/value
            FROM EHR e[ehr_id/value=$ehrUid]
                CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1]
            ORDER BY c/name/value DESC
            LIMIT 10
        "
        .parse()
        .expect("query");
        assert_eq!(execute(&two, &descending, &params()).unwrap().rows.len(), 2);

        let parse_error = "SELECT".parse::<AqlQuery>().unwrap_err();
        assert!(matches!(
            crate::StoreError::from(parse_error),
            crate::StoreError::AqlUnsupported(_)
        ));
    }

    #[test]
    fn internal_planning_guards_refuse_or_panic_when_called_out_of_order() {
        let bare: AqlQuery = "SELECT e/ehr_id/value FROM EHR e LIMIT 1"
            .parse()
            .expect("query");
        assert!(single_containment(&bare.from).is_err());

        let nested: AqlQuery = "
            SELECT o/name/value
            FROM EHR e[ehr_id/value=$ehrUid]
                CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1]
                CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.test.v1]
            LIMIT 1
        "
        .parse()
        .expect("query");
        assert!(single_containment(&nested.from).is_err());
        let AqlFrom::Contains { left, right, .. } = &nested.from else {
            panic!("fixture did not parse containment");
        };
        let rightmost = match right.as_ref() {
            AqlFrom::Contains { right, .. } => right.clone(),
            AqlFrom::Class(_) => right.clone(),
            other => panic!("unexpected fixture tree: {other:?}"),
        };
        let left_nested = AqlFrom::Contains {
            left: Box::new(AqlFrom::Contains {
                left: left.clone(),
                negated: false,
                right: rightmost.clone(),
            }),
            negated: false,
            right: rightmost,
        };
        assert!(single_containment(&left_nested).is_err());

        let composition = conformance::sample_composition("Encounter");
        let exists_query: AqlQuery = "
            SELECT c/name/value
            FROM EHR e[ehr_id/value=$ehrUid]
                CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1]
            WHERE EXISTS c/context
            LIMIT 1
        "
        .parse()
        .expect("query");
        let exists = exists_query.where_clause.as_ref().expect("where");
        assert!(matches!(exists, Expr::Exists(_)));
        assert!(
            std::panic::catch_unwind(|| eval_expr(exists, &composition, &params(), "uid")).is_err()
        );

        let function_query: AqlQuery = "
            SELECT c/name/value
            FROM EHR e[ehr_id/value=$ehrUid]
                CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1]
            WHERE LENGTH(c/name/value) = 1
            LIMIT 1
        "
        .parse()
        .expect("query");
        let Some(Expr::Compare {
            lhs: Operand::Function { .. },
            ..
        }) = function_query.where_clause.as_ref()
        else {
            panic!("fixture did not parse a function operand");
        };
        let Expr::Compare { lhs, .. } = function_query.where_clause.as_ref().unwrap() else {
            unreachable!()
        };
        assert!(
            std::panic::catch_unwind(|| eval_operand(lhs, &composition, &params(), "uid")).is_err()
        );
    }
}
