//! `Terminology_code`/`Terminology_term`: BASE's own terminology reference
//! types.
//!
//! Not [`CodePhrase`](crate::rm::data_types::CodePhrase). `CODE_PHRASE` is a
//! Reference Model data type whose `terminology_id` is a structured
//! [`TerminologyId`](crate::base::TerminologyId); `Terminology_code` is a BASE
//! Foundation Types primitive whose `terminology_id` is a bare namespace
//! string (e.g. `"snomed_ct"`) and which additionally carries an optional
//! `terminology_version` and an optional `uri` into a terminology service.
//! openEHR uses `Terminology_code`, not `CODE_PHRASE` or a plain `String`,
//! wherever a *resource* names its own language rather than a clinical
//! concept: `AUTHORED_RESOURCE.original_language`,
//! `RESOURCE_DESCRIPTION_ITEM.language`, and `TRANSLATION_DETAILS.language`
//! (`org.openehr.base.resource.*.adoc`, `openEHR/specifications-BASE`) are all
//! typed `Terminology_code`.
//!
//! `Terminology_term` pairs one `Terminology_code` with its display text —
//! the terminology-reference half of what `CODE_PHRASE`/`preferred_term`
//! bundle together for the Reference Model, kept as two attributes here
//! because BASE has no `DV_TEXT` to lean on.
//!
//! # No invariant to enforce
//!
//! Unlike `CODE_PHRASE`'s `Code_string_valid`, neither
//! `org.openehr.base.foundation_types.terminology_code.adoc` nor
//! `...terminology_term.adoc` declares an invariant — there is nothing to
//! check beyond the mandatory fields being present, which the Rust type
//! system already does by having no `Option` around them. `new` is therefore
//! infallible; do not add a non-empty-string check by analogy with
//! `CODE_PHRASE` without first finding the invariant that would require it.
//!
//! # Not yet built on this
//!
//! `AUTHORED_RESOURCE`, `RESOURCE_DESCRIPTION`, `RESOURCE_DESCRIPTION_ITEM`,
//! and `TRANSLATION_DETAILS` — the classes that actually use
//! `Terminology_code` — remain unmodelled. This is the prerequisite type
//! only, added on its own because it is independently well-formed and small;
//! it is not a claim that any of the four is now closer to done.

use serde::{Deserialize, Serialize};

/// A standalone reference to a terminology concept: a terminology
/// identifier, optional version, and a code or code string
/// (`org.openehr.base.foundation_types.terminology_code.adoc`).
///
/// ```
/// use openehr::base::TerminologyCode;
///
/// let en = TerminologyCode::new("ISO_639-1", "en");
/// assert_eq!(en.terminology_id(), "ISO_639-1");
/// assert_eq!(en.code_string(), "en");
/// assert!(en.terminology_version().is_none());
///
/// let versioned = TerminologyCode::new("snomed_ct", "271649006")
///     .with_terminology_version("2024-01-31");
/// assert_eq!(versioned.terminology_version(), Some("2024-01-31"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TerminologyCode {
    terminology_id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    terminology_version: Option<String>,
    code_string: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    uri: Option<String>,
}

impl TerminologyCode {
    /// Builds a terminology code from its namespace and code string.
    #[must_use]
    pub fn new(terminology_id: impl Into<String>, code_string: impl Into<String>) -> Self {
        Self {
            terminology_id: terminology_id.into(),
            terminology_version: None,
            code_string: code_string.into(),
            uri: None,
        }
    }

    /// Attaches a terminology version, typically a date or dotted numeric.
    #[must_use]
    pub fn with_terminology_version(mut self, version: impl Into<String>) -> Self {
        self.terminology_version = Some(version.into());
        self
    }

    /// Attaches a URI into a terminology service.
    ///
    /// This is BASE's own `Uri` foundation type, which this crate does not
    /// otherwise model — do not confuse it with
    /// [`DvUri`](crate::rm::data_types::DvUri), the Reference Model data
    /// value, which validates a scheme. Nothing here checks that this string
    /// is a well-formed URI at all; it is carried exactly as given.
    #[must_use]
    pub fn with_uri(mut self, uri: impl Into<String>) -> Self {
        self.uri = Some(uri.into());
        self
    }

