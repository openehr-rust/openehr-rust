//! `OBJECT_REF` and its descendants: references from one object to another.
//!
//! A reference is three things — a namespace saying *whose* identifier space,
//! a type saying what class to expect, and the identifier itself. All three
//! matter, and the type is the one implementers drop. openEHR keeps it because
//! resolution is a service call: a client holding `PARTY_REF` must know whether
//! to ask the demographic service for a `PERSON` or an `ORGANISATION` before it
//! has the object, and a reference without a type cannot be resolved without
//! trying each in turn.

use crate::base::object_id::{ObjectId, UidBasedId};
use crate::error::ParseError;
use core::fmt;
use serde::{Deserialize, Serialize};

/// The namespace value openEHR reserves for identifiers minted by the system
/// holding the reference.
pub const NAMESPACE_LOCAL: &str = "local";

/// The namespace value openEHR reserves for a reference whose origin is not
/// recorded.
///
/// Distinct from `local`: `unknown` says the namespace was never captured,
/// `local` asserts it is this system's. Collapsing the two turns missing
/// provenance into a claim of provenance.
pub const NAMESPACE_UNKNOWN: &str = "unknown";

/// Validates an `OBJECT_REF` namespace.
///
/// The grammar is `[a-zA-Z][a-zA-Z0-9_.:/&?=+-]*`, plus the two reserved
/// values, which already satisfy it.
///
/// # Errors
///
/// Returns [`ParseError`] if the namespace is empty, starts with a
/// non-alphabetic character, or contains a character outside the permitted set.
pub fn validate_namespace(namespace: &str) -> Result<(), ParseError> {
    let mut bytes = namespace.bytes();
    let Some(first) = bytes.next() else {
        return Err(ParseError::new("OBJECT_REF", "empty namespace", namespace));
    };
    if !first.is_ascii_alphabetic() {
        return Err(ParseError::new(
            "OBJECT_REF",
            "namespace does not start with a letter",
            namespace,
        ));
    }
    if !bytes.all(|b| {
        b.is_ascii_alphanumeric()
            || matches!(
                b,
                b'_' | b'.' | b':' | b'/' | b'&' | b'?' | b'=' | b'+' | b'-'
            )
    }) {
        return Err(ParseError::new(
            "OBJECT_REF",
            "namespace has a character outside [a-zA-Z0-9_.:/&?=+-]",
            namespace,
        ));
    }
    Ok(())
}

/// A reference to an object in some namespace.
///
/// ```
/// use openehr::base::{ObjectRef, ObjectId, HierObjectId};
///
/// let r = ObjectRef::new(
///     "local",
///     "COMPOSITION",
///     ObjectId::HierObjectId(HierObjectId::from_uid_str("87284370-2D4B-4E3D-A3F3-F303D2F4F34B").unwrap()),
/// ).unwrap();
/// assert_eq!(r.namespace(), "local");
/// assert_eq!(r.type_name(), "COMPOSITION");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectRef {
    #[serde(rename = "_type", skip_serializing_if = "Option::is_none", default)]
    declared_type: Option<String>,
    namespace: String,
    #[serde(rename = "type")]
    type_name: String,
    id: ObjectId,
}

impl ObjectRef {
    /// Builds a reference.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the namespace is malformed or the type name is
    /// empty.
    pub fn new(
        namespace: impl Into<String>,
        type_name: impl Into<String>,
        id: ObjectId,
    ) -> Result<Self, ParseError> {
        let namespace = namespace.into();
        let type_name = type_name.into();
        validate_namespace(&namespace)?;
        if type_name.is_empty() {
            return Err(ParseError::new("OBJECT_REF", "empty type", &type_name));
        }
        Ok(Self {
            declared_type: None,
            namespace,
            type_name,
            id,
        })
    }

    /// The identifier space the reference is resolved in.
    #[must_use]
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// The RM class the referent is expected to be, or `ANY`.
    #[must_use]
    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    /// The identifier of the referent.
    #[must_use]
    pub fn id(&self) -> &ObjectId {
        &self.id
    }

    /// Whether the reference points into this system's own identifier space.
    #[must_use]
    pub fn is_local(&self) -> bool {
        self.namespace == NAMESPACE_LOCAL
    }
}

