//! The openEHR **Demographic Information Model**: people, organisations,
//! roles, and the relationships between them.
//!
//! ```text
//! PARTY ─┬─ ACTOR ─┬─ PERSON        PARTY_IDENTITY   CONTACT ── ADDRESS
//!        │         ├─ ORGANISATION  CAPABILITY       PARTY_RELATIONSHIP
//!        │         ├─ GROUP
//!        │         └─ AGENT
//!        └─ ROLE
//! ```
//!
//! # Why this is a separate model and not attributes on the record
//!
//! The EHR references parties by [`crate::base::PartyRef`] and never embeds
//! them. That is a privacy boundary as much as a modelling one: a clinical
//! extract can be shipped with the demographic service left behind, and what
//! travels is a record about `local:PERSON:87284370-…` rather than a record
//! about a named individual. Collapsing demographics into the EHR removes the
//! ability to make that separation later, when it is needed and expensive.
//!
//! # `ROLE` is a party, not an attribute of one
//!
//! openEHR models "consultant cardiologist at St Elsewhere" as a `ROLE` with
//! its own identity and its own validity period, performed by a `PERSON`. The
//! reason is that clinical statements attribute to the *role*: an entry signed
//! by the on-call registrar stays attributed to that role after the person
//! moves on, and the role's `time_validity` is what makes "was this person
//! entitled to sign this in 2019?" answerable.

use crate::base::{Interval, PartyRef};
use crate::error::ParseError;
use crate::rm::common::{Locatable, LocatableAttrs, impl_locatable};
use crate::rm::data_structures::ItemStructure;
use crate::rm::data_types::{DvDate, DvText};
use serde::{Deserialize, Serialize};

/// A period during which something is valid, in whole days.
pub type DateValidity = Interval<DvDate>;

/// A name a party is known by.
///
/// The details are an archetyped [`ItemStructure`] rather than fixed fields,
/// because "a name" is not the same shape in every culture: given/family
/// splits, patronymics, generational suffixes, and mononyms are all legitimate
/// and a fixed schema excludes some of them. openEHR pushes the structure into
/// archetypes so that a jurisdiction can constrain it without a model change.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartyIdentity {
    #[serde(flatten)]
    locatable: LocatableAttrs,
    details: ItemStructure,
}

impl_locatable!(PartyIdentity, "PARTY_IDENTITY");

impl PartyIdentity {
    /// Builds an identity.
    #[must_use]
    pub fn new(locatable: LocatableAttrs, details: ItemStructure) -> Self {
        Self { locatable, details }
    }

    /// The name's parts.
    #[must_use]
    pub fn details(&self) -> &ItemStructure {
        &self.details
    }

    /// The identity's purpose, taken from its `name` — `legal identity`,
    /// `preferred name`, `alias`.
    #[must_use]
    pub fn purpose(&self) -> &str {
        self.name().value()
    }
}

/// One way of reaching a party.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Address {
    #[serde(flatten)]
    locatable: LocatableAttrs,
    details: ItemStructure,
}

impl_locatable!(Address, "ADDRESS");

impl Address {
    /// Builds an address.
    #[must_use]
    pub fn new(locatable: LocatableAttrs, details: ItemStructure) -> Self {
        Self { locatable, details }
    }

    /// The address's parts.
    #[must_use]
    pub fn details(&self) -> &ItemStructure {
        &self.details
    }
}

/// A set of addresses valid over a period — "work", "home until 2024".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contact {
    #[serde(flatten)]
    locatable: LocatableAttrs,
    addresses: Vec<Address>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    time_validity: Option<DateValidity>,
}

impl_locatable!(Contact, "CONTACT");

impl Contact {
    /// Builds a contact.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the address list is empty
    /// (`Addresses_valid`). A contact with no address is a label for a way of
    /// reaching someone that does not say how.
    pub fn new(locatable: LocatableAttrs, addresses: Vec<Address>) -> Result<Self, ParseError> {
        if addresses.is_empty() {
            return Err(ParseError::invariant("CONTACT", "Addresses_valid"));
        }
        Ok(Self {
            locatable,
            addresses,
            time_validity: None,
        })
    }

