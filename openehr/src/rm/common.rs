//! The openEHR **Common Information Model**: archetyping, parties,
//! participation, audit, and change control.
//!
//! This module holds the two ideas that make openEHR openEHR.
//!
//! **Archetyping.** Every clinical node is a [`Locatable`]: it carries a
//! runtime `name`, a design-time `archetype_node_id`, and — at archetype roots
//! — an [`Archetyped`] saying which archetype and template shaped it. That is
//! what lets one `COMPOSITION` be read by software that has never seen the
//! archetype it was built from.
//!
//! **Change control.** Nothing in an openEHR record is updated in place. A
//! [`VersionedObject`] holds a tree of [`Version`]s; each version carries the
//! [`AuditDetails`] of its commit; a [`Contribution`] groups the versions
//! committed together. Deletion is a version whose `data` is absent and whose
//! `lifecycle_state` is `deleted` — the record of the deletion is itself a
//! record.
//!
//! # What this module enforces that a plain struct would not
//!
//! [`VersionedObject::commit`] refuses a version whose `uid` names a different
//! versioned object, whose `preceding_version_uid` is not the current latest,
//! or whose version id already exists. Each of those produces a history that
//! *reads back* — it just does not connect, and it does not connect in a way
//! that is invisible until someone asks what changed.

use crate::base::{
    ArchetypeId, HierObjectId, Interval, LocatableRef, ObjectRef, ObjectVersionId, PartyRef,
    TemplateId, UidBasedId,
};
use crate::error::ParseError;
use crate::rm::data_types::{
    DataValue, DvCodedText, DvDate, DvDateTime, DvEhrUri, DvIdentifier, DvMultimedia, Text,
};
use crate::terminology;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Archetyping
// ---------------------------------------------------------------------------

/// The archetype and template that shaped a node.
///
/// Present only at archetype roots — that is what `is_archetype_root` means.
/// An `ARCHETYPED` on every node would be a lie about where the archetype
/// boundaries are, and the boundaries are what a template composes.
///
/// ```
/// use openehr::rm::common::Archetyped;
///
/// let a = Archetyped::new("openEHR-EHR-OBSERVATION.blood_pressure.v2", "1.1.0").unwrap();
/// assert!(a.archetype_id().constrains("OBSERVATION"));
/// assert_eq!(a.rm_version(), "1.1.0");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Archetyped {
    archetype_id: ArchetypeId,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    template_id: Option<TemplateId>,
    rm_version: String,
}

impl Archetyped {
    /// Builds archetype details.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the archetype id is malformed or the RM
    /// version is empty. The RM version matters: it says which release's
    /// invariants the data was built against, and data built against 1.0.2
    /// can contain things 1.1.0 forbids.
    pub fn new(archetype_id: &str, rm_version: impl Into<String>) -> Result<Self, ParseError> {
        let rm_version = rm_version.into();
        if rm_version.is_empty() {
            return Err(ParseError::invariant("ARCHETYPED", "Rm_version_valid"));
        }
        Ok(Self {
            archetype_id: archetype_id.parse()?,
            template_id: None,
            rm_version,
        })
    }

    /// Records the template that composed this archetype into a form.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the template id is malformed.
    pub fn with_template(mut self, template_id: &str) -> Result<Self, ParseError> {
        self.template_id = Some(template_id.parse()?);
        Ok(self)
    }

    /// The archetype.
    #[must_use]
    pub fn archetype_id(&self) -> &ArchetypeId {
        &self.archetype_id
    }

    /// The template, if one was used.
    #[must_use]
    pub fn template_id(&self) -> Option<&TemplateId> {
        self.template_id.as_ref()
    }

    /// The openEHR release the data was built against.
    #[must_use]
    pub fn rm_version(&self) -> &str {
        &self.rm_version
    }
}

/// A typed relationship from one archetyped structure to another.
///
/// `LINK.target` is a [`DvEhrUri`] and not a free URI: a link inside a health
/// record points inside a health record. See [`crate::rm::data_types::uri`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// `link_type` shadows the struct name only because openEHR calls the attribute
// `type`, which is a Rust keyword. Renaming it to something that reads better
// in Rust would break the correspondence with the class definition.
#[allow(clippy::struct_field_names)]
pub struct Link {
    meaning: Text,
    #[serde(rename = "type")]
    link_type: Text,
    target: DvEhrUri,
}

impl Link {
    /// Builds a link.
    #[must_use]
    pub fn new(meaning: Text, link_type: Text, target: DvEhrUri) -> Self {
        Self {
            meaning,
            link_type,
            target,
        }
    }

    /// What the relationship means clinically.
    #[must_use]
    pub fn meaning(&self) -> &Text {
        &self.meaning
    }

    /// The relationship's domain-level classification, such as `problem` or
    /// `issue`.
    #[must_use]
    pub fn link_type(&self) -> &Text {
        &self.link_type
    }

    /// What the link points at.
    #[must_use]
    pub fn target(&self) -> &DvEhrUri {
        &self.target
    }
}

/// What one system in a feeder chain did with the data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeederAuditDetails {
    system_id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    location: Option<PartyIdentified>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    subject: Option<PartyProxy>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    provider: Option<PartyIdentified>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    time: Option<DvDateTime>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    version_id: Option<String>,
}

impl FeederAuditDetails {
    /// Builds feeder audit details.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the system id is empty. A feeder audit whose
    /// system id is blank records that data came from somewhere without saying
    /// where, which is the one fact it exists to carry.
    pub fn new(system_id: impl Into<String>) -> Result<Self, ParseError> {
        let system_id = system_id.into();
        if system_id.is_empty() {
            return Err(ParseError::invariant(
                "FEEDER_AUDIT_DETAILS",
                "System_id_valid",
            ));
        }
        Ok(Self {
            system_id,
            location: None,
            subject: None,
            provider: None,
            time: None,
            version_id: None,
        })
    }

    /// Records when the system handled the data.
    #[must_use]
    pub fn with_time(mut self, time: DvDateTime) -> Self {
        self.time = Some(time);
        self
    }

    /// The system's identifier.
    #[must_use]
    pub fn system_id(&self) -> &str {
        &self.system_id
    }

    /// When the system handled the data.
    #[must_use]
    pub fn time(&self) -> Option<&DvDateTime> {
        self.time.as_ref()
    }

    /// The data's version in the originating system.
    #[must_use]
    pub fn version_id(&self) -> Option<&str> {
        self.version_id.as_deref()
    }
}

/// Provenance for data that was converted from a non-openEHR system.
///
/// The reason this exists rather than a note in `other_details`: converted data
/// is the data most likely to be wrong, and the conversion is the most likely
/// place for it to have gone wrong. A `FEEDER_AUDIT` keeps the original
/// content and the identifiers it had, so a discrepancy found years later can
/// be traced to the transform rather than argued about.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeederAudit {
    originating_system_audit: FeederAuditDetails,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    originating_system_item_ids: Vec<DvIdentifier>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    feeder_system_audit: Option<FeederAuditDetails>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    feeder_system_item_ids: Vec<DvIdentifier>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    original_content: Option<DataValue>,
}

impl FeederAudit {
    /// Builds a feeder audit.
    #[must_use]
    pub fn new(originating_system_audit: FeederAuditDetails) -> Self {
        Self {
            originating_system_audit,
            originating_system_item_ids: Vec::new(),
            feeder_system_audit: None,
            feeder_system_item_ids: Vec::new(),
            original_content: None,
        }
    }

