//! `ARCHETYPE`: the artefact itself.
//!
//! An archetype is an identifier, a constraint tree, and the terminology that
//! says what the tree's codes mean. This type holds those three and checks the
//! AOM2 validity conditions that can be decided from them alone.
//!
//! # What is checked here, and what needs something this crate does not have
//!
//! | AOM2 rule | Checked | Why |
//! | --- | --- | --- |
//! | `VARDT` — definition type matches the identifier | yes | both are in hand |
//! | `VATDF` — every `at`/`id`-code in the definition is defined | yes | the terminology is in hand |
//! | `VACDF` — every `ac`-code names a value set | yes | as above |
//! | `VATCD` — code specialisation depth within the archetype's own | yes | derived from the identifier |
//! | `Inv_valid_assumed_value` — a `C_PRIMITIVE_OBJECT`'s `assumed_value` conforms to its own `constraint` | yes | needs the terminology, for a `C_TERMINOLOGY_CODE` naming an `ac`-code — `A-48`'s own residual, closed here rather than at `CPrimitiveObject::with_assumed_value`, which builds a node in isolation |
//! | `VASID` — the `specialise` clause names the immediate parent | **no** | needs the parent artefact, which needs retrieval (`K15.24`) |
//! | `VACSD` — concept depth is one greater than the parent's | **no** | as above |
//!
//! The two unchecked rules are stated rather than omitted, because an
//! unenforced rule that nobody wrote down reads as an enforced one (`C0.9`).

use crate::am::constraint::is_c_string_pattern;
use crate::am::{
    ArchetypeHrid, ArchetypeTerminology, CComplexObject, CObject, CPrimitive, MultiplicityInterval,
    NodeIdSyntax, PrimitiveValue, RmOverlay,
};
use crate::base::{ArchetypeId, Date, DateTime, Duration, Time};
use crate::error::ParseError;
use core::cmp::Ordering;
use core::str::FromStr;
use serde::{Deserialize, Serialize};

/// A parsed, in-memory archetype.
///
/// ```
/// use openehr::am::{
///     Archetype, ArchetypeTerminology, CAttribute, CComplexObject, CObject,
///     MultiplicityInterval, TermDefinition,
/// };
/// use std::collections::BTreeMap;
///
/// let mut terms = BTreeMap::new();
/// terms.insert("id1".to_owned(), TermDefinition::new("Blood pressure", None).unwrap());
/// terms.insert("at0004".to_owned(), TermDefinition::new("Systolic", None).unwrap());
///
/// let systolic = CObject::Complex(
///     CComplexObject::new("ELEMENT", Some("at0004".to_owned()),
///         Some(MultiplicityInterval::MANDATORY), Vec::new()).unwrap(),
/// );
/// let definition = CComplexObject::new(
///     "OBSERVATION",
///     Some("id1".to_owned()),
///     Some(MultiplicityInterval::MANDATORY),
///     vec![CAttribute::single("data", MultiplicityInterval::MANDATORY, vec![systolic]).unwrap()],
/// ).unwrap();
///
/// let archetype = Archetype::new(
///     "openEHR-EHR-OBSERVATION.blood_pressure.v2".parse().unwrap(),
///     definition,
///     ArchetypeTerminology::new("en", terms).unwrap(),
/// ).unwrap();
///
/// assert_eq!(archetype.rm_type_name(), "OBSERVATION");
/// assert_eq!(archetype.specialisation_depth(), 0);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// `archetype_id` and `parent_archetype_id` repeat the struct's name because
// AOM2 calls the attributes exactly that, and the serialised form has to match
// what an artefact carries. Renaming them to read better in Rust would break
// the correspondence with the class definition, which is the same trade
// `LINK.link_type` and `OBJECT_ID` already make here.
#[allow(clippy::struct_field_names)]
pub struct Archetype {
    archetype_id: ArchetypeHrid,
    parent_archetype_id: Option<ArchetypeId>,
    /// The AM release the artefact declares. Carried, not enforced (`K15.2`),
    /// on the same terms `S1.16` sets for `ARCHETYPED.rm_version`.
    adl_version: Option<String>,
    rm_release: Option<String>,
    is_template: bool,
    definition: CComplexObject,
    terminology: ArchetypeTerminology,
    /// Authoring-tool visibility/aliasing statements for RM attributes
    /// outside the constrained structure. Not read by [`crate::am::validate`]
    /// — see [`RmOverlay`]'s own module documentation for why.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rm_overlay: Option<RmOverlay>,
}

impl Archetype {
    /// Builds an archetype, checking the validity conditions listed in the
    /// module header.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] naming the AOM2 validity code that failed:
    ///
    /// - `VARDT` — the definition's RM type name is not the class the
    ///   identifier says the archetype constrains.
    /// - `VATDF` — the definition uses a node identifier the terminology does
    ///   not define.
    /// - `VACDF` — a `C_TERMINOLOGY_CODE` names an `ac`-code with no value set.
    /// - `VATCD` — a code is specialised more deeply than the archetype is.
    pub fn new(
        archetype_id: ArchetypeHrid,
        definition: CComplexObject,
        terminology: ArchetypeTerminology,
    ) -> Result<Self, ParseError> {
        let archetype = Self {
            archetype_id,
            parent_archetype_id: None,
            adl_version: None,
            rm_release: None,
            is_template: false,
            definition,
            terminology,
            rm_overlay: None,
        };
        archetype.check()?;
        Ok(archetype)
    }

