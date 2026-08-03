//! Text and terminology: `DV_TEXT`, `DV_CODED_TEXT`, `CODE_PHRASE`,
//! `TERM_MAPPING`, `DV_PARAGRAPH`.
//!
//! # The invariant that carries the clinical weight
//!
//! `DV_CODED_TEXT.value` is not a label the author chose — openEHR states it
//! **must be the rubric of `defining_code`**. That is what makes a coded text
//! self-describing: a reader with no terminology server still knows what the
//! code meant *to the system that wrote it*, and a reader with one can detect
//! that the two have drifted apart.
//!
//! This crate cannot enforce the invariant in general, because checking a
//! SNOMED rubric needs SNOMED. It enforces it for the openEHR support
//! terminology, where it has the rubrics, and says so rather than implying
//! wider coverage — see [`DvCodedText::check_openehr_rubric`].

use crate::base::TerminologyId;
use crate::error::ParseError;
use core::fmt;
use serde::{Deserialize, Serialize};

/// A term in a terminology: the terminology's identifier plus a code.
///
/// ```
/// use openehr::rm::data_types::CodePhrase;
///
/// let snomed = CodePhrase::new("SNOMED-CT", "271649006").unwrap();
/// assert_eq!(snomed.code_string(), "271649006");
/// assert!(!snomed.is_openehr());
///
/// let flavour = CodePhrase::openehr("271").unwrap();
/// assert!(flavour.is_openehr());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CodePhrase {
    terminology_id: TerminologyId,
    code_string: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    preferred_term: Option<String>,
}

impl CodePhrase {
    /// Builds a code phrase.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the terminology id is malformed or the code
    /// string is empty (`Code_string_valid`).
    pub fn new(terminology_id: &str, code_string: impl Into<String>) -> Result<Self, ParseError> {
        let code_string = code_string.into();
        if code_string.is_empty() {
            return Err(ParseError::invariant("CODE_PHRASE", "Code_string_valid"));
        }
        Ok(Self {
            terminology_id: terminology_id.parse()?,
            code_string,
            preferred_term: None,
        })
    }

    /// Builds a code phrase in the openEHR support terminology.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the code string is empty.
    pub fn openehr(code_string: impl Into<String>) -> Result<Self, ParseError> {
        let code_string = code_string.into();
        if code_string.is_empty() {
            return Err(ParseError::invariant("CODE_PHRASE", "Code_string_valid"));
        }
        Ok(Self {
            terminology_id: TerminologyId::openehr(),
            code_string,
            preferred_term: None,
        })
    }

    /// Attaches a preferred term.
    #[must_use]
    pub fn with_preferred_term(mut self, term: impl Into<String>) -> Self {
        self.preferred_term = Some(term.into());
        self
    }

    /// The terminology this code belongs to.
    #[must_use]
    pub fn terminology_id(&self) -> &TerminologyId {
        &self.terminology_id
    }

    /// The code.
    #[must_use]
    pub fn code_string(&self) -> &str {
        &self.code_string
    }

    /// The terminology's own preferred display term, if recorded.
    #[must_use]
    pub fn preferred_term(&self) -> Option<&str> {
        self.preferred_term.as_deref()
    }

    /// Whether the code comes from the openEHR support terminology.
    #[must_use]
    pub fn is_openehr(&self) -> bool {
        self.terminology_id.is_openehr()
    }
}

impl fmt::Display for CodePhrase {
    /// The `terminology::code` form openEHR uses in prose and in ADL.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", self.terminology_id, self.code_string)
    }
}

/// How a mapped term relates to the term it maps from.
///
/// Modelled as an enum rather than the specification's `Character`, because
/// four of the 1,114,112 possible characters are legal and the other
/// 1,114,108 silently mean nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MappingMatch {
    /// `>` — the target is broader than the source term.
    Broader,
    /// `=` — the target is equivalent.
    Equivalent,
    /// `<` — the target is narrower.
    Narrower,
    /// `?` — the relationship is not known.
    Unknown,
}

impl MappingMatch {
    /// The specification's single-character encoding.
    #[must_use]
    pub fn as_char(self) -> char {
        match self {
            Self::Broader => '>',
            Self::Equivalent => '=',
            Self::Narrower => '<',
            Self::Unknown => '?',
        }
    }

