//! Validating a Reference Model instance against an archetype: `K15.18`–`K15.23`.
//!
//! # What this checks, and against what
//!
//! [`validate_against_archetype`] walks an RM instance against an
//! [`Archetype`]'s `definition` in parallel, attribute by attribute, and
//! reports every place the instance does not conform: a missing mandatory
//! attribute (`existence`), too many or too few of something (`cardinality`,
//! `occurrences`), a node identifier or RM class that does not match, and a
//! primitive value outside what its `C_PRIMITIVE` permits.
//!
//! **The archetype is used as though it were already flat.** This crate does
//! not implement specialisation and flattening (`K15.11`–`K15.13`) or template
//! expansion (`K15.14`–`K15.17`): an [`Archetype`] built with everything the
//! check needs already in its own `definition` — which `K15.4` guarantees is
//! constructible without a parser — validates correctly, and a specialised
//! archetype's constraints inherited from an ancestor are **not** merged in
//! first. `K15.31` requires this said plainly rather than left for a reader to
//! discover: this function validates against an archetype, not against an
//! operational template (`K15.15`), because this crate does not produce one.
//!
//! # `ARCHETYPE_SLOT` and `C_ARCHETYPE_ROOT`: resolved only with a repository
//!
//! [`validate_against_archetype`] never resolves either — there is nowhere
//! for it to resolve them *from* (`K15.24`). [`validate_with_repository`]
//! takes an [`crate::am::ArchetypeRepository`] and, for a `C_ARCHETYPE_ROOT`
//! (a slot a template has already filled, naming the filler archetype by
//! id), resolves it and validates the same subtree against the filler's own
//! `definition` and terminology.
//!
//! A bare `ARCHETYPE_SLOT` is different in kind: **which** archetype filled it
//! is recorded on the *instance*'s `ARCHETYPED.archetype_id`, which
//! [`crate::path::Node::archetype_details`] exposes (`A-60`). That alone
//! settles two of the three cases — a closed slot filled anyway is a
//! violation, an unrestricted open slot has nothing left to check — and the
//! third, a slot restricted by `include`/`exclude` assertions, carries them
//! (`A-66`) without evaluating them (`K15.10`'s remainder), so its filler is
//! reported unchecked, naming the filler's own archetype id.
//!
//! # `K15.20`: no partial pass
//!
//! A construct this module cannot check — a restricted `ARCHETYPE_SLOT`'s
//! filler, against `include`/`exclude` assertions it carries but does not
//! evaluate; a
//! `C_ARCHETYPE_ROOT` when no repository is supplied, or when retrieval
//! fails (`K15.27`), or when the caller has not opted into an
//! unestablished-provenance result (`K15.26`); a `C_UNSUPPORTED` primitive
//! constraint; a `C_STRING` pattern, carried but not compiled or applied
//! (see [`crate::am::CPrimitive`]); a `C_ATTRIBUTE_TUPLE` co-varying
//! constraint (see [`crate::am::CAttributeTuple`]) — is recorded as
//! [`Unchecked`], never silently treated as satisfied.
//! [`ArchetypeReport::is_conformant`] is `false` whenever anything is
//! unchecked, exactly as it is when something is violated: an unchecked node
//! is not a passing node.
//!
//! # `K15.19`: a separate verdict from Reference-Model validation
//!
//! [`ArchetypeViolation`] is a different type from [`crate::error::Violation`]
//! on purpose. "This is not a valid `COMPOSITION`" ([`crate::validation`]) and
//! "this is a valid `COMPOSITION` that does not conform to this archetype"
//! (here) are different facts about a document, and this module does not
//! re-check any RM-level invariant — a caller wanting both runs both.
//!
//! # `K15.21`: no node content
//!
//! A violation carries an archetype path, the archetype identifier, and which
//! check failed — never the value that failed it (`X11.7`), on the same terms
//! [`crate::validation`] holds for Reference-Model violations.
//!
//! # `K15.22`: internal codes only
//!
//! An `at`-code is checked against the archetype's own value sets and inline
//! code lists — the terminology this crate has in hand. A binding to an
//! external terminology (SNOMED CT, LOINC) is never consulted here, so it is
//! never reported as satisfied; `S1.10` still governs external terminology
//! generally.
//!
//! # `K15.23`: deterministic and offline
//!
//! Every function here is a pure walk over two trees already in memory. There
//! is no I/O to make non-deterministic in the first place.

use crate::am::archetype::Archetype;
use crate::am::archetype_hrid::ArchetypeHrid;
use crate::am::constraint::{
    ArchetypeSlot, CAttribute, CAttributeTuple, CComplexObject, CObject, CPrimitive,
    CPrimitiveObject, CPrimitiveTuple, ConstraintStatus, is_c_string_pattern,
};
use crate::am::repository::ArchetypeRepository;
use crate::am::terminology::ArchetypeTerminology;
use crate::base::{ArchetypeId, Interval, Real};
use crate::path::{Node, Scalar};
use core::cmp::Ordering;
use core::fmt;

/// One way an instance failed to conform to the archetype it was checked
/// against.
///
/// Carries no node content (`X11.7`, `K15.21`): only the archetype path, the
/// archetype identifier, and a stable tag for which check failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchetypeViolation {
    archetype_path: String,
    archetype_id: ArchetypeHrid,
    constraint: &'static str,
}

impl ArchetypeViolation {
    /// The archetype path to the offending node, from the root of the
    /// validated object. Empty for the root itself.
    #[must_use]
    pub fn archetype_path(&self) -> &str {
        &self.archetype_path
    }

    /// The archetype this instance was checked against.
    #[must_use]
    pub const fn archetype_id(&self) -> &ArchetypeHrid {
        &self.archetype_id
    }

    /// Which check failed: `Existence`, `Cardinality`, `Occurrences`,
    /// `Rm_type_name_matches`, `Unrecognised_node_id`,
    /// `Primitive_kind_mismatch`, or the `C_*` constraint kind that rejected
    /// the value.
    #[must_use]
    pub const fn constraint(&self) -> &'static str {
        self.constraint
    }
}

impl fmt::Display for ArchetypeViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} does not satisfy {}",
            if self.archetype_path.is_empty() {
                "/"
            } else {
                &self.archetype_path
            },
            self.archetype_id,
            self.constraint
        )
    }
}

/// A node this module could not check against its constraint, and why
/// (`K15.20`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unchecked {
    archetype_path: String,
    reason: &'static str,
    /// Design-time vocabulary safe to echo (`X11.7`) — an archetype id, a
    /// [`crate::am::RepositoryError`]'s own text — never node content.
    /// `None` when `reason` alone says everything there is to say.
    detail: Option<String>,
}

impl Unchecked {
    /// The archetype path to the unchecked node.
    #[must_use]
    pub fn archetype_path(&self) -> &str {
        &self.archetype_path
    }

    /// Why it could not be checked.
    #[must_use]
    pub const fn reason(&self) -> &'static str {
        self.reason
    }

    /// Further detail, where `reason` alone does not say everything — an
    /// archetype identifier, a retrieval failure's own message. Design-time
    /// vocabulary only, never node content (`X11.7`).
    #[must_use]
    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }
}

/// The outcome of validating one instance against one archetype.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchetypeReport {
    violations: Vec<ArchetypeViolation>,
    unchecked: Vec<Unchecked>,
    unverified_provenance: Vec<String>,
}

impl ArchetypeReport {
    /// Every place the instance did not conform, in document order.
    #[must_use]
    pub fn violations(&self) -> &[ArchetypeViolation] {
        &self.violations
    }

    /// Every node this module could not check, in document order.
    #[must_use]
    pub fn unchecked(&self) -> &[Unchecked] {
        &self.unchecked
    }

    /// Archetype paths validated using a filler archetype whose provenance
    /// could not be established, because the caller opted in
    /// (`RepositoryOptions::allow_unestablished_provenance`, `K15.26`).
    /// Recorded, not hidden, even though the caller allowed it.
    #[must_use]
    pub fn unverified_provenance(&self) -> &[String] {
        &self.unverified_provenance
    }

    /// Whether the instance conforms.
    ///
    /// `K15.20`: **no partial pass.** `false` if anything violated a
    /// constraint, and equally `false` if anything was left unchecked — a
    /// node this module could not evaluate is not a node it can vouch for.
    #[must_use]
    pub fn is_conformant(&self) -> bool {
        self.violations.is_empty() && self.unchecked.is_empty()
    }
}

/// Whether to use a resolved archetype whose provenance could not be
/// established (`K15.26`). Off by default: retrieval that cannot verify what
/// it retrieved does not get to validate against it silently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RepositoryOptions {
    allow_unestablished_provenance: bool,
}

impl RepositoryOptions {
    /// Allows validation to proceed using a [`crate::am::Resolved`] archetype
    /// with no [`crate::am::Provenance`]. Every path where this happens is
    /// recorded in [`ArchetypeReport::unverified_provenance`] regardless.
    #[must_use]
    pub const fn allow_unestablished_provenance(mut self) -> Self {
        self.allow_unestablished_provenance = true;
        self
    }
}

/// Accumulates violations and unchecked nodes while walking the two trees.
///
/// `archetype_id` and `terminology` are **not** stored here — they are
/// per-node-tree, and change when the walk crosses into a `C_ARCHETYPE_ROOT`
/// filler resolved from a repository, which is an [`Archetype`] this
/// `Ctx`'s own lifetime cannot be tied to (it is retrieved, and owned,
/// partway through the walk). Passed as explicit arguments to every `walk_*`
/// function instead, so each recursion carries whichever archetype it is
/// actually inside.
struct Ctx<'a> {
    repository: Option<&'a dyn ArchetypeRepository>,
    options: RepositoryOptions,
    violations: Vec<ArchetypeViolation>,
    unchecked: Vec<Unchecked>,
    unverified_provenance: Vec<String>,
}

impl Ctx<'_> {
    fn violation(&mut self, archetype_id: &ArchetypeHrid, path: &str, constraint: &'static str) {
        self.violations.push(ArchetypeViolation {
            archetype_path: path.to_owned(),
            archetype_id: archetype_id.clone(),
            constraint,
        });
    }

    fn unchecked(&mut self, path: &str, reason: &'static str) {
        self.unchecked.push(Unchecked {
            archetype_path: path.to_owned(),
            reason,
            detail: None,
        });
    }

    fn unchecked_detail(&mut self, path: &str, reason: &'static str, detail: impl Into<String>) {
        self.unchecked.push(Unchecked {
            archetype_path: path.to_owned(),
            reason,
            detail: Some(detail.into()),
        });
    }

    fn finish(self) -> ArchetypeReport {
        ArchetypeReport {
            violations: self.violations,
            unchecked: self.unchecked,
            unverified_provenance: self.unverified_provenance,
        }
    }
}