    /// Records the parent this archetype specialises.
    ///
    /// **`VASID` and `VACSD` are not checked**: both compare this artefact with
    /// the parent artefact, and retrieving it is `K15.24`, which does not
    /// exist. The identifier is recorded so that a later flattening step has
    /// something to resolve, and so that a reader can see the claim.
    #[must_use]
    pub fn specialising(mut self, parent: ArchetypeId) -> Self {
        self.parent_archetype_id = Some(parent);
        self
    }

    /// Records the ADL and RM versions the artefact declared.
    #[must_use]
    pub fn with_versions(mut self, adl_version: Option<String>, rm_release: Option<String>) -> Self {
        self.adl_version = adl_version;
        self.rm_release = rm_release;
        self
    }

    /// Marks this artefact as a template rather than a plain archetype.
    ///
    /// The flag is carried; **expanding** a template is `K15.14` and is not
    /// implemented, so nothing here treats a template differently from the
    /// archetype it is.
    #[must_use]
    pub const fn as_template(mut self) -> Self {
        self.is_template = true;
        self
    }

    /// Attaches authoring-tool visibility/aliasing statements
    /// (`org.openehr.am.aom2.archetype.adoc`'s `rm_overlay`). See
    /// [`RmOverlay`]'s own module documentation for what reads it — nothing
    /// in this crate's own validation does.
    #[must_use]
    pub fn with_rm_overlay(mut self, rm_overlay: RmOverlay) -> Self {
        self.rm_overlay = Some(rm_overlay);
        self
    }

    /// The archetype identifier — `ARCHETYPE.archetype_id: ARCHETYPE_HRID`
    /// (`org.openehr.am.aom2.archetype.adoc`), the archetype's own declared
    /// identity, distinct from [`crate::base::ArchetypeId`], the narrower
    /// runtime form `ARCHETYPED.archetype_id` carries on Reference Model
    /// data (see [`ArchetypeHrid`]'s own module documentation for exactly
    /// what separates the two).
    #[must_use]
    pub const fn archetype_id(&self) -> &ArchetypeHrid {
        &self.archetype_id
    }

    /// The parent this archetype specialises, if it declared one.
    #[must_use]
    pub const fn parent_archetype_id(&self) -> Option<&ArchetypeId> {
        self.parent_archetype_id.as_ref()
    }

    /// The RM class this archetype constrains.
    #[must_use]
    pub fn rm_type_name(&self) -> &str {
        self.definition.rm_type_name()
    }

    /// The constraint tree.
    #[must_use]
    pub const fn definition(&self) -> &CComplexObject {
        &self.definition
    }

    /// The terminology.
    #[must_use]
    pub const fn terminology(&self) -> &ArchetypeTerminology {
        &self.terminology
    }

    /// Whether the artefact declared itself a template.
    #[must_use]
    pub const fn is_template(&self) -> bool {
        self.is_template
    }

    /// The authoring-tool visibility overlay, if [`Self::with_rm_overlay`]
    /// attached one.
    #[must_use]
    pub const fn rm_overlay(&self) -> Option<&RmOverlay> {
        self.rm_overlay.as_ref()
    }

    /// How deep in a specialisation hierarchy this archetype sits, derived from
    /// its identifier: `blood_pressure` is 0, `blood_pressure-ambulatory` is 1.
    #[must_use]
    pub fn specialisation_depth(&self) -> usize {
        self.archetype_id.specialisations().count()
    }

    /// Every node identifier the definition uses, in document order.
    #[must_use]
    pub fn node_ids(&self) -> Vec<&str> {
        let mut out = Vec::new();
        if let Some(id) = self.definition.node_id() {
            out.push(id);
        }
        collect_from_attributes(self.definition.attributes(), &mut out);
        out
    }

    /// Re-runs the construction-time checks.
    ///
    /// # Errors
    ///
    /// As [`Archetype::new`]. This exists because `Deserialize` writes fields
    /// straight in and never calls a constructor (`L10.1a`): an archetype that
    /// arrived as JSON has passed no gate until this is called, exactly as an
    /// RM object has passed none until [`crate::validation::Validate::validate`]
    /// is.
    pub fn check(&self) -> Result<(), ParseError> {
        // VARDT: the outer type name is the class the identifier names.
        if self.definition.rm_type_name() != self.archetype_id.rm_class() {
            return Err(ParseError::invariant("ARCHETYPE", "VARDT"));
        }

        let depth = self.specialisation_depth();
        for code in self.node_ids() {
            // VATDF: every code the definition uses is defined.
            if !self.terminology.defines(code) {
                return Err(ParseError::new("ARCHETYPE", "VATDF", code));
            }
            // VATCD: a code may not be specialised more deeply than its archetype.
            if NodeIdSyntax::specialisation_depth(code) > depth {
                return Err(ParseError::new("ARCHETYPE", "VATCD", code));
            }
        }

        // VACDF: every ac-code a terminology constraint names has a value set.
        for ac_code in terminology_constraints(self.definition.attributes()) {
            if self.terminology.value_set(&ac_code).is_none() {
                return Err(ParseError::new("ARCHETYPE", "VACDF", &ac_code));
            }
        }

        // Inv_valid_assumed_value: a primitive object's assumed_value, if it
        // has one, must conform to its own constraint.
        let mut mismatches = Vec::new();
        assumed_value_mismatches(self.definition.attributes(), &self.terminology, &mut mismatches);
        if let Some(node_id) = mismatches.first() {
            return Err(ParseError::new("ARCHETYPE", "Inv_valid_assumed_value", node_id));
        }

        Ok(())
    }
}

