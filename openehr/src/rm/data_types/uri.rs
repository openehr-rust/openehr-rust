//! `DV_URI` and `DV_EHR_URI`.
//!
//! `DV_EHR_URI` is the reference model's internal pointer: it addresses a node
//! *inside* an EHR using the `ehr` scheme, and it is what makes
//! [`crate::rm::common::Link`] able to say "this diagnosis is the reason for
//! that prescription" across compositions without duplicating either.
//!
//! # Validation is structural, not resolvable
//!
//! This module checks that a URI has a scheme and that a `DV_EHR_URI`'s scheme
//! is `ehr`. It does not check that the target exists — nothing in-process
//! can, because the target is in a repository this crate does not have. A
//! "valid" `DV_EHR_URI` therefore means *well formed*, and callers that need
//! referential integrity have to resolve it. Saying so is the point: a
//! validator that returned "valid" for a dangling link would be worse than one
//! that never looked.

use crate::error::ParseError;
use core::fmt;
use core::str::FromStr;
use serde::{Deserialize, Serialize};

/// A URI.
///
/// ```
/// use openehr::rm::data_types::DvUri;
///
/// let u: DvUri = "https://example.org/guidelines/htn".parse().unwrap();
/// assert_eq!(u.scheme(), "https");
/// assert!("not a uri".parse::<DvUri>().is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DvUri {
    value: String,
}

impl DvUri {
    /// Builds a URI.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the value is empty, has no `scheme:` prefix,
    /// has a scheme that does not match RFC 3986's
    /// `ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )`, or contains a space or
    /// control character.
    pub fn new(value: impl Into<String>) -> Result<Self, ParseError> {
        let value = value.into();
        let bad = |reason| ParseError::new("DV_URI", reason, &value);
        if value.is_empty() {
            return Err(bad("empty"));
        }
        if value.chars().any(|c| c == ' ' || c.is_control()) {
            return Err(bad("contains a space or control character"));
        }
        let Some((scheme, _)) = value.split_once(':') else {
            return Err(bad("no scheme"));
        };
        let mut bytes = scheme.bytes();
        let Some(first) = bytes.next() else {
            return Err(bad("empty scheme"));
        };
        if !first.is_ascii_alphabetic()
            || !bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
        {
            return Err(bad("malformed scheme"));
        }
        Ok(Self { value })
    }

    /// The whole URI.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// The scheme, without the colon. Empty when there is none.
    ///
    /// This said "# Panics — Never: the constructor guarantees a colon is
    /// present", and it was wrong. `Deserialize` is derived and writes `value`
    /// straight in, so a `DV_URI` read from JSON has passed no constructor
    /// (`L10.1a`); `{"value":"nocolon"}` deserialized cleanly and panicked
    /// here. See [`crate::validation`] and `lib:A-36`.
    ///
    /// Empty is the right answer rather than a fallback: a value with no colon
    /// has no scheme, and `""` compares unequal to every real one — including
    /// `ehr` — so a caller that dispatches on the scheme fails closed.
    /// `validate()` is what *reports* it.
    #[must_use]
    pub fn scheme(&self) -> &str {
        self.value.split_once(':').map_or("", |(s, _)| s)
    }

    /// Everything after the scheme's colon. Empty when there is no colon.
    ///
    /// Total, for the reason [`Self::scheme`] gives.
    #[must_use]
    pub fn rest(&self) -> &str {
        self.value.split_once(':').map_or("", |(_, r)| r)
    }
}

impl fmt::Display for DvUri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.value)
    }
}

impl FromStr for DvUri {
    type Err = ParseError;

    /// # Errors
    ///
    /// See [`DvUri::new`].
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

/// A URI in the `ehr` scheme, addressing something inside an EHR.
///
/// ```
/// use openehr::rm::data_types::DvEhrUri;
///
/// let u: DvEhrUri =
///     "ehr://87284370-2D4B-4E3D-A3F3-F303D2F4F34B/content[0]/data".parse().unwrap();
/// assert_eq!(u.scheme(), "ehr");
///
/// // A DV_EHR_URI that is not an EHR URI is the mistake this type exists to
/// // catch: LINK.target is typed DV_EHR_URI precisely so a link cannot point
/// // out of the record without saying so.
/// assert!("https://example.org/x".parse::<DvEhrUri>().is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DvEhrUri {
    uri: DvUri,
}

impl DvEhrUri {
    /// The scheme every `DV_EHR_URI` must use.
    pub const SCHEME: &'static str = "ehr";

    /// Builds an EHR URI.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the value is not a well-formed URI, or if its
    /// scheme is not `ehr` (`Scheme_valid`).
    pub fn new(value: impl Into<String>) -> Result<Self, ParseError> {
        let uri = DvUri::new(value)?;
        if uri.scheme() != Self::SCHEME {
            return Err(ParseError::invariant("DV_EHR_URI", "Scheme_valid"));
        }
        Ok(Self { uri })
    }

    /// The whole URI.
    #[must_use]
    pub fn value(&self) -> &str {
        self.uri.value()
    }

    /// Always `ehr`.
    #[must_use]
    pub fn scheme(&self) -> &str {
        self.uri.scheme()
    }

    /// The URI read as a plain [`DvUri`].
    #[must_use]
    pub fn as_uri(&self) -> &DvUri {
        &self.uri
    }
}

impl fmt::Display for DvEhrUri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.value())
    }
}

impl FromStr for DvEhrUri {
    type Err = ParseError;

    /// # Errors
    ///
    /// See [`DvEhrUri::new`].
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schemes_are_checked_not_assumed() {
        assert!(DvUri::new("ehr://x/y").is_ok());
        assert!(DvUri::new("//x/y").is_err());
        assert!(DvUri::new("1http://x").is_err());
        assert!(DvUri::new("http://exa mple.org").is_err());
    }

    #[test]
    fn ehr_uri_refuses_an_external_target() {
        assert!(DvEhrUri::new("ehr://1/2").is_ok());
        assert!(DvEhrUri::new("http://example.org").is_err());
    }
}
