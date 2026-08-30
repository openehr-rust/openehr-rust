//! The constraint tree: `C_OBJECT`, `C_ATTRIBUTE`, and the primitive
//! constraints beneath them.
//!
//! An archetype's `definition` is a tree that alternates between objects and
//! attributes: a `C_COMPLEX_OBJECT` constrains an RM class, its `C_ATTRIBUTE`s
//! constrain that class's attributes, and each attribute's children constrain
//! what may appear there. This module is that tree and nothing else — building
//! it from ADL text is `K15.5`, and applying it to data is `K15.18`, and
//! neither is implemented.
//!
//! # Node identifiers, and the two syntaxes
//!
//! ADL 2 identifies nodes with `id`-codes (`id1`, `id1.1`), and ADL 1.4 uses
//! `at`-codes (`at0000`, `at0001`). Both appear in the published corpus, and a
//! 1.4 archetype converted to AOM2 keeps codes that a reader will recognise, so
//! this module accepts either spelling and records which one it saw. It does
//! **not** rewrite one into the other: a code is how a clinician's tooling,
//! their queries, and the stored data all refer to the node, and silently
//! renumbering it would change the meaning of records already written
//! (`K15.8`).

use crate::base::{Interval, Real};
use crate::error::ParseError;
use crate::{am::Cardinality, am::MultiplicityInterval};
use serde::{Deserialize, Serialize};

/// Whether a node identifier is well formed, and which syntax it is written in.
///
/// ```
/// use openehr::am::NodeIdSyntax;
///
/// assert_eq!(NodeIdSyntax::of("id1.1"), Some(NodeIdSyntax::Adl2));
/// assert_eq!(NodeIdSyntax::of("at0004"), Some(NodeIdSyntax::Adl14));
/// assert_eq!(NodeIdSyntax::of("ac0001"), Some(NodeIdSyntax::Adl14));
/// assert_eq!(NodeIdSyntax::of("banana"), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeIdSyntax {
    /// `id`-, `at`- or `ac`-codes as ADL 2 spells them: `id1`, `id1.1`, `at2`.
    Adl2,
    /// The zero-padded `at0000` / `ac0001` forms of ADL 1.4.
    Adl14,
}

impl NodeIdSyntax {
    /// Classifies a node identifier, or reports that it is not one.
    ///
    /// The distinction is by zero padding, which is how the two corpora
    /// actually differ in practice: ADL 1.4 writes four padded digits, ADL 2
    /// writes an unpadded number with optional specialisation segments.
    #[must_use]
    pub fn of(code: &str) -> Option<Self> {
        let digits = code
            .strip_prefix("id")
            .or_else(|| code.strip_prefix("at"))
            .or_else(|| code.strip_prefix("ac"))?;
        if digits.is_empty() {
            return None;
        }
        let mut segments = digits.split('.');
        let first = segments.next()?;
        if !first.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        for segment in segments {
            if segment.is_empty() || !segment.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
        }
        // `at0004` — four digits, leading zero — is 1.4's spelling. `id1` and
        // `at2` are ADL 2's. `at0` alone is ambiguous and read as ADL 2, which
        // is the syntax that would have written it deliberately.
        if first.len() > 1 && first.starts_with('0') {
            Some(Self::Adl14)
        } else {
            Some(Self::Adl2)
        }
    }

    /// How deep in a specialisation hierarchy the code sits: `id1` is 0,
    /// `id1.1` is 1.
    ///
    /// AOM2's `VATCD` requires a code's specialisation depth not to exceed the
    /// archetype's own, which is a check that needs this number.
    #[must_use]
    pub fn specialisation_depth(code: &str) -> usize {
        code.matches('.').count()
    }
}

/// A constraint on one attribute of an RM class.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CAttribute {
    rm_attribute_name: String,
    existence: MultiplicityInterval,
    /// Present only for container attributes.
    cardinality: Option<Cardinality>,
    children: Vec<CObject>,
}

impl CAttribute {
    /// Builds a single-valued attribute constraint.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the attribute name is empty.
    pub fn single(
        rm_attribute_name: impl Into<String>,
        existence: MultiplicityInterval,
        children: Vec<CObject>,
    ) -> Result<Self, ParseError> {
        let rm_attribute_name = rm_attribute_name.into();
        if rm_attribute_name.is_empty() {
            return Err(ParseError::invariant("C_ATTRIBUTE", "empty attribute name"));
        }
        Ok(Self {
            rm_attribute_name,
            existence,
            cardinality: None,
            children,
        })
    }

