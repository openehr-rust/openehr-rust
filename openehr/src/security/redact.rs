//! Redaction: producing a view of a composition with content withheld.
//!
//! # Redaction is not deletion, and openEHR already has the word for it
//!
//! A redacted `ELEMENT` is not removed. It becomes a null element with
//! `272｜masked｜`, which says *there is a value here and you are not being
//! shown it*. That distinction is the whole point:
//!
//! | If a filtered element is… | The reader concludes |
//! | --- | --- |
//! | deleted | the question was never asked |
//! | masked | the answer exists and is withheld |
//!
//! A consent filter that deletes rather than masks turns "the patient has
//! withheld their sexual health history" into "the patient has no sexual health
//! history", and the second is a clinical statement nobody made.
//!
//! # Redaction is a new document, not a mutation
//!
//! The RM types in this crate are immutable after construction — every
//! invariant is checked once, at the boundary, and nothing can subsequently
//! break it. Redaction therefore *produces* a composition rather than editing
//! one, which also means the original is still available to a caller entitled
//! to it, and the two cannot be confused for each other.
//!
//! Internally it goes through canonical JSON, because rebuilding twenty RM
//! classes through their constructors would duplicate the model with no
//! behavioural difference. The cost is that redaction is fallible on a document
//! that does not round-trip; the benefit is that the transformation is one
//! traversal of one recursive structure, which is small enough to read.
//!
//! ```
//! # use openehr::rm::common::{LocatableAttrs, PartyIdentified};
//! # use openehr::rm::data_structures::{Element, ItemTree};
//! # use openehr::rm::data_types::{CodePhrase, DataValue, DvText};
//! # use openehr::rm::ehr::{Composition, EntryAttrs, Evaluation};
//! # use openehr::terminology::composition_category;
//! use openehr::security::redact::{Redactor, RedactionRule};
//!
//! # let attrs = |n: &str, c: &str| LocatableAttrs::named(n, c).unwrap();
//! # let hiv = Element::new(attrs("HIV status", "at0011"),
//! #     DataValue::Text(DvText::new("Positive").unwrap()));
//! # let other = Element::new(attrs("Weight", "at0012"),
//! #     DataValue::Text(DvText::new("70 kg").unwrap()));
//! # let data = ItemTree::new(attrs("tree", "at0001"), vec![hiv.into(), other.into()]);
//! # let composition = Composition::new(
//! #     attrs("Encounter", "openEHR-EHR-COMPOSITION.encounter.v1"),
//! #     composition_category::EVENT,
//! #     PartyIdentified::named("Dr A Nurse").unwrap().into(),
//! #     CodePhrase::new("ISO_639-1", "en").unwrap(),
//! #     CodePhrase::new("ISO_3166-1", "GB").unwrap(),
//! # ).unwrap().with_content(
//! #     Evaluation::new(attrs("Problem", "at0000"),
//! #         EntryAttrs::about_subject(
//! #             CodePhrase::new("ISO_639-1", "en").unwrap(),
//! #             CodePhrase::new("IANA_character-sets", "UTF-8").unwrap()),
//! #         data.into()).into());
//! let redactor = Redactor::new().with_rule(RedactionRule::node_id("at0011"));
//! let filtered = redactor.redact(&composition).unwrap();
//!
//! let json = serde_json::to_string(&filtered).unwrap();
//! assert!(!json.contains("Positive"));   // the value is gone
//! assert!(json.contains("masked"));      // and its absence is stated
//! assert!(json.contains("70 kg"));       // everything else is untouched
//! ```

use crate::rm::ehr::Composition;
use crate::terminology;
use core::fmt;
use serde_json::{Map, Value};

/// Which elements to withhold.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RedactionRule {
    /// Withhold elements whose `archetype_node_id` matches exactly.
    NodeId(String),
    /// Withhold elements whose `name/value` matches exactly.
    Name(String),
    /// Withhold every element under an archetype root with this id.
    ///
    /// The blunt instrument, and the right one for a whole-archetype consent
    /// rule: "do not disclose anything from the sexual health assessment".
    ArchetypeRoot(String),
}

impl RedactionRule {
    /// Withhold by `archetype_node_id`.
    #[must_use]
    pub fn node_id(id: impl Into<String>) -> Self {
        Self::NodeId(id.into())
    }

    /// Withhold by runtime name.
    #[must_use]
    pub fn name(name: impl Into<String>) -> Self {
        Self::Name(name.into())
    }

