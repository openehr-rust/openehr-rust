//! Primitive unique identifiers: `UID`, `ISO_OID`, `UUID`, `INTERNET_ID`.
//!
//! openEHR's `UID` is an abstract type with exactly three concrete forms, and
//! the form is inferred from the lexical shape rather than declared. That
//! inference is the whole of this module, and the order it is done in matters:
//!
//! | Tried | Recognised by |
//! | --- | --- |
//! | [`Uuid`] | five hex groups sized 8-4-4-4-12 |
//! | [`IsoOid`] | digits and dots only |
//! | [`InternetId`] | RFC 1034 dotted labels |
//!
//! `ISO_OID` is tried before `INTERNET_ID` because an all-digit dotted string
//! satisfies both grammars — `2.16.840` is a valid ISO OID and, read as a
//! domain name, three valid RFC 1034 labels. openEHR uses OIDs for issuing
//! authorities and internet ids for systems, so classifying `2.16.840.1` as a
//! hostname would be wrong in every case it arises. `UUID` is tried first only
//! because it is cheapest to reject.
//!
//! # Case
//!
//! A `UUID` is compared case-insensitively but **stored verbatim**. RFC 4122
//! says lower-case on output and accept either on input; openEHR instances in
//! the wild carry both, and an `OBJECT_VERSION_ID` is a string key in every
//! REST path and every `LOCATABLE_REF`. Normalising the case on parse would
//! silently change the identifier a caller round-trips.

use crate::error::ParseError;
use core::fmt;
use core::str::FromStr;

/// A primitive unique identifier: an ISO OID, a UUID, or an internet domain
/// name.
///
/// ```
/// use openehr::base::Uid;
///
/// let oid: Uid = "2.16.840.1.113883.6.96".parse().unwrap();
/// assert!(matches!(oid, Uid::IsoOid(_)));
///
/// let uuid: Uid = "87284370-2D4B-4e3d-A3F3-F303D2F4F34B".parse().unwrap();
/// assert!(matches!(uuid, Uid::Uuid(_)));
///
/// let host: Uid = "ehr1.nhs.uk".parse().unwrap();
/// assert!(matches!(host, Uid::InternetId(_)));
///
/// // Round-trip is exact, including the mixed case of the UUID.
/// assert_eq!(uuid.to_string(), "87284370-2D4B-4e3d-A3F3-F303D2F4F34B");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Uid {
    /// An ISO object identifier, such as `2.16.840.1.113883.6.96`.
    IsoOid(IsoOid),
    /// An RFC 4122 UUID.
    Uuid(Uuid),
    /// An internet domain name, such as `ehr1.nhs.uk`.
    InternetId(InternetId),
}

impl Uid {
    /// The identifier's lexical form, exactly as parsed.
    #[must_use]
    pub fn value(&self) -> &str {
        match self {
            Self::IsoOid(v) => v.as_str(),
            Self::Uuid(v) => v.as_str(),
            Self::InternetId(v) => v.as_str(),
        }
    }
}

impl fmt::Display for Uid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.value())
    }
}

impl FromStr for Uid {
    type Err = ParseError;

    /// # Errors
    ///
    /// Returns [`ParseError`] if the text matches none of the three `UID`
    /// grammars.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(u) = s.parse::<Uuid>() {
            return Ok(Self::Uuid(u));
        }
        if let Ok(o) = s.parse::<IsoOid>() {
            return Ok(Self::IsoOid(o));
        }
        if let Ok(i) = s.parse::<InternetId>() {
            return Ok(Self::InternetId(i));
        }
        Err(ParseError::new(
            "UID",
            "not a UUID, an ISO OID, or an internet id",
            s,
        ))
    }
}

crate::impl_string_serde!(Uid, "UID");

/// An ISO object identifier: one or more dot-separated numbers.
///
/// ```
/// use openehr::base::IsoOid;
///
/// assert!("2.16.840.1.113883.6.96".parse::<IsoOid>().is_ok());
/// assert!("2.16..840".parse::<IsoOid>().is_err()); // empty arc
/// assert!("2.16.840a".parse::<IsoOid>().is_err()); // not a number
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IsoOid(String);

impl IsoOid {
    /// The OID's lexical form.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The OID's arcs, in order.
    ///
    /// ```
    /// use openehr::base::IsoOid;
    ///
    /// let oid: IsoOid = "1.3.6.1".parse().unwrap();
    /// assert_eq!(oid.arcs().collect::<Vec<_>>(), ["1", "3", "6", "1"]);
    /// ```
    pub fn arcs(&self) -> impl Iterator<Item = &str> {
        self.0.split('.')
    }
}

impl fmt::Display for IsoOid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for IsoOid {
    type Err = ParseError;

    /// # Errors
    ///
    /// Returns [`ParseError`] if any arc is empty or contains a non-digit.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(ParseError::new("ISO_OID", "empty", s));
        }
        for arc in s.split('.') {
            if arc.is_empty() {
                return Err(ParseError::new("ISO_OID", "empty arc", s));
            }
            if !arc.bytes().all(|b| b.is_ascii_digit()) {
                return Err(ParseError::new("ISO_OID", "arc is not a number", s));
            }
        }
        Ok(Self(s.to_owned()))
    }
}