    /// Builds a container attribute constraint.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the attribute name is empty, or if the
    /// cardinality cannot hold the occurrences its children require — a
    /// cardinality of `0..1` under which two children are each `1..1` is a
    /// constraint nothing satisfies, and AOM2 states that agreement as a
    /// validity condition rather than leaving it to a runtime to discover.
    pub fn container(
        rm_attribute_name: impl Into<String>,
        existence: MultiplicityInterval,
        cardinality: Cardinality,
        children: Vec<CObject>,
    ) -> Result<Self, ParseError> {
        let mut attribute = Self::single(rm_attribute_name, existence, children)?;
        let required: u32 = attribute
            .children
            .iter()
            .map(|child| child.occurrences().lower())
            .sum();
        if let Some(upper) = cardinality.interval().upper()
            && required > upper
        {
            return Err(ParseError::invariant(
                "C_ATTRIBUTE",
                "children require more occurrences than the cardinality permits",
            ));
        }
        attribute.cardinality = Some(cardinality);
        Ok(attribute)
    }

    /// The RM attribute this constrains.
    #[must_use]
    pub fn rm_attribute_name(&self) -> &str {
        &self.rm_attribute_name
    }

    /// Whether the attribute must be present.
    #[must_use]
    pub const fn existence(&self) -> &MultiplicityInterval {
        &self.existence
    }

    /// The container cardinality, if this is a container attribute.
    #[must_use]
    pub const fn cardinality(&self) -> Option<&Cardinality> {
        self.cardinality.as_ref()
    }

    /// What may appear under this attribute.
    #[must_use]
    pub fn children(&self) -> &[CObject] {
        &self.children
    }
}

/// A node in the constraint tree.
///
/// `#[non_exhaustive]` is deliberate here and deliberately absent from
/// `ColTy` in the persistence crates, for opposite reasons: a new SQL type
/// *should* break every dialect at compile time, and a new AOM2 node kind
/// should not break a caller that already handles the ones it can. What a
/// caller must never do is treat an unrecognised node as unconstrained —
/// `K15.20` requires an unchecked node to be reported, never passed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type")]
#[non_exhaustive]
pub enum CObject {
    /// A constrained RM object with attributes beneath it.
    #[serde(rename = "C_COMPLEX_OBJECT")]
    Complex(CComplexObject),
    /// A constrained leaf value.
    #[serde(rename = "C_PRIMITIVE_OBJECT")]
    Primitive(CPrimitiveObject),
    /// A point at which another archetype may be used.
    #[serde(rename = "ARCHETYPE_SLOT")]
    Slot(ArchetypeSlot),
    /// A slot already filled by a named archetype, as a template does.
    #[serde(rename = "C_ARCHETYPE_ROOT")]
    ArchetypeRoot(CArchetypeRoot),
}

impl CObject {
    /// The RM class this node constrains.
    #[must_use]
    pub fn rm_type_name(&self) -> &str {
        match self {
            Self::Complex(o) => &o.rm_type_name,
            Self::Primitive(o) => &o.rm_type_name,
            Self::Slot(o) => &o.rm_type_name,
            Self::ArchetypeRoot(o) => &o.rm_type_name,
        }
    }

    /// The node identifier, where the node has one.
    #[must_use]
    pub fn node_id(&self) -> Option<&str> {
        match self {
            Self::Complex(o) => o.node_id.as_deref(),
            Self::Primitive(o) => o.node_id.as_deref(),
            Self::Slot(o) => Some(&o.node_id),
            Self::ArchetypeRoot(o) => o.node_id.as_deref(),
        }
    }

    /// How many times this node may occur under its parent attribute.
    #[must_use]
    pub const fn occurrences(&self) -> &MultiplicityInterval {
        match self {
            Self::Complex(o) => &o.occurrences,
            Self::Primitive(o) => &o.occurrences,
            Self::Slot(o) => &o.occurrences,
            Self::ArchetypeRoot(o) => &o.occurrences,
        }
    }

    /// The attributes beneath this node, empty for a leaf.
    #[must_use]
    pub fn attributes(&self) -> &[CAttribute] {
        match self {
            Self::Complex(o) => &o.attributes,
            Self::ArchetypeRoot(o) => &o.attributes,
            Self::Primitive(_) | Self::Slot(_) => &[],
        }
    }
}