/// A reference to a demographic entity.
///
/// The `type` attribute is constrained: openEHR permits only the six
/// demographic classes plus the abstract `PARTY` and `ACTOR`. That constraint
/// is enforced at construction rather than at validation, because a
/// `PARTY_REF` naming `COMPOSITION` is not a record that needs fixing — it is a
/// call that should not have compiled, and the earlier it fails the less
/// clinical data has been written against it.
///
/// ```
/// use openehr::base::{PartyRef, ObjectId, HierObjectId};
///
/// let id = ObjectId::HierObjectId(HierObjectId::from_uid_str("1.2.826.0.1.3680043.8.1000").unwrap());
/// assert!(PartyRef::new("demographic", "PERSON", id.clone()).is_ok());
/// assert!(PartyRef::new("demographic", "COMPOSITION", id).is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartyRef {
    #[serde(rename = "_type", skip_serializing_if = "Option::is_none", default)]
    declared_type: Option<String>,
    namespace: String,
    #[serde(rename = "type")]
    type_name: String,
    id: ObjectId,
}

impl PartyRef {
    /// The RM class names a `PARTY_REF` may name.
    pub const PERMITTED_TYPES: [&'static str; 8] = [
        "PERSON",
        "ORGANISATION",
        "GROUP",
        "AGENT",
        "ROLE",
        "PARTY",
        "ACTOR",
        "ANY",
    ];

    /// Builds a party reference.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the namespace is malformed or the type is not
    /// one of [`PartyRef::PERMITTED_TYPES`].
    pub fn new(
        namespace: impl Into<String>,
        type_name: impl Into<String>,
        id: ObjectId,
    ) -> Result<Self, ParseError> {
        let namespace = namespace.into();
        let type_name = type_name.into();
        validate_namespace(&namespace)?;
        if !Self::PERMITTED_TYPES.contains(&type_name.as_str()) {
            return Err(ParseError::new(
                "PARTY_REF",
                "type is not a demographic class",
                &type_name,
            ));
        }
        Ok(Self {
            declared_type: None,
            namespace,
            type_name,
            id,
        })
    }

    /// The identifier space the reference is resolved in.
    #[must_use]
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// The demographic class the referent is expected to be.
    #[must_use]
    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    /// The identifier of the referent.
    #[must_use]
    pub fn id(&self) -> &ObjectId {
        &self.id
    }
}

/// A reference to a node *inside* a versioned object.
///
/// The extra attribute over [`ObjectRef`] is `path`: a `LOCATABLE_REF` names
/// the version and then a path within it, which is how an [`crate::rm::common::Attestation`]
/// attests to one `ELEMENT` rather than to a whole `COMPOSITION`.
///
/// ```
/// use openehr::base::{LocatableRef, UidBasedId};
///
/// let uid: UidBasedId = "87284370-2D4B-4E3D-A3F3-F303D2F4F34B::ehr1.example::1".parse().unwrap();
/// let r = LocatableRef::new("local", "COMPOSITION", uid, Some("/content[0]".into())).unwrap();
/// assert_eq!(r.uri(), "local:87284370-2D4B-4E3D-A3F3-F303D2F4F34B::ehr1.example::1/content[0]");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocatableRef {
    #[serde(rename = "_type", skip_serializing_if = "Option::is_none", default)]
    declared_type: Option<String>,
    namespace: String,
    #[serde(rename = "type")]
    type_name: String,
    id: UidBasedId,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    path: Option<String>,
}

impl LocatableRef {
    /// Builds a locatable reference.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the namespace is malformed, the type is empty,
    /// or the path is present but empty. An empty path is refused because
    /// openEHR gives "no path" the meaning *the root object*, and an empty
    /// string would be a second spelling of that with no way to tell them apart
    /// in a URI.
    pub fn new(
        namespace: impl Into<String>,
        type_name: impl Into<String>,
        id: UidBasedId,
        path: Option<String>,
    ) -> Result<Self, ParseError> {
        let namespace = namespace.into();
        let type_name = type_name.into();
        validate_namespace(&namespace)?;
        if type_name.is_empty() {
            return Err(ParseError::new("LOCATABLE_REF", "empty type", &type_name));
        }
        if path.as_ref().is_some_and(String::is_empty) {
            return Err(ParseError::new(
                "LOCATABLE_REF",
                "empty path (omit it to mean the root object)",
                "",
            ));
        }
        Ok(Self {
            declared_type: None,
            namespace,
            type_name,
            id,
            path,
        })
    }