    /// Parses the specification's single-character encoding.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] for any character other than `>`, `=`, `<`, `?`.
    pub fn from_char(c: char) -> Result<Self, ParseError> {
        match c {
            '>' => Ok(Self::Broader),
            '=' => Ok(Self::Equivalent),
            '<' => Ok(Self::Narrower),
            '?' => Ok(Self::Unknown),
            _ => Err(ParseError::invariant("TERM_MAPPING", "Match_valid")),
        }
    }
}

impl Serialize for MappingMatch {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(&self.as_char())
    }
}

impl<'de> Deserialize<'de> for MappingMatch {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        let mut chars = raw.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => Self::from_char(c).map_err(serde::de::Error::custom),
            _ => Err(serde::de::Error::custom(
                "TERM_MAPPING.match must be exactly one character",
            )),
        }
    }
}

/// A mapping from this term to a term in another terminology.
///
/// ```
/// use openehr::rm::data_types::{CodePhrase, MappingMatch, TermMapping};
///
/// let m = TermMapping::new(CodePhrase::new("ICD10", "I10").unwrap(), MappingMatch::Broader);
/// assert!(m.is_broader());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermMapping {
    target: CodePhrase,
    #[serde(rename = "match")]
    match_kind: MappingMatch,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    purpose: Option<DvCodedText>,
}

impl TermMapping {
    /// Builds a mapping.
    #[must_use]
    pub fn new(target: CodePhrase, match_kind: MappingMatch) -> Self {
        Self {
            target,
            match_kind,
            purpose: None,
        }
    }

    /// Records why the mapping was made.
    #[must_use]
    pub fn with_purpose(mut self, purpose: DvCodedText) -> Self {
        self.purpose = Some(purpose);
        self
    }

    /// The mapped-to term.
    #[must_use]
    pub fn target(&self) -> &CodePhrase {
        &self.target
    }

    /// The relationship.
    #[must_use]
    pub fn match_kind(&self) -> MappingMatch {
        self.match_kind
    }

    /// Why the mapping was made, if recorded.
    #[must_use]
    pub fn purpose(&self) -> Option<&DvCodedText> {
        self.purpose.as_ref()
    }

    /// Whether the target is broader than the source.
    #[must_use]
    pub fn is_broader(&self) -> bool {
        self.match_kind == MappingMatch::Broader
    }

    /// Whether the target is equivalent to the source.
    #[must_use]
    pub fn is_equivalent(&self) -> bool {
        self.match_kind == MappingMatch::Equivalent
    }

    /// Whether the target is narrower than the source.
    #[must_use]
    pub fn is_narrower(&self) -> bool {
        self.match_kind == MappingMatch::Narrower
    }
}

/// The three formatting values openEHR names for [`DvText::formatting`].
pub mod formatting {
    /// Plain text, newlines permitted.
    pub const PLAIN: &str = "plain";
    /// Plain text with no newlines — for values that must render on one line.
    pub const PLAIN_NO_NEWLINES: &str = "plain_no_newlines";
    /// CommonMark-flavoured markdown.
    pub const MARKDOWN: &str = "markdown";
}

/// Displayable text, optionally with language, encoding, formatting, and
/// mappings into other terminologies.
///
/// ```
/// use openehr::rm::data_types::DvText;
///
/// let t = DvText::new("Patient reports no chest pain").unwrap();
/// assert_eq!(t.value(), "Patient reports no chest pain");
///
/// // The empty string is not text (Value_valid). A field that is present but
/// // empty is indistinguishable from one that is absent, and openEHR already
/// // has a way to say absent.
/// assert!(DvText::new("").is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DvText {
    value: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    language: Option<CodePhrase>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    encoding: Option<CodePhrase>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    formatting: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    hyperlink: Option<super::uri::DvUri>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    mappings: Vec<TermMapping>,
}

impl DvText {
    /// Builds plain text.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the value is empty (`Value_valid`).
    pub fn new(value: impl Into<String>) -> Result<Self, ParseError> {
        let value = value.into();
        if value.is_empty() {
            return Err(ParseError::invariant("DV_TEXT", "Valid_value"));
        }
        Ok(Self {
            value,
            language: None,
            encoding: None,
            formatting: None,
            hyperlink: None,
            mappings: Vec::new(),
        })
    }

    /// Records the language of the text.
    #[must_use]
    pub fn with_language(mut self, language: CodePhrase) -> Self {
        self.language = Some(language);
        self
    }