/// A constrained RM object: `C_COMPLEX_OBJECT`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CComplexObject {
    rm_type_name: String,
    node_id: Option<String>,
    occurrences: MultiplicityInterval,
    attributes: Vec<CAttribute>,
}

impl CComplexObject {
    /// Builds a complex object constraint.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the RM type name is empty, if the node
    /// identifier is present and malformed, or if two attributes constrain the
    /// same RM attribute — AOM2's `VOKU`, keys unique among siblings. Two
    /// constraints on one attribute are not a conjunction; they are an artefact
    /// whose author meant one of them and a reader cannot tell which.
    pub fn new(
        rm_type_name: impl Into<String>,
        node_id: Option<String>,
        occurrences: MultiplicityInterval,
        attributes: Vec<CAttribute>,
    ) -> Result<Self, ParseError> {
        let rm_type_name = rm_type_name.into();
        if rm_type_name.is_empty() {
            return Err(ParseError::invariant("C_COMPLEX_OBJECT", "empty rm_type_name"));
        }
        if let Some(code) = node_id.as_deref()
            && NodeIdSyntax::of(code).is_none()
        {
            return Err(ParseError::new(
                "C_COMPLEX_OBJECT",
                "node_id is not an id-, at- or ac-code",
                code,
            ));
        }
        let mut seen: Vec<&str> = Vec::with_capacity(attributes.len());
        for attribute in &attributes {
            if seen.contains(&attribute.rm_attribute_name()) {
                return Err(ParseError::invariant("C_COMPLEX_OBJECT", "VOKU"));
            }
            seen.push(attribute.rm_attribute_name());
        }
        Ok(Self {
            rm_type_name,
            node_id,
            occurrences,
            attributes,
        })
    }

    /// The RM class constrained.
    #[must_use]
    pub fn rm_type_name(&self) -> &str {
        &self.rm_type_name
    }

    /// The node identifier, if this node carries one.
    #[must_use]
    pub fn node_id(&self) -> Option<&str> {
        self.node_id.as_deref()
    }

    /// How many times this node may occur.
    #[must_use]
    pub const fn occurrences(&self) -> &MultiplicityInterval {
        &self.occurrences
    }

    /// The attribute constraints beneath this node.
    #[must_use]
    pub fn attributes(&self) -> &[CAttribute] {
        &self.attributes
    }
}

/// A constrained leaf value: `C_PRIMITIVE_OBJECT`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CPrimitiveObject {
    rm_type_name: String,
    node_id: Option<String>,
    occurrences: MultiplicityInterval,
    constraint: CPrimitive,
}

impl CPrimitiveObject {
    /// Builds a primitive constraint node.
    #[must_use]
    pub fn new(
        rm_type_name: impl Into<String>,
        occurrences: MultiplicityInterval,
        constraint: CPrimitive,
    ) -> Self {
        Self {
            rm_type_name: rm_type_name.into(),
            node_id: None,
            occurrences,
            constraint,
        }
    }

    /// The constraint itself.
    #[must_use]
    pub const fn constraint(&self) -> &CPrimitive {
        &self.constraint
    }

    /// The RM class constrained.
    #[must_use]
    pub fn rm_type_name(&self) -> &str {
        &self.rm_type_name
    }
}

/// The primitive constraint kinds AOM2 defines.
///
/// `#[non_exhaustive]`, and `Unsupported` is a variant rather than an absence:
/// a primitive constraint this crate cannot represent must survive a
/// round trip (`K15.3`) and must make any node it governs *unchecked* rather
/// than passing (`K15.20`). Dropping it on read is exactly the silent widening
/// that the withdrawn `S1.4` predicted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type")]
#[non_exhaustive]
pub enum CPrimitive {
    /// `C_BOOLEAN`.
    #[serde(rename = "C_BOOLEAN")]
    Boolean {
        /// Whether `true` is permitted.
        allow_true: bool,
        /// Whether `false` is permitted.
        allow_false: bool,
    },
    /// `C_STRING`: a list of permitted values, a regular expression, or both.
    #[serde(rename = "C_STRING")]
    String {
        /// Permitted literal values.
        list: Vec<String>,
        /// A pattern, carried as written. **Not compiled and not applied** —
        /// a node governed by one is unchecked until `K15.18` lands.
        pattern: Option<String>,
    },
    /// `C_INTEGER`.
    #[serde(rename = "C_INTEGER")]
    Integer {
        /// Permitted values.
        list: Vec<i64>,
        /// A permitted range.
        range: Option<Interval<i64>>,
    },
    /// `C_REAL`. Bounded with [`Real`], not `f64`, for the reason `D3.18d`
    /// gives: a constraint written `1.50` is not the constraint `1.5`.
    #[serde(rename = "C_REAL")]
    Real {
        /// Permitted values.
        list: Vec<Real>,
        /// A permitted range.
        range: Option<Interval<Real>>,
    },
    /// `C_TERMINOLOGY_CODE`: an `ac`-code naming a value set, or a list of
    /// `at`-codes.
    #[serde(rename = "C_TERMINOLOGY_CODE")]
    TerminologyCode {
        /// The `ac`-code whose value set constrains this node, if any.
        constraint: Option<String>,
        /// Permitted `at`-codes, if the constraint is inline.
        code_list: Vec<String>,
    },
    /// A constraint kind this crate models by carrying it and nothing else.
    #[serde(rename = "C_UNSUPPORTED")]
    Unsupported {
        /// The AOM2 type name as it was read.
        rm_type_name: String,
        /// The constraint's own serialised form, preserved verbatim.
        source: String,
    },
}