    /// Records when the contact is valid.
    #[must_use]
    pub fn with_time_validity(mut self, validity: DateValidity) -> Self {
        self.time_validity = Some(validity);
        self
    }

    /// The addresses.
    #[must_use]
    pub fn addresses(&self) -> &[Address] {
        &self.addresses
    }

    /// When the contact is valid.
    #[must_use]
    pub fn time_validity(&self) -> Option<&DateValidity> {
        self.time_validity.as_ref()
    }

    /// Whether the contact is valid on a given date.
    ///
    /// A contact with no recorded validity is treated as always valid, which
    /// is openEHR's reading of an absent interval. A contact whose validity is
    /// recorded but not comparable with `date` — differing date precision —
    /// answers `false`, because "we cannot tell" must not become "yes" for
    /// something used to reach a patient.
    #[must_use]
    pub fn is_valid_on(&self, date: &DvDate) -> bool {
        self.time_validity.as_ref().is_none_or(|v| v.contains(date))
    }
}

/// A competency a role is qualified for, with its credentials.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Capability {
    #[serde(flatten)]
    locatable: LocatableAttrs,
    credentials: ItemStructure,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    time_validity: Option<DateValidity>,
}

impl_locatable!(Capability, "CAPABILITY");

impl Capability {
    /// Builds a capability.
    #[must_use]
    pub fn new(locatable: LocatableAttrs, credentials: ItemStructure) -> Self {
        Self {
            locatable,
            credentials,
            time_validity: None,
        }
    }

    /// Records when the capability is valid.
    ///
    /// This is a registration period — a licence, a certification — and it
    /// expires. A capability with no validity period asserts a qualification
    /// that never lapses, which is not how any clinical registration works.
    #[must_use]
    pub fn with_time_validity(mut self, validity: DateValidity) -> Self {
        self.time_validity = Some(validity);
        self
    }

    /// The credentials.
    #[must_use]
    pub fn credentials(&self) -> &ItemStructure {
        &self.credentials
    }

    /// When the capability is valid.
    #[must_use]
    pub fn time_validity(&self) -> Option<&DateValidity> {
        self.time_validity.as_ref()
    }

    /// Whether the capability was valid on a given date.
    ///
    /// Returns `None` when no validity period is recorded — *not* `true`. The
    /// question this method answers is usually "was this clinician registered
    /// when they signed?", and an unrecorded period is an unanswered question,
    /// not a yes.
    #[must_use]
    pub fn was_valid_on(&self, date: &DvDate) -> Option<bool> {
        self.time_validity.as_ref().map(|v| v.contains(date))
    }
}

/// A directional relationship between two parties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartyRelationship {
    #[serde(flatten)]
    locatable: LocatableAttrs,
    source: PartyRef,
    target: PartyRef,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    details: Option<ItemStructure>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    time_validity: Option<DateValidity>,
}

impl_locatable!(PartyRelationship, "PARTY_RELATIONSHIP");

impl PartyRelationship {
    /// Builds a relationship.
    #[must_use]
    pub fn new(locatable: LocatableAttrs, source: PartyRef, target: PartyRef) -> Self {
        Self {
            locatable,
            source,
            target,
            details: None,
            time_validity: None,
        }
    }

    /// The party the relationship is stored with.
    #[must_use]
    pub fn source(&self) -> &PartyRef {
        &self.source
    }

    /// The other party.
    #[must_use]
    pub fn target(&self) -> &PartyRef {
        &self.target
    }

    /// Archetyped relationship details.
    #[must_use]
    pub fn details(&self) -> Option<&ItemStructure> {
        self.details.as_ref()
    }

    /// When the relationship is valid.
    #[must_use]
    pub fn time_validity(&self) -> Option<&DateValidity> {
        self.time_validity.as_ref()
    }
}

/// The attributes every `PARTY` carries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartyAttrs {
    #[serde(flatten)]
    locatable: LocatableAttrs,
    identities: Vec<PartyIdentity>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    contacts: Vec<Contact>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    details: Option<ItemStructure>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    relationships: Vec<PartyRelationship>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    reverse_relationships: Vec<crate::base::LocatableRef>,
}