/// An RFC 4122 UUID in the canonical 8-4-4-4-12 hexadecimal form.
///
/// Equality is case-insensitive, because `A3F3` and `a3f3` are the same
/// identifier and a record keyed on one must be found by the other. The stored
/// form keeps whatever case it was given.
///
/// ```
/// use openehr::base::Uuid;
///
/// let upper: Uuid = "87284370-2D4B-4E3D-A3F3-F303D2F4F34B".parse().unwrap();
/// let lower: Uuid = "87284370-2d4b-4e3d-a3f3-f303d2f4f34b".parse().unwrap();
/// assert_eq!(upper, lower);
/// assert_ne!(upper.as_str(), lower.as_str()); // but the text is preserved
/// ```
#[derive(Debug, Clone, Eq)]
pub struct Uuid(String);

impl Uuid {
    /// Group lengths of the canonical hexadecimal form.
    const GROUPS: [usize; 5] = [8, 4, 4, 4, 12];

    /// The UUID's lexical form, with its original case.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl PartialEq for Uuid {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl core::hash::Hash for Uuid {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        // Must agree with the case-insensitive PartialEq: two values that
        // compare equal have to hash equal, or a HashMap keyed on OBJECT_ID
        // silently gains a duplicate entry per case variant.
        for b in self.0.bytes() {
            state.write_u8(b.to_ascii_lowercase());
        }
    }
}

impl fmt::Display for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for Uuid {
    type Err = ParseError;

    /// # Errors
    ///
    /// Returns [`ParseError`] if the text is not five hexadecimal groups of
    /// lengths 8-4-4-4-12 separated by hyphens.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut groups = s.split('-');
        for want in Self::GROUPS {
            let Some(group) = groups.next() else {
                return Err(ParseError::new("UUID", "too few groups", s));
            };
            if group.len() != want {
                return Err(ParseError::new("UUID", "group has the wrong length", s));
            }
            if !group.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(ParseError::new("UUID", "group is not hexadecimal", s));
            }
        }
        if groups.next().is_some() {
            return Err(ParseError::new("UUID", "too many groups", s));
        }
        Ok(Self(s.to_owned()))
    }
}

/// An internet domain name used as an identifier, per RFC 1034 with openEHR's
/// widening to permit `_`.
///
/// ```
/// use openehr::base::InternetId;
///
/// assert!("ehr1.nhs.uk".parse::<InternetId>().is_ok());
/// assert!("openehr_test-1.example".parse::<InternetId>().is_ok());
/// assert!("-leading.hyphen".parse::<InternetId>().is_err());
/// assert!("trailing.hyphen-".parse::<InternetId>().is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InternetId(String);

impl InternetId {
    /// The identifier's lexical form.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The dot-separated labels, in order.
    pub fn labels(&self) -> impl Iterator<Item = &str> {
        self.0.split('.')
    }
}

impl fmt::Display for InternetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for InternetId {
    type Err = ParseError;

    /// # Errors
    ///
    /// Returns [`ParseError`] if any label is empty, starts or ends with a
    /// hyphen or underscore, or contains a character outside
    /// `[A-Za-z0-9_-]`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(ParseError::new("INTERNET_ID", "empty", s));
        }
        for label in s.split('.') {
            if label.is_empty() {
                return Err(ParseError::new("INTERNET_ID", "empty label", s));
            }
            if !label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            {
                return Err(ParseError::new(
                    "INTERNET_ID",
                    "label has a character outside [A-Za-z0-9_-]",
                    s,
                ));
            }
            let first = label.as_bytes()[0];
            let last = label.as_bytes()[label.len() - 1];
            if !first.is_ascii_alphanumeric() || !last.is_ascii_alphanumeric() {
                return Err(ParseError::new(
                    "INTERNET_ID",
                    "label starts or ends with a separator",
                    s,
                ));
            }
        }
        Ok(Self(s.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oid_wins_over_internet_id_for_all_digit_text() {
        // The ordering argument in the module header, as a test: if this
        // flipped, every issuing-authority OID would classify as a hostname.
        let uid: Uid = "2.16.840".parse().unwrap();
        assert!(matches!(uid, Uid::IsoOid(_)));
    }

    #[test]
    fn uuid_case_is_preserved_but_not_significant() {
        let a: Uid = "87284370-2D4B-4e3d-A3F3-F303D2F4F34B".parse().unwrap();
        let b: Uid = "87284370-2d4b-4e3d-a3f3-f303d2f4f34b".parse().unwrap();
        assert_eq!(a, b);
        assert_eq!(a.value(), "87284370-2D4B-4e3d-A3F3-F303D2F4F34B");
    }

    #[test]
    fn uuid_hash_agrees_with_eq() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(
            "87284370-2D4B-4E3D-A3F3-F303D2F4F34B"
                .parse::<Uuid>()
                .unwrap(),
        );
        assert!(
            set.contains(
                &"87284370-2d4b-4e3d-a3f3-f303d2f4f34b"
                    .parse::<Uuid>()
                    .unwrap()
            )
        );
    }

    #[test]
    fn rejects_near_misses() {
        assert!("87284370-2D4B-4E3D-A3F3".parse::<Uuid>().is_err());
        assert!(
            "87284370-2D4B-4E3D-A3F3-F303D2F4F34B-0"
                .parse::<Uuid>()
                .is_err()
        );
        assert!(
            "87284370_2D4B_4E3D_A3F3_F303D2F4F34B"
                .parse::<Uuid>()
                .is_err()
        );
        assert!("".parse::<Uid>().is_err());
    }
}