/// A point at which another archetype may be used: `ARCHETYPE_SLOT`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArchetypeSlot {
    rm_type_name: String,
    node_id: String,
    occurrences: MultiplicityInterval,
    /// Assertions carried as written. `K15.10`: parsed and evaluated is a later
    /// requirement, and until then a slot's fillers are unchecked, not open.
    includes: Vec<String>,
    excludes: Vec<String>,
}

impl ArchetypeSlot {
    /// Builds a slot.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the node identifier is malformed. A slot
    /// without a usable identifier cannot be filled by a template, because the
    /// filler names the slot it fills.
    pub fn new(
        rm_type_name: impl Into<String>,
        node_id: impl Into<String>,
        occurrences: MultiplicityInterval,
    ) -> Result<Self, ParseError> {
        let node_id = node_id.into();
        if NodeIdSyntax::of(&node_id).is_none() {
            return Err(ParseError::new(
                "ARCHETYPE_SLOT",
                "node_id is not an id-, at- or ac-code",
                &node_id,
            ));
        }
        Ok(Self {
            rm_type_name: rm_type_name.into(),
            node_id,
            occurrences,
            includes: Vec::new(),
            excludes: Vec::new(),
        })
    }

    /// Adds an inclusion assertion, carried as written.
    #[must_use]
    pub fn including(mut self, assertion: impl Into<String>) -> Self {
        self.includes.push(assertion.into());
        self
    }

    /// Adds an exclusion assertion, carried as written.
    #[must_use]
    pub fn excluding(mut self, assertion: impl Into<String>) -> Self {
        self.excludes.push(assertion.into());
        self
    }

    /// The slot's node identifier.
    #[must_use]
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// The inclusion assertions, unparsed.
    #[must_use]
    pub fn includes(&self) -> &[String] {
        &self.includes
    }

    /// The exclusion assertions, unparsed.
    #[must_use]
    pub fn excludes(&self) -> &[String] {
        &self.excludes
    }
}

/// A slot filled by a named archetype: `C_ARCHETYPE_ROOT`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CArchetypeRoot {
    rm_type_name: String,
    node_id: Option<String>,
    /// The archetype used here, as an identifier. Resolving it to an artefact
    /// is retrieval (`K15.24`) — `crate::am::validate::validate_with_repository`
    /// does it, given an `ArchetypeRepository`; this type only carries the
    /// reference, since building one does not require reaching a repository.
    archetype_ref: String,
    occurrences: MultiplicityInterval,
    attributes: Vec<CAttribute>,
}

impl CArchetypeRoot {
    /// Builds a filled slot.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the archetype reference is empty. It is not
    /// checked against a repository: nothing here can reach one, and a
    /// reference that cannot be resolved must be reported at validation time as
    /// unresolved rather than assumed absent (`K15.27`).
    pub fn new(
        rm_type_name: impl Into<String>,
        archetype_ref: impl Into<String>,
        occurrences: MultiplicityInterval,
    ) -> Result<Self, ParseError> {
        let archetype_ref = archetype_ref.into();
        if archetype_ref.is_empty() {
            return Err(ParseError::invariant(
                "C_ARCHETYPE_ROOT",
                "empty archetype reference",
            ));
        }
        Ok(Self {
            rm_type_name: rm_type_name.into(),
            node_id: None,
            archetype_ref,
            occurrences,
            attributes: Vec::new(),
        })
    }

