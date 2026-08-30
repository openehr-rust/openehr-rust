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
//! # `K15.20`: no partial pass
//!
//! A construct this module cannot check — an `ARCHETYPE_SLOT` or
//! `C_ARCHETYPE_ROOT` filler, which needs retrieval (`K15.24`, not
//! implemented); a `C_UNSUPPORTED` primitive constraint; a `C_STRING` pattern,
//! which is carried but not compiled or applied (see [`crate::am::CPrimitive`])
//! — is recorded as [`Unchecked`], never silently treated as satisfied.
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
use crate::am::constraint::{CAttribute, CComplexObject, CObject, CPrimitive};
use crate::am::terminology::ArchetypeTerminology;
use crate::base::{ArchetypeId, Real};
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
    archetype_id: ArchetypeId,
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
    pub const fn archetype_id(&self) -> &ArchetypeId {
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
}

/// The outcome of validating one instance against one archetype.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchetypeReport {
    violations: Vec<ArchetypeViolation>,
    unchecked: Vec<Unchecked>,
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

/// Accumulates violations and unchecked nodes while walking the two trees.
struct Ctx<'a> {
    archetype_id: &'a ArchetypeId,
    terminology: &'a ArchetypeTerminology,
    violations: Vec<ArchetypeViolation>,
    unchecked: Vec<Unchecked>,
}

impl Ctx<'_> {
    fn violation(&mut self, path: &str, constraint: &'static str) {
        self.violations.push(ArchetypeViolation {
            archetype_path: path.to_owned(),
            archetype_id: self.archetype_id.clone(),
            constraint,
        });
    }

    fn unchecked(&mut self, path: &str, reason: &'static str) {
        self.unchecked.push(Unchecked {
            archetype_path: path.to_owned(),
            reason,
        });
    }

    fn finish(self) -> ArchetypeReport {
        ArchetypeReport {
            violations: self.violations,
            unchecked: self.unchecked,
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
        archetype_id: archetype.archetype_id(),
        terminology: archetype.terminology(),
        violations: Vec::new(),
        unchecked: Vec::new(),
    };

    if root.type_name() != archetype.rm_type_name() {
        // A root of the wrong RM class invalidates everything beneath it too
        // — walking further would only produce a cascade of "attribute not
        // found" noise from a class the archetype never described.
        ctx.violation("", "Rm_type_name_matches");
        return ctx.finish();
    }

    walk_complex(archetype.definition(), root, "", &mut ctx);
    ctx.finish()
}

/// Checks every attribute constraint against the data reachable through it.
fn walk_complex(constraint: &CComplexObject, node: Node<'_>, path: &str, ctx: &mut Ctx<'_>) {
    for attribute in constraint.attributes() {
        walk_attribute(attribute, node, path, ctx);
    }
}