    /// Keeps the pre-conversion content.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] unless the content is a `DV_ENCAPSULATED`
    /// descendant — `DV_PARSABLE` or `DV_MULTIMEDIA`. openEHR types this
    /// attribute `DV_ENCAPSULATED` because the original content is by
    /// definition in a formalism openEHR does not model; accepting a
    /// `DV_TEXT` here would invite a lossy stringification of the very thing
    /// that exists to be lossless.
    pub fn with_original_content(mut self, content: DataValue) -> Result<Self, ParseError> {
        if !matches!(content, DataValue::Parsable(_) | DataValue::Multimedia(_)) {
            return Err(ParseError::invariant(
                "FEEDER_AUDIT",
                "Original_content_encapsulated",
            ));
        }
        self.original_content = Some(content);
        Ok(self)
    }

    /// Records the identifiers the data had in the originating system.
    #[must_use]
    pub fn with_originating_item_id(mut self, id: DvIdentifier) -> Self {
        self.originating_system_item_ids.push(id);
        self
    }

    /// The originating system's audit details.
    #[must_use]
    pub fn originating_system_audit(&self) -> &FeederAuditDetails {
        &self.originating_system_audit
    }

    /// The identifiers the data had in the originating system.
    #[must_use]
    pub fn originating_system_item_ids(&self) -> &[DvIdentifier] {
        &self.originating_system_item_ids
    }

    /// The pre-conversion content, if kept.
    #[must_use]
    pub fn original_content(&self) -> Option<&DataValue> {
        self.original_content.as_ref()
    }
}

/// The attributes every `LOCATABLE` carries.
///
/// Held as one struct and flattened into each clinical class, mirroring
/// openEHR's own placement of them on the abstract parent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocatableAttrs {
    name: Text,
    archetype_node_id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    uid: Option<UidBasedId>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    links: Vec<Link>,
    // Boxed for deserializer frame size, not for ownership. `LocatableAttrs`
    // is flattened into every clinical class, so serde generates one
    // `visit_map` per embedding class holding an `Option<T>` local per field.
    // `FEEDER_AUDIT` transitively contains a `DATA_VALUE` — a 22-variant enum
    // — and it is present on well under 1% of nodes. Boxing trades a rare
    // allocation for a stack cost paid on every node of every document.
    // See spec/audit.md A-03 for the measurements.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    archetype_details: Option<Box<Archetyped>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    feeder_audit: Option<Box<FeederAudit>>,
}

impl LocatableAttrs {
    /// Builds the locatable attributes.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if `archetype_node_id` is empty. Every locatable
    /// has one — an at-code such as `at0004` for an internal node, or a full
    /// archetype id at a root — and a node without one cannot be addressed by
    /// a path, which makes it unreachable by AQL and by templates alike.
    pub fn new(name: Text, archetype_node_id: impl Into<String>) -> Result<Self, ParseError> {
        let archetype_node_id = archetype_node_id.into();
        if archetype_node_id.is_empty() {
            return Err(ParseError::invariant(
                "LOCATABLE",
                "Archetype_node_id_valid",
            ));
        }
        Ok(Self {
            name,
            archetype_node_id,
            uid: None,
            links: Vec::new(),
            archetype_details: None,
            feeder_audit: None,
        })
    }

    /// Builds locatable attributes with a plain-text name.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the name is empty or the node id is empty.
    pub fn named(name: &str, archetype_node_id: &str) -> Result<Self, ParseError> {
        Self::new(Text::plain(name)?, archetype_node_id)
    }

    /// Marks this node as an archetype root.
    #[must_use]
    pub fn with_archetype_details(mut self, details: Archetyped) -> Self {
        self.archetype_details = Some(Box::new(details));
        self
    }

    /// Gives the node a persistent identifier.
    #[must_use]
    pub fn with_uid(mut self, uid: UidBasedId) -> Self {
        self.uid = Some(uid);
        self
    }

    /// Adds a link to another archetyped structure.
    #[must_use]
    pub fn with_link(mut self, link: Link) -> Self {
        self.links.push(link);
        self
    }

    /// Records where the data came from.
    #[must_use]
    pub fn with_feeder_audit(mut self, audit: FeederAudit) -> Self {
        self.feeder_audit = Some(Box::new(audit));
        self
    }

    /// Whether the node carries a persistent identifier.
    ///
    /// Optional on most locatables and **required** on a `PARTY`, which is why
    /// this is a question the attributes can answer directly rather than only
    /// through the [`Locatable`] trait.
    #[must_use]
    pub fn has_uid(&self) -> bool {
        self.uid.is_some()
    }

    /// The node's runtime name.
    #[must_use]
    pub fn name(&self) -> &Text {
        &self.name
    }

    /// The node's design-time code.
    #[must_use]
    pub fn archetype_node_id(&self) -> &str {
        &self.archetype_node_id
    }

    /// The node's persistent identifier, if it has one.
    #[must_use]
    pub fn uid(&self) -> Option<&UidBasedId> {
        self.uid.as_ref()
    }
}

/// Read access to the attributes of `LOCATABLE`.
///
/// Implemented by every clinical class. The trait is what lets path
/// navigation, validation, and redaction walk a composition without a match
/// arm per class.
pub trait Locatable {
    /// The `LOCATABLE` attributes.
    fn locatable(&self) -> &LocatableAttrs;

    /// The openEHR class name, as it appears in `_type`.
    fn rm_type_name(&self) -> &'static str;

    /// The runtime name of the node — what a clinician sees as its label.
    fn name(&self) -> &Text {
        &self.locatable().name
    }

    /// The design-time code: an at-code, or an archetype id at a root.
    fn archetype_node_id(&self) -> &str {
        &self.locatable().archetype_node_id
    }

    /// The node's persistent identifier, if it has one.
    fn uid(&self) -> Option<&UidBasedId> {
        self.locatable().uid.as_ref()
    }

    /// Links to other archetyped structures.
    fn links(&self) -> &[Link] {
        &self.locatable().links
    }

    /// The archetype details, present only at archetype roots.
    fn archetype_details(&self) -> Option<&Archetyped> {
        self.locatable().archetype_details.as_deref()
    }

    /// Provenance for converted data, if any.
    fn feeder_audit(&self) -> Option<&FeederAudit> {
        self.locatable().feeder_audit.as_deref()
    }

    /// Whether this node is the root of an archetype.
    fn is_archetype_root(&self) -> bool {
        self.locatable().archetype_details.is_some()
    }

    /// The clinical concept this node expresses.
    ///
    /// openEHR defines `concept` as the name of the *archetype root*, so this
    /// answers only at a root and returns `None` elsewhere rather than
    /// offering an internal node's name as though it were a concept.
    fn concept(&self) -> Option<&Text> {
        self.is_archetype_root().then(|| self.name())
    }
}

/// Implements [`Locatable`] for a struct with a `locatable: LocatableAttrs`
/// field.
macro_rules! impl_locatable {
    ($ty:ty, $class:literal) => {
        impl $crate::rm::common::Locatable for $ty {
            fn locatable(&self) -> &$crate::rm::common::LocatableAttrs {
                &self.locatable
            }

            fn rm_type_name(&self) -> &'static str {
                $class
            }
        }
    };
}

pub(crate) use impl_locatable;

// ---------------------------------------------------------------------------
// Parties
// ---------------------------------------------------------------------------

/// The subject of the record, referred to without naming them.
///
/// `PARTY_SELF` with no `external_ref` is how openEHR supports a fully
/// anonymous record: the entry says "the subject of this record", and nothing
/// in the record says who that is. This is not a degenerate case to be
/// normalised away — it is the representation a research extract needs.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PartySelf {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    external_ref: Option<PartyRef>,
}

impl PartySelf {
    /// The record subject, anonymously.
    #[must_use]
    pub fn anonymous() -> Self {
        Self::default()
    }

    /// The record subject, with a demographic reference.
    #[must_use]
    pub fn with_external_ref(external_ref: PartyRef) -> Self {
        Self {
            external_ref: Some(external_ref),
        }
    }

