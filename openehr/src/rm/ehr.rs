//! The openEHR **EHR Information Model**: the record, its status, its access
//! settings, and the compositions that hold clinical content.
//!
//! ```text
//! EHR ─┬─ EHR_STATUS       COMPOSITION ── CONTENT_ITEM ─┬─ SECTION
//!      ├─ EHR_ACCESS            │                       └─ ENTRY ─┬─ ADMIN_ENTRY
//!      ├─ FOLDER                └─ EVENT_CONTEXT                  └─ CARE_ENTRY ─┬─ OBSERVATION
//!      ├─ COMPOSITION*                                                           ├─ EVALUATION
//!      └─ CONTRIBUTION*                                                          ├─ INSTRUCTION
//!                                                                                └─ ACTION
//! ```
//!
//! # `EHR` holds references, not objects
//!
//! Every attribute of [`Ehr`] except `ehr_id` and `time_created` is an
//! `OBJECT_REF`. That is not laziness in the model: a composition is a
//! *versioned* object, and an EHR that embedded compositions would have to
//! embed their whole version trees, which is the entire record. The indirection
//! is what makes an EHR a manageable object and what makes `version_at_time`
//! answerable per composition rather than per record.
//!
//! # `INSTRUCTION` and `ACTION` are two halves of one thing
//!
//! An `INSTRUCTION` is an order: what should happen. Each `ACTION` records one
//! transition of that order's state machine: what did happen. They are separate
//! classes because they are committed at different times by different people,
//! and because an order can be superseded without rewriting the record of what
//! was already done to fulfil it. [`InstructionDetails`] is the link back.

use crate::base::{HierObjectId, LocatableRef, ObjectRef};
use crate::error::ParseError;
use crate::rm::common::{
    Locatable, LocatableAttrs, Participation, PartyProxy, PartySelf, impl_locatable,
};
use crate::rm::data_structures::{History, ItemStructure};
use crate::rm::data_types::{CodePhrase, DvCodedText, DvDateTime, DvParsable, DvText, Text};
use crate::rm::rm_type_tag;
use crate::terminology;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// The record
// ---------------------------------------------------------------------------

rm_type_tag!(CompositionTag, "COMPOSITION");
rm_type_tag!(EhrStatusTag, "EHR_STATUS");
rm_type_tag!(FolderTag, "FOLDER");
rm_type_tag!(EventContextTag, "EVENT_CONTEXT");
rm_type_tag!(ActivityTag, "ACTIVITY");
rm_type_tag!(IsmTransitionTag, "ISM_TRANSITION");
rm_type_tag!(InstructionDetailsTag, "INSTRUCTION_DETAILS");

/// The root of a health record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// Three fields end in `_id` because openEHR names them that way, and one of
// them (`ehr_id`) is the identifier every other system refers to this record
// by. Renaming them for Rust's aesthetics would cost the correspondence with
// the class definition and gain nothing.
#[allow(clippy::struct_field_names)]
pub struct Ehr {
    system_id: HierObjectId,
    ehr_id: HierObjectId,
    ehr_status: ObjectRef,
    ehr_access: ObjectRef,
    time_created: DvDateTime,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    compositions: Vec<ObjectRef>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    contributions: Vec<ObjectRef>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    folders: Vec<ObjectRef>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    directory: Option<ObjectRef>,
}

impl Ehr {
    /// Builds an EHR.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if `ehr_status` or `ehr_access` does not reference
    /// the type openEHR requires — `Ehr_status_valid` and `Ehr_access_valid`.
    ///
    /// An `OBJECT_REF` carries its target's type as a **string**, so nothing in
    /// Rust's type system stops an `EHR` pointing its status at a versioned
    /// composition. openEHR states both as invariants; this crate did not check
    /// either, and its own fixture built both references with type `"EHR"` from
    /// the day it was written (`A-21`).
    pub fn new(
        system_id: HierObjectId,
        ehr_id: HierObjectId,
        ehr_status: ObjectRef,
        ehr_access: ObjectRef,
        time_created: DvDateTime,
    ) -> Result<Self, ParseError> {
        if ehr_status.type_name() != "VERSIONED_EHR_STATUS" {
            return Err(ParseError::invariant("EHR", "Ehr_status_valid"));
        }
        if ehr_access.type_name() != "VERSIONED_EHR_ACCESS" {
            return Err(ParseError::invariant("EHR", "Ehr_access_valid"));
        }
        Ok(Self {
            system_id,
            ehr_id,
            ehr_status,
            ehr_access,
            time_created,
            compositions: Vec::new(),
            contributions: Vec::new(),
            folders: Vec::new(),
            directory: None,
        })
    }

    /// Adds a composition reference.
    #[must_use]
    pub fn with_composition(mut self, composition: ObjectRef) -> Self {
        self.compositions.push(composition);
        self
    }

    /// Adds a contribution reference.
    #[must_use]
    pub fn with_contribution(mut self, contribution: ObjectRef) -> Self {
        self.contributions.push(contribution);
        self
    }

    /// Sets the folder hierarchy.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the folder list is empty
    /// (`Directory_in_folders`): the `directory` attribute is defined to be the
    /// first folder, and setting it from nothing would leave the two
    /// disagreeing.
    pub fn with_folders(mut self, folders: Vec<ObjectRef>) -> Result<Self, ParseError> {
        let Some(first) = folders.first().cloned() else {
            return Err(ParseError::invariant("EHR", "Directory_in_folders"));
        };
        self.directory = Some(first);
        self.folders = folders;
        Ok(self)
    }

    /// The EHR's identifier.
    #[must_use]
    pub fn ehr_id(&self) -> &HierObjectId {
        &self.ehr_id
    }

    /// The system managing the record.
    #[must_use]
    pub fn system_id(&self) -> &HierObjectId {
        &self.system_id
    }

    /// Reference to the versioned `EHR_STATUS`.
    #[must_use]
    pub fn ehr_status(&self) -> &ObjectRef {
        &self.ehr_status
    }

    /// Reference to the versioned `EHR_ACCESS`.
    #[must_use]
    pub fn ehr_access(&self) -> &ObjectRef {
        &self.ehr_access
    }

    /// When the record was created.
    #[must_use]
    pub fn time_created(&self) -> &DvDateTime {
        &self.time_created
    }