    /// Withhold everything under an archetype root.
    #[must_use]
    pub fn archetype_root(id: impl Into<String>) -> Self {
        Self::ArchetypeRoot(id.into())
    }
}

/// What went wrong during redaction.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RedactionError {
    /// The composition could not be serialized or read back.
    ///
    /// Carries the underlying `serde_json` error, whose message names a JSON
    /// path and a type — never a clinical value, because the failure is
    /// structural.
    #[error("redaction could not round-trip the composition: {0}")]
    RoundTrip(#[from] serde_json::Error),
}

/// A count of what a redaction pass did.
///
/// Returned alongside the document so that a caller can log *that* redaction
/// happened and how much, without logging *what* was redacted. Both facts are
/// needed — an access log saying "12 elements withheld" is auditable; one
/// saying "HIV status withheld" is a disclosure in the audit trail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RedactionCount {
    /// How many elements were masked.
    pub masked: usize,
    /// How many elements were examined.
    pub examined: usize,
}

impl fmt::Display for RedactionCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} of {} elements masked", self.masked, self.examined)
    }
}

/// Applies redaction rules to a composition.
#[derive(Debug, Clone, Default)]
pub struct Redactor {
    rules: Vec<RedactionRule>,
    reason: Option<String>,
}

impl Redactor {
    /// A redactor with no rules, which withholds nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a rule.
    #[must_use]
    pub fn with_rule(mut self, rule: RedactionRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Records a reason on every masked element, as `null_reason`.
    ///
    /// The reason is written into the document, so it must not name the value
    /// being withheld or the clinical category it belongs to. "Withheld under
    /// patient consent preferences" is safe; "HIV status withheld" defeats the
    /// purpose by disclosing the category in the redacted copy.
    #[must_use]
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// The rules.
    #[must_use]
    pub fn rules(&self) -> &[RedactionRule] {
        &self.rules
    }

    /// Produces a redacted copy of a composition.
    ///
    /// # Errors
    ///
    /// Returns [`RedactionError::RoundTrip`] if the composition cannot be
    /// serialized and read back. **Fails closed**: on error nothing is
    /// returned, so a caller cannot accidentally forward the unredacted
    /// original.
    pub fn redact(&self, composition: &Composition) -> Result<Composition, RedactionError> {
        Ok(self.redact_counting(composition)?.0)
    }

    /// Produces a redacted copy and reports how much was withheld.
    ///
    /// # Errors
    ///
    /// See [`Redactor::redact`].
    pub fn redact_counting(
        &self,
        composition: &Composition,
    ) -> Result<(Composition, RedactionCount), RedactionError> {
        let mut value = serde_json::to_value(composition)?;
        let mut count = RedactionCount::default();
        self.walk(&mut value, false, &mut count);
        let redacted: Composition = serde_json::from_value(value)?;
        Ok((redacted, count))
    }

    /// Walks the JSON tree, masking elements that match.
    ///
    /// `under_redacted_root` carries down the [`RedactionRule::ArchetypeRoot`]
    /// decision: once inside a withheld archetype, every element below is
    /// withheld regardless of its own node id.
    fn walk(&self, value: &mut Value, under_redacted_root: bool, count: &mut RedactionCount) {
        match value {
            Value::Array(items) => {
                for item in items {
                    self.walk(item, under_redacted_root, count);
                }
            }
            Value::Object(map) => {
                let inside = under_redacted_root || self.is_redacted_root(map);
                if is_element(map) {
                    count.examined += 1;
                    if inside || self.matches_element(map) {
                        self.mask(map);
                        count.masked += 1;
                        // Do not descend: the value is gone, and anything it
                        // contained went with it.
                        return;
                    }
                }
                for (_, child) in map.iter_mut() {
                    self.walk(child, inside, count);
                }
            }
            _ => {}
        }
    }

    fn is_redacted_root(&self, map: &Map<String, Value>) -> bool {
        let Some(archetype_id) = map
            .get("archetype_details")
            .and_then(|d| d.get("archetype_id"))
            .and_then(|a| a.get("value").or(Some(a)))
            .and_then(Value::as_str)
        else {
            return false;
        };
        self.rules.iter().any(|r| match r {
            RedactionRule::ArchetypeRoot(id) => id == archetype_id,
            RedactionRule::NodeId(_) | RedactionRule::Name(_) => false,
        })
    }

    fn matches_element(&self, map: &Map<String, Value>) -> bool {
        let node_id = map.get("archetype_node_id").and_then(Value::as_str);
        let name = map
            .get("name")
            .and_then(|n| n.get("value"))
            .and_then(Value::as_str);
        self.rules.iter().any(|r| match r {
            RedactionRule::NodeId(id) => node_id == Some(id.as_str()),
            RedactionRule::Name(n) => name == Some(n.as_str()),
            RedactionRule::ArchetypeRoot(_) => false,
        })
    }

    fn mask(&self, map: &mut Map<String, Value>) {
        map.remove("value");
        map.insert(
            "null_flavour".to_owned(),
            serde_json::json!({
                "_type": "DV_CODED_TEXT",
                "value": "masked",
                "defining_code": {
                    "_type": "CODE_PHRASE",
                    "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "openehr"},
                    "code_string": terminology::null_flavour::MASKED,
                }
            }),
        );
        if let Some(reason) = &self.reason {
            map.insert(
                "null_reason".to_owned(),
                serde_json::json!({"_type": "DV_TEXT", "value": reason}),
            );
        } else {
            map.remove("null_reason");
        }
    }
}

/// Whether a JSON object is an `ELEMENT`.
///
/// Recognised by `_type` where present, and otherwise by the shape openEHR
/// gives `ELEMENT` alone: an `archetype_node_id` plus a `value` or a
/// `null_flavour` and no child-bearing attribute. The `_type` check is first
/// because it is exact; the shape check exists because canonical JSON omits
/// `_type` where the declared type is concrete, and `CLUSTER.items` is declared
/// `List<ITEM>`, which is not.
fn is_element(map: &Map<String, Value>) -> bool {
    match map.get("_type").and_then(Value::as_str) {
        Some("ELEMENT") => return true,
        Some(_) => return false,
        None => {}
    }
    map.contains_key("archetype_node_id")
        && (map.contains_key("value") || map.contains_key("null_flavour"))
        && !map.contains_key("items")
        && !map.contains_key("rows")
        && !map.contains_key("content")
}

/// A wrapper whose [`Display`](fmt::Display) and [`Debug`](fmt::Debug) never
/// reveal what it holds.
///
/// For values that must be carried through code that logs — a request context,
/// an error payload, a struct that someone will eventually `{:?}`. The value is
/// reachable only through [`Sensitive::expose`], which is deliberately ugly to
/// read at a call site.
///
/// ```
/// use openehr::security::redact::Sensitive;
///
/// let mrn = Sensitive::new("943-476-5919".to_string());
/// assert_eq!(format!("{mrn}"), "<redacted>");
/// assert_eq!(format!("{mrn:?}"), "<redacted>");
/// assert_eq!(mrn.expose(), "943-476-5919");
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Sensitive<T>(T);

impl<T> Sensitive<T> {
    /// Wraps a value.
    pub const fn new(value: T) -> Self {
        Self(value)
    }

