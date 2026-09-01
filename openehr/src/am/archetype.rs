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
//! | `VASID` — the `specialise` clause names the immediate parent | **no** | needs the parent artefact, which needs retrieval (`K15.24`) |
//! | `VACSD` — concept depth is one greater than the parent's | **no** | as above |
//!
//! The two unchecked rules are stated rather than omitted, because an
//! unenforced rule that nobody wrote down reads as an enforced one (`C0.9`).

use crate::am::{
    ArchetypeTerminology, CComplexObject, CObject, MultiplicityInterval, NodeIdSyntax, RmOverlay,
};
use crate::base::ArchetypeId;
use crate::error::ParseError;
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
///         MultiplicityInterval::MANDATORY, Vec::new()).unwrap(),
/// );
/// let definition = CComplexObject::new(
///     "OBSERVATION",
///     Some("id1".to_owned()),
///     MultiplicityInterval::MANDATORY,
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
    archetype_id: ArchetypeId,
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
        archetype_id: ArchetypeId,
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

    /// The archetype identifier.
    #[must_use]
    pub const fn archetype_id(&self) -> &ArchetypeId {
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
        if self.definition.rm_type_name() != self.archetype_id.rm_entity() {
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

/// Every `ac`-code named by a terminology constraint in the tree.
fn terminology_constraints(attributes: &[crate::am::CAttribute]) -> Vec<String> {
    let mut out = Vec::new();
    for attribute in attributes {
        for child in attribute.children() {
            if let CObject::Primitive(primitive) = child
                && let crate::am::CPrimitive::TerminologyCode {
                    constraint: Some(ac_code),
                    ..
                } = primitive.constraint()
            {
                out.push(ac_code.clone());
            }
            out.extend(terminology_constraints(child.attributes()));
        }
    }
    out
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
        VisibilityType,
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
            ROOT_OCCURRENCES,
            vec![CAttribute::single("data", MultiplicityInterval::MANDATORY, children).unwrap()],
        )
        .unwrap()
    }

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
    fn a_definition_constraining_the_wrong_rm_class_is_refused() {
        let definition = CComplexObject::new(
            "EVALUATION",
            Some("id1".to_owned()),
            ROOT_OCCURRENCES,
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
            MultiplicityInterval::MANDATORY,
            CPrimitive::TerminologyCode {
                constraint: Some("ac0001".to_owned()),
                code_list: Vec::new(),
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