    /// The terminology's namespace identifier, e.g. `"snomed_ct"`.
    #[must_use]
    pub fn terminology_id(&self) -> &str {
        &self.terminology_id
    }

    /// The terminology's version, if recorded.
    #[must_use]
    pub fn terminology_version(&self) -> Option<&str> {
        self.terminology_version.as_deref()
    }

    /// The code, or post-coordinated code expression.
    #[must_use]
    pub fn code_string(&self) -> &str {
        &self.code_string
    }

    /// The URI into a terminology service, if recorded. See
    /// [`Self::with_uri`] for what this does and does not validate.
    #[must_use]
    pub fn uri(&self) -> Option<&str> {
        self.uri.as_deref()
    }
}

/// A standalone term from a terminology: the term text and the concept
/// reference that names it
/// (`org.openehr.base.foundation_types.terminology_term.adoc`).
///
/// ```
/// use openehr::base::{TerminologyCode, TerminologyTerm};
///
/// let term = TerminologyTerm::new(TerminologyCode::new("ISO_639-1", "en"), "English");
/// assert_eq!(term.text(), "English");
/// assert_eq!(term.concept().code_string(), "en");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TerminologyTerm {
    concept: TerminologyCode,
    text: String,
}

impl TerminologyTerm {
    /// Builds a terminology term from its concept reference and display text.
    #[must_use]
    pub fn new(concept: TerminologyCode, text: impl Into<String>) -> Self {
        Self {
            concept,
            text: text.into(),
        }
    }

    /// The concept reference this term names.
    #[must_use]
    pub const fn concept(&self) -> &TerminologyCode {
        &self.concept
    }

    /// The term's display text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

#[cfg(test)]
mod tests {
    use super::{TerminologyCode, TerminologyTerm};

    #[test]
    fn a_terminology_code_carries_its_namespace_and_code_with_no_optional_fields_set() {
        let code = TerminologyCode::new("ISO_639-1", "en");
        assert_eq!(code.terminology_id(), "ISO_639-1");
        assert_eq!(code.code_string(), "en");
        assert!(code.terminology_version().is_none());
        assert!(code.uri().is_none());
    }

    #[test]
    fn a_terminology_code_can_carry_a_version_and_a_uri_independently() {
        let versioned_only =
            TerminologyCode::new("snomed_ct", "271649006").with_terminology_version("2024-01-31");
        assert_eq!(versioned_only.terminology_version(), Some("2024-01-31"));
        assert!(versioned_only.uri().is_none());

        let uri_only = TerminologyCode::new("snomed_ct", "271649006")
            .with_uri("https://terminology.example.org/snomed_ct/271649006");
        assert!(uri_only.terminology_version().is_none());
        assert_eq!(
            uri_only.uri(),
            Some("https://terminology.example.org/snomed_ct/271649006")
        );
    }

    #[test]
    fn a_terminology_term_pairs_its_concept_with_display_text() {
        let term = TerminologyTerm::new(TerminologyCode::new("ISO_639-1", "en"), "English");
        assert_eq!(term.text(), "English");
        assert_eq!(term.concept().terminology_id(), "ISO_639-1");
        assert_eq!(term.concept().code_string(), "en");
    }

    #[test]
    fn a_terminology_code_round_trips_through_canonical_json() {
        let code = TerminologyCode::new("snomed_ct", "271649006")
            .with_terminology_version("2024-01-31")
            .with_uri("https://terminology.example.org/snomed_ct/271649006");
        let json = serde_json::to_string(&code).unwrap();
        let back: TerminologyCode = serde_json::from_str(&json).unwrap();
        assert_eq!(code, back);
    }

    #[test]
    fn a_terminology_code_with_no_optional_fields_omits_them_from_json_rather_than_writing_null() {
        let code = TerminologyCode::new("ISO_639-1", "en");
        let json = serde_json::to_value(&code).unwrap();
        let object = json.as_object().unwrap();
        assert!(!object.contains_key("terminology_version"));
        assert!(!object.contains_key("uri"));
    }
}