    /// References to the record's compositions.
    #[must_use]
    pub fn compositions(&self) -> &[ObjectRef] {
        &self.compositions
    }

    /// References to the record's contributions.
    #[must_use]
    pub fn contributions(&self) -> &[ObjectRef] {
        &self.contributions
    }

    /// References to the record's folder hierarchies.
    #[must_use]
    pub fn folders(&self) -> &[ObjectRef] {
        &self.folders
    }

    /// The first folder hierarchy, kept for backward compatibility.
    #[must_use]
    pub fn directory(&self) -> Option<&ObjectRef> {
        self.directory.as_ref()
    }
}

/// The record's operational flags and its subject.
///
/// # `is_modifiable` and `is_queryable` are not the same switch
///
/// A record is deactivated — `is_modifiable = false` — on death, on a merge
/// into another record, on migration, or on the patient opting out of further
/// recording. It usually stays **queryable**, because the history is still
/// clinically and legally live. Code that treats deactivation as deletion
/// removes a record that law requires be retained.
///
/// ```
/// use openehr::rm::common::{LocatableAttrs, PartySelf};
/// use openehr::rm::ehr::EhrStatus;
///
/// let status = EhrStatus::new(
///     LocatableAttrs::named("EHR Status", "openEHR-EHR-EHR_STATUS.generic.v1").unwrap(),
///     PartySelf::anonymous(),
///     true,
///     true,
/// );
/// assert!(status.is_active());
///
/// let deceased = status.clone().set_modifiable(false);
/// assert!(!deceased.is_active());
/// assert!(deceased.is_queryable()); // still readable, and must be
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EhrStatus {
    #[serde(rename = "_type", default)]
    rm_type: EhrStatusTag,
    #[serde(flatten)]
    locatable: LocatableAttrs,
    subject: PartySelf,
    is_queryable: bool,
    is_modifiable: bool,
    // Boxed for deserializer frame size; see the note on
    // `LocatableAttrs::archetype_details` and spec/audit.md A-03.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    other_details: Option<Box<ItemStructure>>,
}

impl_locatable!(EhrStatus, "EHR_STATUS");

impl EhrStatus {
    /// Builds an EHR status.
    #[must_use]
    pub fn new(
        locatable: LocatableAttrs,
        subject: PartySelf,
        is_queryable: bool,
        is_modifiable: bool,
    ) -> Self {
        Self {
            locatable,
            subject,
            rm_type: EhrStatusTag,
            is_queryable,
            is_modifiable,
            other_details: None,
        }
    }

    /// Attaches archetyped record-level metadata.
    #[must_use]
    pub fn with_other_details(mut self, other_details: ItemStructure) -> Self {
        self.other_details = Some(Box::new(other_details));
        self
    }

    /// The record's subject.
    #[must_use]
    pub fn subject(&self) -> &PartySelf {
        &self.subject
    }

    /// Whether the record appears in population queries.
    #[must_use]
    pub fn is_queryable(&self) -> bool {
        self.is_queryable
    }

    /// Whether new content may be written to the record.
    #[must_use]
    pub fn is_modifiable(&self) -> bool {
        self.is_modifiable
    }

    /// Archetyped record-level metadata.
    #[must_use]
    pub fn other_details(&self) -> Option<&ItemStructure> {
        self.other_details.as_deref()
    }

    /// Whether the record is active — that is, writable.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.is_modifiable
    }

    /// Returns a copy with `is_modifiable` set.
    #[must_use]
    pub fn set_modifiable(mut self, modifiable: bool) -> Self {
        self.is_modifiable = modifiable;
        self
    }

    /// Returns a copy with `is_queryable` set.
    #[must_use]
    pub fn set_queryable(mut self, queryable: bool) -> Self {
        self.is_queryable = queryable;
        self
    }
}

/// A hierarchical grouping of compositions.
///
/// A folder holds **references**, so more than one folder tree can classify the
/// same composition — by episode, by problem, by specialty — without
/// duplicating it. A model that nested compositions inside folders would force
/// a single classification and make the second one a copy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Folder {
    #[serde(rename = "_type", default)]
    rm_type: FolderTag,
    #[serde(flatten)]
    locatable: LocatableAttrs,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    items: Vec<ObjectRef>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    folders: Vec<Folder>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    details: Option<ItemStructure>,
}

impl_locatable!(Folder, "FOLDER");

impl Folder {
    /// Builds a folder.
    #[must_use]
    pub fn new(locatable: LocatableAttrs) -> Self {
        Self {
            locatable,
            rm_type: FolderTag,
            items: Vec::new(),
            folders: Vec::new(),
            details: None,
        }
    }

    /// Adds a reference to a composition or other object.
    #[must_use]
    pub fn with_item(mut self, item: ObjectRef) -> Self {
        self.items.push(item);
        self
    }

    /// Adds a sub-folder.
    #[must_use]
    pub fn with_folder(mut self, folder: Folder) -> Self {
        self.folders.push(folder);
        self
    }

    /// The referenced objects.
    #[must_use]
    pub fn items(&self) -> &[ObjectRef] {
        &self.items
    }

    /// The sub-folders.
    #[must_use]
    pub fn folders(&self) -> &[Folder] {
        &self.folders
    }

    /// Archetyped folder metadata.
    #[must_use]
    pub fn details(&self) -> Option<&ItemStructure> {
        self.details.as_ref()
    }
}

// ---------------------------------------------------------------------------
// Composition
// ---------------------------------------------------------------------------

/// The circumstances of the healthcare event a composition documents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventContext {
    #[serde(rename = "_type", default)]
    rm_type: EventContextTag,
    start_time: DvDateTime,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    end_time: Option<DvDateTime>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    location: Option<String>,
    setting: DvCodedText,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    health_care_facility: Option<crate::rm::common::PartyIdentified>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    participations: Vec<Participation>,
    // Boxed for deserializer frame size; see the note on
    // `LocatableAttrs::archetype_details` and spec/audit.md A-03.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    other_context: Option<Box<ItemStructure>>,
}

impl EventContext {
    /// Builds an event context from an openEHR `setting` code.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the code is not in the `setting` group.
    pub fn new(start_time: DvDateTime, setting_code: &str) -> Result<Self, ParseError> {
        let setting = terminology::setting::GROUP
            .coded_text(setting_code)
            .ok_or_else(|| ParseError::invariant("EVENT_CONTEXT", "Setting_valid"))?;
        Ok(Self {
            start_time,
            rm_type: EventContextTag,
            end_time: None,
            location: None,
            setting,
            health_care_facility: None,
            participations: Vec::new(),
            other_context: None,
        })
    }