/// Walks attributes depth-first, collecting node identifiers.
fn collect_from_attributes<'a>(attributes: &'a [crate::am::CAttribute], out: &mut Vec<&'a str>) {
    for attribute in attributes {
        for child in attribute.children() {
            if let Some(id) = child.node_id() {
                out.push(id);
            }
            collect_from_attributes(child.attributes(), out);
        }
    }
}

/// Every `ac`-code named by a terminology constraint in the tree. `VACDF`
/// governs `ac`-codes specifically — "every `ac`-code a terminology
/// constraint names has a value set" — and `constraint` may equally hold a
/// plain `at`-code (an exact required value, needing no value set at all)
/// since `A-51`'s second pass, so this filters on AOM2's own leader
/// convention rather than treating every non-empty `constraint` as one.
fn terminology_constraints(attributes: &[crate::am::CAttribute]) -> Vec<String> {
    let mut out = Vec::new();
    for attribute in attributes {
        for child in attribute.children() {
            if let CObject::Primitive(primitive) = child
                && let crate::am::CPrimitive::TerminologyCode {
                    constraint: Some(code),
                    ..
                } = primitive.constraint()
                && code.starts_with("ac")
            {
                out.push(code.clone());
            }
            out.extend(terminology_constraints(child.attributes()));
        }
    }
    out
}

/// The node identifier of every primitive object in the tree whose
/// `assumed_value` does not conform to its own `constraint` —
/// `Inv_valid_assumed_value`. `"<unidentified>"` stands in for a node with
/// no `node_id` of its own (a hand-built object with no `with_node_id` call;
/// AOM2 requires one, `A-46`'s own residual, so this only happens with a
/// deliberately malformed in-memory tree).
fn assumed_value_mismatches<'a>(
    attributes: &'a [crate::am::CAttribute],
    terminology: &ArchetypeTerminology,
    out: &mut Vec<&'a str>,
) {
    for attribute in attributes {
        for child in attribute.children() {
            if let CObject::Primitive(primitive) = child
                && let Some(value) = primitive.assumed_value()
                && !assumed_value_conforms(primitive.constraint(), value, terminology)
            {
                out.push(primitive.node_id().unwrap_or("<unidentified>"));
            }
            assumed_value_mismatches(child.attributes(), terminology, out);
        }
    }
}

/// Whether `value` satisfies `constraint` — AOM2's own `C_PRIMITIVE_OBJECT
/// .valid_value`, the function `Inv_valid_assumed_value` calls
/// (`org.openehr.am.aom2.c_primitive_object.adoc`). `terminology` matters
/// only for a `C_TERMINOLOGY_CODE` naming an `ac`-code; every other kind
/// decides on its own.
///
/// A kind mismatch — a `Boolean` value against a `C_INTEGER` constraint,
/// say — does not conform. `PrimitiveValue::Text` stands in for `C_STRING`,
/// `C_DATE`, `C_TIME`, `C_DATE_TIME`, `C_DURATION`, and `C_TERMINOLOGY_CODE`
/// alike ([`PrimitiveValue`]'s own module documentation), so which of those
/// five a given `Text` means is decided here by which `CPrimitive` variant
/// it is paired with, the same way [`crate::am::validate`] decides for RM
/// data. `CPrimitive::Unsupported` is excluded rather than treated as a pass
/// or a failure — this crate cannot interpret it, so neither claim is
/// verified, the same reasoning that leaves `VASID`/`VACSD` unchecked above
/// rather than guessed.
fn assumed_value_conforms(
    constraint: &CPrimitive,
    value: &PrimitiveValue,
    terminology: &ArchetypeTerminology,
) -> bool {
    match (constraint, value) {
        (
            CPrimitive::Boolean {
                allow_true,
                allow_false,
            },
            PrimitiveValue::Boolean(b),
        ) => {
            if *b {
                *allow_true
            } else {
                *allow_false
            }
        }
        (CPrimitive::String { list }, PrimitiveValue::Text(s)) => {
            // A regex element is carried, not evaluated — the same
            // "unchecked, not silently passed" position `am::validate`
            // takes, but this function returns a plain `bool` with no
            // unchecked outcome to return, so a `list` naming only regex
            // elements is treated as unconstrained rather than as a
            // literal-match failure (`is_c_string_pattern`'s own callers).
            let literals = list.iter().filter(|item| !is_c_string_pattern(item));
            let mut literals = literals.peekable();
            literals.peek().is_none() || literals.any(|item| item == s)
        }
        (CPrimitive::Integer { list, range }, PrimitiveValue::Integer(n)) => {
            (list.is_empty() || list.contains(n)) && range.as_ref().is_none_or(|r| r.contains(n))
        }
        (CPrimitive::Real { list, range }, PrimitiveValue::Real(r)) => {
            (list.is_empty()
                || list
                    .iter()
                    .any(|item| item.semantic_cmp(r) == Some(Ordering::Equal)))
                && range.as_ref().is_none_or(|iv| iv.contains(r))
        }
        (CPrimitive::TerminologyCode { constraint, .. }, PrimitiveValue::Text(code)) => {
            match constraint.as_deref() {
                None | Some("") => true,
                Some(c) if c.starts_with("ac") => {
                    terminology.value_set(c).is_some_and(|set| set.contains(code))
                }
                Some(c) => code == c,
            }
        }
        (CPrimitive::Date { range, .. }, PrimitiveValue::Text(s)) => {
            Date::from_str(s).is_ok_and(|d| range.is_empty() || range.iter().any(|r| r.contains(&d)))
        }
        (CPrimitive::Time { range, .. }, PrimitiveValue::Text(s)) => {
            Time::from_str(s).is_ok_and(|t| range.is_empty() || range.iter().any(|r| r.contains(&t)))
        }
        (CPrimitive::DateTime { range, .. }, PrimitiveValue::Text(s)) => {
            DateTime::from_str(s).is_ok_and(|dt| range.is_empty() || range.iter().any(|r| r.contains(&dt)))
        }
        (CPrimitive::Duration { range, .. }, PrimitiveValue::Text(s)) => {
            Duration::from_str(s).is_ok_and(|d| range.is_empty() || range.iter().any(|r| r.contains(&d)))
        }
        (CPrimitive::Unsupported { .. }, _) => true,
        _ => false,
    }
}

