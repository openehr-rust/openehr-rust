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

use crate::base::{Date, DateTime, Duration, Interval, Real, Time};
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
    /// The shared part of [`single`](Self::single) and
    /// [`container`](Self::container): the name check and the struct, with no
    /// cardinality and neither's own occurrences rule applied yet. Kept
    /// private and separate from `single` specifically so that `container`
    /// does not go through `single`'s `VACSO` check — a check that belongs
    /// only to single-valued attributes, and would wrongly refuse a
    /// container's own children (which legitimately may occur more than
    /// once) if `container` built on `single` directly.
    fn new_raw(
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

    /// Builds a single-valued attribute constraint.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the attribute name is empty, or if a child's
    /// occurrences has a finite upper bound greater than `1` (`VACSO`): a
    /// single-valued attribute holds at most one object, so a child declaring
    /// it may occur more than once is a constraint this attribute shape
    /// cannot satisfy.
    pub fn single(
        rm_attribute_name: impl Into<String>,
        existence: MultiplicityInterval,
        children: Vec<CObject>,
    ) -> Result<Self, ParseError> {
        let attribute = Self::new_raw(rm_attribute_name, existence, children)?;
        if attribute
            .children
            .iter()
            .any(|child| child.occurrences().upper().is_some_and(|upper| upper > 1))
        {
            return Err(ParseError::invariant(
                "C_ATTRIBUTE",
                "a single-valued attribute's child occurrences upper bound exceeds 1",
            ));
        }
        Ok(attribute)
    }

    /// Builds a container attribute constraint.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the attribute name is empty, if the
    /// cardinality cannot hold the occurrences its children require — a
    /// cardinality of `0..1` under which two children are each `1..1` is a
    /// constraint nothing satisfies, and AOM2 states that agreement as a
    /// validity condition rather than leaving it to a runtime to discover —
    /// or if a child's occurrences has a finite upper bound greater than the
    /// cardinality's own finite upper bound (`VACMCU`): a cardinality of
    /// `0..2` cannot hold a single child declared `0..10`, independent of
    /// how many children there are or what their lower bounds sum to.
    pub fn container(
        rm_attribute_name: impl Into<String>,
        existence: MultiplicityInterval,
        cardinality: Cardinality,
        children: Vec<CObject>,
    ) -> Result<Self, ParseError> {
        let mut attribute = Self::new_raw(rm_attribute_name, existence, children)?;
        let required: u32 = attribute
            .children
            .iter()
            .map(|child| child.occurrences().lower())
            .sum();
        if let Some(upper) = cardinality.interval().upper() {
            if required > upper {
                return Err(ParseError::invariant(
                    "C_ATTRIBUTE",
                    "children require more occurrences than the cardinality permits",
                ));
            }
            if attribute
                .children
                .iter()
                .any(|child| child.occurrences().upper().is_some_and(|u| u > upper))
            {
                return Err(ParseError::invariant(
                    "C_ATTRIBUTE",
                    "a child's occurrences upper bound exceeds the cardinality upper bound",
                ));
            }
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

    /// The co-varying (tuple) constraints beneath this node, empty for every
    /// kind but `Complex` — `C_ARCHETYPE_ROOT` inherits `attribute_tuples`
    /// from `C_COMPLEX_OBJECT` in AOM2, but this crate's [`CArchetypeRoot`]
    /// has no way to carry one, the same asymmetry it already has for
    /// `attributes` (`CArchetypeRoot::new` leaves that empty and
    /// unsettable too, matching "In all uses within source archetypes and
    /// templates, the `_children_` attribute is `Void`",
    /// `org.openehr.am.aom2.c_archetype_root.adoc`).
    #[must_use]
    pub fn attribute_tuples(&self) -> &[CAttributeTuple] {
        match self {
            Self::Complex(o) => &o.attribute_tuples,
            Self::ArchetypeRoot(_) | Self::Primitive(_) | Self::Slot(_) => &[],
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
    /// `0..1`, like `attributes` — absent is `Vec::new()`, not a distinct
    /// state. See [`Self::with_attribute_tuples`].
    #[serde(default)]
    attribute_tuples: Vec<CAttributeTuple>,
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
            return Err(ParseError::invariant(
                "C_COMPLEX_OBJECT",
                "empty rm_type_name",
            ));
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
            attribute_tuples: Vec::new(),
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

    /// Attaches co-varying (tuple) constraints
    /// (`org.openehr.am.aom2.c_complex_object.adoc`'s `attribute_tuples`).
    ///
    /// A builder, not a `new` parameter, for the same reason
    /// [`CPrimitiveObject::with_assumed_value`] is one: most callers never use
    /// this attribute, and `attribute_tuples` reached this crate later than
    /// `attributes` did.
    #[must_use]
    pub fn with_attribute_tuples(mut self, attribute_tuples: Vec<CAttributeTuple>) -> Self {
        self.attribute_tuples = attribute_tuples;
        self
    }

    /// The co-varying constraints beneath this node, if any were attached
    /// with [`Self::with_attribute_tuples`].
    ///
    /// **Carried, not evaluated.** `crate::am::validate::validate_against_archetype`
    /// reports a node with any here as [`crate::am::Unchecked`] rather than
    /// walking it — see that module's own documentation for why.
    #[must_use]
    pub fn attribute_tuples(&self) -> &[CAttributeTuple] {
        &self.attribute_tuples
    }
}

/// One co-varying value combination in a tuple constraint:
/// `C_PRIMITIVE_TUPLE`.
///
/// One row of a [`CAttributeTuple`]'s [`tuples`](CAttributeTuple::tuples): a
/// vector of primitive-object constraints, positionally aligned with the
/// owning `C_ATTRIBUTE_TUPLE`'s own [`members`](CAttributeTuple::members).
/// AOM2's own words for the correspondence: "Each such instance is a vector
/// of object constraints, where each member (each `C_PRIMITIVE_OBJECT`)
/// corresponds to one of the `C_ATTRIBUTEs` referred to by the owning
/// `C_ATTRIBUTE_TUPLE`" (`openEHR/specifications-AM`,
/// `docs/UML/classes/org.openehr.am.aom2.c_primitive_tuple.adoc`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CPrimitiveTuple {
    members: Vec<CPrimitiveObject>,
}

impl CPrimitiveTuple {
    /// Builds one tuple row.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if `members` is empty. `C_PRIMITIVE_TUPLE
    /// .members` is `1..1` in AOM2 — mandatory — unlike `C_ATTRIBUTE_TUPLE
    /// .members` and `.tuples`, both `0..1`; a Rust `Vec` has no `Void` to
    /// distinguish "empty" from "absent" the way Eiffel does, so the
    /// mandatory case is translated the same way `ArchetypeTerminology::new`
    /// already translates one: an empty vector is refused rather than
    /// silently standing in for "unset".
    pub fn new(members: Vec<CPrimitiveObject>) -> Result<Self, ParseError> {
        if members.is_empty() {
            return Err(ParseError::invariant("C_PRIMITIVE_TUPLE", "empty members"));
        }
        Ok(Self { members })
    }

    /// The row's own values, one per co-varying attribute, positionally
    /// aligned with the owning [`CAttributeTuple::members`].
    #[must_use]
    pub fn members(&self) -> &[CPrimitiveObject] {
        &self.members
    }
}

/// A co-varying constraint on more than one attribute at once:
/// `C_ATTRIBUTE_TUPLE`.
///
/// AOM2's answer to a `DV_QUANTITY`'s `{units, magnitude}`, or a
/// `DV_ORDINAL`'s `{value, symbol}`: constraining each attribute separately
/// would allow any combination of the two lists — `"deg F"` paired with a
/// range meant for Centigrade — and a tuple constraint pairs them instead, so
/// only the combinations actually listed in [`Self::tuples`] are permitted
/// (`openEHR/specifications-AM`,
/// `docs/ADL2/master04.4-cadl_second_order.adoc`). It "replaces all
/// domain-specific constraint types defined in ADL/AOM 1.4, including
/// `C_DV_QUANTITY` and `C_DV_ORDINAL`"
/// (`docs/AOM2/master04.3-constraint_model-second_order.adoc`) — without it,
/// this crate could not represent either pattern at all, not even as
/// [`CPrimitive::Unsupported`], because [`CComplexObject`] had nowhere to put
/// one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CAttributeTuple {
    members: Vec<CAttribute>,
    tuples: Vec<CPrimitiveTuple>,
}

impl CAttributeTuple {
    /// Builds a tuple constraint.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if any row in `tuples` has a different number of
    /// values than `members` has attributes. See [`CPrimitiveTuple`]'s own
    /// documentation for the AOM2 text this positional correspondence comes
    /// from — a row naming three values for a two-attribute tuple has a value
    /// with nothing to correspond to.
    pub fn new(
        members: Vec<CAttribute>,
        tuples: Vec<CPrimitiveTuple>,
    ) -> Result<Self, ParseError> {
        if tuples.iter().any(|row| row.members().len() != members.len()) {
            return Err(ParseError::invariant(
                "C_ATTRIBUTE_TUPLE",
                "a tuple row's arity does not match the number of co-varying attributes",
            ));
        }
        Ok(Self { members, tuples })
    }

    /// The attributes this tuple co-constrains, in the order every row in
    /// [`Self::tuples`] is aligned against.
    #[must_use]
    pub fn members(&self) -> &[CAttribute] {
        &self.members
    }

    /// The permitted value combinations.
    #[must_use]
    pub fn tuples(&self) -> &[CPrimitiveTuple] {
        &self.tuples
    }
}

/// A concrete value of one of the kinds `CPrimitive` constrains, carried as
/// [`CPrimitiveObject::assumed_value`] (AOM2's `C_PRIMITIVE_OBJECT
/// .assumed_value: Any`).
///
/// One `Text` variant stands in for `C_STRING`, `C_DATE`, `C_TIME`,
/// `C_DATE_TIME`, `C_DURATION`, and `C_TERMINOLOGY_CODE` alike, each as its
/// own lexical text — the same collapsing `crate::path::Scalar::Str` already
/// makes for the corresponding `DataValue`s. Which of the six a given `Text`
/// means is decided by which `CPrimitive` variant it is attached to via
/// [`CPrimitiveObject::with_assumed_value`], not by this type itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PrimitiveValue {
    /// `C_BOOLEAN`'s assumed value.
    Boolean(bool),
    /// `C_STRING`, `C_DATE`, `C_TIME`, `C_DATE_TIME`, `C_DURATION`, or
    /// `C_TERMINOLOGY_CODE`'s assumed value, as lexical text.
    Text(String),
    /// `C_INTEGER`'s assumed value.
    Integer(i64),
    /// `C_REAL`'s assumed value, tried only once `Integer` has failed to
    /// parse the same JSON number — so `5` still reads as `Integer(5)`, not
    /// `Real`.
    Real(Real),
}

/// A constrained leaf value: `C_PRIMITIVE_OBJECT`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CPrimitiveObject {
    rm_type_name: String,
    node_id: Option<String>,
    occurrences: MultiplicityInterval,
    constraint: CPrimitive,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    assumed_value: Option<PrimitiveValue>,
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
            assumed_value: None,
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

    /// The AOM2 sentinel `_node_id_` value for a `C_PRIMITIVE_OBJECT`
    /// written inline in ADL, with no node id of its own
    /// (`openEHR/specifications-AM`,
    /// `docs/UML/classes/org.openehr.am.aom2.c_primitive_object.adoc`:
    /// "the `_node_id_` attribute will have the special value
    /// `Primitive_node_id`; otherwise it will have the node id read during
    /// parsing").
    pub const PRIMITIVE_NODE_ID: &'static str = "Primitive_node_id";

    /// Records this node's own identifier — either a real id-, at-, or
    /// ac-code, when this primitive constraint was written with one in the
    /// source, or [`Self::PRIMITIVE_NODE_ID`], AOM2's own sentinel for the
    /// inline form that has none.
    ///
    /// Every `C_OBJECT` has a `node_id` (`org.openehr.am.aom2.c_object.adoc`:
    /// `1..1`); this crate had no way to give a `C_PRIMITIVE_OBJECT` one at
    /// all before this existed — [`CObject::node_id`] already read the
    /// field, which stayed `None` because nothing could set it.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if `node_id` is neither a valid id-, at-, or
    /// ac-code nor the [`Self::PRIMITIVE_NODE_ID`] sentinel — a bare
    /// [`NodeIdSyntax::of`] check would reject the sentinel, since it is not
    /// coded-identifier syntax at all.
    pub fn with_node_id(mut self, node_id: impl Into<String>) -> Result<Self, ParseError> {
        let node_id = node_id.into();
        if node_id != Self::PRIMITIVE_NODE_ID && NodeIdSyntax::of(&node_id).is_none() {
            return Err(ParseError::new(
                "C_PRIMITIVE_OBJECT",
                "node_id is neither an id-, at-, or ac-code nor the Primitive_node_id sentinel",
                &node_id,
            ));
        }
        self.node_id = Some(node_id);
        Ok(self)
    }

    /// This node's own identifier, if [`Self::with_node_id`] recorded one.
    /// [`CObject::node_id`] is what most callers want: it reaches this
    /// through every `C_OBJECT` variant uniformly, rather than requiring a
    /// caller to already know it holds a `C_PRIMITIVE_OBJECT`.
    #[must_use]
    pub fn node_id(&self) -> Option<&str> {
        self.node_id.as_deref()
    }

    /// Attaches the value to be assumed when data supplies none
    /// (`org.openehr.am.aom2.c_primitive_object.adoc`'s `assumed_value`).
    ///
    /// **`Inv_valid_assumed_value` is not checked.** AOM2 requires
    /// `valid_value(assumed_value)` — that the value conforms to
    /// [`Self::constraint`] — and this crate does not evaluate it, the same
    /// choice already made for `C_STRING`'s `pattern` and carried here for
    /// the same reason: nothing calls this before a template author or a
    /// form generator would, and neither exists in this crate. A value of
    /// the wrong [`PrimitiveValue`] shape for the attached `CPrimitive` —
    /// `Boolean` on a `C_INTEGER`, say — is accepted and carried exactly as
    /// given.
    #[must_use]
    pub fn with_assumed_value(mut self, value: PrimitiveValue) -> Self {
        self.assumed_value = Some(value);
        self
    }

    /// The value to be assumed when data supplies none, if
    /// [`Self::with_assumed_value`] recorded one. See its own documentation
    /// for what is and is not checked about it.
    #[must_use]
    pub fn assumed_value(&self) -> Option<&PrimitiveValue> {
        self.assumed_value.as_ref()
    }
}