    /// Records when the event ended.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the end time is established to be before the
    /// start time. Times that are not comparable — one with a UTC offset and
    /// one without — are accepted rather than guessed at, and reported by
    /// [`crate::validation`] instead.
    pub fn with_end_time(mut self, end_time: DvDateTime) -> Result<Self, ParseError> {
        if matches!(
            end_time.partial_cmp(&self.start_time),
            Some(core::cmp::Ordering::Less)
        ) {
            return Err(ParseError::invariant("EVENT_CONTEXT", "End_time_valid"));
        }
        self.end_time = Some(end_time);
        Ok(self)
    }

    /// Records where the event happened, as free text.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the location is empty
    /// (`EVENT_CONTEXT.location_valid`). A present-but-empty location is
    /// indistinguishable from an absent one to every reader, and openEHR has a
    /// way to say absent.
    pub fn with_location(mut self, location: impl Into<String>) -> Result<Self, ParseError> {
        let location = location.into();
        if location.is_empty() {
            return Err(ParseError::invariant("EVENT_CONTEXT", "location_valid"));
        }
        self.location = Some(location);
        Ok(self)
    }

    /// Adds a participation.
    #[must_use]
    pub fn with_participation(mut self, participation: Participation) -> Self {
        self.participations.push(participation);
        self
    }

    /// Attaches archetyped context data.
    #[must_use]
    pub fn with_other_context(mut self, other_context: ItemStructure) -> Self {
        self.other_context = Some(Box::new(other_context));
        self
    }

    /// When the event started.
    #[must_use]
    pub fn start_time(&self) -> &DvDateTime {
        &self.start_time
    }

    /// When the event ended, if it has.
    #[must_use]
    pub fn end_time(&self) -> Option<&DvDateTime> {
        self.end_time.as_ref()
    }

    /// The care setting.
    #[must_use]
    pub fn setting(&self) -> &DvCodedText {
        &self.setting
    }

    /// Where the event happened.
    #[must_use]
    pub fn location(&self) -> Option<&str> {
        self.location.as_deref()
    }

    /// Who took part.
    #[must_use]
    pub fn participations(&self) -> &[Participation] {
        &self.participations
    }

    /// Archetyped context data.
    #[must_use]
    pub fn other_context(&self) -> Option<&ItemStructure> {
        self.other_context.as_deref()
    }
}

/// The unit of committal to a health record.
///
/// ```
/// use openehr::rm::common::{Archetyped, LocatableAttrs, PartyIdentified};
/// use openehr::rm::data_types::CodePhrase;
/// use openehr::rm::ehr::Composition;
/// use openehr::terminology::composition_category;
///
/// let composition = Composition::new(
///     LocatableAttrs::named("Encounter", "openEHR-EHR-COMPOSITION.encounter.v1").unwrap()
///         .with_archetype_details(
///             Archetyped::new("openEHR-EHR-COMPOSITION.encounter.v1", "1.1.0").unwrap(),
///         ),
///     composition_category::EVENT,
///     PartyIdentified::named("Dr A Nurse").unwrap().into(),
///     CodePhrase::new("ISO_639-1", "en").unwrap(),
///     CodePhrase::new("ISO_3166-1", "GB").unwrap(),
/// ).unwrap();
///
/// assert!(composition.is_event());
/// assert!(!composition.is_persistent());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Composition {
    #[serde(rename = "_type", default)]
    rm_type: CompositionTag,
    #[serde(flatten)]
    locatable: LocatableAttrs,
    language: CodePhrase,
    territory: CodePhrase,
    category: DvCodedText,
    composer: PartyProxy,
    // Boxed for deserializer frame size; see the note on
    // `LocatableAttrs::archetype_details` and spec/audit.md A-03.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    context: Option<Box<EventContext>>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    content: Vec<ContentItem>,
}

impl_locatable!(Composition, "COMPOSITION");

impl Composition {
    /// Builds a composition from an openEHR `composition_category` code.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the code is not in the
    /// `composition_category` group.
    pub fn new(
        locatable: LocatableAttrs,
        category_code: &str,
        composer: PartyProxy,
        language: CodePhrase,
        territory: CodePhrase,
    ) -> Result<Self, ParseError> {
        let category = terminology::composition_category::GROUP
            .coded_text(category_code)
            .ok_or_else(|| ParseError::invariant("COMPOSITION", "Category_validity"))?;
        Ok(Self {
            locatable,
            rm_type: CompositionTag,
            language,
            territory,
            category,
            composer,
            context: None,
            content: Vec::new(),
        })
    }

    /// Attaches the healthcare event context.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if this composition is **persistent**.
    /// openEHR's `Is_persistent_validity: is_persistent implies context = Void`
    /// is a formal invariant, and the prose is blunter still: *"Persistent
    /// Compositions do not have an Event context."*
    ///
    /// The reason is that a persistent composition — a problem list, a
    /// medication list — is not a record of an encounter. It is a running
    /// summary updated across many encounters, so attaching *one* encounter's
    /// context to it asserts that the whole list belongs to that visit.
    pub fn with_context(mut self, context: EventContext) -> Result<Self, ParseError> {
        if self.is_persistent() {
            return Err(ParseError::invariant(
                "COMPOSITION",
                "Is_persistent_validity",
            ));
        }
        self.context = Some(Box::new(context));
        Ok(self)
    }

    /// Adds a content item.
    #[must_use]
    pub fn with_content(mut self, item: ContentItem) -> Self {
        self.content.push(item);
        self
    }

    /// The recording language.
    #[must_use]
    pub fn language(&self) -> &CodePhrase {
        &self.language
    }

    /// The jurisdiction the composition was recorded in.
    #[must_use]
    pub fn territory(&self) -> &CodePhrase {
        &self.territory
    }

    /// How long the content stays current.
    #[must_use]
    pub fn category(&self) -> &DvCodedText {
        &self.category
    }

    /// Who authored the content.
    #[must_use]
    pub fn composer(&self) -> &PartyProxy {
        &self.composer
    }