    /// Reads the value.
    ///
    /// Named `expose` rather than `get` or `inner` so that a reviewer scanning
    /// a diff sees the disclosure.
    pub const fn expose(&self) -> &T {
        &self.0
    }

    /// Unwraps the value.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> fmt::Display for Sensitive<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

impl<T> fmt::Debug for Sensitive<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

impl<T: serde::Serialize> serde::Serialize for Sensitive<T> {
    /// Serializes the **value**, not the placeholder.
    ///
    /// Serialization is how the value reaches storage and an authorised
    /// recipient; redacting there would corrupt the record rather than protect
    /// it. The protection this type offers is against logging, which is where
    /// the accidents happen.
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(s)
    }
}

impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for Sensitive<T> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        T::deserialize(d).map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rm::common::{Archetyped, LocatableAttrs, PartyIdentified};
    use crate::rm::data_structures::{Element, ItemTree};
    use crate::rm::data_types::{CodePhrase, DataValue, DvText};
    use crate::rm::ehr::{Composition, EntryAttrs, Evaluation};
    use crate::validation::Validate as _;

    fn attrs(name: &str, node: &str) -> LocatableAttrs {
        LocatableAttrs::named(name, node).unwrap()
    }

    fn composition() -> Composition {
        let sensitive = Element::new(
            attrs("HIV status", "at0011"),
            DataValue::Text(DvText::new("ZZ-SENSITIVE-9999").unwrap()),
        );
        let ordinary = Element::new(
            attrs("Weight", "at0012"),
            DataValue::Text(DvText::new("ZZ-ORDINARY-1111").unwrap()),
        );
        let data = ItemTree::new(
            attrs("tree", "at0001"),
            vec![sensitive.into(), ordinary.into()],
        );
        let evaluation = Evaluation::new(
            attrs("Problem", "openEHR-EHR-EVALUATION.problem_diagnosis.v1").with_archetype_details(
                Archetyped::new("openEHR-EHR-EVALUATION.problem_diagnosis.v1", "1.1.0").unwrap(),
            ),
            EntryAttrs::about_subject(
                CodePhrase::new("ISO_639-1", "en").unwrap(),
                CodePhrase::new("IANA_character-sets", "UTF-8").unwrap(),
            ),
            data.into(),
        );
        Composition::new(
            attrs("Encounter", "openEHR-EHR-COMPOSITION.encounter.v1").with_archetype_details(
                Archetyped::new("openEHR-EHR-COMPOSITION.encounter.v1", "1.1.0").unwrap(),
            ),
            terminology::composition_category::EVENT,
            PartyIdentified::named("Dr A Nurse").unwrap().into(),
            CodePhrase::new("ISO_639-1", "en").unwrap(),
            CodePhrase::new("ISO_3166-1", "GB").unwrap(),
        )
        .unwrap()
        .with_content(evaluation.into())
    }