    /// The demographic reference, if any.
    #[must_use]
    pub fn external_ref(&self) -> Option<&PartyRef> {
        self.external_ref.as_ref()
    }
}

/// A party other than the record subject, identified by name, by formal
/// identifiers, or by a demographic reference.
///
/// ```
/// use openehr::rm::common::PartyIdentified;
///
/// let composer = PartyIdentified::named("Dr A Nurse").unwrap();
/// assert_eq!(composer.name(), Some("Dr A Nurse"));
///
/// // At least one of name, identifiers, or external_ref is required: a
/// // PARTY_IDENTIFIED with none of them identifies nobody.
/// assert!(PartyIdentified::new(None, Vec::new(), None).is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartyIdentified {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    external_ref: Option<PartyRef>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    name: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    identifiers: Vec<DvIdentifier>,
}

impl PartyIdentified {
    /// Builds an identified party.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if all three of `name`, `identifiers`, and
    /// `external_ref` are absent (`Basic_validity`), or if `name` is present
    /// and empty.
    pub fn new(
        name: Option<String>,
        identifiers: Vec<DvIdentifier>,
        external_ref: Option<PartyRef>,
    ) -> Result<Self, ParseError> {
        if name.as_ref().is_some_and(String::is_empty) {
            return Err(ParseError::invariant("PARTY_IDENTIFIED", "Name_valid"));
        }
        if name.is_none() && identifiers.is_empty() && external_ref.is_none() {
            return Err(ParseError::invariant("PARTY_IDENTIFIED", "Basic_validity"));
        }
        Ok(Self {
            external_ref,
            name,
            identifiers,
        })
    }

    /// Builds a party identified only by name.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the name is empty.
    pub fn named(name: impl Into<String>) -> Result<Self, ParseError> {
        Self::new(Some(name.into()), Vec::new(), None)
    }

    /// The party's name, if recorded.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The party's formal identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &[DvIdentifier] {
        &self.identifiers
    }

    /// The demographic reference, if any.
    #[must_use]
    pub fn external_ref(&self) -> Option<&PartyRef> {
        self.external_ref.as_ref()
    }
}

/// A party whose relationship to the record subject is recorded.
///
/// This is how an entry says it is *about someone else* — a family history
/// about a mother, a finding about a foetus. Without it, an entry recorded
/// during a consultation reads as a finding about the patient, which is how a
/// mother's breast cancer becomes the patient's.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartyRelated {
    #[serde(flatten)]
    identified: PartyIdentified,
    relationship: DvCodedText,
}

impl PartyRelated {
    /// Builds a related party from an openEHR `subject_relationship` code.
    ///
    /// Taking a code rather than a `DV_CODED_TEXT` makes
    /// `Relationship_valid` hold by construction, as it does for every other
    /// coded attribute in this crate: the rubric cannot disagree with the code
    /// because the caller never supplies one.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the code is not in the openEHR
    /// `subject_relationship` group (`Relationship_valid`).
    pub fn new(identified: PartyIdentified, relationship_code: &str) -> Result<Self, ParseError> {
        let relationship = terminology::subject_relationship::GROUP
            .coded_text(relationship_code)
            .ok_or_else(|| ParseError::invariant("PARTY_RELATED", "Relationship_valid"))?;
        Ok(Self {
            identified,
            relationship,
        })
    }

    /// Builds a related party from an already-coded relationship.
    ///
    /// For reading data that arrived from elsewhere, where the relationship may
    /// legitimately be coded against a terminology this crate does not carry.
    /// The group membership is then checked by [`crate::validation`] rather
    /// than here.
    #[must_use]
    pub fn from_coded(identified: PartyIdentified, relationship: DvCodedText) -> Self {
        Self {
            identified,
            relationship,
        }
    }

    /// The relationship to the record subject.
    #[must_use]
    pub fn relationship(&self) -> &DvCodedText {
        &self.relationship
    }

    /// The party's identifying attributes.
    #[must_use]
    pub fn as_identified(&self) -> &PartyIdentified {
        &self.identified
    }

    /// Whether the relationship is `0|self|` — the record subject themselves.
    #[must_use]
    pub fn is_self(&self) -> bool {
        self.relationship.defining_code().is_openehr()
            && self.relationship.defining_code().code_string()
                == terminology::subject_relationship::SELF
    }
}

/// Any reference to a party from inside the health record.
///
/// The variants differ substantially in size — a [`PartySelf`] is one optional
/// reference and a [`PartyRelated`] carries identifiers and a coded
/// relationship. Boxing the large variant would shrink the enum and put a heap
/// allocation on the path of every clinical statement's `subject`, which is the
/// hottest field in the model; the enum stays flat deliberately.
///
/// # Reading a payload with no `_type`
///
/// `relationship` implies `PARTY_RELATED`; `name` or `identifiers` imply
/// `PARTY_IDENTIFIED`; anything else is `PARTY_SELF`. The order matters for
/// the same reason it does in [`Text`]: reading a `PARTY_RELATED` as a
/// `PARTY_IDENTIFIED` would drop the relationship, and the relationship is the
/// attribute that says whom the data is about.
#[derive(Debug, Clone, PartialEq)]
// See the note on the variants' sizes above.
#[allow(clippy::large_enum_variant)]
pub enum PartyProxy {
    /// The record subject.
    SelfParty(PartySelf),
    /// A named or identified party.
    Identified(PartyIdentified),
    /// A party with a stated relationship to the record subject.
    Related(PartyRelated),
}

impl PartyProxy {
    /// The demographic reference, if any.
    #[must_use]
    pub fn external_ref(&self) -> Option<&PartyRef> {
        match self {
            Self::SelfParty(p) => p.external_ref(),
            Self::Identified(p) => p.external_ref(),
            Self::Related(p) => p.as_identified().external_ref(),
        }
    }

    /// The party's name, if this form carries one.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::SelfParty(_) => None,
            Self::Identified(p) => p.name(),
            Self::Related(p) => p.as_identified().name(),
        }
    }

    /// The openEHR class name, as it appears in `_type`.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::SelfParty(_) => "PARTY_SELF",
            Self::Identified(_) => "PARTY_IDENTIFIED",
            Self::Related(_) => "PARTY_RELATED",
        }
    }

    /// Whether this proxy denotes the record subject.
    ///
    /// True for `PARTY_SELF`, and for a `PARTY_RELATED` whose relationship is
    /// `0|self|`. Both spellings occur, and code that checks only the first
    /// treats a self-related entry as being about somebody else.
    #[must_use]
    pub fn is_subject(&self) -> bool {
        match self {
            Self::SelfParty(_) => true,
            Self::Identified(_) => false,
            Self::Related(p) => p.is_self(),
        }
    }
}

impl From<PartySelf> for PartyProxy {
    fn from(v: PartySelf) -> Self {
        Self::SelfParty(v)
    }
}

impl From<PartyIdentified> for PartyProxy {
    fn from(v: PartyIdentified) -> Self {
        Self::Identified(v)
    }
}

impl From<PartyRelated> for PartyProxy {
    fn from(v: PartyRelated) -> Self {
        Self::Related(v)
    }
}

impl Serialize for PartyProxy {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Tagged<'a, T: Serialize> {
            #[serde(rename = "_type")]
            ty: &'static str,
            #[serde(flatten)]
            inner: &'a T,
        }
        match self {
            Self::SelfParty(p) => Tagged {
                ty: "PARTY_SELF",
                inner: p,
            }
            .serialize(s),
            Self::Identified(p) => Tagged {
                ty: "PARTY_IDENTIFIED",
                inner: p,
            }
            .serialize(s),
            Self::Related(p) => Tagged {
                ty: "PARTY_RELATED",
                inner: p,
            }
            .serialize(s),
        }
    }
}