    /// The healthcare event context, if any.
    #[must_use]
    pub fn context(&self) -> Option<&EventContext> {
        self.context.as_deref()
    }

    /// The clinical and administrative content.
    #[must_use]
    pub fn content(&self) -> &[ContentItem] {
        &self.content
    }

    /// The category's openEHR code.
    #[must_use]
    pub fn category_code(&self) -> &str {
        self.category.defining_code().code_string()
    }

    /// Whether the content is a running list rather than a snapshot —
    /// `431｜persistent｜`.
    #[must_use]
    pub fn is_persistent(&self) -> bool {
        self.category_code() == terminology::composition_category::PERSISTENT
    }

    /// Whether the content documents one healthcare activity —
    /// `433｜event｜`.
    #[must_use]
    pub fn is_event(&self) -> bool {
        self.category_code() == terminology::composition_category::EVENT
    }

    /// Every [`Entry`] in the composition, however deeply nested in sections.
    ///
    /// The traversal every consumer needs and nobody wants to write: sections
    /// nest arbitrarily, and an entry three sections deep is exactly as
    /// clinically significant as one at the top.
    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.content.iter().flat_map(ContentItem::entries)
    }
}

/// Either kind of composition content.
///
/// # Serialization is flat, not nested
///
/// The obvious encoding — a `SECTION` variant plus an untagged `Entry` variant
/// — makes serde buffer the entire content subtree before it can decide, once
/// per nesting level. On a real composition that is measurably slower and
/// deeply recursive. [`ContentItemWire`] flattens the two levels into one
/// six-way `_type` dispatch, which is also exactly what the JSON looks like.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "ContentItemWire", into = "ContentItemWire")]
// An ENTRY is much larger than a SECTION, which is a name and a list. Boxing
// the entry would put an allocation on every clinical statement in every
// composition — the commonest object in the model — to save stack in the rare
// section-only case.
#[allow(clippy::large_enum_variant)]
pub enum ContentItem {
    /// A navigational heading.
    Section(Section),
    /// A clinical or administrative statement.
    Entry(Entry),
}

/// The flat six-way `_type` dispatch [`ContentItem`] serializes through.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type")]
#[allow(clippy::large_enum_variant)]
#[doc(hidden)]
pub enum ContentItemWire {
    /// `SECTION`
    #[serde(rename = "SECTION")]
    Section(Section),
    /// `OBSERVATION`
    #[serde(rename = "OBSERVATION")]
    Observation(Observation),
    /// `EVALUATION`
    #[serde(rename = "EVALUATION")]
    Evaluation(Evaluation),
    /// `INSTRUCTION`
    #[serde(rename = "INSTRUCTION")]
    Instruction(Instruction),
    /// `ACTION`
    #[serde(rename = "ACTION")]
    Action(Action),
    /// `ADMIN_ENTRY`
    #[serde(rename = "ADMIN_ENTRY")]
    AdminEntry(AdminEntry),
}

impl From<ContentItem> for ContentItemWire {
    fn from(v: ContentItem) -> Self {
        match v {
            ContentItem::Section(s) => Self::Section(s),
            ContentItem::Entry(Entry::Observation(e)) => Self::Observation(e),
            ContentItem::Entry(Entry::Evaluation(e)) => Self::Evaluation(e),
            ContentItem::Entry(Entry::Instruction(e)) => Self::Instruction(e),
            ContentItem::Entry(Entry::Action(e)) => Self::Action(e),
            ContentItem::Entry(Entry::AdminEntry(e)) => Self::AdminEntry(e),
        }
    }
}

impl From<ContentItemWire> for ContentItem {
    fn from(v: ContentItemWire) -> Self {
        match v {
            ContentItemWire::Section(s) => Self::Section(s),
            ContentItemWire::Observation(e) => Self::Entry(Entry::Observation(e)),
            ContentItemWire::Evaluation(e) => Self::Entry(Entry::Evaluation(e)),
            ContentItemWire::Instruction(e) => Self::Entry(Entry::Instruction(e)),
            ContentItemWire::Action(e) => Self::Entry(Entry::Action(e)),
            ContentItemWire::AdminEntry(e) => Self::Entry(Entry::AdminEntry(e)),
        }
    }
}

impl ContentItem {
    /// Every entry at or under this item.
    pub fn entries(&self) -> Box<dyn Iterator<Item = &Entry> + '_> {
        match self {
            Self::Entry(e) => Box::new(core::iter::once(e)),
            Self::Section(s) => Box::new(s.items().iter().flat_map(ContentItem::entries)),
        }
    }

    /// The locatable attributes.
    #[must_use]
    pub fn locatable(&self) -> &LocatableAttrs {
        match self {
            Self::Section(s) => s.locatable(),
            Self::Entry(e) => e.locatable(),
        }
    }
}

/// A heading that organises content without adding clinical meaning.
///
/// openEHR is explicit that a `SECTION` is **navigational**: moving an entry
/// from one section to another must not change what it says. Software that
/// derives meaning from section membership — "everything under Diagnoses is a
/// diagnosis" — breaks the moment a template is redesigned.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Section {
    #[serde(flatten)]
    locatable: LocatableAttrs,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    items: Vec<ContentItem>,
}

impl_locatable!(Section, "SECTION");

impl Section {
    /// Builds a section.
    #[must_use]
    pub fn new(locatable: LocatableAttrs, items: Vec<ContentItem>) -> Self {
        Self { locatable, items }
    }

    /// The contained items.
    #[must_use]
    pub fn items(&self) -> &[ContentItem] {
        &self.items
    }
}

// ---------------------------------------------------------------------------
// Entries
// ---------------------------------------------------------------------------

/// The attributes every `ENTRY` carries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntryAttrs {
    language: CodePhrase,
    encoding: CodePhrase,
    subject: PartyProxy,
    // Boxed for deserializer frame size; see the note on
    // `LocatableAttrs::archetype_details` and spec/audit.md A-03.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    provider: Option<Box<PartyProxy>>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    other_participations: Vec<Participation>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    workflow_id: Option<ObjectRef>,
}

impl EntryAttrs {
    /// Builds entry attributes for a statement about the record subject.
    #[must_use]
    pub fn about_subject(language: CodePhrase, encoding: CodePhrase) -> Self {
        Self {
            language,
            encoding,
            subject: PartySelf::anonymous().into(),
            provider: None,
            other_participations: Vec::new(),
            workflow_id: None,
        }
    }

