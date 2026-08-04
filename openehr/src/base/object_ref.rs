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

    /// An `OBJECT_REF` reports every attribute it was built with, and knows
    /// its own locality.
    ///
    /// `is_local` had three surviving mutants (`lib:A-09`): the comparison
    /// could invert or become a constant. This is the flag an access-control
    /// decision reads first — a reference into this system's own identifier
    /// space is one this system can resolve and enforce policy on; a foreign
    /// one is not. `namespace` and `type_name` could also each answer a
    /// constant, and `Display` could print nothing.
    #[test]
    fn an_object_ref_reports_its_locality_and_every_field() {
        let uid = HierObjectId::from_uid_str("6BA7B810-9DAD-11D1-80B4-00C04FD430C8").unwrap();
        let local = ObjectRef::new(
            "local",
            "VERSIONED_COMPOSITION",
            ObjectId::HierObjectId(uid.clone()),
        )
        .unwrap();
        assert!(local.is_local());
        assert_eq!(local.namespace(), "local");
        assert_eq!(local.type_name(), "VERSIONED_COMPOSITION");
        assert_eq!(local.to_string(), format!("local:{uid}"));

        // A different namespace is not local, and a different type name must
        // read back as itself and not as the first reference's.
        let foreign = ObjectRef::new(
            "other.example.org",
            "EHR",
            ObjectId::HierObjectId(uid.clone()),
        )
        .unwrap();
        assert!(
            !foreign.is_local(),
            "a reference into a foreign namespace was reported local"
        );
        assert_ne!(foreign.namespace(), local.namespace());
        assert_ne!(foreign.type_name(), local.type_name());
        assert_ne!(foreign.to_string(), local.to_string());

        // An empty type name is refused.
        assert!(ObjectRef::new("local", "", ObjectId::HierObjectId(uid)).is_err());
    }

    /// A `PARTY_REF` is constrained to the demographic classes, and reports
    /// them back.
    ///
    /// `namespace` and `type_name` could each be a constant, and `Display`
    /// could print nothing. `PERMITTED_TYPES` is enforced at construction
    /// (`lib:S1.x`-adjacent: a wrong reference should fail to compile a valid
    /// record, not need fixing later), so what the accessor reports is exactly
    /// what the constructor already checked.
    #[test]
    fn a_party_ref_is_constrained_and_reports_what_it_was_built_with() {
        let uid = HierObjectId::from_uid_str("6BA7B810-9DAD-11D1-80B4-00C04FD430C8").unwrap();
        for class in PartyRef::PERMITTED_TYPES {
            let r = PartyRef::new("demographic", class, ObjectId::HierObjectId(uid.clone()))
                .unwrap_or_else(|e| panic!("{class}: {e}"));
            assert_eq!(r.type_name(), class, "a permitted class was misreported");
        }
        let r = PartyRef::new("demographic", "PERSON", ObjectId::HierObjectId(uid.clone())).unwrap();
        assert_eq!(r.namespace(), "demographic");
        assert_eq!(r.to_string(), format!("demographic:{uid}"));

        let other = PartyRef::new("registry", "ORGANISATION", ObjectId::HierObjectId(uid.clone())).unwrap();
        assert_ne!(other.namespace(), r.namespace());
        assert_ne!(other.type_name(), r.type_name());
        assert_ne!(other.to_string(), r.to_string());

        // A class outside the demographic model does not compile a valid
        // reference — a `COMPOSITION` is not a party.
        assert!(PartyRef::new("demographic", "COMPOSITION", ObjectId::HierObjectId(uid)).is_err());
    }

    /// A `LOCATABLE_REF` reports its namespace and type, and its `Display`
    /// prints the full URI.
    ///
    /// `namespace` and `type_name` could each be a constant, and `Display`
    /// could print nothing — this is the reference an `ATTESTATION` uses to
    /// point at one `ELEMENT` rather than a whole `COMPOSITION`, so a wrong
    /// namespace or type points the attestation at the wrong record.
    #[test]
    fn a_locatable_ref_reports_its_fields_and_renders_its_uri() {
        let id = UidBasedId::from(
            "6BA7B810-9DAD-11D1-80B4-00C04FD430C8"
                .parse::<HierObjectId>()
                .unwrap(),
        );
        let r = LocatableRef::new(
            "local",
            "VERSIONED_COMPOSITION",
            id.clone(),
            Some("/content[at0001]".to_owned()),
        )
        .unwrap();
        assert_eq!(r.namespace(), "local");
        assert_eq!(r.type_name(), "VERSIONED_COMPOSITION");
        assert_eq!(r.path(), Some("/content[at0001]"));
        assert_eq!(r.to_string(), r.uri());
        assert!(r.to_string().contains("/content[at0001]"));

        let other = LocatableRef::new("local", "EHR", id, None).unwrap();
        assert_ne!(other.type_name(), r.type_name());
        assert_ne!(other.to_string(), r.to_string());
    }

    /// An `ACCESS_GROUP_REF` always names `ACCESS_GROUP`, whatever the mutant
    /// says.
    ///
    /// `namespace` and `type_name` could each be a constant, and `Display`
    /// could print nothing. This is what `EHR_ACCESS.GroupSettings` names a
    /// group of subjects by (`lib:A-09` and the `access.rs` round earlier).
    #[test]
    fn an_access_group_ref_reports_its_namespace_and_fixed_type() {
        let uid = HierObjectId::from_uid_str("6BA7B810-9DAD-11D1-80B4-00C04FD430C8").unwrap();
        let group = AccessGroupRef::new("local", ObjectId::HierObjectId(uid.clone())).unwrap();
        assert_eq!(group.namespace(), "local");
        assert_eq!(group.type_name(), "ACCESS_GROUP");

        let other = AccessGroupRef::new("registry", ObjectId::HierObjectId(uid)).unwrap();
        assert_ne!(other.namespace(), group.namespace());
    }
}