    #[test]
    fn a_masked_element_says_the_value_exists() {
        let (redacted, count) = Redactor::new()
            .with_rule(RedactionRule::node_id("at0011"))
            .redact_counting(&composition())
            .unwrap();
        let json = serde_json::to_string(&redacted).unwrap();
        assert!(!json.contains("ZZ-SENSITIVE"), "{json}");
        assert!(json.contains("masked"));
        assert_eq!(count.masked, 1);
        assert_eq!(count.examined, 2);
    }

    #[test]
    fn everything_not_matched_survives_untouched() {
        let redacted = Redactor::new()
            .with_rule(RedactionRule::node_id("at0011"))
            .redact(&composition())
            .unwrap();
        let json = serde_json::to_string(&redacted).unwrap();
        assert!(json.contains("ZZ-ORDINARY-1111"), "{json}");
    }

    #[test]
    fn a_redacted_composition_is_still_valid() {
        // The check that redaction produces a record and not a wreck: the
        // masked element must satisfy ELEMENT's own invariants.
        let redacted = Redactor::new()
            .with_rule(RedactionRule::name("HIV status"))
            .redact(&composition())
            .unwrap();
        let report = redacted.validate();
        assert!(report.is_empty(), "{report}");
    }

    #[test]
    fn an_archetype_root_rule_withholds_everything_under_it() {
        let (redacted, count) = Redactor::new()
            .with_rule(RedactionRule::archetype_root(
                "openEHR-EHR-EVALUATION.problem_diagnosis.v1",
            ))
            .redact_counting(&composition())
            .unwrap();
        assert_eq!(count.masked, 2);
        let json = serde_json::to_string(&redacted).unwrap();
        assert!(!json.contains("ZZ-SENSITIVE"), "{json}");
        assert!(!json.contains("ZZ-ORDINARY"), "{json}");
    }

    #[test]
    fn a_reason_appears_and_does_not_disclose_the_category() {
        let redacted = Redactor::new()
            .with_rule(RedactionRule::node_id("at0011"))
            .with_reason("Withheld under the patient's recorded consent preferences")
            .redact(&composition())
            .unwrap();
        let json = serde_json::to_string(&redacted).unwrap();
        assert!(json.contains("recorded consent preferences"), "{json}");
        assert!(!json.contains("ZZ-SENSITIVE"));
    }

    #[test]
    fn no_rules_withhold_nothing() {
        let (redacted, count) = Redactor::new().redact_counting(&composition()).unwrap();
        assert_eq!(count.masked, 0);
        assert_eq!(redacted, composition());
    }

    #[test]
    fn sensitive_hides_from_display_and_debug_but_not_from_serde() {
        let marker = "ZZ-DISTINCTIVE-9999";
        let s = Sensitive::new(marker.to_string());
        assert!(!format!("{s}").contains(marker));
        assert!(!format!("{s:?}").contains(marker));
        // Storage and authorised transmission must keep the value.
        assert!(serde_json::to_string(&s).unwrap().contains(marker));
    }
}