impl<'de> Deserialize<'de> for PartyProxy {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;

        // A flat wire struct rather than an intermediate `serde_json::Value`,
        // for the reason given on [`Text`]'s deserializer: `subject` appears on
        // every entry, and a `Value` round trip per party doubled the stack
        // depth of reading a composition.
        #[derive(Deserialize)]
        struct Wire {
            #[serde(rename = "_type", default)]
            ty: Option<String>,
            #[serde(default)]
            external_ref: Option<PartyRef>,
            #[serde(default)]
            name: Option<String>,
            #[serde(default)]
            identifiers: Vec<DvIdentifier>,
            #[serde(default)]
            relationship: Option<DvCodedText>,
        }

        let wire = Wire::deserialize(d)?;
        let kind = match wire.ty.as_deref() {
            Some(known @ ("PARTY_SELF" | "PARTY_IDENTIFIED" | "PARTY_RELATED")) => known,
            Some(other) => {
                return Err(D::Error::custom(format!(
                    "{other} is not a PARTY_PROXY class"
                )));
            }
            None if wire.relationship.is_some() => "PARTY_RELATED",
            None if wire.name.is_some() || !wire.identifiers.is_empty() => "PARTY_IDENTIFIED",
            None => "PARTY_SELF",
        };
        if kind == "PARTY_SELF" {
            return Ok(Self::SelfParty(PartySelf {
                external_ref: wire.external_ref,
            }));
        }
        let identified = PartyIdentified {
            external_ref: wire.external_ref,
            name: wire.name,
            identifiers: wire.identifiers,
        };
        if kind == "PARTY_IDENTIFIED" {
            return Ok(Self::Identified(identified));
        }
        let relationship = wire
            .relationship
            .ok_or_else(|| D::Error::missing_field("relationship"))?;
        Ok(Self::Related(PartyRelated {
            identified,
            relationship,
        }))
    }
}

/// A party's involvement in an activity, and in what capacity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Participation {
    function: Text,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    mode: Option<DvCodedText>,
    performer: PartyProxy,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    time: Option<Interval<DvDateTime>>,
}

impl Participation {
    /// Builds a participation.
    #[must_use]
    pub fn new(function: Text, performer: PartyProxy) -> Self {
        Self {
            function,
            mode: None,
            performer,
            time: None,
        }
    }

    /// Records how the party took part — in person, by telephone, by letter.
    #[must_use]
    pub fn with_mode(mut self, mode: DvCodedText) -> Self {
        self.mode = Some(mode);
        self
    }

    /// Records when the participation happened.
    #[must_use]
    pub fn with_time(mut self, time: Interval<DvDateTime>) -> Self {
        self.time = Some(time);
        self
    }

    /// What the party did.
    #[must_use]
    pub fn function(&self) -> &Text {
        &self.function
    }

    /// How the party took part.
    #[must_use]
    pub fn mode(&self) -> Option<&DvCodedText> {
        self.mode.as_ref()
    }

    /// Who took part.
    #[must_use]
    pub fn performer(&self) -> &PartyProxy {
        &self.performer
    }

    /// When they took part.
    #[must_use]
    pub fn time(&self) -> Option<&Interval<DvDateTime>> {
        self.time.as_ref()
    }
}

// ---------------------------------------------------------------------------
// Audit
// ---------------------------------------------------------------------------

/// Who committed what, when, and why.
///
/// Every version in an openEHR record carries one. This is the class that
/// answers HIPAA §164.312(b) and IEC 62304's traceability requirement, and the
/// reason [`VersionedObject::commit`] will not accept a version without one.
///
/// ```
/// use openehr::rm::common::{AuditDetails, PartyIdentified};
/// use openehr::rm::data_types::DvDateTime;
/// use openehr::terminology::audit_change_type;
///
/// let audit = AuditDetails::new(
///     "ehr1.example.org",
///     DvDateTime::new("2026-07-31T09:15:00Z").unwrap(),
///     audit_change_type::CREATION,
///     PartyIdentified::named("Dr A Nurse").unwrap().into(),
/// ).unwrap();
/// assert_eq!(audit.change_type().value(), "creation");
/// assert!(audit.is_creation());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditDetails {
    system_id: String,
    time_committed: DvDateTime,
    change_type: DvCodedText,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    description: Option<Text>,
    committer: PartyProxy,
}

impl AuditDetails {
    /// Builds audit details from an openEHR change-type code.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the system id is empty, or if the code is not
    /// in the `audit_change_type` group. Taking the code rather than a
    /// `DV_CODED_TEXT` is deliberate: it makes it impossible to build an audit
    /// record whose rubric and code disagree.
    pub fn new(
        system_id: impl Into<String>,
        time_committed: DvDateTime,
        change_type_code: &str,
        committer: PartyProxy,
    ) -> Result<Self, ParseError> {
        let system_id = system_id.into();
        if system_id.is_empty() {
            return Err(ParseError::invariant("AUDIT_DETAILS", "System_id_valid"));
        }
        let change_type = terminology::audit_change_type::GROUP
            .coded_text(change_type_code)
            .ok_or_else(|| ParseError::invariant("AUDIT_DETAILS", "Change_type_valid"))?;
        Ok(Self {
            system_id,
            time_committed,
            change_type,
            description: None,
            committer,
        })
    }

    /// Records why the change was made.
    #[must_use]
    pub fn with_description(mut self, description: Text) -> Self {
        self.description = Some(description);
        self
    }

    /// The repository that accepted the commit.
    #[must_use]
    pub fn system_id(&self) -> &str {
        &self.system_id
    }

    /// When the commit happened.
    #[must_use]
    pub fn time_committed(&self) -> &DvDateTime {
        &self.time_committed
    }

    /// What kind of change it was.
    #[must_use]
    pub fn change_type(&self) -> &DvCodedText {
        &self.change_type
    }

    /// Why the change was made, if recorded.
    #[must_use]
    pub fn description(&self) -> Option<&Text> {
        self.description.as_ref()
    }

    /// Who committed it.
    #[must_use]
    pub fn committer(&self) -> &PartyProxy {
        &self.committer
    }

    /// The openEHR change-type code.
    #[must_use]
    pub fn change_type_code(&self) -> &str {
        self.change_type.defining_code().code_string()
    }

    /// Whether this audit records the creation of the object.
    #[must_use]
    pub fn is_creation(&self) -> bool {
        self.change_type_code() == terminology::audit_change_type::CREATION
    }

    /// Whether this audit records a logical deletion.
    #[must_use]
    pub fn is_deletion(&self) -> bool {
        self.change_type_code() == terminology::audit_change_type::DELETED
    }
}

/// A signature over content by a healthcare agent.
///
/// An `ATTESTATION` is an `AUDIT_DETAILS` that also says *this is right, and I
/// say so*. The `proof` is an `OpenPGP` signature; this crate carries it and does
/// **not** verify it, because verifying needs a key ring and a trust policy
/// that belong to the deployment. [`Attestation::proof`] returning `Some` is
/// therefore not evidence of anything, and the documentation says so where a
/// caller will read it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attestation {
    #[serde(flatten)]
    audit: AuditDetails,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    attested_view: Option<DvMultimedia>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    proof: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    items: Vec<DvEhrUri>,
    reason: Text,
    is_pending: bool,
}

impl Attestation {
    /// Builds an attestation.
    #[must_use]
    pub fn new(audit: AuditDetails, reason: Text, is_pending: bool) -> Self {
        Self {
            audit,
            attested_view: None,
            proof: None,
            items: Vec::new(),
            reason,
            is_pending,
        }
    }