impl PartyAttrs {
    /// Builds party attributes.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the identity list is empty
    /// (`Identities_valid`), or if the locatable has no `uid` (`Uid_valid`).
    /// The `uid` requirement is openEHR's and it is load-bearing: every EHR
    /// reference into the demographic model is by uid, so a party without one
    /// is unreachable from any clinical record.
    pub fn new(
        locatable: LocatableAttrs,
        identities: Vec<PartyIdentity>,
    ) -> Result<Self, ParseError> {
        if identities.is_empty() {
            return Err(ParseError::invariant("PARTY", "Identities_valid"));
        }
        let attrs = Self {
            locatable,
            identities,
            contacts: Vec::new(),
            details: None,
            relationships: Vec::new(),
            reverse_relationships: Vec::new(),
        };
        if !attrs.locatable.has_uid() {
            return Err(ParseError::invariant("PARTY", "Uid_valid"));
        }
        Ok(attrs)
    }

    /// Adds a contact.
    #[must_use]
    pub fn with_contact(mut self, contact: Contact) -> Self {
        self.contacts.push(contact);
        self
    }

    /// Adds a relationship.
    #[must_use]
    pub fn with_relationship(mut self, relationship: PartyRelationship) -> Self {
        self.relationships.push(relationship);
        self
    }

    /// The party's names.
    #[must_use]
    pub fn identities(&self) -> &[PartyIdentity] {
        &self.identities
    }

    /// Ways of reaching the party.
    #[must_use]
    pub fn contacts(&self) -> &[Contact] {
        &self.contacts
    }

    /// Archetyped demographic detail.
    #[must_use]
    pub fn details(&self) -> Option<&ItemStructure> {
        self.details.as_ref()
    }

    /// Relationships stored with this party.
    #[must_use]
    pub fn relationships(&self) -> &[PartyRelationship] {
        &self.relationships
    }
}

/// The attributes every `ACTOR` adds to `PARTY`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ActorAttrs {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    languages: Vec<DvText>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    roles: Vec<PartyRef>,
}

impl ActorAttrs {
    /// Adds a language the actor communicates in.
    #[must_use]
    pub fn with_language(mut self, language: DvText) -> Self {
        self.languages.push(language);
        self
    }

    /// Adds a role the actor performs.
    #[must_use]
    pub fn with_role(mut self, role: PartyRef) -> Self {
        self.roles.push(role);
        self
    }

    /// Languages the actor communicates in.
    #[must_use]
    pub fn languages(&self) -> &[DvText] {
        &self.languages
    }

    /// Roles the actor performs.
    #[must_use]
    pub fn roles(&self) -> &[PartyRef] {
        &self.roles
    }
}

macro_rules! actor {
    (
        $(#[$attr:meta])*
        $ty:ident, $class:literal
    ) => {
        $(#[$attr])*
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub struct $ty {
            #[serde(flatten)]
            party: PartyAttrs,
            #[serde(flatten)]
            actor: ActorAttrs,
        }

        impl $ty {
            /// Builds the actor.
            #[must_use]
            pub fn new(party: PartyAttrs) -> Self {
                Self {
                    party,
                    actor: ActorAttrs::default(),
                }
            }

            /// Sets the actor attributes.
            #[must_use]
            pub fn with_actor_attrs(mut self, actor: ActorAttrs) -> Self {
                self.actor = actor;
                self
            }

            /// The party attributes.
            #[must_use]
            pub fn party(&self) -> &PartyAttrs {
                &self.party
            }

            /// The actor attributes.
            #[must_use]
            pub fn actor(&self) -> &ActorAttrs {
                &self.actor
            }
        }

        impl $crate::rm::common::Locatable for $ty {
            fn locatable(&self) -> &LocatableAttrs {
                &self.party.locatable
            }

            fn rm_type_name(&self) -> &'static str {
                $class
            }
        }
    };
}

actor! {
    /// A human being.
    Person, "PERSON"
}

actor! {
    /// A legally constituted body that outlives its members.
    Organisation, "ORGANISATION"
}

actor! {
    /// A set of parties assembled for a purpose — a clinical team, a ward
    /// round.
    ///
    /// Distinct from [`Organisation`] because a group is created by another
    /// party and has no independent legal existence. "The stroke MDT" is a
    /// group; "St Elsewhere NHS Trust" is an organisation.
    Group, "GROUP"
}

actor! {
    /// A non-human participant: a device, a piece of software, a service.
    ///
    /// Modelled as an actor because it can perform roles and be attributed to.
    /// An automated ECG interpretation is authored by an `AGENT`, and recording
    /// it as authored by the ordering clinician would misattribute a machine
    /// reading to a person.
    Agent, "AGENT"
}

/// A competency performed by an [`ActorAttrs`]-bearing party.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Role {
    #[serde(flatten)]
    party: PartyAttrs,
    performer: PartyRef,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    capabilities: Vec<Capability>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    time_validity: Option<DateValidity>,
}

