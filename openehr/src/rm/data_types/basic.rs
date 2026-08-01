//! The basic data types: `DV_BOOLEAN`, `DV_STATE`, `DV_IDENTIFIER`.

use super::text::DvCodedText;
use crate::error::ParseError;
use serde::{Deserialize, Serialize};

/// A boolean value.
///
/// # Why this exists rather than a bare `bool`
///
/// openEHR deliberately does not use `DV_BOOLEAN` for "is a thing true" —
/// its own guidance is that a boolean answer to a clinical question should be
/// a `DV_CODED_TEXT`, because "no" and "not asked" and "not applicable" are
/// three different answers and a `bool` has room for two. `DV_BOOLEAN` is for
/// genuinely binary facts. Keeping it as a distinct type keeps that decision
/// visible at every use site.
///
/// ```
/// use openehr::rm::data_types::DvBoolean;
///
/// let b = DvBoolean::new(true);
/// assert!(b.value());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DvBoolean {
    value: bool,
}

impl DvBoolean {
    /// Builds a boolean value.
    #[must_use]
    pub fn new(value: bool) -> Self {
        Self { value }
    }

    /// The value.
    #[must_use]
    pub fn value(self) -> bool {
        self.value
    }
}

/// A state in an archetype-defined state machine.
///
/// ```
/// use openehr::rm::data_types::{CodePhrase, DvCodedText, DvState};
///
/// let symbol = DvCodedText::new("active", CodePhrase::openehr("245").unwrap()).unwrap();
/// let s = DvState::new(symbol, false);
/// assert!(!s.is_terminal());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DvState {
    value: DvCodedText,
    is_terminal: bool,
}

impl DvState {
    /// Builds a state.
    #[must_use]
    pub fn new(value: DvCodedText, is_terminal: bool) -> Self {
        Self { value, is_terminal }
    }

    /// The state's name.
    #[must_use]
    pub fn value(&self) -> &DvCodedText {
        &self.value
    }

    /// Whether the state machine can leave this state.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        self.is_terminal
    }
}

/// An identifier issued by some authority: an MRN, an NHS number, a
/// prescription number.
///
/// # This is the PHI-bearing data type
///
/// Of everything in the data-types package, `DV_IDENTIFIER` is the one whose
/// `id` is *directly* identifying by design. That is why its
/// [`Display`](core::fmt::Display) implementation prints the **type and
/// issuer**, not the value: a `{}` in a log line is the most common accidental
/// disclosure route there is, and this type takes it away.
///
/// ```
/// use openehr::rm::data_types::DvIdentifier;
///
/// let nhs = DvIdentifier::new("943-476-5919").unwrap()
///     .with_type("NHS number")
///     .with_issuer("NHS England");
///
/// assert_eq!(nhs.id(), "943-476-5919");
/// // The identifier itself is never what Display prints.
/// assert_eq!(nhs.to_string(), "NHS number issued by NHS England");
/// assert!(!nhs.to_string().contains("943"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DvIdentifier {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    issuer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    assigner: Option<String>,
    id: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none", default)]
    id_type: Option<String>,
}

impl DvIdentifier {
    /// Builds an identifier.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the id is empty (`Id_valid`).
    pub fn new(id: impl Into<String>) -> Result<Self, ParseError> {
        let id = id.into();
        if id.is_empty() {
            return Err(ParseError::invariant("DV_IDENTIFIER", "Id_valid"));
        }
        Ok(Self {
            issuer: None,
            assigner: None,
            id,
            id_type: None,
        })
    }

    /// Records what kind of identifier this is.
    #[must_use]
    pub fn with_type(mut self, id_type: impl Into<String>) -> Self {
        self.id_type = Some(id_type.into());
        self
    }

    /// Records the authority that defines the identifier type.
    #[must_use]
    pub fn with_issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = Some(issuer.into());
        self
    }

    /// Records the organisation that allocated this particular identifier.
    #[must_use]
    pub fn with_assigner(mut self, assigner: impl Into<String>) -> Self {
        self.assigner = Some(assigner.into());
        self
    }

    /// The identifier value.
    ///
    /// Named `id`, and reached only deliberately — nothing formats it for you.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// What kind of identifier this is.
    #[must_use]
    pub fn id_type(&self) -> Option<&str> {
        self.id_type.as_deref()
    }

    /// The authority that defines the identifier type.
    #[must_use]
    pub fn issuer(&self) -> Option<&str> {
        self.issuer.as_deref()
    }

    /// The organisation that allocated this identifier.
    #[must_use]
    pub fn assigner(&self) -> Option<&str> {
        self.assigner.as_deref()
    }
}

impl core::fmt::Display for DvIdentifier {
    /// Describes the identifier **without printing it**. See the type's
    /// documentation for why.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match (&self.id_type, &self.issuer) {
            (Some(t), Some(i)) => write!(f, "{t} issued by {i}"),
            (Some(t), None) => write!(f, "{t}"),
            (None, Some(i)) => write!(f, "identifier issued by {i}"),
            (None, None) => f.write_str("identifier"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_never_reveals_the_identifier() {
        // The mutation check: replace the Display impl with one that writes
        // self.id, and this fails.
        let marker = "ZZ-DISTINCTIVE-9999";
        let id = DvIdentifier::new(marker).unwrap().with_type("MRN");
        assert!(!format!("{id}").contains(marker));
        assert!(!format!("{id:}").contains(marker));
        // Debug is a different contract: it is for developers reading a
        // struct, and hiding fields there produces bug reports nobody can act
        // on. The rule this crate enforces is that *Display* is safe.
        assert!(format!("{id:?}").contains(marker));
    }

    #[test]
    fn an_empty_identifier_is_refused() {
        assert!(DvIdentifier::new("").is_err());
    }
}