    /// Attaches the rendering the attester actually saw.
    ///
    /// This matters more than it looks: a clinician signs what was on the
    /// screen, and the screen is a template's rendering of the data, not the
    /// data. Keeping the rendering is what makes the signature meaningful
    /// years later when the template has changed.
    #[must_use]
    pub fn with_attested_view(mut self, view: DvMultimedia) -> Self {
        self.attested_view = Some(view);
        self
    }

    /// Attaches an `OpenPGP` signature.
    ///
    /// **Not verified by this crate.** See the type's documentation.
    #[must_use]
    pub fn with_proof(mut self, proof: impl Into<String>) -> Self {
        self.proof = Some(proof.into());
        self
    }

    /// Narrows the attestation to specific nodes.
    #[must_use]
    pub fn with_item(mut self, item: DvEhrUri) -> Self {
        self.items.push(item);
        self
    }

    /// The commit metadata.
    #[must_use]
    pub fn audit(&self) -> &AuditDetails {
        &self.audit
    }

    /// Why the content was attested.
    #[must_use]
    pub fn reason(&self) -> &Text {
        &self.reason
    }

    /// Whether the attestation is still outstanding.
    #[must_use]
    pub fn is_pending(&self) -> bool {
        self.is_pending
    }

    /// The rendering the attester saw, if kept.
    #[must_use]
    pub fn attested_view(&self) -> Option<&DvMultimedia> {
        self.attested_view.as_ref()
    }

    /// The `OpenPGP` signature, if present. **Unverified.**
    #[must_use]
    pub fn proof(&self) -> Option<&str> {
        self.proof.as_deref()
    }

    /// The specific nodes attested to; empty means the whole object.
    #[must_use]
    pub fn items(&self) -> &[DvEhrUri] {
        &self.items
    }
}

/// One version's audit trail: the commit, plus any attestations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RevisionHistoryItem {
    version_id: ObjectVersionId,
    audits: Vec<AuditDetails>,
}

impl RevisionHistoryItem {
    /// Builds a revision history item.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the audit list is empty (`Audits_valid`). A
    /// version with no audit is a change nobody made.
    pub fn new(version_id: ObjectVersionId, audits: Vec<AuditDetails>) -> Result<Self, ParseError> {
        if audits.is_empty() {
            return Err(ParseError::invariant(
                "REVISION_HISTORY_ITEM",
                "Audit_valid",
            ));
        }
        Ok(Self { version_id, audits })
    }

    /// Which version this describes.
    #[must_use]
    pub fn version_id(&self) -> &ObjectVersionId {
        &self.version_id
    }

    /// The commit and attestation records.
    #[must_use]
    pub fn audits(&self) -> &[AuditDetails] {
        &self.audits
    }
}

/// The audit trail of a versioned object, **most recent last**.
///
/// # openEHR contradicts itself here, and this is the resolution
///
/// The `REVISION_HISTORY` class table says three things about the order of
/// `items`, and they do not agree:
///
/// | Where | Says |
/// | --- | --- |
/// | class *Purpose* | "The list is in most-recent-**first** order" |
/// | `items` *Meaning* | "The items in this history in most-recent-**last** order" |
/// | `most_recent_version` postcondition | `Result.is_equal(items.last.version_id.value)` |
///
/// Two of the three say most-recent-last, and one of those two is executable.
/// A postcondition is a statement a conformant implementation can be tested
/// against; a Purpose line is prose. This crate follows the postcondition, so
/// [`RevisionHistory::items`] is oldest-first and
/// [`RevisionHistory::most_recent_version`] returns the last.
///
/// Recorded rather than silently chosen, because a caller iterating this list
/// to render an audit trail will get it backwards if they trust the other
/// sentence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RevisionHistory {
    items: Vec<RevisionHistoryItem>,
}

impl RevisionHistory {
    /// Builds a revision history.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the item list is empty.
    pub fn new(items: Vec<RevisionHistoryItem>) -> Result<Self, ParseError> {
        if items.is_empty() {
            return Err(ParseError::invariant("REVISION_HISTORY", "Items_valid"));
        }
        Ok(Self { items })
    }

    /// The items, oldest first — see the type's documentation.
    #[must_use]
    pub fn items(&self) -> &[RevisionHistoryItem] {
        &self.items
    }

    /// The most recent version's identifier: the **last** item.
    ///
    /// # Panics
    ///
    /// Never: the constructor rejects an empty history.
    #[must_use]
    pub fn most_recent_version(&self) -> &ObjectVersionId {
        self.items
            .last()
            .map(RevisionHistoryItem::version_id)
            .expect("constructor rejects an empty revision history")
    }

    /// The commit time of the most recent item.
    ///
    /// openEHR's `most_recent_version_time_committed`, whose postcondition
    /// reads `items.last.audits.first.time_committed`.
    ///
    /// # Panics
    ///
    /// Never: both lists are non-empty by construction.
    #[must_use]
    pub fn most_recent_version_time_committed(&self) -> &DvDateTime {
        self.items
            .last()
            .map(|i| i.audits()[0].time_committed())
            .expect("constructor rejects an empty revision history")
    }
}

// ---------------------------------------------------------------------------
// Change control
// ---------------------------------------------------------------------------

/// A version of a versioned object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: Deserialize<'de>"))]
#[serde(tag = "_type")]
// An IMPORTED_VERSION wraps an ORIGINAL_VERSION plus its own audit, so it is
// necessarily larger. Boxing it would add an allocation per imported version
// and change nothing a caller can observe; the size difference is inherent to
// the model rather than a modelling slip.
#[allow(clippy::large_enum_variant)]
pub enum Version<T> {
    /// A version created here.
    #[serde(rename = "ORIGINAL_VERSION")]
    Original(OriginalVersion<T>),
    /// A version received from another system, wrapped so that its original
    /// identity survives the import.
    #[serde(rename = "IMPORTED_VERSION")]
    Imported(ImportedVersion<T>),
}

impl<T> Version<T> {
    /// The version's identifier.
    #[must_use]
    pub fn uid(&self) -> &ObjectVersionId {
        match self {
            Self::Original(v) => &v.uid,
            // Delegating, per openEHR: an imported version's identity is the
            // identity it had where it was created, not a new one minted here.
            // Minting a local id would make the same clinical fact appear twice
            // when the two systems next exchange data.
            Self::Imported(v) => &v.item.uid,
        }
    }

    /// The version this one succeeds, if any.
    #[must_use]
    pub fn preceding_version_uid(&self) -> Option<&ObjectVersionId> {
        match self {
            Self::Original(v) => v.preceding_version_uid.as_ref(),
            Self::Imported(v) => v.item.preceding_version_uid.as_ref(),
        }
    }

    /// Attestations attached to this version.
    ///
    /// Empty for an imported version: an `IMPORTED_VERSION` wraps the item it
    /// received, and an attestation made elsewhere travels inside that item
    /// rather than being re-asserted here.
    #[must_use]
    pub fn attestations(&self) -> &[Attestation] {
        match self {
            Self::Original(v) => &v.attestations,
            Self::Imported(v) => &v.item.attestations,
        }
    }

    /// The versions merged into this one, empty when it is not a merge.
    #[must_use]
    pub fn other_input_version_uids(&self) -> &[ObjectVersionId] {
        match self {
            Self::Original(v) => &v.other_input_version_uids,
            Self::Imported(v) => &v.item.other_input_version_uids,
        }
    }

    /// The signature over this version, where one was supplied.
    ///
    /// Carried opaquely: this crate does not verify it (`S1.11`), because doing
    /// so needs key management that belongs to the deployment.
    ///
    /// An imported version's signature is the **importer's**, over the wrapper
    /// it created; the wrapped original keeps its own. They are different
    /// assertions by different parties, so this returns the outer one and
    /// `Version::Imported(v) => v.item().signature()` reaches the inner.
    #[must_use]
    pub fn signature(&self) -> Option<&str> {
        match self {
            Self::Original(v) => v.signature.as_deref(),
            Self::Imported(v) => v.signature.as_deref(),
        }
    }