/// Validates `root` against `archetype`'s definition (`K15.18`).
///
/// See the module documentation for what "against" means here: `archetype` is
/// used as an already-flat constraint tree, not as an operational template
/// this crate does not yet produce.
///
/// **Whether `archetype` is the right archetype for `root` is the caller's
/// question, not this function's.** The definition's own root node id
/// (`id1`, conventionally) is an ADL-internal label, not what an instance
/// carries: openEHR's own convention gives the *root* of an archetyped
/// substructure an `archetype_node_id` equal to the archetype identifier
/// itself, recorded on `ARCHETYPED.archetype_id` rather than mirrored onto
/// `archetype_node_id` in every RM class this crate's [`crate::path::Node`]
/// walks — [`crate::validation`] already checks that identifier against the
/// RM class it annotates (`Archetype_id_rm_entity_matches`). Comparing the
/// root's own id here would be comparing two different things that happen to
/// look alike, so this function checks the RM class the archetype declares
/// and starts walking from there.
#[must_use]
pub fn validate_against_archetype(archetype: &Archetype, root: Node<'_>) -> ArchetypeReport {
    let mut ctx = Ctx {
        repository: None,
        options: RepositoryOptions::default(),
        violations: Vec::new(),
        unchecked: Vec::new(),
        unverified_provenance: Vec::new(),
    };
    run(archetype, root, &mut ctx);
    ctx.finish()
}

/// Validates `root` against `archetype`'s definition, resolving any
/// `C_ARCHETYPE_ROOT` filler through `repository` (`K15.18`, `K15.24`).
///
/// See the module documentation for exactly what is and is not resolved: a
/// filled slot (`C_ARCHETYPE_ROOT`) is; a bare `ARCHETYPE_SLOT` is not, even
/// here, because nothing in this crate can name which archetype fills it.
#[must_use]
pub fn validate_with_repository(
    archetype: &Archetype,
    root: Node<'_>,
    repository: &dyn ArchetypeRepository,
    options: RepositoryOptions,
) -> ArchetypeReport {
    let mut ctx = Ctx {
        repository: Some(repository),
        options,
        violations: Vec::new(),
        unchecked: Vec::new(),
        unverified_provenance: Vec::new(),
    };
    run(archetype, root, &mut ctx);
    ctx.finish()
}

fn run(archetype: &Archetype, root: Node<'_>, ctx: &mut Ctx<'_>) {
    if root.type_name() != archetype.rm_type_name() {
        // A root of the wrong RM class invalidates everything beneath it too
        // — walking further would only produce a cascade of "attribute not
        // found" noise from a class the archetype never described.
        ctx.violation(archetype.archetype_id(), "", "Rm_type_name_matches");
        return;
    }
    walk_complex(
        archetype.archetype_id(),
        archetype.terminology(),
        archetype.definition(),
        root,
        "",
        ctx,
    );
}

/// Checks every attribute constraint against the data reachable through it.
fn walk_complex(
    archetype_id: &ArchetypeHrid,
    terminology: &ArchetypeTerminology,
    constraint: &CComplexObject,
    node: Node<'_>,
    path: &str,
    ctx: &mut Ctx<'_>,
) {
    for attribute in constraint.attributes() {
        walk_attribute(archetype_id, terminology, attribute, node, path, ctx);
    }
    for tuple in constraint.attribute_tuples() {
        walk_attribute_tuple(archetype_id, terminology, tuple, node, path, ctx);
    }
}

/// Three-valued outcome for one `C_PRIMITIVE_OBJECT` (a tuple row's one
/// column) or, aggregated, for a whole row or the whole tuple —
/// `walk_primitive`'s own binary violation-or-not is not enough here,
/// because an *unchecked* column (a `C_STRING` pattern, `C_UNSUPPORTED`, …)
/// must keep a row's fate open rather than being folded into either
/// "matches" or "does not match".
#[derive(Clone, Copy, PartialEq, Eq)]
enum TupleVerdict {
    Conforms,
    Violates,
    Unchecked,
}

/// A `C_ATTRIBUTE_TUPLE`: whether the instance's actual values across every
/// co-varying attribute, taken together, match one of the permitted rows.
///
/// Delegates each column's own comparison to [`walk_primitive`] itself —
/// against a scratch [`Ctx`] whose result is inspected and discarded —
/// rather than re-implementing the six primitive kinds' comparison rules a
/// second time (`lib:A-33` is the standing reason this crate keeps a rule in
/// exactly one place). The three-valued combination above it is the only
/// logic this function adds.
fn walk_attribute_tuple(
    archetype_id: &ArchetypeHrid,
    terminology: &ArchetypeTerminology,
    tuple: &CAttributeTuple,
    node: Node<'_>,
    path: &str,
    ctx: &mut Ctx<'_>,
) {
    // Resolve each co-varying attribute to exactly one instance value.
    // AOM2's own tuple examples are always single-valued attributes
    // (`units`, `magnitude`, `value`, `symbol`) — a container attribute, or
    // one absent entirely, is not a shape this check can resolve to a
    // single value to compare, so it is reported unchecked rather than
    // guessed at.
    let mut values: Vec<Node<'_>> = Vec::with_capacity(tuple.members().len());
    for attribute in tuple.members() {
        let attr_path = format!("{path}/{}", attribute.rm_attribute_name());
        match node.children(attribute.rm_attribute_name()).as_slice() {
            [only] => values.push(*only),
            other => {
                ctx.unchecked_detail(
                    &attr_path,
                    "C_ATTRIBUTE_TUPLE column does not resolve to exactly one value",
                    format!("{} value(s)", other.len()),
                );
                return;
            }
        }
    }

    let column_verdict = |primitive: &CPrimitiveObject, value: Node<'_>| -> TupleVerdict {
        let mut scratch = Ctx {
            repository: ctx.repository,
            options: ctx.options,
            violations: Vec::new(),
            unchecked: Vec::new(),
            unverified_provenance: Vec::new(),
        };
        walk_primitive(
            archetype_id,
            terminology,
            primitive.constraint(),
            value,
            "",
            &mut scratch,
        );
        if !scratch.violations.is_empty() {
            TupleVerdict::Violates
        } else if !scratch.unchecked.is_empty() {
            TupleVerdict::Unchecked
        } else {
            TupleVerdict::Conforms
        }
    };

    // A row conforms only if every column does (`Violates` anywhere wins);
    // an `Unchecked` column keeps the row open unless a later column
    // outright violates.
    let row_verdict = |row: &CPrimitiveTuple| -> TupleVerdict {
        row.members()
            .iter()
            .zip(&values)
            .map(|(primitive, &value)| column_verdict(primitive, value))
            .fold(TupleVerdict::Conforms, |acc, v| match (acc, v) {
                (TupleVerdict::Violates, _) | (_, TupleVerdict::Violates) => TupleVerdict::Violates,
                (TupleVerdict::Unchecked, _) | (_, TupleVerdict::Unchecked) => {
                    TupleVerdict::Unchecked
                }
                (TupleVerdict::Conforms, TupleVerdict::Conforms) => TupleVerdict::Conforms,
            })
    };

    // The tuple as a whole conforms if any row does (`Conforms` anywhere
    // wins); an `Unchecked` row keeps the whole tuple open unless a
    // `Conforms` row is found. Only when every row definitely violates is
    // the tuple itself a violation — the instance's values do not match any
    // permitted combination. An empty `tuples` list (AOM2 allows it, `0..1`)
    // has no combination to compare against at all, so `reduce` returning
    // `None` becomes `Unchecked` rather than `Violates` — this crate has no
    // stated reading of what an attribute tuple naming no rows means for
    // conformance, and is not inventing one here.
    let verdict = tuple
        .tuples()
        .iter()
        .map(row_verdict)
        .reduce(|acc, v| match (acc, v) {
            (TupleVerdict::Conforms, _) | (_, TupleVerdict::Conforms) => TupleVerdict::Conforms,
            (TupleVerdict::Unchecked, _) | (_, TupleVerdict::Unchecked) => TupleVerdict::Unchecked,
            (TupleVerdict::Violates, TupleVerdict::Violates) => TupleVerdict::Violates,
        })
        .unwrap_or(TupleVerdict::Unchecked);

    match verdict {
        TupleVerdict::Conforms => {}
        TupleVerdict::Violates => ctx.violation(archetype_id, path, "C_ATTRIBUTE_TUPLE"),
        TupleVerdict::Unchecked => {
            let names = tuple
                .members()
                .iter()
                .map(CAttribute::rm_attribute_name)
                .collect::<Vec<_>>()
                .join(", ");
            ctx.unchecked_detail(
                path,
                "C_ATTRIBUTE_TUPLE: no row could be confirmed or ruled out for every column",
                names,
            );
        }
    }
}

/// One `C_ATTRIBUTE`: existence, cardinality, and each alternative beneath it.
fn walk_attribute(
    archetype_id: &ArchetypeHrid,
    terminology: &ArchetypeTerminology,
    attribute: &CAttribute,
    node: Node<'_>,
    path: &str,
    ctx: &mut Ctx<'_>,
) {
    let children = node.children(attribute.rm_attribute_name());
    let attr_path = format!("{path}/{}", attribute.rm_attribute_name());

    if children.is_empty() {
        if attribute.existence().lower() > 0 {
            ctx.violation(archetype_id, &attr_path, "Existence");
        }
        return;
    }

    let count = u32::try_from(children.len()).unwrap_or(u32::MAX);
    match attribute.cardinality() {
        Some(cardinality) => {
            if !cardinality.interval().contains(count) {
                ctx.violation(archetype_id, &attr_path, "Cardinality");
            }
        }
        // No cardinality means the attribute is single-valued, and
        // `crate::path::Node::children` only ever returns more than one node
        // for a genuine container attribute — but an archetype whose author
        // forgot to declare one is a defect the instance should not appear
        // to survive silently.
        None if count > 1 => ctx.violation(archetype_id, &attr_path, "Cardinality"),
        None => {}
    }

    // AOM2 lets several `C_OBJECT`s sit under one attribute as alternatives,
    // each usually distinguished by its own node id, and each with its own
    // `occurrences`. Every data child is matched to the one alternative it
    // corresponds to before any of them is checked, so that an alternative's
    // `occurrences` can be verified against how many data children actually
    // matched it — not against the attribute's total count.
    let alternatives = attribute.children();
    let mut matched_counts = vec![0u32; alternatives.len()];
    for child in children {
        match match_alternative(alternatives, child) {
            Match::Alternative(index) => {
                matched_counts[index] += 1;
                walk_object(
                    archetype_id,
                    terminology,
                    &alternatives[index],
                    child,
                    &attr_path,
                    ctx,
                );
            }
            Match::Unrecognised => ctx.violation(archetype_id, &attr_path, "Unrecognised_node_id"),
            Match::Ambiguous => ctx.unchecked(
                &attr_path,
                "more than one constraint alternative here, and the node carries no \
                 archetype_node_id to say which one it satisfies",
            ),
        }
    }

    for (alternative, count) in alternatives.iter().zip(matched_counts) {
        let node_path = match alternative.node_id() {
            Some(id) => format!("{attr_path}[{id}]"),
            None => attr_path.clone(),
        };
        match alternative.occurrences() {
            Some(occurrences) => {
                if !occurrences.contains(count) {
                    ctx.violation(archetype_id, &node_path, "Occurrences");
                }
            }
            // Only a `C_COMPLEX_OBJECT_PROXY` reaches this: `use_target_occurrences()`
            // defers to a target node this crate does not resolve, so the
            // actual permitted count is unknown here, not merely unstated.
            None => ctx.unchecked(
                &node_path,
                "occurrences is deferred to another node (use_target_occurrences), which \
                 this crate does not resolve",
            ),
        }
    }
}