    /// Builds entry attributes for a statement about someone else.
    ///
    /// Taking the subject explicitly rather than defaulting it: an entry that
    /// is about a family member and does not say so reads as a finding about
    /// the patient, and that error is invisible in every rendering.
    #[must_use]
    pub fn about(language: CodePhrase, encoding: CodePhrase, subject: PartyProxy) -> Self {
        Self {
            language,
            encoding,
            subject,
            provider: None,
            other_participations: Vec::new(),
            workflow_id: None,
        }
    }

    /// Records who supplied the information.
    #[must_use]
    pub fn with_provider(mut self, provider: PartyProxy) -> Self {
        self.provider = Some(Box::new(provider));
        self
    }

    /// Adds a participation.
    #[must_use]
    pub fn with_participation(mut self, participation: Participation) -> Self {
        self.other_participations.push(participation);
        self
    }

    /// Who the statement is about.
    #[must_use]
    pub fn subject(&self) -> &PartyProxy {
        &self.subject
    }

    /// Who supplied it.
    #[must_use]
    pub fn provider(&self) -> Option<&PartyProxy> {
        self.provider.as_deref()
    }

    /// The recording language.
    #[must_use]
    pub fn language(&self) -> &CodePhrase {
        &self.language
    }

    /// The character encoding.
    #[must_use]
    pub fn encoding(&self) -> &CodePhrase {
        &self.encoding
    }

    /// Other participants.
    #[must_use]
    pub fn other_participations(&self) -> &[Participation] {
        &self.other_participations
    }
}

/// The attributes every `CARE_ENTRY` adds to `ENTRY`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CareEntryAttrs {
    // Boxed for deserializer frame size; see the note on
    // `LocatableAttrs::archetype_details` and spec/audit.md A-03.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    protocol: Option<Box<ItemStructure>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    guideline_id: Option<ObjectRef>,
}

impl CareEntryAttrs {
    /// Records how the information was obtained — the method, the device, the
    /// cuff size.
    #[must_use]
    pub fn with_protocol(mut self, protocol: ItemStructure) -> Self {
        self.protocol = Some(Box::new(protocol));
        self
    }

    /// The method by which the information was obtained.
    #[must_use]
    pub fn protocol(&self) -> Option<&ItemStructure> {
        self.protocol.as_deref()
    }

    /// The guideline the entry was recorded under.
    #[must_use]
    pub fn guideline_id(&self) -> Option<&ObjectRef> {
        self.guideline_id.as_ref()
    }
}

/// An administrative statement: an admission, a discharge, an appointment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdminEntry {
    #[serde(flatten)]
    locatable: LocatableAttrs,
    #[serde(flatten)]
    entry: EntryAttrs,
    data: ItemStructure,
}

impl_locatable!(AdminEntry, "ADMIN_ENTRY");

impl AdminEntry {
    /// Builds an administrative entry.
    #[must_use]
    pub fn new(locatable: LocatableAttrs, entry: EntryAttrs, data: ItemStructure) -> Self {
        Self {
            locatable,
            entry,
            data,
        }
    }

    /// The entry attributes.
    #[must_use]
    pub fn entry(&self) -> &EntryAttrs {
        &self.entry
    }

    /// The administrative data.
    #[must_use]
    pub fn data(&self) -> &ItemStructure {
        &self.data
    }
}

/// Something observed or measured.
///
/// `data` is a [`History`] even for a single reading, because openEHR treats
/// every observation as a time series of length ≥ 1. That is what makes a
/// one-off blood pressure and a continuous arterial trace the same shape, and
/// it is why a consumer never has to special-case the second reading.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    #[serde(flatten)]
    locatable: LocatableAttrs,
    #[serde(flatten)]
    entry: EntryAttrs,
    #[serde(flatten)]
    care_entry: CareEntryAttrs,
    data: History,
    // Boxed for deserializer frame size; see the note on
    // `LocatableAttrs::archetype_details` and spec/audit.md A-03.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    state: Option<Box<History>>,
}

impl_locatable!(Observation, "OBSERVATION");

impl Observation {
    /// Builds an observation.
    #[must_use]
    pub fn new(locatable: LocatableAttrs, entry: EntryAttrs, data: History) -> Self {
        Self {
            locatable,
            entry,
            care_entry: CareEntryAttrs::default(),
            data,
            state: None,
        }
    }

    /// Attaches the subject's state series.
    #[must_use]
    pub fn with_state(mut self, state: History) -> Self {
        self.state = Some(Box::new(state));
        self
    }

    /// Attaches the care-entry attributes.
    #[must_use]
    pub fn with_care_entry(mut self, care_entry: CareEntryAttrs) -> Self {
        self.care_entry = care_entry;
        self
    }

    /// The entry attributes.
    #[must_use]
    pub fn entry(&self) -> &EntryAttrs {
        &self.entry
    }

    /// The care-entry attributes.
    #[must_use]
    pub fn care_entry(&self) -> &CareEntryAttrs {
        &self.care_entry
    }

    /// The observed series.
    #[must_use]
    pub fn data(&self) -> &History {
        &self.data
    }

    /// The subject's state series.
    #[must_use]
    pub fn state(&self) -> Option<&History> {
        self.state.as_deref()
    }
}

/// A clinical judgement: a diagnosis, a risk assessment, a goal.
///
/// No `HISTORY`, because an evaluation is an opinion held at the moment of
/// recording rather than a series of measurements. Superseding it means a new
/// version, not a new event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Evaluation {
    #[serde(flatten)]
    locatable: LocatableAttrs,
    #[serde(flatten)]
    entry: EntryAttrs,
    #[serde(flatten)]
    care_entry: CareEntryAttrs,
    data: ItemStructure,
}

impl_locatable!(Evaluation, "EVALUATION");

impl Evaluation {
    /// Builds an evaluation.
    #[must_use]
    pub fn new(locatable: LocatableAttrs, entry: EntryAttrs, data: ItemStructure) -> Self {
        Self {
            locatable,
            entry,
            care_entry: CareEntryAttrs::default(),
            data,
        }
    }

    /// The entry attributes.
    #[must_use]
    pub fn entry(&self) -> &EntryAttrs {
        &self.entry
    }