    /// The version's content, absent for a logically deleted version.
    #[must_use]
    pub fn data(&self) -> Option<&T> {
        match self {
            Self::Original(v) => v.data.as_ref(),
            Self::Imported(v) => v.item.data.as_ref(),
        }
    }

    /// The commit metadata.
    #[must_use]
    pub fn commit_audit(&self) -> &AuditDetails {
        match self {
            Self::Original(v) => &v.commit_audit,
            Self::Imported(v) => &v.commit_audit,
        }
    }

    /// The lifecycle state code.
    #[must_use]
    pub fn lifecycle_state_code(&self) -> &str {
        match self {
            Self::Original(v) => v.lifecycle_state.defining_code().code_string(),
            Self::Imported(v) => v.item.lifecycle_state.defining_code().code_string(),
        }
    }

    /// Whether this version marks the object as deleted.
    #[must_use]
    pub fn is_deleted(&self) -> bool {
        self.lifecycle_state_code() == terminology::version_lifecycle_state::DELETED
    }
}

/// A version created by this system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: Deserialize<'de>"))]
pub struct OriginalVersion<T> {
    uid: ObjectVersionId,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    preceding_version_uid: Option<ObjectVersionId>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    other_input_version_uids: Vec<ObjectVersionId>,
    lifecycle_state: DvCodedText,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    data: Option<T>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    attestations: Vec<Attestation>,
    commit_audit: AuditDetails,
    contribution: ObjectRef,
    /// openEHR declares `signature` on `VERSION`, so an original version
    /// inherits it as surely as an imported one does (`A-18`). It was missing
    /// here, which meant a locally created version could not be signed at all —
    /// only a version that had arrived from somewhere else.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    signature: Option<String>,
}

impl<T> OriginalVersion<T> {
    /// Attaches a signature over this version.
    ///
    /// **Carried, never verified.** Verifying it needs key management that
    /// belongs to the deployment (`S1.11`, `X11.11`), and a library that
    /// checked a signature it could not establish trust for would be asserting
    /// something it does not know.
    #[must_use]
    pub fn with_signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }

    /// The signature over this version, if one was supplied. **Unverified.**
    #[must_use]
    pub fn signature(&self) -> Option<&str> {
        self.signature.as_deref()
    }

    /// Builds a version.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the lifecycle state code is not in the
    /// `version_lifecycle_state` group, or if `data` is absent while the state
    /// is not `deleted`. The second rule is the one worth having: a version
    /// with no data and a `complete` state claims the content is finished and
    /// then does not supply it.
    pub fn new(
        uid: ObjectVersionId,
        preceding_version_uid: Option<ObjectVersionId>,
        lifecycle_state_code: &str,
        data: Option<T>,
        commit_audit: AuditDetails,
        contribution: ObjectRef,
    ) -> Result<Self, ParseError> {
        let lifecycle_state = terminology::version_lifecycle_state::GROUP
            .coded_text(lifecycle_state_code)
            .ok_or_else(|| ParseError::invariant("ORIGINAL_VERSION", "Lifecycle_state_valid"))?;
        if data.is_none() && lifecycle_state_code != terminology::version_lifecycle_state::DELETED {
            return Err(ParseError::invariant("ORIGINAL_VERSION", "Data_valid"));
        }
        Ok(Self {
            uid,
            preceding_version_uid,
            other_input_version_uids: Vec::new(),
            signature: None,
            lifecycle_state,
            data,
            attestations: Vec::new(),
            commit_audit,
            contribution,
        })
    }

    /// Adds an attestation.
    #[must_use]
    pub fn with_attestation(mut self, attestation: Attestation) -> Self {
        self.attestations.push(attestation);
        self
    }

    /// Records the other versions merged into this one.
    #[must_use]
    pub fn with_other_input_version_uid(mut self, uid: ObjectVersionId) -> Self {
        self.other_input_version_uids.push(uid);
        self
    }

    /// The version's identifier.
    #[must_use]
    pub fn uid(&self) -> &ObjectVersionId {
        &self.uid
    }

    /// The content.
    #[must_use]
    pub fn data(&self) -> Option<&T> {
        self.data.as_ref()
    }

    /// The commit metadata.
    #[must_use]
    pub fn commit_audit(&self) -> &AuditDetails {
        &self.commit_audit
    }

    /// The contribution this version was committed in.
    #[must_use]
    pub fn contribution(&self) -> &ObjectRef {
        &self.contribution
    }

    /// The attestations over this version.
    #[must_use]
    pub fn attestations(&self) -> &[Attestation] {
        &self.attestations
    }

    /// Whether this version was produced by merging others.
    #[must_use]
    pub fn is_merged(&self) -> bool {
        !self.other_input_version_uids.is_empty()
    }
}

/// A version received from another system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: Deserialize<'de>"))]
pub struct ImportedVersion<T> {
    item: OriginalVersion<T>,
    commit_audit: AuditDetails,
    contribution: ObjectRef,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    signature: Option<String>,
}

impl<T> ImportedVersion<T> {
    /// Wraps an original version for import.
    ///
    /// The wrapping is the point: the import gets its own audit — who imported
    /// it, into which system, when — while the wrapped version keeps the
    /// original's audit and identifier. Flattening the two would leave no way
    /// to tell an authored record from a received one.
    #[must_use]
    pub fn new(
        item: OriginalVersion<T>,
        commit_audit: AuditDetails,
        contribution: ObjectRef,
    ) -> Self {
        Self {
            item,
            commit_audit,
            contribution,
            signature: None,
        }
    }

    /// The wrapped version, as it was where it was created.
    #[must_use]
    pub fn item(&self) -> &OriginalVersion<T> {
        &self.item
    }

    /// The audit of the import itself.
    #[must_use]
    pub fn commit_audit(&self) -> &AuditDetails {
        &self.commit_audit
    }
}

impl<T> From<OriginalVersion<T>> for Version<T> {
    fn from(v: OriginalVersion<T>) -> Self {
        Self::Original(v)
    }
}

impl<T> From<ImportedVersion<T>> for Version<T> {
    fn from(v: ImportedVersion<T>) -> Self {
        Self::Imported(v)
    }
}

/// Why a commit to a [`VersionedObject`] was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CommitError {
    /// The version's `uid` names a different versioned object.
    #[error("version belongs to a different versioned object")]
    WrongObject,

    /// A version with this identifier is already in the history.
    #[error("a version with this id already exists")]
    DuplicateVersion,

    /// The first version of an object must not claim a predecessor, and every
    /// later version must.
    #[error("preceding_version_uid is absent on a successor, or present on the first version")]
    PrecedingVersionMismatch,

    /// The claimed predecessor is not the current latest version.
    ///
    /// This is the concurrent-write case: two clients read version 3 and both
    /// commit version 4. openEHR's answer is a branch, not a silent overwrite,
    /// so this refusal is the point at which a caller must decide.
    #[error("preceding version is not the current latest (concurrent modification)")]
    NotLatest,
}