    /// The identifier space the reference is resolved in.
    #[must_use]
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// The RM class the referent is expected to be.
    #[must_use]
    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    /// The identifier of the containing versioned object or version.
    #[must_use]
    pub fn id(&self) -> &UidBasedId {
        &self.id
    }

    /// The path within the referent, or `None` for the root object.
    #[must_use]
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// The reference as a URI: `namespace ':' id [ '/' path ]`.
    ///
    /// openEHR defines this form so a `LOCATABLE_REF` can be written into a
    /// `DV_EHR_URI`. Note that the path already begins with `/`, so this does
    /// not insert a second one.
    #[must_use]
    pub fn uri(&self) -> String {
        match &self.path {
            Some(p) if p.starts_with('/') => format!("{}:{}{p}", self.namespace, self.id),
            Some(p) => format!("{}:{}/{p}", self.namespace, self.id),
            None => format!("{}:{}", self.namespace, self.id),
        }
    }
}

/// A reference to an access-control group.
///
/// Present because `EHR_ACCESS` schemes name groups, and a group id is not a
/// party id: a group is a security principal set, not a demographic entity, and
/// conflating them is how "the cardiology team" ends up modelled as a person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessGroupRef {
    #[serde(rename = "_type", skip_serializing_if = "Option::is_none", default)]
    declared_type: Option<String>,
    namespace: String,
    #[serde(rename = "type")]
    type_name: String,
    id: ObjectId,
}

impl AccessGroupRef {
    /// Builds an access-group reference. The `type` is fixed to `ACCESS_GROUP`.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the namespace is malformed.
    pub fn new(namespace: impl Into<String>, id: ObjectId) -> Result<Self, ParseError> {
        let namespace = namespace.into();
        validate_namespace(&namespace)?;
        Ok(Self {
            declared_type: None,
            namespace,
            type_name: "ACCESS_GROUP".to_owned(),
            id,
        })
    }

    /// The identifier space the reference is resolved in.
    #[must_use]
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Always `ACCESS_GROUP`.
    #[must_use]
    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    /// The identifier of the group.
    #[must_use]
    pub fn id(&self) -> &ObjectId {
        &self.id
    }
}

impl fmt::Display for ObjectRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.namespace, self.id)
    }
}

impl fmt::Display for PartyRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.namespace, self.id)
    }
}

impl fmt::Display for LocatableRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.uri())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::HierObjectId;

    fn an_id() -> ObjectId {
        ObjectId::HierObjectId(
            HierObjectId::from_uid_str("87284370-2D4B-4E3D-A3F3-F303D2F4F34B").unwrap(),
        )
    }

    #[test]
    fn namespace_grammar_is_enforced() {
        assert!(validate_namespace("local").is_ok());
        assert!(validate_namespace("uk.nhs.spine").is_ok());
        assert!(validate_namespace("https://ehr.example/ns").is_ok());
        assert!(validate_namespace("1ns").is_err()); // must start with a letter
        assert!(validate_namespace("").is_err());
        assert!(validate_namespace("a b").is_err());
    }

    #[test]
    fn party_ref_refuses_a_non_demographic_class() {
        assert!(PartyRef::new("local", "OBSERVATION", an_id()).is_err());
        assert!(PartyRef::new("local", "PERSON", an_id()).is_ok());
    }

    #[test]
    fn locatable_ref_uri_does_not_double_the_slash() {
        let uid: UidBasedId = "87284370-2D4B-4E3D-A3F3-F303D2F4F34B::s.example::1"
            .parse()
            .unwrap();
        let with_slash = LocatableRef::new(
            "local",
            "COMPOSITION",
            uid.clone(),
            Some("/content[0]".into()),
        )
        .unwrap();
        let without =
            LocatableRef::new("local", "COMPOSITION", uid, Some("content[0]".into())).unwrap();
        assert_eq!(with_slash.uri(), without.uri());
    }
}