/// The occurrences of the definition root, which AOM2 fixes at exactly one.
///
/// Exposed as a constant rather than written inline at the two places that need
/// it, because "the root occurs once" is a rule and a literal `1..1` in two
/// files is a rule stated twice (`W0.1`).
pub const ROOT_OCCURRENCES: MultiplicityInterval = MultiplicityInterval::MANDATORY;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::am::{
        CAttribute, CPrimitive, CPrimitiveObject, RmAttributeVisibility, RmOverlay, TermDefinition,
        VersionStatus, VisibilityType,
    };
    use std::collections::{BTreeMap, BTreeSet};

    fn terms(codes: &[&str]) -> BTreeMap<String, TermDefinition> {
        codes
            .iter()
            .map(|code| {
                (
                    (*code).to_owned(),
                    TermDefinition::new(format!("term {code}"), None).unwrap(),
                )
            })
            .collect()
    }

    fn observation(node_ids: &[&str], children: Vec<CObject>) -> CComplexObject {
        let _ = node_ids;
        CComplexObject::new(
            "OBSERVATION",
            Some("id1".to_owned()),
            Some(ROOT_OCCURRENCES),
            vec![CAttribute::single("data", MultiplicityInterval::MANDATORY, children).unwrap()],
        )
        .unwrap()
    }

    fn element(node_id: &str) -> CObject {
        CObject::Complex(
            CComplexObject::new(
                "ELEMENT",
                Some(node_id.to_owned()),
                Some(MultiplicityInterval::MANDATORY),
                Vec::new(),
            )
            .unwrap(),
        )
    }

    #[test]
    fn a_definition_constraining_the_wrong_rm_class_is_refused() {
        let definition = CComplexObject::new(
            "EVALUATION",
            Some("id1".to_owned()),
            Some(ROOT_OCCURRENCES),
            Vec::new(),
        )
        .unwrap();
        let err = Archetype::new(
            "openEHR-EHR-OBSERVATION.blood_pressure.v2".parse().unwrap(),
            definition,
            ArchetypeTerminology::new("en", terms(&["id1"])).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err.reason, "VARDT");
    }

    #[test]
    fn a_node_the_terminology_does_not_define_is_refused() {
        let err = Archetype::new(
            "openEHR-EHR-OBSERVATION.blood_pressure.v2".parse().unwrap(),
            observation(&[], vec![element("at0004")]),
            ArchetypeTerminology::new("en", terms(&["id1"])).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err.reason, "VATDF");
        assert_eq!(err.input, "at0004");
    }

    /// `A-69`: `Archetype.archetype_id` is `ArchetypeHrid`
    /// (`ARCHETYPE.archetype_id: ARCHETYPE_HRID`), not `base::ArchetypeId`
    /// — so a namespaced or prerelease-suffixed identifier, which
    /// `ArchetypeId::from_str` refuses (`A-49`'s own finding), is now
    /// something `Archetype::new` can actually hold, not merely something
    /// this crate's other readers can lex and then have nowhere to put.
    #[test]
    fn a_namespaced_and_a_prerelease_archetype_id_are_held_not_just_lexed() {
        let namespaced = Archetype::new(
            "acme.health::openEHR-EHR-OBSERVATION.blood_pressure.v1"
                .parse()
                .unwrap(),
            observation(&[], Vec::new()),
            ArchetypeTerminology::new("en", terms(&["id1"])).unwrap(),
        )
        .unwrap();
        assert_eq!(namespaced.archetype_id().namespace(), Some("acme.health"));

        let prerelease = Archetype::new(
            "openEHR-EHR-OBSERVATION.blood_pressure.v1.8.2-rc.4"
                .parse()
                .unwrap(),
            observation(&[], Vec::new()),
            ArchetypeTerminology::new("en", terms(&["id1"])).unwrap(),
        )
        .unwrap();
        assert_eq!(prerelease.archetype_id().version_status(), VersionStatus::ReleaseCandidate);
    }

    /// `specialisation_depth` is derived from `ArchetypeHrid::specialisations`
    /// now, not `ArchetypeId::specialisations` — the same `-`-splitting
    /// rule, applied to `concept_id` instead of `domain_concept`.
    #[test]
    fn specialisation_depth_counts_concept_id_segments() {
        let unspecialised = Archetype::new(
            "openEHR-EHR-OBSERVATION.blood_pressure.v2".parse().unwrap(),
            observation(&[], Vec::new()),
            ArchetypeTerminology::new("en", terms(&["id1"])).unwrap(),
        )
        .unwrap();
        assert_eq!(unspecialised.specialisation_depth(), 0);

        let specialised = Archetype::new(
            "openEHR-EHR-OBSERVATION.blood_pressure-ambulatory.v2"
                .parse()
                .unwrap(),
            observation(&[], Vec::new()),
            ArchetypeTerminology::new("en", terms(&["id1"])).unwrap(),
        )
        .unwrap();
        assert_eq!(specialised.specialisation_depth(), 1);
    }

    #[test]
    fn a_code_specialised_deeper_than_its_archetype_is_refused() {
        let err = Archetype::new(
            "openEHR-EHR-OBSERVATION.blood_pressure.v2".parse().unwrap(),
            observation(&[], vec![element("id2.1")]),
            ArchetypeTerminology::new("en", terms(&["id1", "id2.1"])).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err.reason, "VATCD");

        // The same code is legitimate one specialisation level down.
        assert!(
            Archetype::new(
                "openEHR-EHR-OBSERVATION.blood_pressure-ambulatory.v2"
                    .parse()
                    .unwrap(),
                observation(&[], vec![element("id2.1")]),
                ArchetypeTerminology::new("en", terms(&["id1", "id2.1"])).unwrap(),
            )
            .is_ok()
        );
    }

    #[test]
    fn a_terminology_constraint_naming_no_value_set_is_refused() {
        let coded = CObject::Primitive(CPrimitiveObject::new(
            "DV_CODED_TEXT",
            Some(MultiplicityInterval::MANDATORY),
            CPrimitive::TerminologyCode {
                constraint: Some("ac0001".to_owned()),
                constraint_status: None,
            },
        ));
        let terminology = ArchetypeTerminology::new("en", terms(&["id1"])).unwrap();
        let err = Archetype::new(
            "openEHR-EHR-OBSERVATION.blood_pressure.v2".parse().unwrap(),
            observation(&[], vec![coded.clone()]),
            terminology,
        )
        .unwrap_err();
        assert_eq!(err.reason, "VACDF");

        let with_set = ArchetypeTerminology::new("en", terms(&["id1", "at0010"]))
            .unwrap()
            .with_value_set("ac0001", BTreeSet::from(["at0010".to_owned()]))
            .unwrap();
        assert!(
            Archetype::new(
                "openEHR-EHR-OBSERVATION.blood_pressure.v2".parse().unwrap(),
                observation(&[], vec![coded]),
                with_set,
            )
            .is_ok()
        );
    }

    #[test]
    fn an_assumed_value_of_the_matching_kind_and_in_range_is_accepted() {
        let leaf = CObject::Primitive(
            CPrimitiveObject::new(
                "DV_COUNT",
                Some(MultiplicityInterval::MANDATORY),
                CPrimitive::Integer {
                    list: Vec::new(),
                    range: Some(crate::base::Interval::closed(0, 10).unwrap()),
                },
            )
            .with_assumed_value(crate::am::PrimitiveValue::Integer(5)),
        );
        assert!(
            Archetype::new(
                "openEHR-EHR-OBSERVATION.blood_pressure.v2".parse().unwrap(),
                observation(&[], vec![leaf]),
                ArchetypeTerminology::new("en", terms(&["id1"])).unwrap(),
            )
            .is_ok()
        );
    }

    #[test]
    fn an_assumed_value_outside_its_own_range_is_refused() {
        let leaf = CObject::Primitive(
            CPrimitiveObject::new(
                "DV_COUNT",
                Some(MultiplicityInterval::MANDATORY),
                CPrimitive::Integer {
                    list: Vec::new(),
                    range: Some(crate::base::Interval::closed(0, 10).unwrap()),
                },
            )
            .with_assumed_value(crate::am::PrimitiveValue::Integer(50)),
        );
        let err = Archetype::new(
            "openEHR-EHR-OBSERVATION.blood_pressure.v2".parse().unwrap(),
            observation(&[], vec![leaf]),
            ArchetypeTerminology::new("en", terms(&["id1"])).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err.reason, "Inv_valid_assumed_value");
    }

    /// `assumed_value_conforms`'s `Boolean` arm, both ways. The
    /// mutation-testing job (CI run 33775455452) found this arm and the
    /// `String` arm below reachable by no test: only `Integer` and
    /// `TerminologyCode` assumed values were ever built.
    #[test]
    fn a_boolean_assumed_value_is_checked_against_its_allowed_values() {
        let build = |allow_true: bool, allow_false: bool, assumed: bool| {
            let leaf = CPrimitiveObject::new(
                "DV_BOOLEAN",
                Some(MultiplicityInterval::MANDATORY),
                CPrimitive::Boolean { allow_true, allow_false },
            )
            .with_assumed_value(crate::am::PrimitiveValue::Boolean(assumed));
            Archetype::new(
                "openEHR-EHR-OBSERVATION.blood_pressure.v2".parse().unwrap(),
                observation(&[], vec![CObject::Primitive(leaf)]),
                ArchetypeTerminology::new("en", terms(&["id1"])).unwrap(),
            )
        };
        assert!(build(true, false, true).is_ok());
        assert!(build(false, true, false).is_ok());
        assert_eq!(build(true, false, false).unwrap_err().reason, "Inv_valid_assumed_value");
        assert_eq!(build(false, true, true).unwrap_err().reason, "Inv_valid_assumed_value");
    }

    /// The `String` arm: a literal must be in the list, a regex element
    /// neither admits nor refuses (carried, not evaluated), and a list of
    /// regexes alone constrains nothing this function can see.
    #[test]
    fn a_string_assumed_value_is_checked_against_the_literals_and_not_the_regexes() {
        let build = |list: &[&str], assumed: &str| {
            let leaf = CPrimitiveObject::new(
                "DV_TEXT",
                Some(MultiplicityInterval::MANDATORY),
                CPrimitive::String {
                    list: list.iter().map(|s| (*s).to_owned()).collect(),
                },
            )
            .with_assumed_value(crate::am::PrimitiveValue::Text(assumed.to_owned()));
            Archetype::new(
                "openEHR-EHR-OBSERVATION.blood_pressure.v2".parse().unwrap(),
                observation(&[], vec![CObject::Primitive(leaf)]),
                ArchetypeTerminology::new("en", terms(&["id1"])).unwrap(),
            )
        };
        assert!(build(&["a"], "a").is_ok());
        assert_eq!(build(&["a"], "c").unwrap_err().reason, "Inv_valid_assumed_value");
        assert_eq!(build(&["a", "b"], "c").unwrap_err().reason, "Inv_valid_assumed_value");
        assert!(build(&["/x.*/", "a"], "a").is_ok());
        assert_eq!(build(&["/x.*/", "a"], "c").unwrap_err().reason, "Inv_valid_assumed_value");
        assert!(build(&["/x.*/"], "anything").is_ok());
    }

    /// One leaf with the given constraint and assumed value, under the
    /// usual root, for the per-kind tests below.
    fn with_assumed(constraint: CPrimitive, assumed: crate::am::PrimitiveValue) -> Result<Archetype, ParseError> {
        let leaf = CPrimitiveObject::new("DV_ANY", Some(MultiplicityInterval::MANDATORY), constraint)
            .with_assumed_value(assumed);
        Archetype::new(
            "openEHR-EHR-OBSERVATION.blood_pressure.v2".parse().unwrap(),
            observation(&[], vec![CObject::Primitive(leaf)]),
            ArchetypeTerminology::new("en", terms(&["id1", "at0010"])).unwrap(),
        )
    }

    /// `assumed_value_conforms`'s `Real` arm: list and range each apply,
    /// and the list compares by `semantic_cmp`, so `1.50` is in a list
    /// naming `1.5`. Running cargo-mutants over the whole function (not
    /// only CI's changed lines) found this arm and the five temporal ones
    /// below reachable by no test.
    #[test]
    fn a_real_assumed_value_is_checked_against_list_and_range() {
        use crate::am::PrimitiveValue;
        let real = |s: &str| PrimitiveValue::Real(s.parse().unwrap());
        let constraint = |list: &[&str], range: Option<(&str, &str)>| CPrimitive::Real {
            list: list.iter().map(|s| s.parse().unwrap()).collect(),
            range: range.map(|(lo, hi)| {
                crate::base::Interval::closed(
                    lo.parse::<crate::base::Real>().unwrap(),
                    hi.parse::<crate::base::Real>().unwrap(),
                )
                .unwrap()
            }),
        };
        assert!(with_assumed(constraint(&["1.5"], None), real("1.5")).is_ok());
        assert!(with_assumed(constraint(&["1.5"], None), real("1.50")).is_ok());
        assert!(with_assumed(constraint(&["1.5"], None), real("2.5")).is_err());
        assert!(with_assumed(constraint(&[], Some(("0.0", "10.0"))), real("5.0")).is_ok());
        assert!(with_assumed(constraint(&[], Some(("0.0", "10.0"))), real("50.0")).is_err());
        assert!(with_assumed(constraint(&["5.0"], Some(("0.0", "10.0"))), real("5.0")).is_ok());
        assert!(with_assumed(constraint(&["50.0"], Some(("0.0", "10.0"))), real("50.0")).is_err());
    }

    /// The `TerminologyCode` arm's second branch: a constraint that is an
    /// `at`-code, not an `ac`-code, admits exactly that code — no value
    /// set is consulted.
    #[test]
    fn an_at_code_constraints_assumed_value_must_be_that_code() {
        use crate::am::PrimitiveValue;
        let constraint = |c: &str| CPrimitive::TerminologyCode {
            constraint: Some(c.to_owned()),
            constraint_status: None,
        };
        assert!(with_assumed(constraint("at0010"), PrimitiveValue::Text("at0010".to_owned())).is_ok());
        assert!(with_assumed(constraint("at0010"), PrimitiveValue::Text("at0011".to_owned())).is_err());
        assert!(with_assumed(constraint(""), PrimitiveValue::Text("anything".to_owned())).is_ok());
    }

    /// The four temporal arms: the assumed text must parse as the kind,
    /// an empty range constrains nothing, and a non-empty range must
    /// contain it.
    #[test]
    fn a_temporal_assumed_value_must_parse_and_fall_in_a_stated_range() {
        use crate::am::PrimitiveValue;
        let text = |s: &str| PrimitiveValue::Text(s.to_owned());
        let date = |range: &[(&str, &str)]| CPrimitive::Date {
            range: range
                .iter()
                .map(|(lo, hi)| {
                    crate::base::Interval::closed(Date::from_str(lo).unwrap(), Date::from_str(hi).unwrap()).unwrap()
                })
                .collect(),
            pattern: None,
        };
        assert!(with_assumed(date(&[]), text("2024-06-15")).is_ok());
        assert!(with_assumed(date(&[]), text("not-a-date")).is_err());
        assert!(with_assumed(date(&[("2024-01-01", "2024-12-31")]), text("2024-06-15")).is_ok());
        assert!(with_assumed(date(&[("2024-01-01", "2024-12-31")]), text("2025-06-15")).is_err());

        let time = |range: &[(&str, &str)]| CPrimitive::Time {
            range: range
                .iter()
                .map(|(lo, hi)| {
                    crate::base::Interval::closed(Time::from_str(lo).unwrap(), Time::from_str(hi).unwrap()).unwrap()
                })
                .collect(),
            pattern: None,
        };
        assert!(with_assumed(time(&[]), text("12:30:00")).is_ok());
        assert!(with_assumed(time(&[]), text("not-a-time")).is_err());
        assert!(with_assumed(time(&[("09:00:00", "17:00:00")]), text("12:30:00")).is_ok());
        assert!(with_assumed(time(&[("09:00:00", "17:00:00")]), text("23:00:00")).is_err());

        let date_time = |range: &[(&str, &str)]| CPrimitive::DateTime {
            range: range
                .iter()
                .map(|(lo, hi)| {
                    crate::base::Interval::closed(DateTime::from_str(lo).unwrap(), DateTime::from_str(hi).unwrap())
                        .unwrap()
                })
                .collect(),
            pattern: None,
        };
        assert!(with_assumed(date_time(&[]), text("2024-06-15T12:30:00Z")).is_ok());
        assert!(with_assumed(date_time(&[]), text("not-a-date-time")).is_err());
        assert!(
            with_assumed(
                date_time(&[("2024-01-01T00:00:00Z", "2024-12-31T23:59:59Z")]),
                text("2024-06-15T12:30:00Z")
            )
            .is_ok()
        );
        assert!(
            with_assumed(
                date_time(&[("2024-01-01T00:00:00Z", "2024-12-31T23:59:59Z")]),
                text("2025-06-15T12:30:00Z")
            )
            .is_err()
        );

        let duration = |range: &[(&str, &str)]| CPrimitive::Duration {
            range: range
                .iter()
                .map(|(lo, hi)| {
                    crate::base::Interval::closed(Duration::from_str(lo).unwrap(), Duration::from_str(hi).unwrap())
                        .unwrap()
                })
                .collect(),
            pattern: None,
        };
        assert!(with_assumed(duration(&[]), text("PT1H")).is_ok());
        assert!(with_assumed(duration(&[]), text("not-a-duration")).is_err());
        assert!(with_assumed(duration(&[("PT0S", "PT2H")]), text("PT1H")).is_ok());
        assert!(with_assumed(duration(&[("PT0S", "PT2H")]), text("PT5H")).is_err());
    }

    /// `CPrimitiveObject::with_assumed_value`'s own documentation says a
    /// mismatched kind is "accepted exactly as given" — true of that
    /// function alone. `Archetype::new` sees the whole tree and the
    /// terminology besides, and this is where `Inv_valid_assumed_value` is
    /// actually enforced: a `Boolean` value can never conform to a
    /// `C_INTEGER` constraint, and building an archetype that claims it
    /// does is refused here rather than silently carried all the way to a
    /// caller who never suspected a mismatch.
    #[test]
    fn a_kind_mismatched_assumed_value_is_refused_at_the_archetype_not_the_leaf() {
        let leaf = CPrimitiveObject::new(
            "DV_COUNT",
            Some(MultiplicityInterval::MANDATORY),
            CPrimitive::Integer {
                list: Vec::new(),
                range: None,
            },
        )
        .with_assumed_value(crate::am::PrimitiveValue::Boolean(true));
        // The leaf alone still carries it unchecked, as documented.
        assert_eq!(
            leaf.assumed_value(),
            Some(&crate::am::PrimitiveValue::Boolean(true))
        );

        let err = Archetype::new(
            "openEHR-EHR-OBSERVATION.blood_pressure.v2".parse().unwrap(),
            observation(&[], vec![CObject::Primitive(leaf)]),
            ArchetypeTerminology::new("en", terms(&["id1"])).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err.reason, "Inv_valid_assumed_value");
    }

    #[test]
    fn a_terminology_code_assumed_value_is_checked_against_the_value_set() {
        let coded = CPrimitiveObject::new(
            "DV_CODED_TEXT",
            Some(MultiplicityInterval::MANDATORY),
            CPrimitive::TerminologyCode {
                constraint: Some("ac0001".to_owned()),
                constraint_status: None,
            },
        )
        .with_assumed_value(crate::am::PrimitiveValue::Text("at0099".to_owned()));
        let terminology = ArchetypeTerminology::new("en", terms(&["id1", "at0010"]))
            .unwrap()
            .with_value_set("ac0001", BTreeSet::from(["at0010".to_owned()]))
            .unwrap();
        let err = Archetype::new(
            "openEHR-EHR-OBSERVATION.blood_pressure.v2".parse().unwrap(),
            observation(&[], vec![CObject::Primitive(coded.clone())]),
            terminology.clone(),
        )
        .unwrap_err();
        assert_eq!(err.reason, "Inv_valid_assumed_value");

        let in_set = coded.with_assumed_value(crate::am::PrimitiveValue::Text("at0010".to_owned()));
        assert!(
            Archetype::new(
                "openEHR-EHR-OBSERVATION.blood_pressure.v2".parse().unwrap(),
                observation(&[], vec![CObject::Primitive(in_set)]),
                terminology,
            )
            .is_ok()
        );
    }

    /// A `C_UNSUPPORTED` constraint's assumed value is excluded from
    /// `Inv_valid_assumed_value` entirely, not passed and not failed — this
    /// crate cannot interpret the constraint, so it has no basis to claim
    /// either.
    #[test]
    fn an_unsupported_constraints_assumed_value_is_not_checked() {
        let leaf = CObject::Primitive(
            CPrimitiveObject::new(
                "DV_INTERVAL",
                Some(MultiplicityInterval::MANDATORY),
                CPrimitive::Unsupported {
                    rm_type_name: "DV_INTERVAL".to_owned(),
                    source: "<unparsed>".to_owned(),
                },
            )
            .with_assumed_value(crate::am::PrimitiveValue::Boolean(true)),
        );
        assert!(
            Archetype::new(
                "openEHR-EHR-OBSERVATION.blood_pressure.v2".parse().unwrap(),
                observation(&[], vec![leaf]),
                ArchetypeTerminology::new("en", terms(&["id1"])).unwrap(),
            )
            .is_ok()
        );
    }

    #[test]
    fn deserialization_bypasses_the_constructor_and_check_catches_it() {
        let archetype = Archetype::new(
            "openEHR-EHR-OBSERVATION.blood_pressure.v2".parse().unwrap(),
            observation(&[], vec![element("at0004")]),
            ArchetypeTerminology::new("en", terms(&["id1", "at0004"])).unwrap(),
        )
        .unwrap();

        // Take the valid artefact, strip one term definition from its JSON, and
        // read it back: serde writes the fields straight in.
        let mut json: serde_json::Value = serde_json::to_value(&archetype).unwrap();
        json["terminology"]["term_definitions"]["en"]
            .as_object_mut()
            .unwrap()
            .remove("at0004");
        let smuggled: Archetype = serde_json::from_value(json).unwrap();

        assert_eq!(smuggled.check().unwrap_err().reason, "VATDF");
    }

    #[test]
    fn rm_overlay_is_absent_by_default_and_attached_with_the_builder() {
        let bare = Archetype::new(
            "openEHR-EHR-OBSERVATION.blood_pressure.v2".parse().unwrap(),
            observation(&[], Vec::new()),
            ArchetypeTerminology::new("en", terms(&["id1"])).unwrap(),
        )
        .unwrap();
        assert!(bare.rm_overlay().is_none());

        let overlay = RmOverlay::default().with_visibility(
            "protocol",
            RmAttributeVisibility::new(Some(VisibilityType::Hide), None).unwrap(),
        );
        let with_overlay = bare.with_rm_overlay(overlay.clone());
        assert_eq!(with_overlay.rm_overlay(), Some(&overlay));
    }

    #[test]
    fn an_archetype_written_before_rm_overlay_existed_still_deserialises() {
        // `#[serde(default)]` on `rm_overlay`: JSON emitted by an earlier
        // version of this crate had no such key at all.
        let archetype = Archetype::new(
            "openEHR-EHR-OBSERVATION.blood_pressure.v2".parse().unwrap(),
            observation(&[], Vec::new()),
            ArchetypeTerminology::new("en", terms(&["id1"])).unwrap(),
        )
        .unwrap();
        let json = serde_json::to_value(&archetype).unwrap();
        assert!(
            !json.as_object().unwrap().contains_key("rm_overlay"),
            "an absent rm_overlay was written instead of omitted"
        );
        let back: Archetype = serde_json::from_value(json).unwrap();
        assert!(back.rm_overlay().is_none());
    }
}