    /// The judgement.
    #[must_use]
    pub fn data(&self) -> &ItemStructure {
        &self.data
    }
}

/// One planned step of an [`Instruction`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Activity {
    #[serde(rename = "_type", default)]
    rm_type: ActivityTag,
    #[serde(flatten)]
    locatable: LocatableAttrs,
    description: ItemStructure,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    timing: Option<DvParsable>,
    action_archetype_id: String,
}

impl_locatable!(Activity, "ACTIVITY");

impl Activity {
    /// Builds an activity.
    ///
    /// `action_archetype_id` is a **regular expression** matching the
    /// archetype ids of the `ACTION`s that may fulfil this activity — openEHR
    /// writes it as `/.*/` when any action will do. It is not an archetype id,
    /// and this crate does not parse it as one; validating it as an id would
    /// reject the commonest value there is.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if `action_archetype_id` is empty.
    pub fn new(
        locatable: LocatableAttrs,
        description: ItemStructure,
        action_archetype_id: impl Into<String>,
    ) -> Result<Self, ParseError> {
        let action_archetype_id = action_archetype_id.into();
        if action_archetype_id.is_empty() {
            return Err(ParseError::invariant(
                "ACTIVITY",
                "Action_archetype_id_valid",
            ));
        }
        Ok(Self {
            locatable,
            description,
            rm_type: ActivityTag,
            timing: None,
            action_archetype_id,
        })
    }

    /// Records when the activity should happen.
    #[must_use]
    pub fn with_timing(mut self, timing: DvParsable) -> Self {
        self.timing = Some(timing);
        self
    }

    /// What should be done.
    #[must_use]
    pub fn description(&self) -> &ItemStructure {
        &self.description
    }

    /// When it should be done.
    #[must_use]
    pub fn timing(&self) -> Option<&DvParsable> {
        self.timing.as_ref()
    }

    /// The pattern matching the archetypes of fulfilling actions.
    #[must_use]
    pub fn action_archetype_id(&self) -> &str {
        &self.action_archetype_id
    }
}

/// An order: something that should be done.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Instruction {
    #[serde(flatten)]
    locatable: LocatableAttrs,
    #[serde(flatten)]
    entry: EntryAttrs,
    #[serde(flatten)]
    care_entry: CareEntryAttrs,
    narrative: Text,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    activities: Vec<Activity>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    expiry_time: Option<DvDateTime>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    wf_definition: Option<DvParsable>,
}

impl_locatable!(Instruction, "INSTRUCTION");

impl Instruction {
    /// Builds an instruction.
    ///
    /// The `narrative` is mandatory and is not a summary of the activities: it
    /// is the human-readable order, the thing a clinician is accountable for
    /// having written. An instruction whose narrative was generated from its
    /// structured activities records what the software understood, not what the
    /// prescriber meant.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the activity list is empty
    /// (`Activities_valid`). An order with no steps orders nothing.
    pub fn new(
        locatable: LocatableAttrs,
        entry: EntryAttrs,
        narrative: Text,
        activities: Vec<Activity>,
    ) -> Result<Self, ParseError> {
        if activities.is_empty() {
            return Err(ParseError::invariant("INSTRUCTION", "Activities_valid"));
        }
        Ok(Self {
            locatable,
            entry,
            care_entry: CareEntryAttrs::default(),
            narrative,
            activities,
            expiry_time: None,
            wf_definition: None,
        })
    }

    /// Records when the order lapses.
    #[must_use]
    pub fn with_expiry_time(mut self, expiry_time: DvDateTime) -> Self {
        self.expiry_time = Some(expiry_time);
        self
    }

    /// The human-readable order.
    #[must_use]
    pub fn narrative(&self) -> &Text {
        &self.narrative
    }

    /// The planned steps.
    #[must_use]
    pub fn activities(&self) -> &[Activity] {
        &self.activities
    }

    /// When the order lapses.
    #[must_use]
    pub fn expiry_time(&self) -> Option<&DvDateTime> {
        self.expiry_time.as_ref()
    }

    /// The entry attributes.
    #[must_use]
    pub fn entry(&self) -> &EntryAttrs {
        &self.entry
    }
}

/// One transition of the Instruction State Machine.
///
/// ```
/// use openehr::rm::ehr::IsmTransition;
/// use openehr::terminology::{instruction_state, instruction_transition};
///
/// let started = IsmTransition::new(instruction_state::ACTIVE).unwrap()
///     .with_transition(instruction_transition::START).unwrap();
/// assert_eq!(started.current_state().value(), "active");
/// assert!(!started.is_terminal());
///
/// let done = IsmTransition::new(instruction_state::COMPLETED).unwrap();
/// assert!(done.is_terminal());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IsmTransition {
    #[serde(rename = "_type", default)]
    rm_type: IsmTransitionTag,
    current_state: DvCodedText,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    transition: Option<DvCodedText>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    careflow_step: Option<DvCodedText>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    reason: Vec<Text>,
}

impl IsmTransition {
    /// The states from which no transition leaves.
    pub const TERMINAL_STATES: [&'static str; 4] = [
        terminology::instruction_state::COMPLETED,
        terminology::instruction_state::ABORTED,
        terminology::instruction_state::CANCELLED,
        terminology::instruction_state::EXPIRED,
    ];

    /// Builds a transition from an openEHR `instruction_states` code.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the code is not in the group.
    pub fn new(current_state_code: &str) -> Result<Self, ParseError> {
        let current_state = terminology::instruction_state::GROUP
            .coded_text(current_state_code)
            .ok_or_else(|| ParseError::invariant("ISM_TRANSITION", "Current_state_valid"))?;
        Ok(Self {
            current_state,
            rm_type: IsmTransitionTag,
            transition: None,
            careflow_step: None,
            reason: Vec::new(),
        })
    }

    /// Records which transition produced this state.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the code is not in the
    /// `instruction_transitions` group.
    pub fn with_transition(mut self, transition_code: &str) -> Result<Self, ParseError> {
        self.transition = Some(
            terminology::instruction_transition::GROUP
                .coded_text(transition_code)
                .ok_or_else(|| ParseError::invariant("ISM_TRANSITION", "Transition_valid"))?,
        );
        Ok(self)
    }