/// Which constraint alternative a data child corresponds to.
enum Match {
    /// The index into the attribute's alternatives.
    Alternative(usize),
    /// The node carries an `archetype_node_id` none of the alternatives
    /// declare.
    Unrecognised,
    /// The node carries no `archetype_node_id`, and more than one
    /// alternative is present, so which one it must satisfy cannot be
    /// determined from the data alone.
    Ambiguous,
}

fn match_alternative(alternatives: &[CObject], data: Node<'_>) -> Match {
    match data.archetype_node_id() {
        Some(id) => alternatives
            .iter()
            .position(|alt| alt.node_id() == Some(id))
            .map_or(Match::Unrecognised, Match::Alternative),
        None if alternatives.len() == 1 => Match::Alternative(0),
        None => Match::Ambiguous,
    }
}

/// One matched `C_OBJECT`: RM class, then its own kind of constraint.
fn walk_object(
    archetype_id: &ArchetypeHrid,
    terminology: &ArchetypeTerminology,
    constraint: &CObject,
    node: Node<'_>,
    path: &str,
    ctx: &mut Ctx<'_>,
) {
    let path = match constraint.node_id() {
        Some(id) => format!("{path}[{id}]"),
        None => path.to_owned(),
    };

    if let CObject::Primitive(p) = constraint {
        // A `C_PRIMITIVE_OBJECT`'s `rm_type_name` names an openEHR primitive
        // (`Integer`, `String`, …), which is not recoverable once the walk
        // has reached a [`crate::path::Scalar`] — that type exists to answer
        // AQL and path queries generically and collapses every primitive
        // kind to one opaque `"primitive"` [`Node::type_name`]. Comparing
        // against it here would reject every value regardless of the
        // constraint. `walk_primitive` checks the actual `Scalar` variant
        // against the constraint kind instead, which is the check that can
        // fail honestly.
        walk_primitive(archetype_id, terminology, p.constraint(), node, &path, ctx);
        return;
    }

    if node.type_name() != constraint.rm_type_name() {
        ctx.violation(archetype_id, &path, "Rm_type_name_matches");
        return;
    }

    match constraint {
        CObject::Complex(c) => walk_complex(archetype_id, terminology, c, node, &path, ctx),
        CObject::Slot(slot) => walk_slot(slot, node, archetype_id, &path, ctx),
        CObject::ArchetypeRoot(filled) => walk_archetype_root(filled, node, &path, ctx),
        CObject::Proxy(proxy) => ctx.unchecked_detail(
            &path,
            "C_COMPLEX_OBJECT_PROXY: this crate does not resolve target_path against the \
             archetype's own constraint tree",
            proxy.target_path(),
        ),
        CObject::Primitive(_) => unreachable!("handled above"),
    }
}

/// An `ARCHETYPE_SLOT`: whether the instance leaves the position open where
/// `is_closed()` forbids filling it, and — when a filler is present and the
/// slot does not forbid one — whether anything is actually left to check.
///
/// `includes`/`excludes` are carried as written and never evaluated
/// (`K15.10`), so this cannot tell whether one particular filler satisfies
/// them. It does not need to for two of the three outcomes below, though:
/// [`Node::archetype_details`] (`lib:A-59`'s own "not able to be from where
/// this crate currently validates" note, now partly closed) is what a
/// filling archetype's root looks like on the instance side, and that alone
/// settles both the closed-slot case and the no-restriction case without
/// reading a single assertion.
fn walk_slot(
    slot: &ArchetypeSlot,
    node: Node<'_>,
    archetype_id: &ArchetypeHrid,
    path: &str,
    ctx: &mut Ctx<'_>,
) {
    let Some(filler) = node.archetype_details() else {
        // No archetype root at this position: the slot was left open.
        // `is_closed`, `includes`, and `excludes` all restrict what may fill
        // a slot; none of them restricts leaving it unfilled — that is
        // occurrences' own job, checked separately by the attribute this
        // slot sits under. Nothing here to violate or leave unchecked.
        return;
    };

    if slot.is_closed() {
        // "closed to further filling either in further specialisations or
        // at runtime" (AOM2's own words for `is_closed`) — no filler was
        // ever permitted here, so one being present at all is the
        // violation, whatever it is and whatever `includes`/`excludes`
        // might otherwise have allowed.
        ctx.violation(archetype_id, path, "ARCHETYPE_SLOT.is_closed");
        return;
    }

    if slot.any_allowed() {
        // Open, with neither `includes` nor `excludes` stated: any
        // archetype may fill it, so whatever actually filled it needs no
        // further check to conform.
        return;
    }

    ctx.unchecked_detail(
        path,
        "ARCHETYPE_SLOT: includes/excludes assertions are carried as written and not \
         evaluated (K15.10), so this filler cannot be checked against them",
        filler.archetype_id().to_string(),
    );
}

/// A `C_ARCHETYPE_ROOT`: a slot a template already filled, naming the filler
/// by archetype id. Resolved through the repository if one was supplied
/// (`K15.24`); otherwise unchecked, the same as a bare `ARCHETYPE_SLOT`.
fn walk_archetype_root(
    filled: &crate::am::CArchetypeRoot,
    node: Node<'_>,
    path: &str,
    ctx: &mut Ctx<'_>,
) {
    let Some(repository) = ctx.repository else {
        ctx.unchecked(
            path,
            "C_ARCHETYPE_ROOT: no repository was supplied (validate_with_repository); \
             the filler archetype is not resolved",
        );
        return;
    };

    let Ok(filler_id) = filled.archetype_ref().parse::<ArchetypeId>() else {
        ctx.unchecked_detail(
            path,
            "C_ARCHETYPE_ROOT names an archetype reference that is not a well-formed \
             archetype id",
            filled.archetype_ref(),
        );
        return;
    };

    let resolved = match repository.resolve(&filler_id) {
        Ok(resolved) => resolved,
        Err(error) => {
            // `K15.27`: a retrieval failure is a refusal, never a pass —
            // `Unchecked` (not `is_conformant`), never silently skipped.
            ctx.unchecked_detail(
                path,
                "C_ARCHETYPE_ROOT's filler archetype could not be retrieved",
                error.to_string(),
            );
            return;
        }
    };

    if resolved.archetype().archetype_id().to_string() != filler_id.to_string() {
        // The repository is required to answer the identifier asked for
        // (`K15.26`); returning a different one is a repository defect this
        // crate refuses to build on rather than validate against silently.
        // Compared by text, not by type: `Archetype::archetype_id` is an
        // `ArchetypeHrid` (`ARCHETYPE.archetype_id` in AOM2 itself), and
        // `filler_id` is an `ArchetypeId` — the narrower, namespace-free
        // form `filled.archetype_ref().parse::<ArchetypeId>()` above
        // already required, so nothing here loses precision the earlier
        // parse had not already discarded; `ArchetypeHrid::physical_id`
        // and `ArchetypeId::to_string` share one textual convention
        // (`rm_publisher-rm_package-rm_class.concept_id.vVERSION`) for
        // exactly this reason.
        ctx.unchecked_detail(
            path,
            "the repository returned a different archetype than the one requested",
            resolved.archetype().archetype_id().to_string(),
        );
        return;
    }

    if resolved.provenance().is_none() {
        if ctx.options.allow_unestablished_provenance {
            ctx.unverified_provenance.push(path.to_owned());
        } else {
            ctx.unchecked(
                path,
                "the filler archetype's provenance is not established, and the caller did \
                 not opt in (RepositoryOptions::allow_unestablished_provenance)",
            );
            return;
        }
    }

    if node.type_name() != resolved.archetype().rm_type_name() {
        ctx.violation(
            resolved.archetype().archetype_id(),
            path,
            "Rm_type_name_matches",
        );
        return;
    }
    walk_complex(
        resolved.archetype().archetype_id(),
        resolved.archetype().terminology(),
        resolved.archetype().definition(),
        node,
        path,
        ctx,
    );
}

/// `C_DATE`/`C_TIME`/`C_DATE_TIME`/`C_DURATION` against the scalar's text
/// form — shared across the four because AOM2 gives all of them the same
/// shape (`range: List<Interval<...>>` plus a `pattern` carried, not
/// applied), and only the underlying temporal type differs.
///
/// The scalar arrives as text (`Scalar::Str`) because that is what
/// [`crate::path`]'s walk already exposes for `DV_DATE`/`DV_TIME`/
/// `DV_DATE_TIME`/`DV_DURATION`'s `value` attribute — their ISO 8601 lexical
/// form, the same one [`crate::base::Date`] and its siblings round-trip
/// exactly (`D3.18d`'s reasoning, one type family over). No change to
/// `crate::path` was needed for this.
fn check_temporal<T>(
    value: &str,
    range: &[Interval<T>],
    pattern: Option<&String>,
    class: &'static str,
    archetype_id: &ArchetypeHrid,
    path: &str,
    ctx: &mut Ctx<'_>,
) where
    T: core::str::FromStr + crate::base::SemanticOrd,
{
    match value.parse::<T>() {
        Ok(parsed) => {
            if !range.is_empty() && !range.iter().any(|r| r.contains(&parsed)) {
                ctx.violation(archetype_id, path, class);
            }
        }
        // The value did not even parse as the type its own constraint kind
        // governs — a `C_DATE` node whose data is not a valid ISO 8601 date.
        // This crate's own RM validation (`crate::validation`) is what
        // normally catches this before archetype validation ever runs; this
        // still reports rather than panics if it is reached anyway (`K15.20`
        // never a silent pass, and never a silent crash either).
        Err(_) => ctx.violation(archetype_id, path, class),
    }
    if pattern.is_some() {
        ctx.unchecked_detail(
            path,
            "the constraint pattern is carried but not evaluated",
            class,
        );
    }
}