/// Whether a terminology constraint is strictly binding, or a preference or
/// example: `CONSTRAINT_STATUS`.
///
/// AOM2's own words for the three non-`Required` values (`openEHR/specifications-AM`,
/// `docs/ADL2/master04.5-cadl_primitive_types.adoc`): *extensible* — the
/// instance must conform to the value set only if the intended concept is
/// available in it, otherwise any other code is conformant; *preferred* — the
/// instance should conform, but any other code is equally conformant;
/// *example* — the constraint is illustrative only. All three, formally, mean
/// the same thing at archetype-conformance level: "validity of the data
/// instance is achieved by supplying *any* terminology code" — narrower
/// semantic checking is left to tooling this crate does not implement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintStatus {
    /// An instance must supply one of the codes the constraint names.
    /// AOM2's default when `constraint_status` is `Void` at the top level of
    /// an archetype.
    Required,
    /// A code from the constraint is used if it covers the intended meaning;
    /// otherwise any other code, including from another terminology, is
    /// conformant.
    Extensible,
    /// A code from the constraint is preferred, but any other code is
    /// equally conformant.
    Preferred,
    /// The constraint is provided as an illustrative example only; any
    /// terminology code is conformant.
    Example,
}

impl ConstraintStatus {
    /// Whether an instance must actually satisfy the coded constraint to
    /// conform. AOM2's own `constraint_required()`: "True if
    /// `constraint_status` is defined and equals `required`" — the `Void`
    /// half of that function ("OR if Void") is not this method's concern,
    /// since a caller already had to unwrap `Option<ConstraintStatus>` to
    /// reach a value to call this on; see [`CPrimitive::TerminologyCode`]'s
    /// own documentation for where `None` is read as `Required` instead.
    #[must_use]
    pub const fn is_required(self) -> bool {
        matches!(self, Self::Required)
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
    ///
    /// **Residual, not fixed here.** AOM2's own `constraint` attribute
    /// (`openEHR/specifications-AM`,
    /// `docs/UML/classes/org.openehr.am.aom2.c_terminology_code.adoc`) is a
    /// single `String` — one `at`-code, or one `ac`-code naming a value set —
    /// never a list; ADL's own way of offering several alternative codes is
    /// several sibling `C_OBJECT`s under one attribute, each with its own
    /// single-code `C_TERMINOLOGY_CODE`, the same alternative-matching shape
    /// [`CAttribute::children`] already gives every other node kind. This
    /// variant's `code_list: Vec<String>` has no such counterpart in AOM2 and
    /// was not re-derived from the primary source when it was written. Fixing
    /// the shape — most likely dropping `code_list` in favour of the sibling
    /// pattern — is a breaking change to an already-published type and is left
    /// open (`A-51` in `spec/audit.md`) rather than made silently in the same
    /// pass that added this variant's `constraint_status` field below.
    #[serde(rename = "C_TERMINOLOGY_CODE")]
    TerminologyCode {
        /// The `ac`-code whose value set constrains this node, if any.
        constraint: Option<String>,
        /// Permitted `at`-codes, if the constraint is inline.
        code_list: Vec<String>,
        /// Whether the constraint is strictly binding, or a preference or
        /// example (`org.openehr.am.aom2.c_terminology_code.adoc`'s
        /// `constraint_status`). `None` reads as [`ConstraintStatus::Required`]
        /// — AOM2's own default for a top-level archetype, which is the only
        /// kind this crate builds (`K15.11` covers the specialised case,
        /// where `Void` instead means "inherit the parent's value", not
        /// modelled here).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        constraint_status: Option<ConstraintStatus>,
    },
    /// `C_DATE`. AOM2's own shape for the four temporal primitives below is
    /// a *list* of ranges rather than `C_INTEGER`/`C_REAL`'s discrete list
    /// plus one optional range — dates are not usually enumerated the way
    /// integers are — so `range` alone, empty meaning unconstrained, is
    /// `C_DATE`'s own `constraint` attribute
    /// (`List<Interval<Iso8601_date>>`, `openEHR/specifications-AM`,
    /// `docs/UML/classes/org.openehr.am.aom2.c_date.adoc`), not an
    /// approximation of [`CPrimitive::Integer`]'s shape.
    #[serde(rename = "C_DATE")]
    Date {
        /// Permitted ranges; a value in any one of them satisfies the
        /// constraint. Empty means unconstrained by range (a `pattern` may
        /// still apply).
        range: Vec<Interval<Date>>,
        /// An ISO 8601 constraint pattern (e.g. `"YYYY-??-??"`), carried as
        /// written. **Not compiled and not applied** — the same choice
        /// [`CPrimitive::String`]'s own `pattern` field already documents,
        /// for the same reason: a node governed by one is unchecked
        /// (`ctx.unchecked`), not silently passed, until a real
        /// pattern-matching implementation exists.
        pattern: Option<String>,
    },
    /// `C_TIME`. See [`CPrimitive::Date`] for why this is a list of ranges
    /// rather than a discrete list plus one range.
    #[serde(rename = "C_TIME")]
    Time {
        /// Permitted ranges.
        range: Vec<Interval<Time>>,
        /// Carried, not applied — see [`CPrimitive::Date::pattern`].
        pattern: Option<String>,
    },
    /// `C_DATE_TIME`. See [`CPrimitive::Date`] for why this is a list of
    /// ranges rather than a discrete list plus one range.
    #[serde(rename = "C_DATE_TIME")]
    DateTime {
        /// Permitted ranges.
        range: Vec<Interval<DateTime>>,
        /// Carried, not applied — see [`CPrimitive::Date::pattern`].
        pattern: Option<String>,
    },
    /// `C_DURATION`. See [`CPrimitive::Date`] for why this is a list of
    /// ranges rather than a discrete list plus one range.
    #[serde(rename = "C_DURATION")]
    Duration {
        /// Permitted ranges.
        range: Vec<Interval<Duration>>,
        /// Carried, not applied — see [`CPrimitive::Date::pattern`].
        pattern: Option<String>,
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

    fn element_with_occurrences(node_id: &str, occurrences: MultiplicityInterval) -> CObject {
        CObject::Complex(
            CComplexObject::new("ELEMENT", Some(node_id.to_owned()), occurrences, Vec::new())
                .unwrap(),
        )
    }

    /// `VACMCU`: a cardinality's finite upper bound must hold every child's
    /// own finite occurrences upper bound, independent of how the lower
    /// bounds sum — `a_cardinality_that_cannot_hold_its_children_is_refused`
    /// above covers the lower-bound sum; this is the other half.
    #[test]
    fn a_childs_occurrences_upper_bound_beyond_the_cardinality_is_refused() {
        let err = CAttribute::container(
            "items",
            MultiplicityInterval::MANDATORY,
            Cardinality::new(MultiplicityInterval::new(0, Some(2)).unwrap()),
            vec![element_with_occurrences(
                "at0001",
                MultiplicityInterval::new(0, Some(10)).unwrap(),
            )],
        )
        .unwrap_err();
        assert_eq!(
            err.reason,
            "a child's occurrences upper bound exceeds the cardinality upper bound"
        );

        // The same child under a cardinality wide enough to hold it is fine.
        assert!(
            CAttribute::container(
                "items",
                MultiplicityInterval::MANDATORY,
                Cardinality::new(MultiplicityInterval::new(0, Some(10)).unwrap()),
                vec![element_with_occurrences(
                    "at0001",
                    MultiplicityInterval::new(0, Some(10)).unwrap(),
                )],
            )
            .is_ok()
        );

        // An unbounded cardinality holds any finite child upper bound.
        assert!(
            CAttribute::container(
                "items",
                MultiplicityInterval::MANDATORY,
                Cardinality::new(MultiplicityInterval::new(0, None).unwrap()),
                vec![element_with_occurrences(
                    "at0001",
                    MultiplicityInterval::new(0, Some(10)).unwrap(),
                )],
            )
            .is_ok()
        );
    }

    /// `VACSO`: a single-valued attribute holds at most one object, so a
    /// child declaring it may occur more than once is a constraint this
    /// attribute shape cannot satisfy — checked in `single`, not `container`,
    /// which legitimately holds children occurring more than once.
    #[test]
    fn a_single_valued_attributes_child_may_not_occur_more_than_once() {
        let err = CAttribute::single(
            "value",
            MultiplicityInterval::MANDATORY,
            vec![element_with_occurrences(
                "at0001",
                MultiplicityInterval::new(0, Some(2)).unwrap(),
            )],
        )
        .unwrap_err();
        assert_eq!(
            err.reason,
            "a single-valued attribute's child occurrences upper bound exceeds 1"
        );

        // Exactly 1 is fine, and so is unbounded-below-but-capped-at-1.
        assert!(
            CAttribute::single(
                "value",
                MultiplicityInterval::MANDATORY,
                vec![element_with_occurrences(
                    "at0001",
                    MultiplicityInterval::new(0, Some(1)).unwrap(),
                )],
            )
            .is_ok()
        );

        // A container attribute is not affected by VACSO at all: the same
        // child that VACSO refuses under `single` is fine under `container`.
        assert!(
            CAttribute::container(
                "items",
                MultiplicityInterval::MANDATORY,
                Cardinality::new(MultiplicityInterval::new(0, None).unwrap()),
                vec![element_with_occurrences(
                    "at0001",
                    MultiplicityInterval::new(0, Some(2)).unwrap(),
                )],
            )
            .is_ok()
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
        let root = CArchetypeRoot::new(
            "SECTION",
            "openEHR-EHR-SECTION.x.v1",
            MultiplicityInterval::MANDATORY,
        )
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
    fn a_primitive_object_can_carry_a_node_id_readable_through_either_accessor() {
        let leaf = CPrimitiveObject::new(
            "DV_BOOLEAN",
            MultiplicityInterval::MANDATORY,
            CPrimitive::Boolean {
                allow_true: true,
                allow_false: true,
            },
        );
        assert!(leaf.node_id().is_none());
        let identified = leaf.with_node_id("at0001").unwrap();
        assert_eq!(identified.node_id(), Some("at0001"));
        // `CObject::node_id` is what most callers use; it must see the same
        // value through the enum, not just through `CPrimitiveObject` itself.
        let wrapped = CObject::Primitive(identified);
        assert_eq!(wrapped.node_id(), Some("at0001"));
    }

    #[test]
    fn a_primitive_object_written_inline_carries_the_aom2_sentinel_node_id() {
        let leaf = CPrimitiveObject::new(
            "DV_BOOLEAN",
            MultiplicityInterval::MANDATORY,
            CPrimitive::Boolean {
                allow_true: true,
                allow_false: true,
            },
        )
        .with_node_id(CPrimitiveObject::PRIMITIVE_NODE_ID)
        .unwrap();
        // `NodeIdSyntax::of` alone would reject this value — it is a literal
        // sentinel, not id-, at-, or ac-coded syntax — so accepting it here
        // is specifically what the `!=` short-circuit in `with_node_id` is
        // for, not something a bare `NodeIdSyntax::of` check would allow.
        assert!(NodeIdSyntax::of(CPrimitiveObject::PRIMITIVE_NODE_ID).is_none());
        assert_eq!(leaf.node_id(), Some("Primitive_node_id"));
    }

    #[test]
    fn a_primitive_object_refuses_a_node_id_that_is_neither_coded_nor_the_sentinel() {
        let leaf = CPrimitiveObject::new(
            "DV_BOOLEAN",
            MultiplicityInterval::MANDATORY,
            CPrimitive::Boolean {
                allow_true: true,
                allow_false: true,
            },
        );
        // Bound rather than inlined for the same `lib:A-25` reason as the
        // malformed-node-id test above: a literal `"DV_BOOLEAN", "banana"`
        // pair would misread as a class/invariant citation.
        let malformed = "banana";
        assert!(leaf.with_node_id(malformed).is_err());
    }

    #[test]
    fn a_primitive_object_carries_an_assumed_value_of_the_matching_kind() {
        let leaf = CPrimitiveObject::new(
            "DV_COUNT",
            MultiplicityInterval::MANDATORY,
            CPrimitive::Integer {
                list: Vec::new(),
                range: None,
            },
        );
        assert!(leaf.assumed_value().is_none());
        let defaulted = leaf.with_assumed_value(PrimitiveValue::Integer(0));
        assert_eq!(defaulted.assumed_value(), Some(&PrimitiveValue::Integer(0)));
    }

    #[test]
    fn an_assumed_value_of_the_wrong_kind_is_carried_rather_than_refused() {
        // `Inv_valid_assumed_value` is deliberately not checked (see
        // `with_assumed_value`'s own doc comment) — a `Boolean` assumed value
        // attached to a `C_INTEGER` constraint is accepted exactly as given,
        // the same way an unmatched `C_STRING` pattern is carried rather than
        // rejected.
        let leaf = CPrimitiveObject::new(
            "DV_COUNT",
            MultiplicityInterval::MANDATORY,
            CPrimitive::Integer {
                list: Vec::new(),
                range: None,
            },
        )
        .with_assumed_value(PrimitiveValue::Boolean(true));
        assert_eq!(leaf.assumed_value(), Some(&PrimitiveValue::Boolean(true)));
    }

    #[test]
    fn an_assumed_value_round_trips_through_canonical_json_and_omits_when_absent() {
        let with_value = CPrimitiveObject::new(
            "DV_TEXT",
            MultiplicityInterval::MANDATORY,
            CPrimitive::String {
                list: Vec::new(),
                pattern: None,
            },
        )
        .with_assumed_value(PrimitiveValue::Text("unknown".to_owned()));
        let json = serde_json::to_value(&with_value).unwrap();
        assert_eq!(json["assumed_value"], "unknown");
        let back: CPrimitiveObject = serde_json::from_value(json).unwrap();
        assert_eq!(back, with_value);

        let without_value = CPrimitiveObject::new(
            "DV_TEXT",
            MultiplicityInterval::MANDATORY,
            CPrimitive::String {
                list: Vec::new(),
                pattern: None,
            },
        );
        let json = serde_json::to_value(&without_value).unwrap();
        assert!(
            !json.as_object().unwrap().contains_key("assumed_value"),
            "an absent assumed_value was written as null instead of omitted"
        );
    }

    #[test]
    fn an_untagged_assumed_value_distinguishes_integer_from_real_by_json_shape() {
        // Order matters for `#[serde(untagged)]`: `Integer` is declared before
        // `Real`, so a whole-number JSON literal like `5` reads as
        // `PrimitiveValue::Integer`, and only a fractional one falls through
        // to `PrimitiveValue::Real`.
        let whole: PrimitiveValue = serde_json::from_str("5").unwrap();
        assert_eq!(whole, PrimitiveValue::Integer(5));
        let fractional: PrimitiveValue = serde_json::from_str("5.5").unwrap();
        assert_eq!(fractional, PrimitiveValue::Real("5.5".parse().unwrap()));
    }

    #[test]
    fn both_node_id_syntaxes_are_recognised_and_kept_apart() {
        assert_eq!(NodeIdSyntax::of("id1"), Some(NodeIdSyntax::Adl2));
        assert_eq!(NodeIdSyntax::of("at0000"), Some(NodeIdSyntax::Adl14));
        assert_eq!(NodeIdSyntax::specialisation_depth("id1.1.2"), 2);
        assert_eq!(NodeIdSyntax::specialisation_depth("at0004"), 0);
    }

    fn quantity_units_magnitude_row(units: &str, low: f64, high: f64) -> CPrimitiveTuple {
        CPrimitiveTuple::new(vec![
            CPrimitiveObject::new(
                "String",
                MultiplicityInterval::MANDATORY,
                CPrimitive::String {
                    list: vec![units.to_owned()],
                    pattern: None,
                },
            ),
            CPrimitiveObject::new(
                "Real",
                MultiplicityInterval::MANDATORY,
                CPrimitive::Real {
                    list: Vec::new(),
                    range: Some(
                        Interval::closed(
                            low.to_string().parse().unwrap(),
                            high.to_string().parse().unwrap(),
                        )
                        .unwrap(),
                    ),
                },
            ),
        ])
        .unwrap()
    }

    /// The `{units, magnitude}` example AOM2's own second-order-constraints
    /// section uses: `"deg F"` only with `32.0..212.0`, `"deg C"` only with
    /// `0.0..100.0`, never mixed — the reason a tuple exists at all rather
    /// than two independent `C_PRIMITIVE_OBJECT` lists.
    #[test]
    fn a_units_magnitude_tuple_pairs_each_unit_with_its_own_range() {
        let tuple = CAttributeTuple::new(
            vec![
                CAttribute::single("units", MultiplicityInterval::MANDATORY, Vec::new()).unwrap(),
                CAttribute::single("magnitude", MultiplicityInterval::MANDATORY, Vec::new())
                    .unwrap(),
            ],
            vec![
                quantity_units_magnitude_row("deg F", 32.0, 212.0),
                quantity_units_magnitude_row("deg C", 0.0, 100.0),
            ],
        )
        .unwrap();
        assert_eq!(tuple.members().len(), 2);
        assert_eq!(tuple.tuples().len(), 2);
    }

    #[test]
    fn a_tuple_row_with_the_wrong_arity_is_refused() {
        let two_members = vec![
            CAttribute::single("units", MultiplicityInterval::MANDATORY, Vec::new()).unwrap(),
            CAttribute::single("magnitude", MultiplicityInterval::MANDATORY, Vec::new()).unwrap(),
        ];
        let one_value_row = CPrimitiveTuple::new(vec![CPrimitiveObject::new(
            "String",
            MultiplicityInterval::MANDATORY,
            CPrimitive::String {
                list: vec!["deg F".to_owned()],
                pattern: None,
            },
        )])
        .unwrap();
        let err = CAttributeTuple::new(two_members, vec![one_value_row]).unwrap_err();
        assert_eq!(
            err.reason,
            "a tuple row's arity does not match the number of co-varying attributes"
        );
    }

    #[test]
    fn a_primitive_tuple_with_no_members_is_refused() {
        // `C_PRIMITIVE_TUPLE.members` is `1..1` in AOM2 — mandatory — unlike
        // `C_ATTRIBUTE_TUPLE`'s own `members` and `tuples`, both `0..1`.
        let err = CPrimitiveTuple::new(Vec::new()).unwrap_err();
        assert_eq!(err.reason, "empty members");
    }

    #[test]
    fn an_attribute_tuple_with_no_rows_is_accepted() {
        // `tuples` is `0..1` — a tuple constraint naming its co-varying
        // attributes but no permitted combinations yet is unusual, not
        // invalid, and nothing in AOM2 requires at least one row.
        let tuple = CAttributeTuple::new(
            vec![CAttribute::single("units", MultiplicityInterval::MANDATORY, Vec::new()).unwrap()],
            Vec::new(),
        )
        .unwrap();
        assert!(tuple.tuples().is_empty());
    }

    #[test]
    fn attribute_tuples_are_absent_by_default_and_attached_with_the_builder() {
        let bare = CComplexObject::new(
            "DV_QUANTITY",
            Some("id14".to_owned()),
            MultiplicityInterval::MANDATORY,
            Vec::new(),
        )
        .unwrap();
        assert!(bare.attribute_tuples().is_empty());
        assert!(CObject::Complex(bare.clone()).attribute_tuples().is_empty());

        let tuple = CAttributeTuple::new(
            vec![
                CAttribute::single("units", MultiplicityInterval::MANDATORY, Vec::new()).unwrap(),
                CAttribute::single("magnitude", MultiplicityInterval::MANDATORY, Vec::new())
                    .unwrap(),
            ],
            vec![quantity_units_magnitude_row("deg C", 0.0, 100.0)],
        )
        .unwrap();
        let with_tuple = bare.with_attribute_tuples(vec![tuple.clone()]);
        assert_eq!(with_tuple.attribute_tuples().len(), 1);
        assert_eq!(with_tuple.attribute_tuples()[0], tuple);
        // Reached the same way through `CObject`, which is what
        // `am::validate::walk_complex` actually calls.
        let wrapped = CObject::Complex(with_tuple.clone());
        assert_eq!(wrapped.attribute_tuples().len(), 1);
        assert_eq!(wrapped.attribute_tuples()[0], tuple);
    }

    #[test]
    fn an_attribute_tuple_round_trips_through_canonical_json() {
        let complex = CComplexObject::new(
            "DV_QUANTITY",
            Some("id14".to_owned()),
            MultiplicityInterval::MANDATORY,
            Vec::new(),
        )
        .unwrap()
        .with_attribute_tuples(vec![
            CAttributeTuple::new(
                vec![
                    CAttribute::single("units", MultiplicityInterval::MANDATORY, Vec::new())
                        .unwrap(),
                    CAttribute::single("magnitude", MultiplicityInterval::MANDATORY, Vec::new())
                        .unwrap(),
                ],
                vec![quantity_units_magnitude_row("deg C", 0.0, 100.0)],
            )
            .unwrap(),
        ]);
        let json = serde_json::to_value(&complex).unwrap();
        let back: CComplexObject = serde_json::from_value(json).unwrap();
        assert_eq!(back, complex);
    }

    #[test]
    fn a_complex_object_serialised_before_attribute_tuples_existed_still_deserialises() {
        // `#[serde(default)]` on `attribute_tuples`: an archetype JSON written
        // by an earlier version of this crate, or by anything else emitting
        // AOM2 JSON without this field, must still read.
        let json = serde_json::json!({
            "rm_type_name": "OBSERVATION",
            "node_id": "id1",
            "occurrences": { "lower": 1, "upper": 1 },
            "attributes": [],
        });
        let back: CComplexObject = serde_json::from_value(json).unwrap();
        assert!(back.attribute_tuples().is_empty());
    }

    #[test]
    fn only_required_reports_that_the_constraint_actually_binds() {
        assert!(ConstraintStatus::Required.is_required());
        assert!(!ConstraintStatus::Extensible.is_required());
        assert!(!ConstraintStatus::Preferred.is_required());
        assert!(!ConstraintStatus::Example.is_required());
    }

    #[test]
    fn constraint_status_round_trips_through_canonical_json_and_omits_when_absent() {
        let with_status = CPrimitive::TerminologyCode {
            constraint: Some("ac0001".to_owned()),
            code_list: Vec::new(),
            constraint_status: Some(ConstraintStatus::Extensible),
        };
        let json = serde_json::to_value(&with_status).unwrap();
        assert_eq!(json["constraint_status"], "Extensible");
        let back: CPrimitive = serde_json::from_value(json).unwrap();
        assert_eq!(back, with_status);

        let without_status = CPrimitive::TerminologyCode {
            constraint: Some("ac0001".to_owned()),
            code_list: Vec::new(),
            constraint_status: None,
        };
        let json = serde_json::to_value(&without_status).unwrap();
        assert!(
            !json.as_object().unwrap().contains_key("constraint_status"),
            "an absent constraint_status was written as null instead of omitted"
        );
    }

    #[test]
    fn a_terminology_code_written_before_constraint_status_existed_still_deserialises() {
        // `#[serde(default)]`: JSON emitted by an earlier version of this
        // crate had no `constraint_status` key at all.
        let json = serde_json::json!({
            "_type": "C_TERMINOLOGY_CODE",
            "constraint": "ac0001",
            "code_list": [],
        });
        let back: CPrimitive = serde_json::from_value(json).unwrap();
        assert_eq!(
            back,
            CPrimitive::TerminologyCode {
                constraint: Some("ac0001".to_owned()),
                code_list: Vec::new(),
                constraint_status: None,
            }
        );
    }
}