    /// Records the archetype-defined careflow step this corresponds to.
    #[must_use]
    pub fn with_careflow_step(mut self, careflow_step: DvCodedText) -> Self {
        self.careflow_step = Some(careflow_step);
        self
    }

    /// Adds a reason for the transition.
    #[must_use]
    pub fn with_reason(mut self, reason: Text) -> Self {
        self.reason.push(reason);
        self
    }

    /// The state the order is now in.
    #[must_use]
    pub fn current_state(&self) -> &DvCodedText {
        &self.current_state
    }

    /// The transition that produced it.
    #[must_use]
    pub fn transition(&self) -> Option<&DvCodedText> {
        self.transition.as_ref()
    }

    /// The archetype-defined careflow step.
    #[must_use]
    pub fn careflow_step(&self) -> Option<&DvCodedText> {
        self.careflow_step.as_ref()
    }

    /// Why the transition happened.
    #[must_use]
    pub fn reason(&self) -> &[Text] {
        &self.reason
    }

    /// Whether the order can no longer change state.
    ///
    /// Useful for the check nobody remembers to write: an `ACTION` recorded
    /// against an order that is already `completed` or `aborted` is either a
    /// late entry that needs a timestamp explanation or a bug.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        Self::TERMINAL_STATES.contains(&self.current_state.defining_code().code_string())
    }
}

/// The link from an [`Action`] back to the [`Instruction`] it fulfils.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstructionDetails {
    #[serde(rename = "_type", default)]
    rm_type: InstructionDetailsTag,
    instruction_id: LocatableRef,
    activity_id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    wf_details: Option<ItemStructure>,
}

impl InstructionDetails {
    /// Builds the link.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if `activity_id` is empty. An action that names
    /// the instruction but not the activity cannot say *which* of a
    /// multi-activity order it fulfilled, and a three-times-daily order has
    /// three.
    pub fn new(
        instruction_id: LocatableRef,
        activity_id: impl Into<String>,
    ) -> Result<Self, ParseError> {
        let activity_id = activity_id.into();
        if activity_id.is_empty() {
            return Err(ParseError::invariant(
                "INSTRUCTION_DETAILS",
                "Activity_path_valid",
            ));
        }
        Ok(Self {
            instruction_id,
            rm_type: InstructionDetailsTag,
            activity_id,
            wf_details: None,
        })
    }

    /// Which instruction is being fulfilled.
    #[must_use]
    pub fn instruction_id(&self) -> &LocatableRef {
        &self.instruction_id
    }

    /// Which activity of it.
    #[must_use]
    pub fn activity_id(&self) -> &str {
        &self.activity_id
    }
}

/// A record that something was done.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Action {
    #[serde(flatten)]
    locatable: LocatableAttrs,
    #[serde(flatten)]
    entry: EntryAttrs,
    #[serde(flatten)]
    care_entry: CareEntryAttrs,
    time: DvDateTime,
    description: ItemStructure,
    ism_transition: IsmTransition,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    instruction_details: Option<InstructionDetails>,
}

impl_locatable!(Action, "ACTION");

impl Action {
    /// Builds an action.
    #[must_use]
    pub fn new(
        locatable: LocatableAttrs,
        entry: EntryAttrs,
        time: DvDateTime,
        description: ItemStructure,
        ism_transition: IsmTransition,
    ) -> Self {
        Self {
            locatable,
            entry,
            care_entry: CareEntryAttrs::default(),
            time,
            description,
            ism_transition,
            instruction_details: None,
        }
    }

    /// Links the action to the order it fulfils.
    #[must_use]
    pub fn with_instruction_details(mut self, details: InstructionDetails) -> Self {
        self.instruction_details = Some(details);
        self
    }

    /// When it was done.
    #[must_use]
    pub fn time(&self) -> &DvDateTime {
        &self.time
    }

    /// What was done.
    #[must_use]
    pub fn description(&self) -> &ItemStructure {
        &self.description
    }

    /// The order's state after this action.
    #[must_use]
    pub fn ism_transition(&self) -> &IsmTransition {
        &self.ism_transition
    }

    /// The order this action fulfils, if any.
    #[must_use]
    pub fn instruction_details(&self) -> Option<&InstructionDetails> {
        self.instruction_details.as_ref()
    }

    /// The entry attributes.
    #[must_use]
    pub fn entry(&self) -> &EntryAttrs {
        &self.entry
    }
}

/// Any clinical or administrative statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type")]
// The five entry kinds differ in size because they differ in content: an
// OBSERVATION carries a HISTORY and an ADMIN_ENTRY carries one structure. See
// [`ContentItem`] for why none of them is boxed.
#[allow(clippy::large_enum_variant)]
pub enum Entry {
    /// Something observed.
    #[serde(rename = "OBSERVATION")]
    Observation(Observation),
    /// A clinical judgement.
    #[serde(rename = "EVALUATION")]
    Evaluation(Evaluation),
    /// An order.
    #[serde(rename = "INSTRUCTION")]
    Instruction(Instruction),
    /// A record that something was done.
    #[serde(rename = "ACTION")]
    Action(Action),
    /// An administrative statement.
    #[serde(rename = "ADMIN_ENTRY")]
    AdminEntry(AdminEntry),
}

impl Entry {
    /// The openEHR class name, as it appears in `_type`.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Observation(_) => "OBSERVATION",
            Self::Evaluation(_) => "EVALUATION",
            Self::Instruction(_) => "INSTRUCTION",
            Self::Action(_) => "ACTION",
            Self::AdminEntry(_) => "ADMIN_ENTRY",
        }
    }

    /// The locatable attributes.
    #[must_use]
    pub fn locatable(&self) -> &LocatableAttrs {
        match self {
            Self::Observation(e) => e.locatable(),
            Self::Evaluation(e) => e.locatable(),
            Self::Instruction(e) => e.locatable(),
            Self::Action(e) => e.locatable(),
            Self::AdminEntry(e) => e.locatable(),
        }
    }

    /// The entry attributes.
    #[must_use]
    pub fn entry_attrs(&self) -> &EntryAttrs {
        match self {
            Self::Observation(e) => e.entry(),
            Self::Evaluation(e) => e.entry(),
            Self::Instruction(e) => e.entry(),
            Self::Action(e) => e.entry(),
            Self::AdminEntry(e) => e.entry(),
        }
    }

    /// Whether this entry is a `CARE_ENTRY` — everything except
    /// `ADMIN_ENTRY`.
    ///
    /// The distinction is not cosmetic: `ADMIN_ENTRY` is explicitly *not*
    /// clinical, so a de-identification or research extract that keeps care
    /// entries and drops admin entries is making a defensible cut, and one
    /// that cannot tell them apart is not.
    #[must_use]
    pub fn is_care_entry(&self) -> bool {
        !matches!(self, Self::AdminEntry(_))
    }

    /// Who the statement is about.
    #[must_use]
    pub fn subject(&self) -> &PartyProxy {
        self.entry_attrs().subject()
    }
}