impl Role {
    /// Builds a role.
    #[must_use]
    pub fn new(party: PartyAttrs, performer: PartyRef) -> Self {
        Self {
            party,
            performer,
            capabilities: Vec::new(),
            time_validity: None,
        }
    }

    /// Adds a capability.
    #[must_use]
    pub fn with_capability(mut self, capability: Capability) -> Self {
        self.capabilities.push(capability);
        self
    }

    /// Records when the role is held.
    #[must_use]
    pub fn with_time_validity(mut self, validity: DateValidity) -> Self {
        self.time_validity = Some(validity);
        self
    }

    /// Who performs the role.
    #[must_use]
    pub fn performer(&self) -> &PartyRef {
        &self.performer
    }

    /// What the role is qualified for.
    #[must_use]
    pub fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    /// When the role is held.
    #[must_use]
    pub fn time_validity(&self) -> Option<&DateValidity> {
        self.time_validity.as_ref()
    }

    /// The party attributes.
    #[must_use]
    pub fn party(&self) -> &PartyAttrs {
        &self.party
    }

    /// Whether the role was held on a given date.
    ///
    /// `None` when no period is recorded, for the same reason as
    /// [`Capability::was_valid_on`]: this answers "was this person the on-call
    /// registrar at the time?", and silence is not a yes.
    #[must_use]
    pub fn was_held_on(&self, date: &DvDate) -> Option<bool> {
        self.time_validity.as_ref().map(|v| v.contains(date))
    }
}

impl Locatable for Role {
    fn locatable(&self) -> &LocatableAttrs {
        &self.party.locatable
    }

    fn rm_type_name(&self) -> &'static str {
        "ROLE"
    }
}

/// Any demographic party.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type")]
// A ROLE carries capabilities and a performer reference that the four actor
// kinds do not. Boxing would allocate per party for no observable difference.
#[allow(clippy::large_enum_variant)]
pub enum Party {
    /// A human being.
    #[serde(rename = "PERSON")]
    Person(Person),
    /// A legally constituted body.
    #[serde(rename = "ORGANISATION")]
    Organisation(Organisation),
    /// A purpose-built set of parties.
    #[serde(rename = "GROUP")]
    Group(Group),
    /// A device or software system.
    #[serde(rename = "AGENT")]
    Agent(Agent),
    /// A competency performed by an actor.
    #[serde(rename = "ROLE")]
    Role(Role),
}