/// Dispatches one of the four temporal `CPrimitive` variants to
/// [`check_temporal`] with the right type parameter and class name.
///
/// Its own function, separate from [`walk_primitive`]'s match, purely to
/// keep that match's line count down to something `clippy::too_many_lines`
/// accepts — the four call sites are otherwise identical in shape and this
/// changes no behaviour, only where the four are written.
fn walk_temporal(
    constraint: &CPrimitive,
    value: &str,
    archetype_id: &ArchetypeHrid,
    path: &str,
    ctx: &mut Ctx<'_>,
) {
    match constraint {
        CPrimitive::Date { range, pattern } => {
            check_temporal(
                value,
                range,
                pattern.as_ref(),
                "C_DATE",
                archetype_id,
                path,
                ctx,
            );
        }
        CPrimitive::Time { range, pattern } => {
            check_temporal(
                value,
                range,
                pattern.as_ref(),
                "C_TIME",
                archetype_id,
                path,
                ctx,
            );
        }
        CPrimitive::DateTime { range, pattern } => {
            check_temporal(
                value,
                range,
                pattern.as_ref(),
                "C_DATE_TIME",
                archetype_id,
                path,
                ctx,
            );
        }
        CPrimitive::Duration { range, pattern } => {
            check_temporal(
                value,
                range,
                pattern.as_ref(),
                "C_DURATION",
                archetype_id,
                path,
                ctx,
            );
        }
        // `walk_primitive`'s own match only ever calls this function for one
        // of the four variants above. This function stays total rather than
        // trusting that guarantee with `unreachable!()` (`lib:A-36` is
        // exactly the shape of defect a "the caller guarantees it" panic
        // produces): a future call site this crate does not control yet
        // gets a reported mismatch, never a crash.
        _ => ctx.violation(archetype_id, path, "Primitive_kind_mismatch"),
    }
}