    /// Records the character encoding of the text.
    #[must_use]
    pub fn with_encoding(mut self, encoding: CodePhrase) -> Self {
        self.encoding = Some(encoding);
        self
    }

    /// Declares the formatting.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the formatting name is empty
    /// (`Formatting_valid`), or if it is `plain_no_newlines` and the value
    /// contains a newline — a self-contradicting pair that would otherwise be
    /// discovered by whichever renderer breaks first.
    pub fn with_formatting(mut self, formatting: impl Into<String>) -> Result<Self, ParseError> {
        let formatting = formatting.into();
        if formatting.is_empty() {
            return Err(ParseError::invariant("DV_TEXT", "Formatting_valid"));
        }
        if formatting == self::formatting::PLAIN_NO_NEWLINES && self.value.contains(['\n', '\r']) {
            return Err(ParseError::invariant(
                "DV_TEXT",
                "Formatting_valid: plain_no_newlines with a newline in value",
            ));
        }
        self.formatting = Some(formatting);
        Ok(self)
    }

    /// Adds a term mapping.
    #[must_use]
    pub fn with_mapping(mut self, mapping: TermMapping) -> Self {
        self.mappings.push(mapping);
        self
    }

    /// The displayable text.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// The language, if recorded.
    #[must_use]
    pub fn language(&self) -> Option<&CodePhrase> {
        self.language.as_ref()
    }

    /// The character encoding, if recorded.
    #[must_use]
    pub fn encoding(&self) -> Option<&CodePhrase> {
        self.encoding.as_ref()
    }

    /// The formatting, if declared.
    #[must_use]
    pub fn formatting(&self) -> Option<&str> {
        self.formatting.as_deref()
    }

    /// The deprecated hyperlink attribute, if present.
    ///
    /// openEHR deprecates this in favour of a markdown link inside `value`.
    /// It is readable here because instances carry it; there is deliberately no
    /// builder that sets it.
    #[must_use]
    pub fn hyperlink(&self) -> Option<&super::uri::DvUri> {
        self.hyperlink.as_ref()
    }

    /// Mappings into other terminologies.
    #[must_use]
    pub fn mappings(&self) -> &[TermMapping] {
        &self.mappings
    }
}

impl fmt::Display for DvText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.value)
    }
}

/// Text whose meaning is fixed by a term in a terminology.
///
/// ```
/// use openehr::rm::data_types::{CodePhrase, DvCodedText};
///
/// let c = DvCodedText::new("Chest pain", CodePhrase::new("SNOMED-CT", "29857009").unwrap()).unwrap();
/// assert_eq!(c.value(), "Chest pain");
/// assert_eq!(c.defining_code().code_string(), "29857009");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DvCodedText {
    #[serde(flatten)]
    text: DvText,
    defining_code: CodePhrase,
}

impl DvCodedText {
    /// Builds a coded text.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the value is empty.
    pub fn new(value: impl Into<String>, defining_code: CodePhrase) -> Result<Self, ParseError> {
        Ok(Self {
            text: DvText::new(value)?,
            defining_code,
        })
    }

    /// The code that fixes the meaning.
    #[must_use]
    pub fn defining_code(&self) -> &CodePhrase {
        &self.defining_code
    }

    /// The rubric.
    #[must_use]
    pub fn value(&self) -> &str {
        self.text.value()
    }

    /// The coded text read as plain text, for the many RM attributes typed
    /// `DV_TEXT` that a `DV_CODED_TEXT` may occupy.
    #[must_use]
    pub fn as_text(&self) -> &DvText {
        &self.text
    }

    /// Mappings into other terminologies.
    #[must_use]
    pub fn mappings(&self) -> &[TermMapping] {
        self.text.mappings()
    }

    /// Adds a term mapping.
    #[must_use]
    pub fn with_mapping(mut self, mapping: TermMapping) -> Self {
        self.text = self.text.with_mapping(mapping);
        self
    }