/// One `C_ATTRIBUTE`: existence, cardinality, and each alternative beneath it.
fn walk_attribute(attribute: &CAttribute, node: Node<'_>, path: &str, ctx: &mut Ctx<'_>) {
    let children = node.children(attribute.rm_attribute_name());
    let attr_path = format!("{path}/{}", attribute.rm_attribute_name());

    if children.is_empty() {
        if attribute.existence().lower() > 0 {
            ctx.violation(&attr_path, "Existence");
        }
        return;
    }

    let count = u32::try_from(children.len()).unwrap_or(u32::MAX);
    match attribute.cardinality() {
        Some(cardinality) => {
            if !cardinality.interval().contains(count) {
                ctx.violation(&attr_path, "Cardinality");
            }
        }
        // No cardinality means the attribute is single-valued, and
        // `crate::path::Node::children` only ever returns more than one node
        // for a genuine container attribute — but an archetype whose author
        // forgot to declare one is a defect the instance should not appear
        // to survive silently.
        None if count > 1 => ctx.violation(&attr_path, "Cardinality"),
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
                walk_object(&alternatives[index], child, &attr_path, ctx);
            }
            Match::Unrecognised => ctx.violation(&attr_path, "Unrecognised_node_id"),
            Match::Ambiguous => ctx.unchecked(
                &attr_path,
                "more than one constraint alternative here, and the node carries no \
                 archetype_node_id to say which one it satisfies",
            ),
        }
    }

    for (alternative, count) in alternatives.iter().zip(matched_counts) {
        if !alternative.occurrences().contains(count) {
            let node_path = match alternative.node_id() {
                Some(id) => format!("{attr_path}[{id}]"),
                None => attr_path.clone(),
            };
            ctx.violation(&node_path, "Occurrences");
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
fn walk_object(constraint: &CObject, node: Node<'_>, path: &str, ctx: &mut Ctx<'_>) {
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
        walk_primitive(p.constraint(), node, &path, ctx);
        return;
    }

    if node.type_name() != constraint.rm_type_name() {
        ctx.violation(&path, "Rm_type_name_matches");
        return;
    }

    match constraint {
        CObject::Complex(c) => walk_complex(c, node, &path, ctx),
        CObject::Slot(_) => ctx.unchecked(
            &path,
            "ARCHETYPE_SLOT: the filler archetype is not resolved (retrieval, K15.24, is not \
             implemented)",
        ),
        CObject::ArchetypeRoot(_) => ctx.unchecked(
            &path,
            "C_ARCHETYPE_ROOT: the filler archetype is not resolved (retrieval, K15.24, is not \
             implemented)",
        ),
        CObject::Primitive(_) => unreachable!("handled above"),
    }
}

/// A leaf primitive constraint against the scalar the path walk reached.
fn walk_primitive(constraint: &CPrimitive, node: Node<'_>, path: &str, ctx: &mut Ctx<'_>) {
    let Node::Scalar(scalar) = node else {
        // A `C_PRIMITIVE_OBJECT` governs a scalar attribute; the RM shape
        // does not match what the archetype expects if the walk reached
        // anything else here. Reported rather than assumed unreachable,
        // because a hand-built archetype (`K15.4`) is not obliged to be
        // well formed, and this module does not trust its input any more
        // than `crate::validation` trusts a deserialized RM instance.
        ctx.violation(path, "Primitive_kind_mismatch");
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
                ctx.violation(path, "C_BOOLEAN");
            }
        }
        (CPrimitive::String { list, pattern }, Scalar::Str(value)) => {
            if !list.is_empty() && !list.iter().any(|item| item == value) {
                ctx.violation(path, "C_STRING");
            } else if pattern.is_some() {
                // Carried but never compiled or applied — see
                // `crate::am::CPrimitive::String`'s own documentation for why.
                ctx.unchecked(path, "C_STRING pattern is not evaluated");
            }
        }
        (CPrimitive::Integer { list, range }, Scalar::Integer(value)) => {
            let in_list = list.is_empty() || list.contains(&value);
            let in_range = range.as_ref().is_none_or(|r| r.contains(&value));
            if !in_list || !in_range {
                ctx.violation(path, "C_INTEGER");
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
                ctx.violation(path, "C_REAL");
            }
        }
        (
            CPrimitive::TerminologyCode {
                constraint: ac_code,
                code_list,
            },
            Scalar::Str(value),
        ) => {
            if ac_code.is_none() && code_list.is_empty() {
                // Neither an `ac`-code nor an inline list: nothing here
                // constrains the code, so nothing can be checked against it.
                ctx.unchecked(
                    path,
                    "C_TERMINOLOGY_CODE names neither an ac-code nor an inline code list",
                );
            } else {
                let inline_ok = code_list.iter().any(|code| code == value);
                let value_set_ok = ac_code
                    .as_deref()
                    .and_then(|ac| ctx.terminology.value_set(ac))
                    .is_some_and(|set| set.contains(value));
                if !inline_ok && !value_set_ok {
                    ctx.violation(path, "C_TERMINOLOGY_CODE");
                }
            }
        }
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
            ctx.violation(path, "Primitive_kind_mismatch");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::am::{
        ArchetypeSlot, ArchetypeTerminology, CAttribute, CComplexObject, CObject, CPrimitiveObject,
        Cardinality, MultiplicityInterval, TermDefinition,
    };
    use crate::base::Interval;
    use crate::path::Pathable as _;
    use crate::rm::common::LocatableAttrs;
    use crate::rm::data_structures::{Element, ItemList};
    use crate::rm::data_types::{CodePhrase, DataValue, DvBoolean, DvCodedText, DvCount};
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
            .map(|(code, text)| ((*code).to_owned(), TermDefinition::new(*text, None).unwrap()))
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
            CComplexObject::new("ELEMENT", Some(node_id.to_owned()), occurrences, Vec::new()).unwrap(),
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
        let value_object =
            CComplexObject::new(value_rm_type, None, MultiplicityInterval::MANDATORY, vec![inner_attr])
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
        let entry = build_evaluation(vec![Element::new(attrs("Systolic", "at0004"), placeholder_value())]);
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
        let entry = build_evaluation(vec![Element::new(attrs("Systolic", "at0004"), placeholder_value())]);
        let report = validate_against_archetype(&archetype, entry.as_node());
        assert_eq!(report.violations()[0].constraint(), "Occurrences");
        assert_eq!(report.violations()[0].archetype_path(), "/data[id2]/items[at0004]");
    }

    #[test]
    fn a_slot_is_reported_unchecked_never_as_passing() {
        // `node_id` goes through a binding rather than sitting beside the
        // class name as a literal: `openehr-assets` counts any adjacent pair
        // of string literals shaped like a class and an invariant as a
        // citation, deliberately, and `("ELEMENT", "at0004")` would enter the
        // invariant-coverage report as one (see `am::constraint::tests`'s own
        // note on this).
        let node_id = "at0004";
        let slot = CObject::Slot(
            ArchetypeSlot::new("ELEMENT", node_id, MultiplicityInterval::MANDATORY).unwrap(),
        );
        let archetype = evaluation_archetype(slot);
        let entry = build_evaluation(vec![Element::new(attrs("Systolic", "at0004"), placeholder_value())]);
        let report = validate_against_archetype(&archetype, entry.as_node());
        assert!(!report.is_conformant());
        assert!(report.violations().is_empty());
        assert_eq!(report.unchecked().len(), 1);
        assert_eq!(report.unchecked()[0].archetype_path(), "/data[id2]/items[at0004]");
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
    fn a_c_string_pattern_is_unchecked_even_when_the_list_passes() {
        let constraint = element_with_value_constraint(
            "at0004",
            "value",
            CPrimitive::String {
                list: Vec::new(),
                pattern: Some(r"\d+".to_owned()),
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
        assert_eq!(report.unchecked()[0].reason(), "C_STRING pattern is not evaluated");
    }

    #[test]
    fn a_c_terminology_code_checks_against_the_archetypes_own_value_set() {
        let constraint = element_with_value_constraint(
            "at0004",
            "defining_code",
            CPrimitive::TerminologyCode {
                constraint: Some("ac0001".to_owned()),
                code_list: Vec::new(),
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

        let permitted = build_evaluation(vec![Element::new(attrs("Sex", "at0004"), coded("at0010"))]);
        assert!(validate_against_archetype(&archetype, permitted.as_node()).is_conformant());

        let refused = build_evaluation(vec![Element::new(attrs("Sex", "at0004"), coded("at0099"))]);
        let report = validate_against_archetype(&archetype, refused.as_node());
        assert_eq!(report.violations()[0].constraint(), "C_TERMINOLOGY_CODE");
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
}