impl Party {
    /// The openEHR class name, as it appears in `_type`.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Person(_) => "PERSON",
            Self::Organisation(_) => "ORGANISATION",
            Self::Group(_) => "GROUP",
            Self::Agent(_) => "AGENT",
            Self::Role(_) => "ROLE",
        }
    }

    /// The party attributes.
    #[must_use]
    pub fn party(&self) -> &PartyAttrs {
        match self {
            Self::Person(p) => p.party(),
            Self::Organisation(p) => p.party(),
            Self::Group(p) => p.party(),
            Self::Agent(p) => p.party(),
            Self::Role(p) => p.party(),
        }
    }

    /// The actor attributes, for the four party kinds that are actors.
    ///
    /// `None` for [`Party::Role`], which is a `PARTY` and not an `ACTOR`: a
    /// role does not itself speak a language or hold roles.
    #[must_use]
    pub fn actor(&self) -> Option<&ActorAttrs> {
        match self {
            Self::Person(p) => Some(p.actor()),
            Self::Organisation(p) => Some(p.actor()),
            Self::Group(p) => Some(p.actor()),
            Self::Agent(p) => Some(p.actor()),
            Self::Role(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::{HierObjectId, ObjectId, UidBasedId};
    use crate::rm::data_structures::{Element, ItemSingle};
    use crate::rm::data_types::{DataValue, DvText};

    fn attrs_with_uid(name: &str, node: &str) -> LocatableAttrs {
        LocatableAttrs::named(name, node)
            .unwrap()
            .with_uid(UidBasedId::HierObjectId(
                HierObjectId::from_uid_str("87284370-2D4B-4E3D-A3F3-F303D2F4F34B").unwrap(),
            ))
    }

    fn identity() -> PartyIdentity {
        PartyIdentity::new(
            LocatableAttrs::named("legal identity", "at0001").unwrap(),
            ItemSingle::new(
                LocatableAttrs::named("structured name", "at0002").unwrap(),
                Element::new(
                    LocatableAttrs::named("family name", "at0003").unwrap(),
                    DataValue::Text(DvText::new("Patient").unwrap()),
                ),
            )
            .into(),
        )
    }

    #[test]
    fn a_party_without_a_uid_is_refused() {
        // Every EHR reference into demographics is by uid, so a party without
        // one is written and then unreachable.
        let no_uid =
            LocatableAttrs::named("Person", "openEHR-DEMOGRAPHIC-PERSON.person.v1").unwrap();
        assert!(PartyAttrs::new(no_uid, vec![identity()]).is_err());
        assert!(
            PartyAttrs::new(
                attrs_with_uid("Person", "openEHR-DEMOGRAPHIC-PERSON.person.v1"),
                vec![identity()],
            )
            .is_ok()
        );
    }

    #[test]
    fn a_party_needs_at_least_one_identity() {
        assert!(
            PartyAttrs::new(
                attrs_with_uid("Person", "openEHR-DEMOGRAPHIC-PERSON.person.v1"),
                Vec::new(),
            )
            .is_err()
        );
    }

    #[test]
    fn an_unrecorded_validity_period_is_not_a_yes() {
        let capability = Capability::new(
            LocatableAttrs::named("GMC registration", "at0001").unwrap(),
            ItemSingle::new(
                LocatableAttrs::named("credentials", "at0002").unwrap(),
                Element::new(
                    LocatableAttrs::named("number", "at0003").unwrap(),
                    DataValue::Text(DvText::new("1234567").unwrap()),
                ),
            )
            .into(),
        );
        let when = DvDate::new("2019-06-01").unwrap();
        // Not Some(true): "was this clinician registered?" with no recorded
        // period is unanswered, and answering yes is how an unregistered
        // signature passes review.
        assert_eq!(capability.was_valid_on(&when), None);

        let dated = capability.with_time_validity(
            Interval::closed(
                DvDate::new("2015-01-01").unwrap(),
                DvDate::new("2020-12-31").unwrap(),
            )
            .unwrap(),
        );
        assert_eq!(dated.was_valid_on(&when), Some(true));
    }

    #[test]
    fn a_role_is_a_party_but_not_an_actor() {
        let role = Role::new(
            PartyAttrs::new(
                attrs_with_uid(
                    "consultant cardiologist",
                    "openEHR-DEMOGRAPHIC-ROLE.role.v1",
                ),
                vec![identity()],
            )
            .unwrap(),
            PartyRef::new(
                "demographic",
                "PERSON",
                ObjectId::HierObjectId(
                    HierObjectId::from_uid_str("11111111-2222-3333-4444-555555555555").unwrap(),
                ),
            )
            .unwrap(),
        );
        assert!(Party::Role(role).actor().is_none());
    }
}