    /// Checks `value` against the openEHR support terminology's rubric for
    /// `defining_code`.
    ///
    /// Returns:
    ///
    /// - `None` — the code is not from the openEHR terminology, or is from a
    ///   group this crate does not carry. **Not checked**, and reported as not
    ///   checked rather than as a pass.
    /// - `Some(true)` — the value matches the rubric.
    /// - `Some(false)` — the value contradicts the rubric.
    ///
    /// ```
    /// use openehr::rm::data_types::{CodePhrase, DvCodedText};
    ///
    /// let right = DvCodedText::new("creation", CodePhrase::openehr("249").unwrap()).unwrap();
    /// assert_eq!(right.check_openehr_rubric(), Some(true));
    ///
    /// let wrong = DvCodedText::new("deletion", CodePhrase::openehr("249").unwrap()).unwrap();
    /// assert_eq!(wrong.check_openehr_rubric(), Some(false));
    ///
    /// let external = DvCodedText::new("Chest pain", CodePhrase::new("SNOMED-CT", "29857009").unwrap()).unwrap();
    /// assert_eq!(external.check_openehr_rubric(), None); // no terminology server here
    /// ```
    #[must_use]
    pub fn check_openehr_rubric(&self) -> Option<bool> {
        if !self.defining_code.is_openehr() {
            return None;
        }
        let code = self.defining_code.code_string();
        // A code may appear in more than one group with different rubrics
        // (`523` is `deleted` in two of them). Agreement with *any* group is
        // acceptance: the attribute the coded text sits on decides the group,
        // and a bare DV_CODED_TEXT does not know which attribute that is.
        let mut seen = false;
        for group in crate::terminology::GROUPS {
            if let Some(rubric) = group.rubric(code) {
                seen = true;
                if rubric == self.value() {
                    return Some(true);
                }
            }
        }
        if seen { Some(false) } else { None }
    }
}

impl fmt::Display for DvCodedText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.value())
    }
}

/// Either kind of text, for the RM attributes typed `DV_TEXT` that admit a
/// `DV_CODED_TEXT` at runtime — `LOCATABLE.name`, `LINK.meaning`,
/// `PARTICIPATION.function`.
///
/// # Reading a payload with no `_type`
///
/// Coded first, plain second, decided by whether `defining_code` is present.
/// The reverse order would deserialize a coded text into a `DV_TEXT` and
/// **drop the code** — a silent loss of the only part that carries computable
/// meaning.
///
/// ```
/// use openehr::rm::data_types::Text;
///
/// let coded: Text = serde_json::from_str(
///     r#"{"value":"creation","defining_code":{"terminology_id":{"value":"openehr"},"code_string":"249"}}"#
/// ).unwrap();
/// assert!(coded.defining_code().is_some());
/// assert_eq!(coded.value(), "creation");
///
/// let plain: Text = serde_json::from_str(r#"{"value":"free text"}"#).unwrap();
/// assert!(plain.defining_code().is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Text {
    /// Uncoded text.
    Plain(DvText),
    /// Coded text.
    Coded(DvCodedText),
}

impl Text {
    /// The displayable value, whichever form this is.
    #[must_use]
    pub fn value(&self) -> &str {
        match self {
            Self::Plain(t) => t.value(),
            Self::Coded(t) => t.value(),
        }
    }

    /// The defining code, if this is coded text.
    #[must_use]
    pub fn defining_code(&self) -> Option<&CodePhrase> {
        match self {
            Self::Plain(_) => None,
            Self::Coded(t) => Some(t.defining_code()),
        }
    }

    /// The plain-text view.
    #[must_use]
    pub fn as_text(&self) -> &DvText {
        match self {
            Self::Plain(t) => t,
            Self::Coded(t) => t.as_text(),
        }
    }

    /// The openEHR class name, as it appears in `_type`.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Plain(_) => "DV_TEXT",
            Self::Coded(_) => "DV_CODED_TEXT",
        }
    }

    /// Builds plain text.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the value is empty.
    pub fn plain(value: impl Into<String>) -> Result<Self, ParseError> {
        Ok(Self::Plain(DvText::new(value)?))
    }
}

impl From<DvText> for Text {
    fn from(v: DvText) -> Self {
        Self::Plain(v)
    }
}

impl From<DvCodedText> for Text {
    fn from(v: DvCodedText) -> Self {
        Self::Coded(v)
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.value())
    }
}

impl Serialize for Text {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        // Emitting `_type` even though the value would round-trip without it:
        // a consumer validating against the openEHR JSON Schema requires it
        // wherever the declared type is abstract or has subclasses, and
        // `DV_TEXT` has one.
        #[derive(Serialize)]
        struct Tagged<'a, T: Serialize> {
            #[serde(rename = "_type")]
            ty: &'static str,
            #[serde(flatten)]
            inner: &'a T,
        }
        match self {
            Self::Plain(t) => Tagged {
                ty: "DV_TEXT",
                inner: t,
            }
            .serialize(s),
            Self::Coded(t) => Tagged {
                ty: "DV_CODED_TEXT",
                inner: t,
            }
            .serialize(s),
        }
    }
}