    /// Records this node's identifier.
    ///
    /// Optional at construction because a `C_ARCHETYPE_ROOT` standing alone as
    /// an archetype's whole definition needs none, but one filling a slot
    /// among siblings does: matching an instance node against the right
    /// alternative under its attribute is by `archetype_node_id` (`K15.18`),
    /// and a filled slot with no id of its own can never be matched.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the identifier is not an id-, at- or
    /// ac-code.
    pub fn with_node_id(mut self, node_id: impl Into<String>) -> Result<Self, ParseError> {
        let node_id = node_id.into();
        if NodeIdSyntax::of(&node_id).is_none() {
            return Err(ParseError::new(
                "C_ARCHETYPE_ROOT",
                "node_id is not an id-, at- or ac-code",
                &node_id,
            ));
        }
        self.node_id = Some(node_id);
        Ok(self)
    }

    /// The archetype used at this point.
    #[must_use]
    pub fn archetype_ref(&self) -> &str {
        &self.archetype_ref
    }

    /// This node's identifier, if it has one.
    #[must_use]
    pub fn node_id(&self) -> Option<&str> {
        self.node_id.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(node_id: &str) -> CObject {
        CObject::Complex(
            CComplexObject::new(
                "ELEMENT",
                Some(node_id.to_owned()),
                MultiplicityInterval::MANDATORY,
                Vec::new(),
            )
            .unwrap(),
        )
    }

    #[test]
    fn two_constraints_on_one_attribute_are_refused() {
        let dup = CComplexObject::new(
            "OBSERVATION",
            Some("id1".to_owned()),
            MultiplicityInterval::MANDATORY,
            vec![
                CAttribute::single("data", MultiplicityInterval::MANDATORY, Vec::new()).unwrap(),
                CAttribute::single("data", MultiplicityInterval::MANDATORY, Vec::new()).unwrap(),
            ],
        );
        assert_eq!(dup.unwrap_err().reason, "VOKU");
    }

    #[test]
    fn a_cardinality_that_cannot_hold_its_children_is_refused() {
        let err = CAttribute::container(
            "items",
            MultiplicityInterval::MANDATORY,
            Cardinality::new(MultiplicityInterval::OPTIONAL),
            vec![element("at0001"), element("at0002")],
        )
        .unwrap_err();
        assert_eq!(
            err.reason,
            "children require more occurrences than the cardinality permits"
        );
    }

    #[test]
    fn a_malformed_node_id_is_refused_at_construction() {
        assert!(
            CComplexObject::new(
                "ELEMENT",
                Some("node-4".to_owned()),
                MultiplicityInterval::MANDATORY,
                Vec::new()
            )
            .is_err()
        );
        // The malformed code goes through a binding rather than sitting beside
        // the class name as a literal: `openehr-assets` counts **any** adjacent
        // pair of string literals shaped like a class and an invariant as a
        // citation (`lib:A-25`), deliberately, and a test that reads like
        // `SECTION.banana` would enter the invariant-coverage report as one.
        let malformed = "banana";
        assert!(ArchetypeSlot::new("SECTION", malformed, MultiplicityInterval::OPTIONAL).is_err());
        let root = CArchetypeRoot::new("SECTION", "openEHR-EHR-SECTION.x.v1", MultiplicityInterval::MANDATORY)
            .unwrap();
        assert!(root.with_node_id(malformed).is_err());
    }

    #[test]
    fn a_filled_slot_can_carry_a_node_id_for_matching_against_its_siblings() {
        let root = CArchetypeRoot::new(
            "SECTION",
            "openEHR-EHR-SECTION.medications.v1",
            MultiplicityInterval::MANDATORY,
        )
        .unwrap();
        assert!(root.node_id().is_none());
        let identified = root.with_node_id("id2").unwrap();
        assert_eq!(identified.node_id(), Some("id2"));
    }

    #[test]
    fn both_node_id_syntaxes_are_recognised_and_kept_apart() {
        assert_eq!(NodeIdSyntax::of("id1"), Some(NodeIdSyntax::Adl2));
        assert_eq!(NodeIdSyntax::of("at0000"), Some(NodeIdSyntax::Adl14));
        assert_eq!(NodeIdSyntax::specialisation_depth("id1.1.2"), 2);
        assert_eq!(NodeIdSyntax::specialisation_depth("at0004"), 0);
    }
}