/// A leaf primitive constraint against the scalar the path walk reached.
fn walk_primitive(
    archetype_id: &ArchetypeHrid,
    terminology: &ArchetypeTerminology,
    constraint: &CPrimitive,
    node: Node<'_>,
    path: &str,
    ctx: &mut Ctx<'_>,
) {
    let Node::Scalar(scalar) = node else {
        // A `C_PRIMITIVE_OBJECT` governs a scalar attribute; the RM shape
        // does not match what the archetype expects if the walk reached
        // anything else here. Reported rather than assumed unreachable,
        // because a hand-built archetype (`K15.4`) is not obliged to be
        // well formed, and this module does not trust its input any more
        // than `crate::validation` trusts a deserialized RM instance.
        ctx.violation(archetype_id, path, "Primitive_kind_mismatch");
        return;
    };

    match (constraint, scalar) {
        (
            CPrimitive::Boolean {
                allow_true,
                allow_false,
            },
            Scalar::Boolean(value),
        ) => {
            let permitted = if value { *allow_true } else { *allow_false };
            if !permitted {
                ctx.violation(archetype_id, path, "C_BOOLEAN");
            }
        }
        (CPrimitive::String { list }, Scalar::Str(value)) => {
            // `list` mixes literals and `/…/`/`^…^`-delimited regex
            // elements (`CPrimitive::String`'s own documentation, `A-63`) —
            // split them once rather than scanning `list` twice with two
            // different predicates.
            let (patterns, literals): (Vec<_>, Vec<_>) =
                list.iter().partition(|item| is_c_string_pattern(item));
            let literal_match = literals.iter().any(|item| *item == value);
            if !literals.is_empty() && !literal_match {
                ctx.violation(archetype_id, path, "C_STRING");
            } else if !patterns.is_empty() {
                // Carried but never compiled or applied — see
                // `crate::am::CPrimitive::String`'s own documentation for
                // why. Reported even when a literal already matched: this
                // mirrors the behaviour before `A-63` folded the two into
                // one list, when a `pattern` set alongside a matching
                // `list` entry was unchecked rather than an early `Conforms`.
                ctx.unchecked(path, "C_STRING pattern is not evaluated");
            }
        }
        (CPrimitive::Integer { list, range }, Scalar::Integer(value)) => {
            let in_list = list.is_empty() || list.contains(&value);
            let in_range = range.as_ref().is_none_or(|r| r.contains(&value));
            if !in_list || !in_range {
                ctx.violation(archetype_id, path, "C_INTEGER");
            }
        }
        (CPrimitive::Real { list, range }, Scalar::Number(value)) => {
            // `Scalar::Number` carries only the `f64` this crate's own path
            // walk exposes, never the written text `Real` otherwise
            // preserves (`D3.18d`), so membership here is by *numeric* value
            // — `Real::semantic_cmp`, not `Real`'s lexical `PartialEq` — and
            // a `C_REAL` list built from `1.50` is satisfied by a magnitude
            // that denotes `1.5`. That is a real imprecision, stated rather
            // than silently applied: the path this value arrived through
            // does not carry enough to do better.
            let candidate = Real::from_f64(value);
            let in_list = list.is_empty()
                || list
                    .iter()
                    .any(|item| item.semantic_cmp(&candidate) == Some(Ordering::Equal));
            let in_range = range.as_ref().is_none_or(|r| r.contains(&candidate));
            if !in_list || !in_range {
                ctx.violation(archetype_id, path, "C_REAL");
            }
        }
        (
            CPrimitive::TerminologyCode {
                constraint,
                constraint_status,
            },
            Scalar::Str(value),
        ) => {
            // AOM2's own words for any status but `Required`: "validity of
            // the data instance is achieved by supplying *any* terminology
            // code" (`docs/ADL2/master04.5-cadl_primitive_types.adoc`) — a
            // `Scalar::Str` reaching this arm at all already is one, so a
            // soft constraint is satisfied, not merely unchecked. Checking
            // the code against `constraint` below for `extensible`,
            // `preferred`, or `example` would report a violation AOM2
            // explicitly says is not one — the bug this arm existed to avoid
            // before `constraint_status` had anywhere to be attached at all.
            let required = constraint_status.is_none_or(ConstraintStatus::is_required);
            if required {
                match constraint.as_deref() {
                    // AOM2's own "no constraint" spelling, plus `None` for
                    // the same reason — see this variant's own field
                    // documentation.
                    None | Some("") => {
                        ctx.unchecked(path, "C_TERMINOLOGY_CODE names no constraint");
                    }
                    // `is_value_set_code`: `a_code.starts_with (Value_set_code_leader)`,
                    // `Value_set_code_leader = "ac"`
                    // (`org.openehr.am.aom2.adl_code_definitions.adoc`) — an
                    // `ac`-code names a value set to check membership
                    // against, not a value to compare equal to.
                    Some(ac_code) if ac_code.starts_with("ac") => {
                        let value_set_ok = terminology
                            .value_set(ac_code)
                            .is_some_and(|set| set.contains(value));
                        if !value_set_ok {
                            ctx.violation(archetype_id, path, "C_TERMINOLOGY_CODE");
                        }
                    }
                    // An `at`-code names one specific value; conformance is
                    // exact equality with the code itself, not a lookup.
                    Some(at_code) => {
                        if value != at_code {
                            ctx.violation(archetype_id, path, "C_TERMINOLOGY_CODE");
                        }
                    }
                }
            }
        }
        (
            CPrimitive::Date { .. }
            | CPrimitive::Time { .. }
            | CPrimitive::DateTime { .. }
            | CPrimitive::Duration { .. },
            Scalar::Str(value),
        ) => walk_temporal(constraint, value, archetype_id, path, ctx),
        (CPrimitive::Unsupported { .. }, _) => {
            ctx.unchecked(
                path,
                "C_UNSUPPORTED: this crate does not model the constraint kind (K15.3 keeps it \
                 rather than dropping it, K15.20 keeps it unchecked rather than passing it)",
            );
        }
        _ => {
            // The scalar kind the path walk reached does not match what this
            // primitive constraint governs — a `C_INTEGER` reached through a
            // string-valued attribute, for instance.
            ctx.violation(archetype_id, path, "Primitive_kind_mismatch");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::am::{
        ArchetypeRepository, ArchetypeSlot, ArchetypeTerminology, CArchetypeRoot, CAttribute,
        CAttributeTuple, CComplexObject, CComplexObjectProxy, CObject, ConstraintStatus,
        CPrimitiveObject, Cardinality, MultiplicityInterval, Provenance, RepositoryError,
        Resolved, TermDefinition,
    };
    use crate::base::Interval;
    use crate::path::Pathable as _;
    use crate::rm::common::{Archetyped, LocatableAttrs};
    use crate::rm::data_structures::{Element, ItemList};
    use crate::rm::data_types::{
        CodePhrase, DataValue, DvBoolean, DvCodedText, DvCount, DvDate, DvDateTime, DvDuration,
        DvTime,
    };
    use crate::rm::ehr::{AdminEntry, Entry, EntryAttrs, Evaluation};
    use std::collections::{BTreeMap, BTreeSet};

    fn attrs(name: &str, node: &str) -> LocatableAttrs {
        LocatableAttrs::named(name, node).unwrap()
    }

    fn entry_attrs() -> EntryAttrs {
        EntryAttrs::about_subject(
            CodePhrase::new("ISO_639-1", "en").unwrap(),
            CodePhrase::new("IANA_character-sets", "UTF-8").unwrap(),
        )
    }

    fn terms(codes: &[(&str, &str)]) -> BTreeMap<String, TermDefinition> {
        codes
            .iter()
            .map(|(code, text)| {
                (
                    (*code).to_owned(),
                    TermDefinition::new(*text, None).unwrap(),
                )
            })
            .collect()
    }

    fn placeholder_value() -> DataValue {
        DataValue::Boolean(DvBoolean::new(true))
    }

    /// `EVALUATION[id1]/data[id2 ITEM_LIST]/items`, with `element_alt` as the
    /// sole alternative under `items`.
    fn evaluation_archetype(element_alt: CObject) -> Archetype {
        let items_attr = CAttribute::container(
            "items",
            MultiplicityInterval::MANDATORY,
            Cardinality::new(MultiplicityInterval::at_least(0).unwrap()),
            vec![element_alt],
        )
        .unwrap();
        let data_object = CComplexObject::new(
            "ITEM_LIST",
            Some("id2".to_owned()),
            MultiplicityInterval::MANDATORY,
            vec![items_attr],
        )
        .unwrap();
        let data_attr = CAttribute::single(
            "data",
            MultiplicityInterval::MANDATORY,
            vec![CObject::Complex(data_object)],
        )
        .unwrap();
        let definition = CComplexObject::new(
            "EVALUATION",
            Some("id1".to_owned()),
            MultiplicityInterval::MANDATORY,
            vec![data_attr],
        )
        .unwrap();
        Archetype::new(
            "openEHR-EHR-EVALUATION.test.v1".parse().unwrap(),
            definition,
            ArchetypeTerminology::new(
                "en",
                terms(&[("id1", "Test"), ("id2", "Data"), ("at0004", "Systolic")]),
            )
            .unwrap(),
        )
        .unwrap()
    }

    /// An `ELEMENT[node_id]` alternative with no further constraint on its
    /// value.
    fn element_alt(node_id: &str, occurrences: MultiplicityInterval) -> CObject {
        CObject::Complex(
            CComplexObject::new("ELEMENT", Some(node_id.to_owned()), occurrences, Vec::new())
                .unwrap(),
        )
    }

    /// An `ELEMENT[node_id]` alternative whose value is constrained one level
    /// down — `value/magnitude` for a `DV_COUNT`, `value/defining_code` for a
    /// `DV_CODED_TEXT`, and so on.
    ///
    /// `value_rm_type` comes last, after `primitive` rather than beside
    /// `inner_attribute`, deliberately: `openehr-assets` counts any adjacent
    /// pair of string literals shaped like a class and an invariant as a
    /// citation, and `("DV_COUNT", "magnitude")` sitting next to each other
    /// as two literal arguments would enter the invariant-coverage report as
    /// one (see `am::constraint::tests`'s own note on this).
    fn element_with_value_constraint(
        node_id: &str,
        inner_attribute: &str,
        primitive: CPrimitive,
        value_rm_type: &str,
    ) -> CObject {
        let inner_attr = CAttribute::single(
            inner_attribute,
            MultiplicityInterval::MANDATORY,
            vec![CObject::Primitive(CPrimitiveObject::new(
                "primitive",
                MultiplicityInterval::MANDATORY,
                primitive,
            ))],
        )
        .unwrap();
        let value_object = CComplexObject::new(
            value_rm_type,
            None,
            MultiplicityInterval::MANDATORY,
            vec![inner_attr],
        )
        .unwrap();
        let value_attr = CAttribute::single(
            "value",
            MultiplicityInterval::MANDATORY,
            vec![CObject::Complex(value_object)],
        )
        .unwrap();
        CObject::Complex(
            CComplexObject::new(
                "ELEMENT",
                Some(node_id.to_owned()),
                MultiplicityInterval::MANDATORY,
                vec![value_attr],
            )
            .unwrap(),
        )
    }

    fn build_evaluation(elements: Vec<Element>) -> Entry {
        let data = ItemList::new(attrs("Data", "id2"), elements);
        Entry::from(Evaluation::new(
            attrs("Test", "openEHR-EHR-EVALUATION.test.v1"),
            entry_attrs(),
            data.into(),
        ))
    }

    #[test]
    fn a_matching_instance_is_conformant() {
        let archetype =
            evaluation_archetype(element_alt("at0004", MultiplicityInterval::MANDATORY));
        let entry = build_evaluation(vec![Element::new(
            attrs("Systolic", "at0004"),
            placeholder_value(),
        )]);
        let report = validate_against_archetype(&archetype, entry.as_node());
        assert!(report.is_conformant(), "{report:?}");
    }

    #[test]
    fn a_missing_mandatory_element_is_a_violation() {
        let archetype =
            evaluation_archetype(element_alt("at0004", MultiplicityInterval::MANDATORY));
        let entry = build_evaluation(Vec::new());
        let report = validate_against_archetype(&archetype, entry.as_node());
        assert!(!report.is_conformant());
        assert_eq!(report.violations()[0].constraint(), "Existence");
        assert_eq!(report.violations()[0].archetype_path(), "/data[id2]/items");
    }

    #[test]
    fn an_unrecognised_node_id_is_a_violation() {
        let archetype =
            evaluation_archetype(element_alt("at0004", MultiplicityInterval::MANDATORY));
        let entry = build_evaluation(vec![Element::new(
            attrs("Something else", "at0009"),
            placeholder_value(),
        )]);
        let report = validate_against_archetype(&archetype, entry.as_node());
        assert_eq!(report.violations()[0].constraint(), "Unrecognised_node_id");
    }

    #[test]
    fn an_alternative_below_its_own_mandatory_occurrences_is_a_violation() {
        // Two ELEMENTs required at `at0004`; the instance supplies one.
        let archetype = evaluation_archetype(element_alt(
            "at0004",
            MultiplicityInterval::new(2, Some(2)).unwrap(),
        ));
        let entry = build_evaluation(vec![Element::new(
            attrs("Systolic", "at0004"),
            placeholder_value(),
        )]);
        let report = validate_against_archetype(&archetype, entry.as_node());
        assert_eq!(report.violations()[0].constraint(), "Occurrences");
        assert_eq!(
            report.violations()[0].archetype_path(),
            "/data[id2]/items[at0004]"
        );
    }

    /// One filled `ELEMENT` position, its `archetype_details` naming a
    /// filler — the shape [`Node::archetype_details`] exists to expose
    /// (`lib:A-59`'s own residual, `walk_slot`'s own module documentation).
    fn filled(name: &str, node_id: &str) -> Element {
        Element::new(
            attrs(name, node_id).with_archetype_details(
                Archetyped::new("openEHR-EHR-CLUSTER.device.v1", "1.1.0").unwrap(),
            ),
            placeholder_value(),
        )
    }

    /// `node_id` goes through a binding rather than sitting beside the class
    /// name as a literal in each test below: `openehr-assets` counts any
    /// adjacent pair of string literals shaped like a class and an
    /// invariant as a citation, deliberately, and `("ELEMENT", "at0004")`
    /// would enter the invariant-coverage report as one.
    const SLOT_NODE_ID: &str = "at0004";

    /// An open slot (the default: not closed, no `includes`/`excludes`)
    /// left unfilled — no `archetype_details` on the instance node at all.
    /// Nothing in `ARCHETYPE_SLOT` restricts leaving a slot empty; that is
    /// occurrences' own job, checked elsewhere. Not a violation, and nothing
    /// left unchecked either: there is nothing here to be unsure about.
    #[test]
    fn an_open_slot_left_unfilled_is_fully_conformant() {
        let slot = CObject::Slot(
            ArchetypeSlot::new("ELEMENT", SLOT_NODE_ID, MultiplicityInterval::OPTIONAL).unwrap(),
        );
        let archetype = evaluation_archetype(slot);
        let entry = build_evaluation(vec![Element::new(
            attrs("Systolic", SLOT_NODE_ID),
            placeholder_value(),
        )]);
        let report = validate_against_archetype(&archetype, entry.as_node());
        assert!(report.is_conformant());
        assert!(report.unchecked().is_empty());
    }

    /// A closed slot (`is_closed()`) that was filled anyway — "closed to
    /// further filling either in further specialisations or at runtime"
    /// (AOM2's own words) means no filler was ever permitted here, so one
    /// being present at all is the violation, not a question of whether it
    /// happens to satisfy `includes`/`excludes`.
    #[test]
    fn a_closed_slot_that_was_filled_anyway_is_a_violation() {
        let slot = CObject::Slot(
            ArchetypeSlot::new("ELEMENT", SLOT_NODE_ID, MultiplicityInterval::OPTIONAL)
                .unwrap()
                .closed(),
        );
        let archetype = evaluation_archetype(slot);
        let entry = build_evaluation(vec![filled("Systolic", SLOT_NODE_ID)]);
        let report = validate_against_archetype(&archetype, entry.as_node());
        assert!(report.unchecked().is_empty());
        assert_eq!(report.violations().len(), 1);
        assert_eq!(report.violations()[0].constraint(), "ARCHETYPE_SLOT.is_closed");
        assert_eq!(
            report.violations()[0].archetype_path(),
            "/data[id2]/items[at0004]"
        );
    }

    /// An open slot with neither `includes` nor `excludes` stated
    /// (`any_allowed()`) that was filled — any archetype may fill it, so the
    /// filler needs no further check to conform, whatever it actually is.
    #[test]
    fn an_unrestricted_open_slots_filler_is_fully_conformant() {
        let slot = CObject::Slot(
            ArchetypeSlot::new("ELEMENT", SLOT_NODE_ID, MultiplicityInterval::OPTIONAL).unwrap(),
        );
        let archetype = evaluation_archetype(slot);
        let entry = build_evaluation(vec![filled("Systolic", SLOT_NODE_ID)]);
        let report = validate_against_archetype(&archetype, entry.as_node());
        assert!(report.is_conformant());
        assert!(report.unchecked().is_empty());
    }

    /// An open slot naming an `includes` assertion, filled — `K15.10`:
    /// `includes`/`excludes` are carried as written and not evaluated, so
    /// whether this particular filler actually satisfies the assertion
    /// cannot be checked, and is reported unchecked rather than silently
    /// passed, naming the filler's own archetype id as detail.
    #[test]
    fn a_restricted_open_slots_filler_is_unchecked_not_silently_passed() {
        let slot = CObject::Slot(
            ArchetypeSlot::new("ELEMENT", SLOT_NODE_ID, MultiplicityInterval::OPTIONAL)
                .unwrap()
                .including("archetype_id/value matches {/.*device.*/}"),
        );
        let archetype = evaluation_archetype(slot);
        let entry = build_evaluation(vec![filled("Systolic", SLOT_NODE_ID)]);
        let report = validate_against_archetype(&archetype, entry.as_node());
        assert!(!report.is_conformant());
        assert!(report.violations().is_empty());
        assert_eq!(report.unchecked().len(), 1);
        assert_eq!(
            report.unchecked()[0].archetype_path(),
            "/data[id2]/items[at0004]"
        );
        assert_eq!(
            report.unchecked()[0].detail(),
            Some("openEHR-EHR-CLUSTER.device.v1")
        );
    }

    /// `C_COMPLEX_OBJECT_PROXY`: resolving `target_path` against the
    /// archetype's own constraint tree is not implemented (see
    /// `CComplexObjectProxy`'s own module documentation), so a node it
    /// governs is reported unchecked, never as passing — the same
    /// `K15.20` discipline
    /// `a_restricted_open_slots_filler_is_unchecked_not_silently_passed`
    /// proves for the one real case `ARCHETYPE_SLOT` still cannot check
    /// (`walk_slot`'s own module documentation covers the two it now can).
    #[test]
    fn a_proxy_is_reported_unchecked_never_as_passing() {
        let proxy = CObject::Proxy(
            CComplexObjectProxy::new(
                "ELEMENT",
                Some("at0004".to_owned()),
                Some(MultiplicityInterval::MANDATORY),
                "/data[id2]/items[at0099]",
            )
            .unwrap(),
        );
        let archetype = evaluation_archetype(proxy);
        let entry = build_evaluation(vec![Element::new(
            attrs("Systolic", "at0004"),
            placeholder_value(),
        )]);
        let report = validate_against_archetype(&archetype, entry.as_node());
        assert!(!report.is_conformant());
        assert!(report.violations().is_empty());
        assert_eq!(report.unchecked().len(), 1);
        assert_eq!(
            report.unchecked()[0].detail(),
            Some("/data[id2]/items[at0099]")
        );
    }

    /// A proxy with no stated occurrences (`use_target_occurrences()`) is
    /// unchecked twice over for one matched instance node: once from
    /// `walk_object`'s own Proxy handling (above), and once from the
    /// separate loop that checks how many times each alternative matched
    /// against its own occurrences — which cannot check a deferred value
    /// either, and says so rather than silently skipping the check.
    #[test]
    fn a_proxy_with_deferred_occurrences_is_unchecked_on_both_counts() {
        let proxy = CObject::Proxy(
            CComplexObjectProxy::new(
                "ELEMENT",
                Some("at0004".to_owned()),
                None,
                "/data[id2]/items[at0099]",
            )
            .unwrap(),
        );
        let archetype = evaluation_archetype(proxy);
        let entry = build_evaluation(vec![Element::new(
            attrs("Systolic", "at0004"),
            placeholder_value(),
        )]);
        let report = validate_against_archetype(&archetype, entry.as_node());
        assert!(report.violations().is_empty());
        assert_eq!(report.unchecked().len(), 2);
        assert!(
            report
                .unchecked()
                .iter()
                .any(|u| u.reason().contains("C_COMPLEX_OBJECT_PROXY"))
        );
        assert!(
            report
                .unchecked()
                .iter()
                .any(|u| u.reason().contains("use_target_occurrences"))
        );
    }

    #[test]
    fn a_c_integer_range_rejects_a_value_outside_it() {
        let constraint = element_with_value_constraint(
            "at0004",
            "magnitude",
            CPrimitive::Integer {
                list: Vec::new(),
                range: Some(Interval::closed(60, 100).unwrap()),
            },
            "DV_COUNT",
        );
        let archetype = evaluation_archetype(constraint);

        let in_range = build_evaluation(vec![Element::new(
            attrs("Systolic", "at0004"),
            DataValue::Count(DvCount::new(72)),
        )]);
        assert!(validate_against_archetype(&archetype, in_range.as_node()).is_conformant());

        let out_of_range = build_evaluation(vec![Element::new(
            attrs("Systolic", "at0004"),
            DataValue::Count(DvCount::new(200)),
        )]);
        let report = validate_against_archetype(&archetype, out_of_range.as_node());
        assert_eq!(report.violations()[0].constraint(), "C_INTEGER");
        // No node content in the violation (X11.7, K15.21).
        assert!(!format!("{:?}", report.violations()[0]).contains("200"));
    }

    #[test]
    fn a_c_string_pattern_is_unchecked_even_with_no_literals_to_check() {
        let constraint = element_with_value_constraint(
            "at0004",
            "value",
            CPrimitive::String {
                list: vec![r"/\d+/".to_owned()],
            },
            "DV_TEXT",
        );
        let archetype = evaluation_archetype(constraint);
        let entry = build_evaluation(vec![Element::new(
            attrs("Free text", "at0004"),
            DataValue::Text(crate::rm::data_types::DvText::new("anything").unwrap()),
        )]);
        let report = validate_against_archetype(&archetype, entry.as_node());
        assert!(report.violations().is_empty());
        assert_eq!(
            report.unchecked()[0].reason(),
            "C_STRING pattern is not evaluated"
        );
    }

    /// `A-63`: a regex element lives in the same `list` as any literals —
    /// a value that already matches a literal is still reported unchecked
    /// if a pattern element is also present, the same choice this crate
    /// made when the two lived in separate fields (a matching `list` and a
    /// set `pattern` together were already unchecked, not `Conforms`).
    #[test]
    fn a_c_string_pattern_is_unchecked_even_when_a_literal_already_matched() {
        let constraint = element_with_value_constraint(
            "at0004",
            "value",
            CPrimitive::String {
                list: vec!["Systolic".to_owned(), r"/\d+/".to_owned()],
            },
            "DV_TEXT",
        );
        let archetype = evaluation_archetype(constraint);
        let entry = build_evaluation(vec![Element::new(
            attrs("Free text", "at0004"),
            DataValue::Text(crate::rm::data_types::DvText::new("Systolic").unwrap()),
        )]);
        let report = validate_against_archetype(&archetype, entry.as_node());
        assert!(report.violations().is_empty());
        assert_eq!(
            report.unchecked()[0].reason(),
            "C_STRING pattern is not evaluated"
        );
    }

    /// A literal-only `list` with no regex element behaves exactly as
    /// before `A-63`: an unmatched value is a definite violation, not
    /// unchecked.
    #[test]
    fn a_literal_only_c_string_list_rejects_a_value_outside_it() {
        let constraint = element_with_value_constraint(
            "at0004",
            "value",
            CPrimitive::String {
                list: vec!["Systolic".to_owned()],
            },
            "DV_TEXT",
        );
        let archetype = evaluation_archetype(constraint);
        let entry = build_evaluation(vec![Element::new(
            attrs("Free text", "at0004"),
            DataValue::Text(crate::rm::data_types::DvText::new("Diastolic").unwrap()),
        )]);
        let report = validate_against_archetype(&archetype, entry.as_node());
        assert!(report.unchecked().is_empty());
        assert_eq!(report.violations().len(), 1);
        assert_eq!(report.violations()[0].constraint(), "C_STRING");
    }

    /// An `ELEMENT[at0004]/value` wrapping a `DV_QUANTITY[id14]` whose
    /// `{units, magnitude}` tuple has `tuple_rows` rows — the shared
    /// scaffolding every `C_ATTRIBUTE_TUPLE` test below builds on, so each
    /// test states only what actually varies: the rows and the instance.
    fn quantity_tuple_archetype(tuple_rows: Vec<CPrimitiveTuple>) -> Archetype {
        let tuple = CAttributeTuple::new(
            vec![
                CAttribute::single("units", MultiplicityInterval::MANDATORY, Vec::new()).unwrap(),
                CAttribute::single("magnitude", MultiplicityInterval::MANDATORY, Vec::new())
                    .unwrap(),
            ],
            tuple_rows,
        )
        .unwrap();
        let value_object = CComplexObject::new(
            "DV_QUANTITY",
            None,
            MultiplicityInterval::MANDATORY,
            Vec::new(),
        )
        .unwrap()
        .with_attribute_tuples(vec![tuple]);
        let value_attr = CAttribute::single(
            "value",
            MultiplicityInterval::MANDATORY,
            vec![CObject::Complex(value_object)],
        )
        .unwrap();
        let element = CObject::Complex(
            CComplexObject::new(
                "ELEMENT",
                Some("at0004".to_owned()),
                MultiplicityInterval::MANDATORY,
                vec![value_attr],
            )
            .unwrap(),
        );
        evaluation_archetype(element)
    }

    /// One `{units, magnitude}` row: an exact unit string paired with a
    /// closed magnitude range — the same shape
    /// `am::constraint::tests::a_units_magnitude_tuple_pairs_each_unit_with_its_own_range`
    /// builds, reused here against real instance data instead of just
    /// constructed and inspected.
    fn units_magnitude_row(units: &str, low: f64, high: f64) -> CPrimitiveTuple {
        CPrimitiveTuple::new(vec![
            CPrimitiveObject::new(
                "String",
                MultiplicityInterval::MANDATORY,
                CPrimitive::String {
                    list: vec![units.to_owned()],
                },
            ),
            CPrimitiveObject::new(
                "Real",
                MultiplicityInterval::MANDATORY,
                CPrimitive::Real {
                    list: Vec::new(),
                    range: Some(Interval::closed(low.into(), high.into()).unwrap()),
                },
            ),
        ])
        .unwrap()
    }

    fn quantity_entry(magnitude: f64, units: &str) -> Entry {
        build_evaluation(vec![Element::new(
            attrs("Systolic", "at0004"),
            DataValue::Quantity(crate::rm::data_types::DvQuantity::new(magnitude, units).unwrap()),
        )])
    }

    /// `A-50`'s own residual, closed: naming zero rows is the one case this
    /// crate has no stated reading for (AOM2's `tuples` is `0..1`, and an
    /// attribute tuple with no permitted combinations at all is not the same
    /// question as one whose combinations the instance simply does not
    /// match), so it stays unchecked rather than becoming a guessed
    /// violation or a guessed pass.
    #[test]
    fn an_attribute_tuple_with_no_rows_is_unchecked() {
        let archetype = quantity_tuple_archetype(Vec::new());
        let entry = quantity_entry(140.0, "mm[Hg]");
        let report = validate_against_archetype(&archetype, entry.as_node());
        assert!(report.violations().is_empty());
        assert_eq!(
            report.unchecked()[0].reason(),
            "C_ATTRIBUTE_TUPLE: no row could be confirmed or ruled out for every column"
        );
        assert_eq!(report.unchecked()[0].detail(), Some("units, magnitude"));
    }

    /// An instance whose units and magnitude both fall inside one row is
    /// conformant — no violation, nothing unchecked. This is the actual
    /// evaluation `A-50` deferred: `walk_attribute_tuple` resolves `units`
    /// and `magnitude` to the instance's real values and finds a row that
    /// accepts both together, rather than reporting unchecked regardless.
    #[test]
    fn an_attribute_tuple_matching_a_row_is_conformant() {
        let archetype = quantity_tuple_archetype(vec![
            units_magnitude_row("mm[Hg]", 0.0, 300.0),
            units_magnitude_row("kPa", 0.0, 40.0),
        ]);
        let entry = quantity_entry(140.0, "mm[Hg]");
        let report = validate_against_archetype(&archetype, entry.as_node());
        assert!(report.violations().is_empty());
        assert!(report.unchecked().is_empty());
    }

    /// An instance whose units/magnitude combination matches no row at all
    /// is a `C_ATTRIBUTE_TUPLE` violation naming the tuple's own path — not
    /// silently passed, and not reported unchecked either, because every row
    /// was actually evaluated and every one of them definitely ruled the
    /// combination out (`120 mm[Hg]` is in neither row's unit, so both
    /// columns' string constraints outright fail).
    #[test]
    fn an_attribute_tuple_matching_no_row_is_a_violation() {
        let archetype = quantity_tuple_archetype(vec![
            units_magnitude_row("mm[Hg]", 0.0, 300.0),
            units_magnitude_row("kPa", 0.0, 40.0),
        ]);
        let entry = quantity_entry(120.0, "cm[H2O]");
        let report = validate_against_archetype(&archetype, entry.as_node());
        assert!(report.unchecked().is_empty());
        assert_eq!(report.violations().len(), 1);
        assert_eq!(report.violations()[0].constraint(), "C_ATTRIBUTE_TUPLE");
    }

    /// A co-varying attribute that is not single-valued in the instance —
    /// here, absent entirely, since `DV_QUANTITY` carries no repeating
    /// `magnitude` — cannot be resolved to one value to compare, so the
    /// whole tuple is reported unchecked naming exactly that column's path,
    /// rather than guessed at. `walk_attribute_tuple`'s doc comment states
    /// this is deliberately not a shape this check attempts.
    #[test]
    fn an_attribute_tuple_column_that_does_not_resolve_is_unchecked() {
        let tuple = CAttributeTuple::new(
            vec![
                CAttribute::single("units", MultiplicityInterval::MANDATORY, Vec::new()).unwrap(),
                CAttribute::single(
                    "accuracy",
                    MultiplicityInterval::MANDATORY,
                    Vec::new(),
                )
                .unwrap(),
            ],
            vec![units_magnitude_row("mm[Hg]", 0.0, 300.0)],
        )
        .unwrap();
        let value_object = CComplexObject::new(
            "DV_QUANTITY",
            None,
            MultiplicityInterval::MANDATORY,
            Vec::new(),
        )
        .unwrap()
        .with_attribute_tuples(vec![tuple]);
        let value_attr = CAttribute::single(
            "value",
            MultiplicityInterval::MANDATORY,
            vec![CObject::Complex(value_object)],
        )
        .unwrap();
        let element = CObject::Complex(
            CComplexObject::new(
                "ELEMENT",
                Some("at0004".to_owned()),
                MultiplicityInterval::MANDATORY,
                vec![value_attr],
            )
            .unwrap(),
        );
        let archetype = evaluation_archetype(element);
        let entry = quantity_entry(140.0, "mm[Hg]");
        let report = validate_against_archetype(&archetype, entry.as_node());
        assert!(report.violations().is_empty());
        assert_eq!(
            report.unchecked()[0].reason(),
            "C_ATTRIBUTE_TUPLE column does not resolve to exactly one value"
        );
        assert_eq!(report.unchecked()[0].detail(), Some("0 value(s)"));
    }

    /// `C_DATE`/`C_TIME`/`C_DATE_TIME`/`C_DURATION`: a value in range is
    /// conformant, one outside it is a violation naming the class, and a
    /// `pattern` alongside a range is unchecked even when the range passes —
    /// the same three shapes `a_c_integer_range_rejects_a_value_outside_it`
    /// and `a_c_string_pattern_is_unchecked_even_when_the_list_passes` above
    /// already prove for the two other primitive kinds a range or a pattern
    /// applies to. One check per temporal type: `check_temporal` is the same
    /// function for all four, so this is confirming it dispatches correctly
    /// through the real `CPrimitive`/`Scalar` match, not re-testing the
    /// function's own logic four times over.
    #[test]
    fn c_date_time_duration_ranges_are_checked_and_their_patterns_are_not() {
        let cases: Vec<(&str, CPrimitive, DataValue, &str, DataValue)> = vec![
            (
                "DV_DATE",
                CPrimitive::Date {
                    range: vec![
                        Interval::closed(
                            "2024-01-01".parse().unwrap(),
                            "2024-12-31".parse().unwrap(),
                        )
                        .unwrap(),
                    ],
                    pattern: None,
                },
                DataValue::Date(DvDate::new("2024-06-15").unwrap()),
                "C_DATE",
                DataValue::Date(DvDate::new("2025-01-01").unwrap()),
            ),
            (
                "DV_TIME",
                CPrimitive::Time {
                    range: vec![
                        Interval::closed("08:00:00".parse().unwrap(), "17:00:00".parse().unwrap())
                            .unwrap(),
                    ],
                    pattern: None,
                },
                DataValue::Time(DvTime::new("12:00:00").unwrap()),
                "C_TIME",
                DataValue::Time(DvTime::new("18:00:00").unwrap()),
            ),
            (
                "DV_DATE_TIME",
                CPrimitive::DateTime {
                    range: vec![
                        Interval::closed(
                            "2024-01-01T00:00:00Z".parse().unwrap(),
                            "2024-01-02T00:00:00Z".parse().unwrap(),
                        )
                        .unwrap(),
                    ],
                    pattern: None,
                },
                DataValue::DateTime(DvDateTime::new("2024-01-01T12:00:00Z").unwrap()),
                "C_DATE_TIME",
                DataValue::DateTime(DvDateTime::new("2024-01-03T00:00:00Z").unwrap()),
            ),
            (
                "DV_DURATION",
                CPrimitive::Duration {
                    range: vec![
                        Interval::closed("PT0S".parse().unwrap(), "PT1H".parse().unwrap()).unwrap(),
                    ],
                    pattern: None,
                },
                DataValue::Duration(DvDuration::new("PT30M").unwrap()),
                "C_DURATION",
                DataValue::Duration(DvDuration::new("PT2H").unwrap()),
            ),
        ];

        for (rm_type, primitive, in_range_value, class, out_of_range_value) in cases {
            let constraint = element_with_value_constraint("at0004", "value", primitive, rm_type);
            let archetype = evaluation_archetype(constraint);

            let conformant =
                build_evaluation(vec![Element::new(attrs("Field", "at0004"), in_range_value)]);
            assert!(
                validate_against_archetype(&archetype, conformant.as_node()).is_conformant(),
                "{rm_type}: an in-range value should conform"
            );

            let out_of_range = build_evaluation(vec![Element::new(
                attrs("Field", "at0004"),
                out_of_range_value,
            )]);
            let report = validate_against_archetype(&archetype, out_of_range.as_node());
            assert_eq!(
                report
                    .violations()
                    .first()
                    .map(ArchetypeViolation::constraint),
                Some(class),
                "{rm_type}: an out-of-range value should violate {class}"
            );
        }
    }

    /// A `pattern` is carried and never evaluated, matching `C_STRING`'s own
    /// precedent exactly — checked separately from the range case above so a
    /// range check that happened to also report the pattern would not hide
    /// behind an assertion that only looked at `violations()`.
    #[test]
    fn a_c_date_pattern_is_unchecked_even_when_the_range_passes() {
        let constraint = element_with_value_constraint(
            "at0004",
            "value",
            CPrimitive::Date {
                range: Vec::new(),
                pattern: Some("YYYY-??-??".to_owned()),
            },
            "DV_DATE",
        );
        let archetype = evaluation_archetype(constraint);
        let entry = build_evaluation(vec![Element::new(
            attrs("Field", "at0004"),
            DataValue::Date(DvDate::new("2024-06-15").unwrap()),
        )]);
        let report = validate_against_archetype(&archetype, entry.as_node());
        assert!(report.violations().is_empty());
        assert_eq!(
            report.unchecked()[0].reason(),
            "the constraint pattern is carried but not evaluated"
        );
        assert_eq!(report.unchecked()[0].detail(), Some("C_DATE"));
    }

    #[test]
    fn a_c_terminology_code_checks_against_the_archetypes_own_value_set() {
        let constraint = element_with_value_constraint(
            "at0004",
            "defining_code",
            CPrimitive::TerminologyCode {
                constraint: Some("ac0001".to_owned()),
                constraint_status: None,
            },
            "DV_CODED_TEXT",
        );
        let items_attr = CAttribute::container(
            "items",
            MultiplicityInterval::MANDATORY,
            Cardinality::new(MultiplicityInterval::at_least(0).unwrap()),
            vec![constraint],
        )
        .unwrap();
        let data_object = CComplexObject::new(
            "ITEM_LIST",
            Some("id2".to_owned()),
            MultiplicityInterval::MANDATORY,
            vec![items_attr],
        )
        .unwrap();
        let data_attr = CAttribute::single(
            "data",
            MultiplicityInterval::MANDATORY,
            vec![CObject::Complex(data_object)],
        )
        .unwrap();
        let definition = CComplexObject::new(
            "EVALUATION",
            Some("id1".to_owned()),
            MultiplicityInterval::MANDATORY,
            vec![data_attr],
        )
        .unwrap();
        let terminology = ArchetypeTerminology::new(
            "en",
            terms(&[
                ("id1", "Test"),
                ("id2", "Data"),
                ("at0004", "Sex"),
                ("at0010", "Male"),
                ("at0011", "Female"),
            ]),
        )
        .unwrap()
        .with_value_set(
            "ac0001",
            BTreeSet::from(["at0010".to_owned(), "at0011".to_owned()]),
        )
        .unwrap();
        let archetype = Archetype::new(
            "openEHR-EHR-EVALUATION.test.v1".parse().unwrap(),
            definition,
            terminology,
        )
        .unwrap();

        let coded = |code: &str| {
            DataValue::CodedText(
                DvCodedText::new("value", CodePhrase::new("local", code).unwrap()).unwrap(),
            )
        };

        let permitted =
            build_evaluation(vec![Element::new(attrs("Sex", "at0004"), coded("at0010"))]);
        assert!(validate_against_archetype(&archetype, permitted.as_node()).is_conformant());

        let refused = build_evaluation(vec![Element::new(attrs("Sex", "at0004"), coded("at0099"))]);
        let report = validate_against_archetype(&archetype, refused.as_node());
        assert_eq!(report.violations()[0].constraint(), "C_TERMINOLOGY_CODE");
    }

    /// A `constraint` naming a single `at`-code (AOM2's leader convention,
    /// not a separate field) checks exact equality with the code itself, not
    /// a value-set lookup — `code_list`'s replacement, `A-51`'s second pass.
    #[test]
    fn a_c_terminology_code_at_code_constraint_checks_exact_equality() {
        let constraint = element_with_value_constraint(
            "at0004",
            "defining_code",
            CPrimitive::TerminologyCode {
                constraint: Some("at0010".to_owned()),
                constraint_status: None,
            },
            "DV_CODED_TEXT",
        );
        let archetype = evaluation_archetype(constraint);
        let coded = |code: &str| {
            DataValue::CodedText(
                DvCodedText::new("value", CodePhrase::new("local", code).unwrap()).unwrap(),
            )
        };

        let exact =
            build_evaluation(vec![Element::new(attrs("Systolic", "at0004"), coded("at0010"))]);
        assert!(validate_against_archetype(&archetype, exact.as_node()).is_conformant());

        let other = build_evaluation(vec![Element::new(
            attrs("Systolic", "at0004"),
            coded("at0011"),
        )]);
        let report = validate_against_archetype(&archetype, other.as_node());
        assert_eq!(report.violations()[0].constraint(), "C_TERMINOLOGY_CODE");
    }

    /// `constraint: None`, and AOM2's own empty-string spelling of the same
    /// thing, are both "no constraint" — unchecked, not a violation and not
    /// silently satisfied.
    #[test]
    fn a_c_terminology_code_with_no_constraint_is_unchecked_either_spelling() {
        for constraint in [None, Some(String::new())] {
            let element =
                element_with_value_constraint(
                    "at0004",
                    "defining_code",
                    CPrimitive::TerminologyCode {
                        constraint,
                        constraint_status: None,
                    },
                    "DV_CODED_TEXT",
                );
            let archetype = evaluation_archetype(element);
            let entry = build_evaluation(vec![Element::new(
                attrs("Systolic", "at0004"),
                DataValue::CodedText(
                    DvCodedText::new("value", CodePhrase::new("local", "at0010").unwrap())
                        .unwrap(),
                ),
            )]);
            let report = validate_against_archetype(&archetype, entry.as_node());
            assert!(report.violations().is_empty());
            assert_eq!(
                report.unchecked()[0].reason(),
                "C_TERMINOLOGY_CODE names no constraint"
            );
        }
    }

    /// `constraint_status`: AOM2's own words for anything but `Required` are
    /// "validity of the data instance is achieved by supplying *any*
    /// terminology code" — a code other than the one named must still be
    /// conformant once the constraint is `extensible`, `preferred`, or
    /// `example`. Before `constraint_status` existed anywhere in this crate,
    /// this exact case reported a `C_TERMINOLOGY_CODE` violation for
    /// conformant data, because nothing distinguished a soft constraint from
    /// a required one (`A-51`).
    #[test]
    fn a_soft_terminology_constraint_accepts_a_code_other_than_the_one_named() {
        let constraint = element_with_value_constraint(
            "at0004",
            "defining_code",
            CPrimitive::TerminologyCode {
                constraint: Some("at0010".to_owned()),
                constraint_status: Some(ConstraintStatus::Extensible),
            },
            "DV_CODED_TEXT",
        );
        let archetype = evaluation_archetype(constraint);
        let coded = |code: &str| {
            DataValue::CodedText(
                DvCodedText::new("value", CodePhrase::new("local", code).unwrap()).unwrap(),
            )
        };
        let entry = build_evaluation(vec![Element::new(
            attrs("Systolic", "at0004"),
            coded("at9999"),
        )]);
        let report = validate_against_archetype(&archetype, entry.as_node());
        assert!(
            report.violations().is_empty(),
            "an extensible constraint refused a code it does not name, contradicting its \
             own AOM2 semantics"
        );
        assert!(report.unchecked().is_empty());
    }

    #[test]
    fn the_wrong_root_rm_class_is_a_single_violation_not_a_cascade() {
        let archetype =
            evaluation_archetype(element_alt("at0004", MultiplicityInterval::MANDATORY));
        // An ADMIN_ENTRY where the archetype expects an EVALUATION.
        let data = ItemList::new(attrs("Data", "id2"), Vec::new());
        let admin = Entry::from(AdminEntry::new(
            attrs("Wrong class", "openEHR-EHR-ADMIN_ENTRY.test.v1"),
            entry_attrs(),
            data.into(),
        ));
        let report = validate_against_archetype(&archetype, admin.as_node());
        assert_eq!(report.violations().len(), 1);
        assert_eq!(report.violations()[0].constraint(), "Rm_type_name_matches");
    }

    // -- `C_ARCHETYPE_ROOT` resolution through a repository (`K15.24`-`K15.27`) --

    /// A repository that always answers the same fixed result, whatever
    /// identifier is asked for -- enough to test the caller's side of the
    /// trait without a real retrieval implementation, which `openehr` never
    /// has (`K15.25`).
    struct FixedRepository(Result<Resolved, RepositoryError>);

    impl ArchetypeRepository for FixedRepository {
        fn resolve(&self, _id: &ArchetypeId) -> Result<Resolved, RepositoryError> {
            self.0.clone()
        }
    }

    /// `ELEMENT[id1]` whose `value` must be a `DV_BOOLEAN` of `true` —
    /// nested the way a real primitive constraint is: `value` holds the
    /// `DV_BOOLEAN` complex object, whose own `value` attribute is the
    /// actual `C_BOOLEAN` (see `element_with_value_constraint` above).
    fn filler_archetype(id: &str) -> Archetype {
        let mut terms = BTreeMap::new();
        terms.insert(
            "id1".to_owned(),
            TermDefinition::new("Filler", None).unwrap(),
        );
        let definition = CComplexObject::new(
            "ELEMENT",
            Some("id1".to_owned()),
            MultiplicityInterval::MANDATORY,
            vec![
                CAttribute::single(
                    "value",
                    MultiplicityInterval::MANDATORY,
                    vec![CObject::Complex(
                        CComplexObject::new(
                            "DV_BOOLEAN",
                            None,
                            MultiplicityInterval::MANDATORY,
                            vec![
                                CAttribute::single(
                                    "value",
                                    MultiplicityInterval::MANDATORY,
                                    vec![CObject::Primitive(CPrimitiveObject::new(
                                        "Boolean",
                                        MultiplicityInterval::MANDATORY,
                                        CPrimitive::Boolean {
                                            allow_true: true,
                                            allow_false: false,
                                        },
                                    ))],
                                )
                                .unwrap(),
                            ],
                        )
                        .unwrap(),
                    )],
                )
                .unwrap(),
            ],
        )
        .unwrap();
        Archetype::new(
            id.parse().unwrap(),
            definition,
            ArchetypeTerminology::new("en", terms).unwrap(),
        )
        .unwrap()
    }

    /// An `EVALUATION[id1]/data[id2 ITEM_LIST]/items` archetype whose sole
    /// alternative is a `C_ARCHETYPE_ROOT[at0004]` naming `filler_id`.
    fn evaluation_archetype_with_filled_slot(filler_id: &str) -> Archetype {
        let root = CArchetypeRoot::new("ELEMENT", filler_id, MultiplicityInterval::MANDATORY)
            .unwrap()
            .with_node_id("at0004")
            .unwrap();
        evaluation_archetype(CObject::ArchetypeRoot(root))
    }

    fn provenance() -> Provenance {
        Provenance::new("ckm.openehr.org", "1.0.0", "2026-08-30T00:00:00Z", "digest")
    }

    #[test]
    fn without_a_repository_a_filled_slot_is_unchecked() {
        let archetype = evaluation_archetype_with_filled_slot("openEHR-EHR-ELEMENT.filler.v1");
        let entry = build_evaluation(vec![Element::new(
            attrs("Filled", "at0004"),
            DataValue::Boolean(DvBoolean::new(true)),
        )]);
        let report = validate_against_archetype(&archetype, entry.as_node());
        assert!(!report.is_conformant());
        assert!(report.violations().is_empty());
        assert_eq!(report.unchecked().len(), 1);
        assert!(
            report.unchecked()[0]
                .reason()
                .contains("no repository was supplied")
        );
    }

    #[test]
    fn a_repository_resolved_filler_validates_the_same_subtree() {
        let filler_id = "openEHR-EHR-ELEMENT.filler.v1";
        let archetype = evaluation_archetype_with_filled_slot(filler_id);
        let repo = FixedRepository(Ok(Resolved::new(filler_archetype(filler_id), provenance())));

        let conforming = build_evaluation(vec![Element::new(
            attrs("Filled", "at0004"),
            DataValue::Boolean(DvBoolean::new(true)),
        )]);
        let report = validate_with_repository(
            &archetype,
            conforming.as_node(),
            &repo,
            RepositoryOptions::default(),
        );
        assert!(report.is_conformant(), "{report:?}");

        // The filler's own constraint (value must be `true`) is what fires,
        // attributed to the *filler's* archetype id, not the outer one.
        let violating = build_evaluation(vec![Element::new(
            attrs("Filled", "at0004"),
            DataValue::Boolean(DvBoolean::new(false)),
        )]);
        let report = validate_with_repository(
            &archetype,
            violating.as_node(),
            &repo,
            RepositoryOptions::default(),
        );
        assert_eq!(report.violations()[0].constraint(), "C_BOOLEAN");
        assert_eq!(report.violations()[0].archetype_id().to_string(), filler_id);
    }

    #[test]
    fn a_retrieval_failure_is_unchecked_and_names_what_happened() {
        let filler_id = "openEHR-EHR-ELEMENT.filler.v1";
        let archetype = evaluation_archetype_with_filled_slot(filler_id);
        let repo = FixedRepository(Err(RepositoryError::NotFound {
            id: filler_id.parse().unwrap(),
        }));
        let entry = build_evaluation(vec![Element::new(
            attrs("Filled", "at0004"),
            DataValue::Boolean(DvBoolean::new(true)),
        )]);
        let report = validate_with_repository(
            &archetype,
            entry.as_node(),
            &repo,
            RepositoryOptions::default(),
        );
        assert!(!report.is_conformant());
        assert!(report.violations().is_empty());
        let unchecked = &report.unchecked()[0];
        assert!(unchecked.reason().contains("could not be retrieved"));
        assert!(unchecked.detail().unwrap().contains("no archetype found"));
    }

    #[test]
    fn a_repository_returning_a_different_archetype_is_unchecked_not_used() {
        let filler_id = "openEHR-EHR-ELEMENT.filler.v1";
        let archetype = evaluation_archetype_with_filled_slot(filler_id);
        // Answers every request with an archetype under a *different* id.
        let repo = FixedRepository(Ok(Resolved::new(
            filler_archetype("openEHR-EHR-ELEMENT.wrong.v1"),
            provenance(),
        )));
        let entry = build_evaluation(vec![Element::new(
            attrs("Filled", "at0004"),
            DataValue::Boolean(DvBoolean::new(true)),
        )]);
        let report = validate_with_repository(
            &archetype,
            entry.as_node(),
            &repo,
            RepositoryOptions::default(),
        );
        assert!(!report.is_conformant());
        assert!(report.violations().is_empty());
        assert!(
            report.unchecked()[0]
                .reason()
                .contains("a different archetype than the one requested")
        );
    }

    #[test]
    fn unestablished_provenance_is_unchecked_unless_the_caller_opts_in() {
        let filler_id = "openEHR-EHR-ELEMENT.filler.v1";
        let archetype = evaluation_archetype_with_filled_slot(filler_id);
        let repo = FixedRepository(Ok(Resolved::without_provenance(filler_archetype(
            filler_id,
        ))));
        let entry = build_evaluation(vec![Element::new(
            attrs("Filled", "at0004"),
            DataValue::Boolean(DvBoolean::new(true)),
        )]);

        let refused = validate_with_repository(
            &archetype,
            entry.as_node(),
            &repo,
            RepositoryOptions::default(),
        );
        assert!(!refused.is_conformant());
        assert!(
            refused.unchecked()[0]
                .reason()
                .contains("provenance is not established")
        );

        let opted_in = validate_with_repository(
            &archetype,
            entry.as_node(),
            &repo,
            RepositoryOptions::default().allow_unestablished_provenance(),
        );
        assert!(opted_in.is_conformant(), "{opted_in:?}");
        assert_eq!(
            opted_in.unverified_provenance(),
            ["/data[id2]/items[at0004]"]
        );
    }
}