impl<'de> Deserialize<'de> for Text {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;

        // A flat wire struct rather than an intermediate `serde_json::Value`:
        // `name` is a `Text` on *every* locatable, so this runs once per node
        // in a composition. Materialising a `Value` and re-deserializing from
        // it doubled both the work and the stack depth of reading a document.
        #[derive(Deserialize)]
        struct Wire {
            #[serde(rename = "_type", default)]
            ty: Option<String>,
            value: String,
            #[serde(default)]
            language: Option<CodePhrase>,
            #[serde(default)]
            encoding: Option<CodePhrase>,
            #[serde(default)]
            formatting: Option<String>,
            #[serde(default)]
            hyperlink: Option<super::uri::DvUri>,
            #[serde(default)]
            mappings: Vec<TermMapping>,
            #[serde(default)]
            defining_code: Option<CodePhrase>,
        }

        let wire = Wire::deserialize(d)?;
        let is_coded = match wire.ty.as_deref() {
            Some("DV_CODED_TEXT") => true,
            Some("DV_TEXT") => false,
            Some(other) => {
                return Err(D::Error::custom(format!(
                    "_type is {other}, expected DV_TEXT or DV_CODED_TEXT"
                )));
            }
            // No declared type: the presence of `defining_code` is the only
            // signal, and it is a reliable one because `DV_TEXT` has no such
            // attribute.
            None => wire.defining_code.is_some(),
        };
        let text = DvText {
            value: wire.value,
            language: wire.language,
            encoding: wire.encoding,
            formatting: wire.formatting,
            hyperlink: wire.hyperlink,
            mappings: wire.mappings,
        };
        if is_coded {
            let defining_code = wire
                .defining_code
                .ok_or_else(|| D::Error::missing_field("defining_code"))?;
            Ok(Self::Coded(DvCodedText {
                text,
                defining_code,
            }))
        } else {
            Ok(Self::Plain(text))
        }
    }
}

/// A series of text items making up a passage of prose.
///
/// **Deprecated by openEHR** in favour of markdown-formatted [`DvText`]. It is
/// modelled because instances written before the deprecation still contain it
/// and must round-trip; new content should not use it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DvParagraph {
    items: Vec<DvText>,
}

impl DvParagraph {
    /// Builds a paragraph.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the item list is empty (`Items_valid`).
    pub fn new(items: Vec<DvText>) -> Result<Self, ParseError> {
        if items.is_empty() {
            return Err(ParseError::invariant("DV_PARAGRAPH", "Items_valid"));
        }
        Ok(Self { items })
    }