/// The container for all versions of one logical object.
///
/// ```
/// use openehr::base::{HierObjectId, ObjectId, ObjectRef, ObjectVersionId};
/// use openehr::rm::common::{AuditDetails, OriginalVersion, PartyIdentified, VersionedObject};
/// use openehr::rm::data_types::DvDateTime;
/// use openehr::terminology::{audit_change_type, version_lifecycle_state};
///
/// let uid = HierObjectId::from_uid_str("87284370-2D4B-4E3D-A3F3-F303D2F4F34B").unwrap();
/// let owner = ObjectRef::new("local", "EHR", ObjectId::HierObjectId(uid.clone())).unwrap();
/// let mut vo: VersionedObject<String> =
///     VersionedObject::new(uid, owner.clone(), DvDateTime::new("2026-07-31T09:00:00Z").unwrap());
///
/// let audit = AuditDetails::new(
///     "ehr1.example.org",
///     DvDateTime::new("2026-07-31T09:15:00Z").unwrap(),
///     audit_change_type::CREATION,
///     PartyIdentified::named("Dr A Nurse").unwrap().into(),
/// ).unwrap();
///
/// let v1 = OriginalVersion::new(
///     "87284370-2D4B-4E3D-A3F3-F303D2F4F34B::ehr1.example.org::1".parse().unwrap(),
///     None,
///     version_lifecycle_state::COMPLETE,
///     Some("first".to_string()),
///     audit.clone(),
///     owner.clone(),
/// ).unwrap();
/// vo.commit(v1.into()).unwrap();
/// assert_eq!(vo.version_count(), 1);
///
/// // A second version claiming no predecessor is refused: it would make the
/// // history two roots, and neither would be wrong on its face.
/// let bad = OriginalVersion::new(
///     "87284370-2D4B-4E3D-A3F3-F303D2F4F34B::ehr1.example.org::2".parse().unwrap(),
///     None,
///     version_lifecycle_state::COMPLETE,
///     Some("second".to_string()),
///     audit,
///     owner,
/// ).unwrap();
/// assert!(vo.commit(bad.into()).is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: Deserialize<'de>"))]
pub struct VersionedObject<T> {
    uid: HierObjectId,
    owner_id: ObjectRef,
    time_created: DvDateTime,
    #[serde(skip_serializing_if = "Vec::is_empty", default = "Vec::new")]
    versions: Vec<Version<T>>,
}

impl<T> VersionedObject<T> {
    /// Builds an empty versioned object.
    #[must_use]
    pub fn new(uid: HierObjectId, owner_id: ObjectRef, time_created: DvDateTime) -> Self {
        Self {
            uid,
            owner_id,
            time_created,
            versions: Vec::new(),
        }
    }

    /// The object's identifier.
    #[must_use]
    pub fn uid(&self) -> &HierObjectId {
        &self.uid
    }

    /// The EHR or repository that owns it.
    #[must_use]
    pub fn owner_id(&self) -> &ObjectRef {
        &self.owner_id
    }

    /// When the object was created.
    #[must_use]
    pub fn time_created(&self) -> &DvDateTime {
        &self.time_created
    }

    /// Every version, oldest first.
    #[must_use]
    pub fn all_versions(&self) -> &[Version<T>] {
        &self.versions
    }

    /// How many versions the object has.
    #[must_use]
    pub fn version_count(&self) -> usize {
        self.versions.len()
    }

    /// The most recently committed version.
    #[must_use]
    pub fn latest_version(&self) -> Option<&Version<T>> {
        self.versions.last()
    }

    /// The version with the given identifier.
    #[must_use]
    pub fn version_with_id(&self, uid: &ObjectVersionId) -> Option<&Version<T>> {
        self.versions.iter().find(|v| v.uid() == uid)
    }

    /// Whether the object has a version with the given identifier.
    #[must_use]
    pub fn has_version_id(&self, uid: &ObjectVersionId) -> bool {
        self.version_with_id(uid).is_some()
    }

    /// The version that was current at a given time.
    ///
    /// Returns the latest version whose `time_committed` is at or before
    /// `time`. Returns `None` if the object had no version yet — which is a
    /// different answer from "the object was empty", and the caller needs the
    /// difference to answer "what did the clinician see at 09:15?".
    ///
    /// Versions whose commit time is not comparable with `time` — one carries
    /// a UTC offset and the other does not — are skipped rather than assumed
    /// to be before it. See [`crate::base::iso8601`].
    #[must_use]
    pub fn version_at_time(&self, time: &DvDateTime) -> Option<&Version<T>> {
        self.versions.iter().rfind(|v| {
            matches!(
                v.commit_audit().time_committed().partial_cmp(time),
                Some(core::cmp::Ordering::Less | core::cmp::Ordering::Equal)
            )
        })
    }

    /// Whether the object had any version at a given time.
    #[must_use]
    pub fn has_version_at_time(&self, time: &DvDateTime) -> bool {
        self.version_at_time(time).is_some()
    }

    /// Appends a version, checking that it belongs to this history.
    ///
    /// # Errors
    ///
    /// See [`CommitError`]. Each variant is a history that would read back and
    /// not connect.
    pub fn commit(&mut self, version: Version<T>) -> Result<(), CommitError> {
        if version.uid().object_id() != self.uid.root() {
            return Err(CommitError::WrongObject);
        }
        if self.has_version_id(version.uid()) {
            return Err(CommitError::DuplicateVersion);
        }
        match (self.latest_version(), version.preceding_version_uid()) {
            (None, None) => {}
            (None, Some(_)) | (Some(_), None) => {
                return Err(CommitError::PrecedingVersionMismatch);
            }
            (Some(latest), Some(preceding)) => {
                if latest.uid() != preceding {
                    return Err(CommitError::NotLatest);
                }
            }
        }
        self.versions.push(version);
        Ok(())
    }

    /// The object's audit trail, oldest first (see [`RevisionHistory`]).
    ///
    /// Returns `None` for an object with no versions: [`RevisionHistory`]
    /// refuses to be empty, and an empty history object would assert that the
    /// audit trail exists and is blank.
    #[must_use]
    pub fn revision_history(&self) -> Option<RevisionHistory> {
        let items: Vec<RevisionHistoryItem> = self
            .versions
            .iter()
            .filter_map(|v| {
                RevisionHistoryItem::new(v.uid().clone(), vec![v.commit_audit().clone()]).ok()
            })
            .collect();
        RevisionHistory::new(items).ok()
    }
}

/// A set of versions committed together as one change.
///
/// The unit a user recognises: "I saved the consultation" is one contribution
/// and five compositions. openEHR restricts a contribution's own change type to
/// three of the nine codes, and this type enforces that.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contribution {
    uid: HierObjectId,
    versions: Vec<ObjectVersionId>,
    audit: AuditDetails,
}

impl Contribution {
    /// The change types a `CONTRIBUTION.audit` may carry.
    pub const PERMITTED_CHANGE_TYPES: [&'static str; 3] = [
        terminology::audit_change_type::CREATION,
        terminology::audit_change_type::AMENDMENT,
        terminology::audit_change_type::DELETED,
    ];

    /// Builds a contribution.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the version list is empty (`Versions_valid`)
    /// or if the audit's change type is not one of
    /// [`Contribution::PERMITTED_CHANGE_TYPES`].
    pub fn new(
        uid: HierObjectId,
        versions: Vec<ObjectVersionId>,
        audit: AuditDetails,
    ) -> Result<Self, ParseError> {
        if versions.is_empty() {
            return Err(ParseError::invariant("CONTRIBUTION", "Versions_valid"));
        }
        if !Self::PERMITTED_CHANGE_TYPES.contains(&audit.change_type_code()) {
            return Err(ParseError::invariant(
                "CONTRIBUTION",
                "Audit_change_type_valid",
            ));
        }
        Ok(Self {
            uid,
            versions,
            audit,
        })
    }

    /// The contribution's identifier.
    #[must_use]
    pub fn uid(&self) -> &HierObjectId {
        &self.uid
    }

    /// The versions committed in this change.
    #[must_use]
    pub fn versions(&self) -> &[ObjectVersionId] {
        &self.versions
    }