macro_rules! entry_from {
    ($($ty:ty => $variant:ident),* $(,)?) => {
        $(
            impl From<$ty> for Entry {
                fn from(v: $ty) -> Self {
                    Self::$variant(v)
                }
            }

            impl From<$ty> for ContentItem {
                fn from(v: $ty) -> Self {
                    Self::Entry(Entry::$variant(v))
                }
            }
        )*
    };
}

entry_from! {
    Observation => Observation,
    Evaluation => Evaluation,
    Instruction => Instruction,
    Action => Action,
    AdminEntry => AdminEntry,
}

impl From<Entry> for ContentItem {
    fn from(v: Entry) -> Self {
        Self::Entry(v)
    }
}

impl From<Section> for ContentItem {
    fn from(v: Section) -> Self {
        Self::Section(v)
    }
}

/// The `DV_TEXT` shorthand used across this module's builders.
pub type Label = DvText;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rm::common::{Archetyped, PartyIdentified, PartyRelated};
    use crate::rm::data_structures::{Element, ItemSingle};
    use crate::rm::data_types::{DataValue, DvCount};

    fn attrs(name: &str, node: &str) -> LocatableAttrs {
        LocatableAttrs::named(name, node).unwrap()
    }

    fn en() -> CodePhrase {
        CodePhrase::new("ISO_639-1", "en").unwrap()
    }

    fn utf8() -> CodePhrase {
        CodePhrase::new("IANA_character-sets", "UTF-8").unwrap()
    }

    fn item_structure() -> ItemStructure {
        ItemSingle::new(
            attrs("d", "at0001"),
            Element::new(attrs("v", "at0002"), DataValue::Count(DvCount::new(1))),
        )
        .into()
    }

    fn composition() -> Composition {
        Composition::new(
            attrs("Encounter", "openEHR-EHR-COMPOSITION.encounter.v1").with_archetype_details(
                Archetyped::new("openEHR-EHR-COMPOSITION.encounter.v1", "1.1.0").unwrap(),
            ),
            terminology::composition_category::EVENT,
            PartyIdentified::named("Dr A Nurse").unwrap().into(),
            en(),
            CodePhrase::new("ISO_3166-1", "GB").unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn entries_are_found_through_nested_sections() {
        let entry: ContentItem = Evaluation::new(
            attrs("Problem", "at0000"),
            EntryAttrs::about_subject(en(), utf8()),
            item_structure(),
        )
        .into();
        let inner = Section::new(attrs("Inner", "at0002"), vec![entry]);
        let outer = Section::new(attrs("Outer", "at0001"), vec![inner.into()]);
        let c = composition().with_content(outer.into());
        assert_eq!(c.entries().count(), 1);
        assert_eq!(c.entries().next().unwrap().type_name(), "EVALUATION");
    }

    #[test]
    fn an_entry_about_someone_else_says_so() {
        let mother = PartyRelated::new(
            PartyIdentified::named("Mother").unwrap(),
            terminology::subject_relationship::MOTHER,
        )
        .unwrap();
        let family_history = Evaluation::new(
            attrs("Family history", "at0000"),
            EntryAttrs::about(en(), utf8(), mother.into()),
            item_structure(),
        );
        let entry: Entry = family_history.into();
        assert!(!entry.subject().is_subject());

        let own = Evaluation::new(
            attrs("Problem", "at0000"),
            EntryAttrs::about_subject(en(), utf8()),
            item_structure(),
        );
        assert!(Entry::from(own).subject().is_subject());
    }

    #[test]
    fn a_composition_round_trips_through_canonical_json() {
        let c = composition().with_content(
            Evaluation::new(
                attrs("Problem", "at0000"),
                EntryAttrs::about_subject(en(), utf8()),
                item_structure(),
            )
            .into(),
        );
        let json = serde_json::to_string(&c).unwrap();
        assert!(json.contains(r#""_type":"EVALUATION""#), "{json}");
        let back: Composition = serde_json::from_str(&json).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn an_instruction_needs_at_least_one_activity() {
        assert!(
            Instruction::new(
                attrs("Order", "at0000"),
                EntryAttrs::about_subject(en(), utf8()),
                Text::plain("Amoxicillin 500mg three times a day").unwrap(),
                Vec::new(),
            )
            .is_err()
        );
    }

    #[test]
    fn terminal_ism_states_are_recognised() {
        for code in IsmTransition::TERMINAL_STATES {
            assert!(IsmTransition::new(code).unwrap().is_terminal(), "{code}");
        }
        for code in [
            terminology::instruction_state::ACTIVE,
            terminology::instruction_state::PLANNED,
            terminology::instruction_state::SUSPENDED,
        ] {
            assert!(!IsmTransition::new(code).unwrap().is_terminal(), "{code}");
        }
    }

    #[test]
    fn an_event_context_cannot_end_before_it_starts() {
        let ctx = EventContext::new(
            DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
            terminology::setting::PRIMARY_MEDICAL_CARE,
        )
        .unwrap();
        assert!(
            ctx.clone()
                .with_end_time(DvDateTime::new("2026-07-31T08:00:00Z").unwrap())
                .is_err()
        );
        assert!(
            ctx.with_end_time(DvDateTime::new("2026-07-31T10:00:00Z").unwrap())
                .is_ok()
        );
    }

    #[test]
    fn deactivating_a_record_does_not_make_it_unreadable() {
        let status = EhrStatus::new(
            attrs("EHR Status", "openEHR-EHR-EHR_STATUS.generic.v1"),
            PartySelf::anonymous(),
            true,
            true,
        );
        let deceased = status.set_modifiable(false);
        assert!(!deceased.is_active());
        assert!(deceased.is_queryable());
    }
}