    /// The text items.
    #[must_use]
    pub fn items(&self) -> &[DvText] {
        &self.items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coded_text_round_trips_through_canonical_json() {
        let c = DvCodedText::new("masked", CodePhrase::openehr("272").unwrap()).unwrap();
        let text = Text::Coded(c.clone());
        let json = serde_json::to_string(&text).unwrap();
        assert!(json.contains(r#""_type":"DV_CODED_TEXT""#), "{json}");
        let back: Text = serde_json::from_str(&json).unwrap();
        assert_eq!(back, text);
    }

    #[test]
    fn a_coded_payload_without_a_type_does_not_lose_its_code() {
        // The failure this guards: reading into DV_TEXT first would succeed and
        // silently drop `defining_code`.
        let json = r#"{"value":"masked","defining_code":{"terminology_id":{"value":"openehr"},"code_string":"272"}}"#;
        let t: Text = serde_json::from_str(json).unwrap();
        assert_eq!(t.defining_code().map(CodePhrase::code_string), Some("272"));
    }

    #[test]
    fn rubric_checking_reports_unchecked_separately_from_valid() {
        let openehr_ok = DvCodedText::new("masked", CodePhrase::openehr("272").unwrap()).unwrap();
        let openehr_bad =
            DvCodedText::new("unmasked", CodePhrase::openehr("272").unwrap()).unwrap();
        let openehr_unknown_code =
            DvCodedText::new("whatever", CodePhrase::openehr("99999").unwrap()).unwrap();
        let external =
            DvCodedText::new("Chest pain", CodePhrase::new("LOINC", "8480-6").unwrap()).unwrap();

        assert_eq!(openehr_ok.check_openehr_rubric(), Some(true));
        assert_eq!(openehr_bad.check_openehr_rubric(), Some(false));
        assert_eq!(openehr_unknown_code.check_openehr_rubric(), None);
        assert_eq!(external.check_openehr_rubric(), None);
    }

    #[test]
    fn plain_no_newlines_is_checked_against_the_value() {
        assert!(
            DvText::new("one line")
                .unwrap()
                .with_formatting(formatting::PLAIN_NO_NEWLINES)
                .is_ok()
        );
        assert!(
            DvText::new("two\nlines")
                .unwrap()
                .with_formatting(formatting::PLAIN_NO_NEWLINES)
                .is_err()
        );
        assert!(
            DvText::new("two\nlines")
                .unwrap()
                .with_formatting(formatting::MARKDOWN)
                .is_ok()
        );
    }

    #[test]
    fn errors_from_text_do_not_echo_the_text() {
        let e = DvText::new("").unwrap_err();
        assert_eq!(e.input, "");
        // And the invariant name is the one openEHR uses, so a reader can find
        // it in the class definition (`L10.4`).
        //
        // This asserted `Value_valid` until 2026-08-02, while its own comment
        // claimed the name came from openEHR. `DV_TEXT`'s invariant is
        // `Valid_value` — the words the other way round. A test can state a
        // requirement and enforce its opposite, and this one did for as long as
        // it existed; nothing compared the string against the specification
        // until `assets/invariant-coverage.md` started doing it every run
        // (`A-20`).
        assert_eq!(e.reason, "Valid_value");
    }

    /// A `CODE_PHRASE` reports its preferred term and renders as
    /// `terminology::code`.
    ///
    /// `preferred_term` could answer `None`, `""` or `"xyzzy"` for every
    /// phrase, and `Display` could print nothing (`lib:A-09`). The rendered
    /// form is what ADL and error messages show a reader, so a blank `Display`
    /// is silent everywhere it appears.
    #[test]
    fn a_code_phrase_reports_its_preferred_term_and_renders_itself() {
        let bare = CodePhrase::new("ISO_639-1", "en").unwrap();
        assert_eq!(bare.preferred_term(), None);
        assert_eq!(bare.to_string(), "ISO_639-1::en");

        let annotated = CodePhrase::new("SNOMED-CT", "38341003")
            .unwrap()
            .with_preferred_term("Hypertensive disorder");
        assert_eq!(annotated.preferred_term(), Some("Hypertensive disorder"));
        assert_eq!(annotated.to_string(), "SNOMED-CT::38341003");

        // Two different codes must render differently — a constant Display
        // makes every code look the same in an error message.
        let other = CodePhrase::new("SNOMED-CT", "271737000").unwrap();
        assert_ne!(annotated.to_string(), other.to_string());
    }

    /// Every `MappingMatch` kind, told apart by `TermMapping`'s three
    /// predicates.
    ///
    /// Five mutants here (`lib:A-09`): each predicate could be a constant, and
    /// two of the three `==` comparisons could be `!=`. A `TermMapping` records
    /// how a coded term relates to a mapped-to term in another terminology —
    /// broader, equivalent, or narrower — and getting that wrong silently
    /// changes what an ICD-10 crosswalk claims about a SNOMED CT code.
    #[test]
    fn every_mapping_match_is_reported_by_exactly_one_predicate() {
        let mapping = |kind: MappingMatch| {
            TermMapping::new(CodePhrase::new("ICD10", "I10").unwrap(), kind)
        };

        let cases = [
            (MappingMatch::Broader, [true, false, false]),
            (MappingMatch::Equivalent, [false, true, false]),
            (MappingMatch::Narrower, [false, false, true]),
            (MappingMatch::Unknown, [false, false, false]),
        ];
        for (kind, [broader, equivalent, narrower]) in cases {
            let m = mapping(kind);
            assert_eq!(m.is_broader(), broader, "{kind:?}");
            assert_eq!(m.is_equivalent(), equivalent, "{kind:?}");
            assert_eq!(m.is_narrower(), narrower, "{kind:?}");
            assert_eq!(m.match_kind(), kind);
            assert_eq!(m.target().code_string(), "I10");
        }

        // The purpose is optional and reported as recorded.
        assert_eq!(mapping(MappingMatch::Equivalent).purpose(), None);
        let with_purpose = mapping(MappingMatch::Equivalent).with_purpose(
            DvCodedText::new("clinical", CodePhrase::new("local", "at0001").unwrap()).unwrap(),
        );
        assert_eq!(with_purpose.purpose().map(DvCodedText::value), Some("clinical"));
    }

    /// A `DV_TEXT`'s optional attributes, and its rendering.
    ///
    /// Six accessors could each answer nothing, and `Display` could print
    /// nothing (`lib:A-09`). `hyperlink` has no builder — the deprecated
    /// attribute arrives only by deserialization, which is the one path that
    /// reads it back.
    #[test]
    fn a_dv_text_reports_every_attribute_it_was_built_with() {
        let bare = DvText::new("systolic blood pressure").unwrap();
        assert_eq!(bare.to_string(), "systolic blood pressure");
        assert_eq!(bare.language(), None);
        assert_eq!(bare.encoding(), None);
        assert_eq!(bare.formatting(), None);
        assert_eq!(bare.hyperlink(), None);
        assert!(bare.mappings().is_empty());

        let en = CodePhrase::new("ISO_639-1", "en").unwrap();
        let utf8 = CodePhrase::new("IANA_character-sets", "UTF-8").unwrap();
        let full = DvText::new("systolic blood pressure")
            .unwrap()
            .with_language(en.clone())
            .with_encoding(utf8.clone())
            .with_formatting("plain")
            .unwrap()
            .with_mapping(TermMapping::new(
                CodePhrase::new("SNOMED-CT", "271649006").unwrap(),
                MappingMatch::Equivalent,
            ));
        assert_eq!(full.language(), Some(&en));
        assert_eq!(full.encoding(), Some(&utf8));
        assert_eq!(full.formatting(), Some("plain"));
        assert_eq!(full.mappings().len(), 1, "a mapping was dropped");

        // The hyperlink has no builder; deserialization is the only path.
        let json = serde_json::to_value(&full).expect("serialize");
        let mut object = json.as_object().expect("an object").clone();
        object.insert(
            "hyperlink".to_owned(),
            serde_json::to_value(crate::rm::data_types::DvUri::new("https://example.org/x").unwrap())
                .expect("serialize"),
        );
        let revived: DvText =
            serde_json::from_value(serde_json::Value::Object(object)).expect("deserialize");
        assert!(revived.hyperlink().is_some(), "a recorded hyperlink was dropped");

        // Two different values must render differently.
        assert_ne!(bare.to_string(), DvText::new("diastolic blood pressure").unwrap().to_string());
    }

    /// A `DV_CODED_TEXT` renders as its plain value and carries the mappings
    /// it was built with; a `Text` renders whichever form it wraps.
    #[test]
    fn a_coded_text_renders_its_value_and_keeps_its_mappings() {
        let coded = DvCodedText::new(
            "hypertension",
            CodePhrase::new("SNOMED-CT", "38341003").unwrap(),
        )
        .unwrap();
        assert_eq!(coded.to_string(), "hypertension");
        assert!(coded.mappings().is_empty());

        let mapped = DvCodedText::new(
            "hypertension",
            CodePhrase::new("SNOMED-CT", "38341003").unwrap(),
        )
        .unwrap()
        .with_mapping(TermMapping::new(
            CodePhrase::new("ICD10", "I10").unwrap(),
            MappingMatch::Broader,
        ));
        assert_eq!(mapped.mappings().len(), 1);

        let plain: Text = Text::Plain(DvText::new("free text").unwrap());
        let as_coded: Text = Text::Coded(coded);
        assert_eq!(plain.to_string(), "free text");
        assert_eq!(as_coded.to_string(), "hypertension");
        assert_ne!(plain.to_string(), as_coded.to_string());
    }

    /// A `DV_PARAGRAPH` reports every line it holds.
    ///
    /// `items` could answer an empty slice for every paragraph (`lib:A-09`),
    /// which silently discards every line but the first read elsewhere.
    #[test]
    fn a_paragraph_reports_every_line() {
        let lines = vec![
            DvText::new("first line").unwrap(),
            DvText::new("second line").unwrap(),
        ];
        let paragraph = DvParagraph::new(lines.clone()).unwrap();
        assert_eq!(paragraph.items().len(), 2, "a line was dropped");
        assert_eq!(paragraph.items()[0].value(), "first line");
        assert_eq!(paragraph.items()[1].value(), "second line");
    }
}