    /// The change's audit metadata.
    #[must_use]
    pub fn audit(&self) -> &AuditDetails {
        &self.audit
    }
}

/// A time-bounded validity period, used across the demographic model.
pub type DateValidity = Interval<DvDate>;

/// A reference to a node inside a version, re-exported here because
/// [`Attestation`] and the demographic model both point at one.
pub type NodeRef = LocatableRef;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::{HierObjectId, ObjectId};

    fn audit(code: &str) -> AuditDetails {
        AuditDetails::new(
            "ehr1.example.org",
            DvDateTime::new("2026-07-31T09:15:00Z").unwrap(),
            code,
            PartyIdentified::named("Dr A Nurse").unwrap().into(),
        )
        .unwrap()
    }

    fn versioned() -> (VersionedObject<String>, ObjectRef) {
        let uid = HierObjectId::from_uid_str("87284370-2D4B-4E3D-A3F3-F303D2F4F34B").unwrap();
        let owner = ObjectRef::new("local", "EHR", ObjectId::HierObjectId(uid.clone())).unwrap();
        (
            VersionedObject::new(
                uid,
                owner.clone(),
                DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
            ),
            owner,
        )
    }

    fn version(n: u32, preceding: Option<u32>, owner: &ObjectRef, code: &str) -> Version<String> {
        let id = |v: u32| -> ObjectVersionId {
            format!("87284370-2D4B-4E3D-A3F3-F303D2F4F34B::ehr1.example.org::{v}")
                .parse()
                .unwrap()
        };
        OriginalVersion::new(
            id(n),
            preceding.map(id),
            terminology::version_lifecycle_state::COMPLETE,
            Some(format!("content {n}")),
            audit(code),
            owner.clone(),
        )
        .unwrap()
        .into()
    }

    #[test]
    fn a_well_formed_history_commits() {
        let (mut vo, owner) = versioned();
        vo.commit(version(
            1,
            None,
            &owner,
            terminology::audit_change_type::CREATION,
        ))
        .unwrap();
        vo.commit(version(
            2,
            Some(1),
            &owner,
            terminology::audit_change_type::AMENDMENT,
        ))
        .unwrap();
        assert_eq!(vo.version_count(), 2);
        assert_eq!(
            vo.latest_version()
                .unwrap()
                .uid()
                .version_tree_id()
                .trunk_version(),
            2
        );
    }

    #[test]
    fn concurrent_writes_are_refused_rather_than_silently_ordered() {
        let (mut vo, owner) = versioned();
        vo.commit(version(
            1,
            None,
            &owner,
            terminology::audit_change_type::CREATION,
        ))
        .unwrap();
        vo.commit(version(
            2,
            Some(1),
            &owner,
            terminology::audit_change_type::AMENDMENT,
        ))
        .unwrap();
        // Two clients both read version 1 and both write version 3 against it.
        let stale = version(
            3,
            Some(1),
            &owner,
            terminology::audit_change_type::AMENDMENT,
        );
        assert_eq!(vo.commit(stale), Err(CommitError::NotLatest));
    }

    #[test]
    fn a_version_of_another_object_is_refused() {
        let (mut vo, owner) = versioned();
        let foreign = OriginalVersion::new(
            "11111111-2222-3333-4444-555555555555::ehr1.example.org::1"
                .parse()
                .unwrap(),
            None,
            terminology::version_lifecycle_state::COMPLETE,
            Some("x".to_string()),
            audit(terminology::audit_change_type::CREATION),
            owner,
        )
        .unwrap();
        assert_eq!(vo.commit(foreign.into()), Err(CommitError::WrongObject));
    }

    #[test]
    fn a_duplicate_version_id_is_refused() {
        let (mut vo, owner) = versioned();
        vo.commit(version(
            1,
            None,
            &owner,
            terminology::audit_change_type::CREATION,
        ))
        .unwrap();
        assert_eq!(
            vo.commit(version(
                1,
                None,
                &owner,
                terminology::audit_change_type::CREATION
            )),
            Err(CommitError::DuplicateVersion)
        );
    }

    #[test]
    fn a_deleted_version_may_have_no_data_and_others_may_not() {
        let (_, owner) = versioned();
        let id: ObjectVersionId = "87284370-2D4B-4E3D-A3F3-F303D2F4F34B::ehr1.example.org::2"
            .parse()
            .unwrap();
        assert!(
            OriginalVersion::<String>::new(
                id.clone(),
                None,
                terminology::version_lifecycle_state::DELETED,
                None,
                audit(terminology::audit_change_type::DELETED),
                owner.clone(),
            )
            .is_ok()
        );
        assert!(
            OriginalVersion::<String>::new(
                id,
                None,
                terminology::version_lifecycle_state::COMPLETE,
                None,
                audit(terminology::audit_change_type::CREATION),
                owner,
            )
            .is_err()
        );
    }

    #[test]
    fn contribution_change_types_are_restricted() {
        let uid = HierObjectId::from_uid_str("87284370-2D4B-4E3D-A3F3-F303D2F4F34B").unwrap();
        let vs: Vec<ObjectVersionId> = vec![
            "87284370-2D4B-4E3D-A3F3-F303D2F4F34B::s.example::1"
                .parse()
                .unwrap(),
        ];
        assert!(
            Contribution::new(
                uid.clone(),
                vs.clone(),
                audit(terminology::audit_change_type::CREATION)
            )
            .is_ok()
        );
        // `synthesis` is a valid AUDIT_DETAILS change type and not a valid
        // CONTRIBUTION one.
        assert!(
            Contribution::new(
                uid.clone(),
                vs,
                audit(terminology::audit_change_type::SYNTHESIS)
            )
            .is_err()
        );
        assert!(
            Contribution::new(
                uid,
                Vec::new(),
                audit(terminology::audit_change_type::CREATION)
            )
            .is_err()
        );
    }

    #[test]
    fn party_proxy_infers_its_class_without_a_type_tag() {
        let related: PartyProxy = serde_json::from_str(
            r#"{"name":"Jane","relationship":{"value":"mother","defining_code":{"terminology_id":{"value":"openehr"},"code_string":"10"}}}"#,
        )
        .unwrap();
        assert_eq!(related.type_name(), "PARTY_RELATED");
        assert!(!related.is_subject());

        let identified: PartyProxy = serde_json::from_str(r#"{"name":"Dr A Nurse"}"#).unwrap();
        assert_eq!(identified.type_name(), "PARTY_IDENTIFIED");

        let subject: PartyProxy = serde_json::from_str("{}").unwrap();
        assert_eq!(subject.type_name(), "PARTY_SELF");
        assert!(subject.is_subject());
    }

    #[test]
    fn a_self_related_party_counts_as_the_subject() {
        // The trap: code that checks only for PARTY_SELF treats an entry whose
        // subject is PARTY_RELATED(0|self|) as being about somebody else.
        let self_related = PartyRelated::new(
            PartyIdentified::named("The patient").unwrap(),
            terminology::subject_relationship::SELF,
        )
        .unwrap();
        assert!(PartyProxy::Related(self_related).is_subject());
    }

    #[test]
    fn version_at_time_skips_incomparable_commit_times() {
        let (mut vo, owner) = versioned();
        vo.commit(version(
            1,
            None,
            &owner,
            terminology::audit_change_type::CREATION,
        ))
        .unwrap();
        // Commit times carry `Z`; a local-time query is not comparable with
        // them, and answering "the latest" would be a guess about the zone.
        let local = DvDateTime::new("2026-07-31T10:00:00").unwrap();
        assert!(vo.version_at_time(&local).is_none());
        let utc = DvDateTime::new("2026-07-31T10:00:00Z").unwrap();
        assert!(vo.version_at_time(&utc).is_some());
    }
}
