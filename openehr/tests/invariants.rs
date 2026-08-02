//! The constructor invariants, tested where they refuse.
//!
//! Closing finding **A-06**: a large share of this crate's requirements are
//! *rejections* — a constructor that refuses to build something openEHR
//! forbids. An untested rejection is the worst kind of untested code, because
//! its failure mode is silent: the check stops working, nothing errors, and
//! invalid clinical data is accepted and stored.
//!
//! Every test here asserts **both** directions — that the invalid case is
//! refused and the valid one is not — because a constructor that refuses
//! everything passes the first half.
//!
//! What is deliberately **not** here: assertions that a type has a field, or
//! that an attribute is of a given type. Those are enforced by the compiler and
//! a runtime test for them cannot fail (`T13.2`). They are marked `type` rather
//! than `?` in the conformance matrix.

use openehr::base::{
    HierObjectId, LocatableRef, ObjectId, ObjectRef, ObjectVersionId, UidBasedId, iso8601,
};
use openehr::rm::common::PartyRelated;
use openehr::rm::common::{
    Archetyped, Attestation, AuditDetails, FeederAudit, FeederAuditDetails, ImportedVersion,
    Locatable, LocatableAttrs, OriginalVersion, PartyIdentified, PartyProxy, PartySelf, Version,
    VersionedObject,
};
use openehr::rm::data_structures::{
    Cluster, Element, History, IntervalEvent, ItemSingle, ItemStructure, ItemTable, ItemTree,
    PointEvent,
};
use openehr::rm::data_types::DvDate;
use openehr::rm::data_types::{
    CodePhrase, DataValue, DvCount, DvDateTime, DvDuration, DvParagraph, DvParsable, DvProportion,
    DvText, MappingMatch, ProportionKind, Text,
};
use openehr::rm::demographic::{Address, Contact, DateValidity};
use openehr::rm::ehr::{
    Action, AdminEntry, Composition, Ehr, EntryAttrs, Evaluation, InstructionDetails,
    IsmTransition, Observation,
};
use openehr::security::Chain;
use openehr::terminology::{
    audit_change_type, composition_category, event_math_function, instruction_state,
    version_lifecycle_state,
};
use openehr::validation::Validate;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn at(name: &str, node: &str) -> LocatableAttrs {
    LocatableAttrs::named(name, node).expect("literal attrs")
}

fn en() -> CodePhrase {
    CodePhrase::new("ISO_639-1", "en").unwrap()
}

fn utf8() -> CodePhrase {
    CodePhrase::new("IANA_character-sets", "UTF-8").unwrap()
}

fn entry_attrs() -> EntryAttrs {
    EntryAttrs::about_subject(en(), utf8())
}

fn count(n: i64) -> DataValue {
    DataValue::Count(DvCount::new(n))
}

fn structure() -> ItemStructure {
    ItemSingle::new(at("d", "at0001"), Element::new(at("v", "at0002"), count(1))).into()
}

fn record_uid() -> HierObjectId {
    HierObjectId::from_uid_str("87284370-2D4B-4E3D-A3F3-F303D2F4F34B").unwrap()
}

fn owner() -> ObjectRef {
    ObjectRef::new("local", "EHR", ObjectId::HierObjectId(record_uid())).unwrap()
}

fn version_id(n: u32) -> ObjectVersionId {
    format!("87284370-2D4B-4E3D-A3F3-F303D2F4F34B::ehr1.example.org::{n}")
        .parse()
        .unwrap()
}

fn audit(change: &str, at_time: &str) -> AuditDetails {
    AuditDetails::new(
        "ehr1.example.org",
        DvDateTime::new(at_time).unwrap(),
        change,
        PartyIdentified::named("Dr A Nurse").unwrap().into(),
    )
    .unwrap()
}

// ---------------------------------------------------------------------------
// §2 Identifiers
// ---------------------------------------------------------------------------

/// `I2.19` — fails if `UID_BASED_ID` starts admitting an identifier class that
/// `LOCATABLE.uid` does not, which would put an archetype id in a uid field and
/// only surface at validation time.
#[test]
fn a_uid_field_admits_only_the_two_uid_based_classes() {
    let hier = r#"{"_type":"HIER_OBJECT_ID","value":"87284370-2D4B-4E3D-A3F3-F303D2F4F34B"}"#;
    let version = r#"{"_type":"OBJECT_VERSION_ID","value":"87284370-2D4B-4E3D-A3F3-F303D2F4F34B::s.example::1"}"#;
    assert!(serde_json::from_str::<UidBasedId>(hier).is_ok());
    assert!(serde_json::from_str::<UidBasedId>(version).is_ok());

    for refused in [
        r#"{"_type":"ARCHETYPE_ID","value":"openEHR-EHR-OBSERVATION.x.v1"}"#,
        r#"{"_type":"TERMINOLOGY_ID","value":"SNOMED-CT"}"#,
        r#"{"_type":"GENERIC_ID","value":"x","scheme":"y"}"#,
    ] {
        assert!(
            serde_json::from_str::<UidBasedId>(refused).is_err(),
            "accepted {refused}"
        );
    }
}

/// `I2.24` — fails if an empty path becomes representable, which would give
/// "the root object" two spellings that no URI could tell apart.
#[test]
fn a_locatable_ref_refuses_an_empty_path() {
    let uid: UidBasedId = "87284370-2D4B-4E3D-A3F3-F303D2F4F34B::s.example::1"
        .parse()
        .unwrap();
    assert!(LocatableRef::new("local", "COMPOSITION", uid.clone(), Some(String::new())).is_err());

    let root = LocatableRef::new("local", "COMPOSITION", uid.clone(), None).unwrap();
    assert_eq!(root.path(), None);
    let inner =
        LocatableRef::new("local", "COMPOSITION", uid, Some("/content[0]".to_owned())).unwrap();
    assert_eq!(inner.path(), Some("/content[0]"));
    assert_ne!(root.uri(), inner.uri());
}

// ---------------------------------------------------------------------------
// §3 Data types
// ---------------------------------------------------------------------------

/// `D3.2` — fails if the deprecated `DV_PARAGRAPH` stops round-tripping, which
/// would lose content authored before openEHR deprecated it.
#[test]
fn the_deprecated_paragraph_type_still_round_trips() {
    assert!(
        DvParagraph::new(Vec::new()).is_err(),
        "an empty paragraph is not prose"
    );

    let paragraph = DvParagraph::new(vec![
        DvText::new("First line.").unwrap(),
        DvText::new("Second line.").unwrap(),
    ])
    .unwrap();
    let value = DataValue::Paragraph(paragraph);
    let json = serde_json::to_string(&value).unwrap();
    assert!(json.contains(r#""_type":"DV_PARAGRAPH""#), "{json}");
    assert_eq!(serde_json::from_str::<DataValue>(&json).unwrap(), value);
}

/// `D3.5` — fails if `CODE_PHRASE` starts accepting a code with no code string
/// or an unparseable terminology, either of which is a term that resolves to
/// nothing.
#[test]
fn a_code_phrase_needs_a_terminology_and_a_code() {
    assert!(CodePhrase::new("SNOMED-CT", "").is_err());
    assert!(CodePhrase::openehr("").is_err());
    assert!(CodePhrase::new("", "271649006").is_err());
    assert!(
        CodePhrase::new("ICD10AM(", "x").is_err(),
        "unclosed version"
    );

    let ok = CodePhrase::new("ICD10AM(3rd_ed)", "I10").unwrap();
    assert_eq!(ok.terminology_id().version_id(), Some("3rd_ed"));
    assert_eq!(ok.code_string(), "I10");
}

/// `D3.6` — fails if `TERM_MAPPING.match` stops being a closed set. openEHR
/// types it `Character`, and four of the million-odd characters mean something.
#[test]
fn a_term_mapping_match_is_one_of_four_characters() {
    for (c, expected) in [
        ('>', MappingMatch::Broader),
        ('=', MappingMatch::Equivalent),
        ('<', MappingMatch::Narrower),
        ('?', MappingMatch::Unknown),
    ] {
        assert_eq!(MappingMatch::from_char(c).unwrap(), expected);
    }
    for c in ['!', 'x', '≈', ' '] {
        assert!(MappingMatch::from_char(c).is_err(), "accepted {c:?}");
    }
    // And on the wire, where the value arrives as a string.
    assert!(serde_json::from_str::<MappingMatch>(r#""=""#).is_ok());
    assert!(serde_json::from_str::<MappingMatch>(r#""""#).is_err());
    assert!(serde_json::from_str::<MappingMatch>(r#""=="""#).is_err());
}

/// `D3.12` — fails if a leap second stops parsing. A record that captured one
/// must read back; refusing it makes the record unreadable, not merely odd.
#[test]
fn a_leap_second_is_accepted_and_a_sixty_first_is_not() {
    assert!("23:59:60Z".parse::<iso8601::Time>().is_ok());
    assert!("23:59:60".parse::<iso8601::Time>().is_ok());
    assert!("2026-06-30T23:59:60Z".parse::<iso8601::DateTime>().is_ok());
    assert!("23:59:61".parse::<iso8601::Time>().is_err());
    assert!("23:60:00".parse::<iso8601::Time>().is_err());
}

/// `D3.17` — fails if proportions of different kinds start comparing. A ratio
/// of 1:128 and a percentage of 0.78% are close as numbers and are not the
/// same statement.
#[test]
fn proportions_of_different_kinds_do_not_compare() {
    let ratio = DvProportion::new(1.0, 128.0, ProportionKind::Ratio).unwrap();
    let percent = DvProportion::new(0.78, 100.0, ProportionKind::Percent).unwrap();
    assert_eq!(ratio.partial_cmp(&percent), None);
    assert_eq!(percent.partial_cmp(&ratio), None);
    assert!(!ratio.is_strictly_comparable_to(&percent));

    // Same kind still compares.
    let bigger = DvProportion::new(1.0, 64.0, ProportionKind::Ratio).unwrap();
    assert!(ratio < bigger);
}

/// `D3.28` — fails if `DV_PARSABLE` starts accepting content with no formalism,
/// which leaves a value nothing can decide how to read.
#[test]
fn parsable_content_needs_a_formalism() {
    assert!(DvParsable::new("", "XML").is_err());
    assert!(DvParsable::new("<x/>", "").is_err());
    let ok = DvParsable::new("<x/>", "XML").unwrap();
    assert_eq!(ok.formalism(), "XML");
}

// ---------------------------------------------------------------------------
// §4 Data structures
// ---------------------------------------------------------------------------

/// `R4.8` — fails if an empty `CLUSTER` becomes constructible. openEHR's way to
/// say "not filled in" is a null `ELEMENT`, which carries a reason; an empty
/// cluster is a heading with silence under it.
#[test]
fn a_cluster_refuses_to_be_empty() {
    assert!(Cluster::new(at("c", "at0001"), Vec::new()).is_err());
    assert!(
        Cluster::new(
            at("c", "at0001"),
            vec![Element::new(at("v", "at0002"), count(1)).into()],
        )
        .is_ok()
    );
}

/// `R4.9` — fails if table regularity stops being reported. A ragged
/// `ITEM_TABLE` renders with cells silently missing from the short rows.
#[test]
fn a_ragged_table_is_reported_rather_than_assumed_regular() {
    let cell = |name: &str| Element::new(at(name, "at0003"), count(1)).into();
    let row = |cells: Vec<openehr::rm::data_structures::Item>| {
        Cluster::new(at("row", "at0002"), cells).unwrap()
    };

    let regular = ItemTable::new(
        at("t", "at0001"),
        vec![
            row(vec![cell("a"), cell("b")]),
            row(vec![cell("c"), cell("d")]),
        ],
    );
    assert!(regular.is_regular());
    assert_eq!((regular.row_count(), regular.column_count()), (2, 2));
    assert!(ItemStructure::Table(regular).validate().is_empty());

    let ragged = ItemTable::new(
        at("t", "at0001"),
        vec![row(vec![cell("a"), cell("b")]), row(vec![cell("c")])],
    );
    assert!(!ragged.is_regular());
    assert!(ragged.element_at_cell(1, 1).is_none());
    let report = ItemStructure::Table(ragged).validate();
    assert_eq!(report.len(), 1, "{report}");
    assert_eq!(report.violations()[0].invariant, "Rows_regular");
}

/// `R4.12` — fails if a zero or negative sampling period becomes constructible.
/// A period of zero describes infinitely many samples at one instant.
#[test]
fn a_periodic_history_needs_a_positive_period() {
    let history = || {
        History::new(
            at("h", "at0001"),
            DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
            vec![
                PointEvent::new(
                    at("e", "at0006"),
                    DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
                    structure(),
                )
                .into(),
            ],
            None,
        )
        .unwrap()
    };
    assert!(!history().is_periodic());
    assert!(
        history()
            .with_period(DvDuration::new("PT0S").unwrap())
            .is_err()
    );
    assert!(
        history()
            .with_period(DvDuration::new("-PT1H").unwrap())
            .is_err()
    );

    let periodic = history()
        .with_period(DvDuration::new("PT8H").unwrap())
        .unwrap();
    assert!(periodic.is_periodic());
    assert_eq!(periodic.period().unwrap().as_str(), "PT8H");
}

/// `R4.15` — fails if an interval event stops checking its width and math
/// function. A negative width puts the interval's start after its end.
#[test]
fn an_interval_event_checks_its_width_and_math_function() {
    let build = |width: &str, math: &str| {
        IntervalEvent::new(
            at("e", "at0009"),
            DvDateTime::new("2026-07-31T16:00:00Z").unwrap(),
            structure(),
            DvDuration::new(width).unwrap(),
            math,
        )
    };
    assert!(build("-PT8H", event_math_function::TOTAL).is_err());
    assert!(build("PT8H", "not-a-code").is_err());
    assert!(build("PT8H", event_math_function::TOTAL).is_ok());
    assert!(
        build("PT0S", event_math_function::ACTUAL).is_ok(),
        "zero width is a point"
    );
}

/// `R4.12a` — fails if a history can declare a period its events do not
/// follow. Software that resamples or graphs a series on the strength of its
/// `period` draws the wrong picture, and nothing in the data looks wrong.
#[test]
fn a_periodic_history_checks_that_its_events_fall_on_the_period() {
    let history = |times: &[&str]| {
        History::new(
            at("h", "at0001"),
            DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
            times
                .iter()
                .map(|t| {
                    PointEvent::new(at("e", "at0006"), DvDateTime::new(t).unwrap(), structure())
                        .into()
                })
                .collect(),
            None,
        )
        .unwrap()
        .with_period(DvDuration::new("PT8H").unwrap())
        .unwrap()
    };

    let consistent = history(&["2026-07-31T09:00:00Z", "2026-07-31T17:00:00Z"]);
    assert_eq!(consistent.is_period_consistent(), Some(true));
    assert!(consistent.validate().is_empty());
    // The offsets are what the invariant is stated over.
    assert_eq!(
        consistent.offset_seconds(&consistent.events()[1]),
        Some(28_800)
    );

    let drifted = history(&["2026-07-31T09:00:00Z", "2026-07-31T16:00:00Z"]);
    assert_eq!(drifted.is_period_consistent(), Some(false));
    let report = drifted.validate();
    assert!(
        report
            .violations()
            .iter()
            .any(|v| v.invariant == "Period_consistency"),
        "{report}"
    );

    // An aperiodic history declares no period, so the question does not arise.
    let aperiodic = History::new(
        at("h", "at0001"),
        DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
        vec![
            PointEvent::new(
                at("e", "at0006"),
                DvDateTime::new("2026-07-31T09:07:00Z").unwrap(),
                structure(),
            )
            .into(),
        ],
        None,
    )
    .unwrap();
    assert_eq!(aperiodic.is_period_consistent(), None);
    assert!(aperiodic.validate().is_empty());
}

// ---------------------------------------------------------------------------
// §5 Common
// ---------------------------------------------------------------------------

/// `M5.5` — fails if `is_archetype_root` or `concept` stops tracking
/// `archetype_details`, which is what says where the archetype boundaries are.
#[test]
fn only_an_archetype_root_has_a_concept() {
    let inner = Element::new(at("Systolic", "at0004"), count(1));
    assert!(!inner.is_archetype_root());
    assert!(inner.concept().is_none());

    let root = Element::new(
        at("Systolic", "openEHR-EHR-OBSERVATION.blood_pressure.v2")
            .with_archetype_details(Archetyped::new("openEHR-EHR-ELEMENT.x.v1", "1.1.0").unwrap()),
        count(1),
    );
    assert!(root.is_archetype_root());
    assert_eq!(root.concept().map(Text::value), Some("Systolic"));
}

/// `M5.10` — fails if `FEEDER_AUDIT` starts accepting non-encapsulated original
/// content, which invites a lossy stringification of the one thing that exists
/// to be lossless.
#[test]
fn feeder_audit_original_content_must_be_encapsulated() {
    let audit = FeederAudit::new(FeederAuditDetails::new("legacy.example.org").unwrap());
    assert!(audit.clone().with_original_content(count(1)).is_err());
    assert!(
        audit
            .clone()
            .with_original_content(DataValue::Text(DvText::new("<x/>").unwrap()))
            .is_err()
    );
    assert!(
        audit
            .with_original_content(DataValue::Parsable(DvParsable::new("<x/>", "XML").unwrap()))
            .is_ok()
    );
}

/// `M5.11` — fails if a feeder audit stops naming its system. Recording that
/// data came from somewhere without saying where is the one fact it carries.
#[test]
fn feeder_audit_details_must_name_a_system() {
    assert!(FeederAuditDetails::new("").is_err());
    let ok = FeederAuditDetails::new("legacy.example.org")
        .unwrap()
        .with_time(DvDateTime::new("2026-07-31T09:00:00Z").unwrap());
    assert_eq!(ok.system_id(), "legacy.example.org");
    assert!(ok.time().is_some());
}

/// `M5.16` — fails if an anonymous record subject stops round-tripping. It is
/// the representation a research extract needs, not a degenerate case.
#[test]
fn an_anonymous_subject_round_trips_as_itself() {
    let anonymous: PartyProxy = PartySelf::anonymous().into();
    assert!(anonymous.is_subject());
    assert_eq!(anonymous.name(), None);
    assert!(anonymous.external_ref().is_none());

    let json = serde_json::to_string(&anonymous).unwrap();
    assert!(json.contains(r#""_type":"PARTY_SELF""#), "{json}");
    assert_eq!(
        serde_json::from_str::<PartyProxy>(&json).unwrap(),
        anonymous
    );
}

// ---------------------------------------------------------------------------
// §6 EHR
// ---------------------------------------------------------------------------

/// `E6.2` — fails if `directory` and `folders` can disagree. openEHR defines
/// the directory to *be* the first folder.
#[test]
fn the_directory_is_the_first_folder_or_there_is_neither() {
    let versioned = |ty: &str| {
        ObjectRef::new(ty, ty, ObjectId::HierObjectId(record_uid())).unwrap()
    };
    let ehr = || {
        Ehr::new(
            record_uid(),
            record_uid(),
            versioned("VERSIONED_EHR_STATUS"),
            versioned("VERSIONED_EHR_ACCESS"),
            DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
        )
        .unwrap()
    };
    assert!(ehr().directory().is_none());
    assert!(ehr().with_folders(Vec::new()).is_err());

    let first = ObjectRef::new("local", "FOLDER", ObjectId::HierObjectId(record_uid())).unwrap();
    let second = ObjectRef::new(
        "local",
        "FOLDER",
        ObjectId::HierObjectId(
            HierObjectId::from_uid_str("11111111-2222-3333-4444-555555555555").unwrap(),
        ),
    )
    .unwrap();
    let with = ehr().with_folders(vec![first.clone(), second]).unwrap();
    assert_eq!(with.directory(), Some(&first));
    assert_eq!(with.folders().len(), 2);
}

/// `E6.6`, `E6.11`, `E6.21` — fails if a coded attribute stops being checked
/// against its openEHR group. Each constructor takes a **code** rather than a
/// `DV_CODED_TEXT` precisely so a rubric that disagrees with its code cannot be
/// built; that only helps if the code itself is checked.
#[test]
fn coded_attributes_are_checked_against_their_openehr_group() {
    let composition = |category: &str| {
        Composition::new(
            at("Encounter", "openEHR-EHR-COMPOSITION.encounter.v1"),
            category,
            PartyIdentified::named("Dr A Nurse").unwrap().into(),
            en(),
            CodePhrase::new("ISO_3166-1", "GB").unwrap(),
        )
    };
    assert!(composition(composition_category::EVENT).is_ok());
    assert!(composition("9999").is_err(), "category not in the group");
    // `433` is `event`; `245` is `active`, an instruction state. A code from
    // the wrong group is the realistic error, not a made-up number.
    assert!(composition(instruction_state::ACTIVE).is_err());

    let context = |setting: &str| {
        openehr::rm::ehr::EventContext::new(
            DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
            setting,
        )
    };
    assert!(context(openehr::terminology::setting::PRIMARY_MEDICAL_CARE).is_ok());
    assert!(context("9999").is_err());
    assert!(context(composition_category::EVENT).is_err());

    assert!(IsmTransition::new(instruction_state::ACTIVE).is_ok());
    assert!(IsmTransition::new("9999").is_err());
    assert!(IsmTransition::new(composition_category::EVENT).is_err());
    assert!(
        IsmTransition::new(instruction_state::ACTIVE)
            .unwrap()
            .with_transition("9999")
            .is_err()
    );

    // And the rubric always matches the code, because nothing can supply one.
    let built = composition(composition_category::PERSISTENT).unwrap();
    assert_eq!(built.category().value(), "persistent");
    assert_eq!(built.category().check_openehr_rubric(), Some(true));
}

/// `E6.6b` — fails if a persistent composition can carry an event context.
///
/// openEHR states this as a formal invariant (`Is_persistent_validity`) and
/// says it again in prose: *"Persistent Compositions do not have an Event
/// context."* A persistent composition is a running summary across many
/// encounters — a problem list, a medication list — so attaching one
/// encounter's context asserts the whole list belongs to that visit.
#[test]
fn a_persistent_composition_may_not_carry_an_event_context() {
    let composition = |category: &str| {
        Composition::new(
            at("Encounter", "openEHR-EHR-COMPOSITION.encounter.v1").with_archetype_details(
                Archetyped::new("openEHR-EHR-COMPOSITION.encounter.v1", "1.1.0").unwrap(),
            ),
            category,
            PartyIdentified::named("Dr A Nurse").unwrap().into(),
            en(),
            CodePhrase::new("ISO_3166-1", "GB").unwrap(),
        )
        .unwrap()
    };
    let context = || {
        openehr::rm::ehr::EventContext::new(
            DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
            openehr::terminology::setting::PRIMARY_MEDICAL_CARE,
        )
        .unwrap()
    };

    // An event composition documents one encounter, so a context belongs.
    assert!(
        composition(composition_category::EVENT)
            .with_context(context())
            .is_ok()
    );
    // A persistent one does not.
    assert!(
        composition(composition_category::PERSISTENT)
            .with_context(context())
            .is_err()
    );
    assert!(
        composition(composition_category::EPISODIC)
            .with_context(context())
            .is_ok(),
        "episodic is not persistent"
    );

    // And the same rule holds for a document that arrives already built, where
    // no constructor was ever called.
    let json = r#"{
      "_type":"COMPOSITION",
      "name":{"value":"Problem list"},
      "archetype_node_id":"openEHR-EHR-COMPOSITION.problem_list.v1",
      "archetype_details":{"archetype_id":{"value":"openEHR-EHR-COMPOSITION.problem_list.v1"},"rm_version":"1.1.0"},
      "language":{"terminology_id":{"value":"ISO_639-1"},"code_string":"en"},
      "territory":{"terminology_id":{"value":"ISO_3166-1"},"code_string":"GB"},
      "category":{"value":"persistent","defining_code":{"terminology_id":{"value":"openehr"},"code_string":"431"}},
      "composer":{"_type":"PARTY_IDENTIFIED","name":"Dr A Nurse"},
      "context":{"start_time":{"value":"2026-07-31T09:00:00Z"},
                 "setting":{"value":"primary medical care","defining_code":{"terminology_id":{"value":"openehr"},"code_string":"228"}}}
    }"#;
    let received: Composition = serde_json::from_str(json).unwrap();
    let report = received.validate();
    assert!(
        report
            .violations()
            .iter()
            .any(|v| v.invariant == "Is_persistent_validity"),
        "{report}"
    );
}

/// `E6.6a`, `E6.3a` — fails if a `COMPOSITION` or `EHR_STATUS` without
/// `archetype_details` stops being reported. Both are archetype roots by
/// definition; one that cannot say which archetype shaped it cannot be
/// validated against one by anything downstream.
#[test]
fn a_composition_and_an_ehr_status_must_be_archetype_roots() {
    let rootless = Composition::new(
        at("Encounter", "openEHR-EHR-COMPOSITION.encounter.v1"),
        composition_category::EVENT,
        PartyIdentified::named("Dr A Nurse").unwrap().into(),
        en(),
        CodePhrase::new("ISO_3166-1", "GB").unwrap(),
    )
    .unwrap();
    let report = rootless.validate();
    assert!(
        report
            .violations()
            .iter()
            .any(|v| v.invariant == "Is_archetype_root"),
        "{report}"
    );

    let status = openehr::rm::ehr::EhrStatus::new(
        at("EHR Status", "openEHR-EHR-EHR_STATUS.generic.v1"),
        PartySelf::anonymous(),
        true,
        true,
    );
    let report = status.validate();
    assert!(
        report
            .violations()
            .iter()
            .any(|v| v.invariant == "Is_archetype_root"),
        "{report}"
    );

    let rooted = openehr::rm::ehr::EhrStatus::new(
        at("EHR Status", "openEHR-EHR-EHR_STATUS.generic.v1").with_archetype_details(
            Archetyped::new("openEHR-EHR-EHR_STATUS.generic.v1", "1.1.0").unwrap(),
        ),
        PartySelf::anonymous(),
        true,
        true,
    );
    assert!(rooted.validate().is_empty());
}

/// `E6.12a` — fails if an empty `location` becomes representable. A
/// present-but-empty location is indistinguishable from an absent one to every
/// reader, and openEHR has a way to say absent.
#[test]
fn an_event_context_location_is_absent_or_non_empty() {
    let context = openehr::rm::ehr::EventContext::new(
        DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
        openehr::terminology::setting::PRIMARY_MEDICAL_CARE,
    )
    .unwrap();
    assert!(context.location().is_none());
    assert!(context.clone().with_location("").is_err());
    assert_eq!(
        context.with_location("Ward A3").unwrap().location(),
        Some("Ward A3")
    );
}

/// `E6.15` — fails if `ADMIN_ENTRY` stops being distinguishable from the
/// clinical entries. A research extract that keeps care entries and drops
/// administrative ones is making a defensible cut; one that cannot tell them
/// apart is not.
#[test]
fn administrative_entries_are_distinguishable_from_clinical_ones() {
    let evaluation = Evaluation::new(at("Problem", "at0000"), entry_attrs(), structure()).into();
    let observation = Observation::new(
        at("Obs", "at0000"),
        entry_attrs(),
        History::new(
            at("h", "at0001"),
            DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
            vec![
                PointEvent::new(
                    at("e", "at0006"),
                    DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
                    structure(),
                )
                .into(),
            ],
            None,
        )
        .unwrap(),
    )
    .into();
    let action = Action::new(
        at("Act", "at0000"),
        entry_attrs(),
        DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
        structure(),
        IsmTransition::new(instruction_state::ACTIVE).unwrap(),
    )
    .into();
    let admin = AdminEntry::new(at("Admission", "at0000"), entry_attrs(), structure()).into();

    for clinical in [&evaluation, &observation, &action] {
        assert!(
            openehr::rm::ehr::Entry::is_care_entry(clinical),
            "{} should be a care entry",
            clinical.type_name()
        );
    }
    assert!(!openehr::rm::ehr::Entry::is_care_entry(&admin));
}

/// `E6.23` — fails if an action can name an instruction without naming which of
/// its activities it fulfilled. A three-times-daily order has three.
#[test]
fn an_action_must_say_which_activity_it_fulfilled() {
    let uid: UidBasedId = "87284370-2D4B-4E3D-A3F3-F303D2F4F34B::s.example::1"
        .parse()
        .unwrap();
    let reference = LocatableRef::new(
        "local",
        "INSTRUCTION",
        uid,
        Some("/activities[at0001]".into()),
    )
    .unwrap();
    assert!(InstructionDetails::new(reference.clone(), "").is_err());
    let ok = InstructionDetails::new(reference, "activities[at0001]").unwrap();
    assert_eq!(ok.activity_id(), "activities[at0001]");
}

// ---------------------------------------------------------------------------
// §7 Demographics
// ---------------------------------------------------------------------------

/// `G7.9`, `G7.11` — fails if a contact can carry no address, or if validity
/// stops being consulted.
#[test]
fn a_contact_needs_an_address_and_may_carry_a_validity_period() {
    let address = || {
        Address::new(
            at("address", "at0002"),
            ItemSingle::new(
                at("details", "at0003"),
                Element::new(
                    at("line", "at0004"),
                    DataValue::Text(DvText::new("1 Example Street").unwrap()),
                ),
            )
            .into(),
        )
    };
    assert!(Contact::new(at("home", "at0001"), Vec::new()).is_err());

    let always = Contact::new(at("home", "at0001"), vec![address()]).unwrap();
    // No recorded period means always valid — openEHR's reading of an absent
    // interval, and the consequence of being wrong is a failed contact attempt
    // rather than a false assurance (`G7.11`).
    assert!(always.is_valid_on(&DvDate::new("1999-01-01").unwrap()));
    assert!(always.is_valid_on(&DvDate::new("2050-01-01").unwrap()));

    let bounded = Contact::new(at("home", "at0001"), vec![address()])
        .unwrap()
        .with_time_validity(
            DateValidity::closed(
                DvDate::new("2020-01-01").unwrap(),
                DvDate::new("2024-12-31").unwrap(),
            )
            .unwrap(),
        );
    assert!(bounded.is_valid_on(&DvDate::new("2022-06-01").unwrap()));
    assert!(!bounded.is_valid_on(&DvDate::new("2025-06-01").unwrap()));
    assert_eq!(bounded.addresses().len(), 1);
}

// ---------------------------------------------------------------------------
// §8 Change control
// ---------------------------------------------------------------------------

/// `V8.7` — fails if an empty versioned object starts reporting an audit trail,
/// which would assert that the trail exists and is blank.
#[test]
fn an_empty_history_has_no_revision_history_at_all() {
    let mut versioned: VersionedObject<String> = VersionedObject::new(
        record_uid(),
        owner(),
        DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
    );
    assert!(versioned.revision_history().is_none());

    for (n, preceding, change, at_time) in [
        (1, None, audit_change_type::CREATION, "2026-07-31T09:05:00Z"),
        (
            2,
            Some(1),
            audit_change_type::AMENDMENT,
            "2026-07-31T09:10:00Z",
        ),
    ] {
        versioned
            .commit(
                OriginalVersion::new(
                    version_id(n),
                    preceding.map(version_id),
                    version_lifecycle_state::COMPLETE,
                    Some(format!("v{n}")),
                    audit(change, at_time),
                    owner(),
                )
                .unwrap()
                .into(),
            )
            .unwrap();
    }

    let history = versioned.revision_history().unwrap();
    assert_eq!(history.items().len(), 2);
    // Most recent **last** — openEHR's `most_recent_version` postcondition is
    // `items.last.version_id.value`, and the class table's Purpose line
    // contradicts it. See `RevisionHistory`'s documentation for the resolution.
    assert_eq!(history.items()[0].version_id(), &version_id(1));
    assert_eq!(history.items()[1].version_id(), &version_id(2));
    assert_eq!(history.most_recent_version(), &version_id(2));
    assert_eq!(
        history.most_recent_version_time_committed().as_str(),
        "2026-07-31T09:10:00Z"
    );
}

/// `V8.11`, `V8.12` — fails if an imported version starts minting a local
/// identity, which would make the same clinical fact appear twice the next time
/// the two systems exchange data.
#[test]
fn an_imported_version_keeps_the_identity_it_arrived_with() {
    let original = OriginalVersion::new(
        version_id(1),
        None,
        version_lifecycle_state::COMPLETE,
        Some("authored elsewhere".to_owned()),
        audit(audit_change_type::CREATION, "2026-07-30T08:00:00Z"),
        owner(),
    )
    .unwrap();

    let import_audit = audit(audit_change_type::CREATION, "2026-07-31T09:00:00Z");
    let imported = ImportedVersion::new(original.clone(), import_audit.clone(), owner());

    // The wrapped version keeps its own audit…
    assert_eq!(
        imported.item().commit_audit().time_committed().as_str(),
        "2026-07-30T08:00:00Z"
    );
    // …and the import has its own.
    assert_eq!(
        imported.commit_audit().time_committed().as_str(),
        "2026-07-31T09:00:00Z"
    );

    // Identity delegates to the wrapped version, unchanged.
    let version: Version<String> = imported.into();
    assert_eq!(version.uid(), &version_id(1));
    assert_eq!(
        version.data().map(String::as_str),
        Some("authored elsewhere")
    );

    // And it commits into a history keyed on that identity.
    let mut versioned: VersionedObject<String> = VersionedObject::new(
        record_uid(),
        owner(),
        DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
    );
    versioned.commit(version).unwrap();
    assert_eq!(versioned.version_count(), 1);
}

/// `V8.17`, `V8.19` — fails if an attestation stops carrying the rendering the
/// attester saw, or its pending flag.
#[test]
fn an_attestation_carries_what_was_signed_and_whether_it_is_outstanding() {
    let pending = Attestation::new(
        audit(audit_change_type::ATTESTATION, "2026-07-31T09:20:00Z"),
        Text::plain("countersignature required").unwrap(),
        true,
    );
    assert!(pending.is_pending());
    assert!(pending.proof().is_none());
    assert!(pending.attested_view().is_none());
    assert!(pending.items().is_empty());

    let view = openehr::rm::data_types::DvMultimedia::inline(
        CodePhrase::new("IANA_media-types", "image/png").unwrap(),
        b"\x89PNG\r\n\x1a\n".to_vec(),
    );
    let signed = Attestation::new(
        audit(audit_change_type::ATTESTATION, "2026-07-31T09:25:00Z"),
        Text::plain("signed").unwrap(),
        false,
    )
    .with_attested_view(view)
    .with_proof("-----BEGIN PGP SIGNATURE-----")
    .with_item(openehr::rm::data_types::DvEhrUri::new("ehr://87284370/content[0]").unwrap());
    assert!(!signed.is_pending());
    assert!(signed.attested_view().is_some());
    assert_eq!(signed.items().len(), 1);
    // Present, and still not evidence of anything (`V8.18`).
    assert!(signed.proof().is_some());
}

/// `M5.13a`, `V8.17a`, `M5.18a` — fails if a coded attribute that openEHR
/// binds to a terminology group stops being checked.
///
/// These three are shaped differently from `COMPOSITION.category`: the
/// attribute is `DV_TEXT` (or optional), so the group applies **only when the
/// value happens to be coded from openEHR's own terminology**. Checking
/// unconditionally would reject a SNOMED-coded participation function, which is
/// the commonest real case; not checking at all lets an invented openEHR code
/// through.
// Long because it covers four attributes across three classes that share one
// rule shape; splitting it would hide that they are the same rule.
#[allow(clippy::too_many_lines)]
#[test]
fn conditionally_coded_attributes_are_checked_only_when_openehr_coded() {
    use openehr::rm::common::{Attestation, Participation};
    use openehr::rm::data_types::DvCodedText;
    use openehr::terminology::{attestation_reason, participation_mode, subject_relationship};

    // PARTY_RELATED.Relationship_valid — refused at construction…
    assert!(PartyRelated::new(PartyIdentified::named("Mother").unwrap(), "9999").is_err());
    assert!(
        PartyRelated::new(
            PartyIdentified::named("Mother").unwrap(),
            subject_relationship::MOTHER
        )
        .is_ok()
    );
    // …and reported for a document that arrived already built.
    let invented = PartyRelated::from_coded(
        PartyIdentified::named("Mother").unwrap(),
        DvCodedText::new("aunt-in-law", CodePhrase::openehr("9999").unwrap()).unwrap(),
    );
    let mut ctx = openehr::validation::Context::new();
    let entry: openehr::rm::ehr::Entry = Evaluation::new(
        at("Family history", "at0000"),
        EntryAttrs::about(en(), utf8(), invented.into()),
        structure(),
    )
    .into();
    entry.visit(&mut ctx);
    let report = ctx.finish();
    assert!(
        report
            .violations()
            .iter()
            .any(|v| v.invariant == "Relationship_valid"),
        "{report}"
    );

    // ATTESTATION.Reason_valid — openEHR-coded and wrong.
    let mut ctx = openehr::validation::Context::new();
    openehr::validation::check_attestation(
        &Attestation::new(
            audit(audit_change_type::ATTESTATION, "2026-07-31T09:20:00Z"),
            DvCodedText::new("countersigned", CodePhrase::openehr("9999").unwrap())
                .unwrap()
                .into(),
            false,
        ),
        &mut ctx,
    );
    assert!(
        ctx.finish()
            .violations()
            .iter()
            .any(|v| v.invariant == "Reason_valid"),
    );

    // …and an openEHR-coded reason that *is* in the group passes.
    let mut ctx = openehr::validation::Context::new();
    openehr::validation::check_attestation(
        &Attestation::new(
            audit(audit_change_type::ATTESTATION, "2026-07-31T09:20:00Z"),
            attestation_reason::GROUP
                .coded_text(attestation_reason::SIGNED)
                .unwrap()
                .into(),
            false,
        ),
        &mut ctx,
    );
    assert!(ctx.finish().is_empty());

    // A reason coded against an *external* terminology is not checked, because
    // this crate cannot resolve it (`S1.10`) — and must not be reported.
    let mut ctx = openehr::validation::Context::new();
    openehr::validation::check_attestation(
        &Attestation::new(
            audit(audit_change_type::ATTESTATION, "2026-07-31T09:20:00Z"),
            DvCodedText::new(
                "Witnessed",
                CodePhrase::new("SNOMED-CT", "1231000").unwrap(),
            )
            .unwrap()
            .into(),
            false,
        ),
        &mut ctx,
    );
    assert!(
        ctx.finish().is_empty(),
        "an external code must not be reported"
    );

    // PARTICIPATION.Mode_valid, through an EVENT_CONTEXT.
    let bad_mode = Participation::new(
        Text::plain("performer").unwrap(),
        PartyIdentified::named("Dr A Nurse").unwrap().into(),
    )
    .with_mode(
        DvCodedText::new("by carrier pigeon", CodePhrase::openehr("9999").unwrap()).unwrap(),
    );
    let context = openehr::rm::ehr::EventContext::new(
        DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
        openehr::terminology::setting::PRIMARY_MEDICAL_CARE,
    )
    .unwrap()
    .with_participation(bad_mode);
    let mut ctx = openehr::validation::Context::new();
    context.visit(&mut ctx);
    let report = ctx.finish();
    assert!(
        report
            .violations()
            .iter()
            .any(|v| v.invariant == "Mode_valid"),
        "{report}"
    );

    let good_mode = Participation::new(
        Text::plain("performer").unwrap(),
        PartyIdentified::named("Dr A Nurse").unwrap().into(),
    )
    .with_mode(
        participation_mode::GROUP
            .coded_text(participation_mode::FACE_TO_FACE_COMMUNICATION)
            .unwrap(),
    );
    let context = openehr::rm::ehr::EventContext::new(
        DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
        openehr::terminology::setting::PRIMARY_MEDICAL_CARE,
    )
    .unwrap()
    .with_participation(good_mode);
    let mut ctx = openehr::validation::Context::new();
    context.visit(&mut ctx);
    assert!(ctx.finish().is_empty());
}

// ---------------------------------------------------------------------------
// §9 Serialization
// ---------------------------------------------------------------------------

/// `J9.7` — fails if a `_type` naming the wrong class stops being an error.
/// Silently reinterpreting it hands back a value of a class the sender did not
/// mean.
#[test]
fn a_type_tag_naming_the_wrong_class_is_refused() {
    assert!(serde_json::from_str::<Text>(r#"{"_type":"DV_TEXT","value":"x"}"#).is_ok());
    assert!(serde_json::from_str::<Text>(r#"{"_type":"DV_QUANTITY","value":"x"}"#).is_err());
    assert!(serde_json::from_str::<PartyProxy>(r#"{"_type":"PARTY_SELF"}"#).is_ok());
    assert!(serde_json::from_str::<PartyProxy>(r#"{"_type":"COMPOSITION"}"#).is_err());
    assert!(
        serde_json::from_str::<HierObjectId>(
            r#"{"_type":"ARCHETYPE_ID","value":"87284370-2D4B-4E3D-A3F3-F303D2F4F34B"}"#
        )
        .is_err()
    );
}

/// `J9.11` — fails if absent attributes start being written as `null` or `[]`.
/// ITS-JSON requires them omitted, and a reader validating against the
/// published JSON Schemas rejects the difference.
#[test]
fn absent_and_empty_attributes_are_omitted_not_nulled() {
    let element = Element::new(at("Systolic", "at0004"), count(1));
    let json = serde_json::to_string(&element).unwrap();
    assert!(!json.contains("null"), "{json}");
    for absent in [
        "uid",
        "links",
        "archetype_details",
        "feeder_audit",
        "null_flavour",
    ] {
        assert!(!json.contains(absent), "{absent} should be omitted: {json}");
    }

    let composition = Composition::new(
        at("Encounter", "openEHR-EHR-COMPOSITION.encounter.v1"),
        composition_category::EVENT,
        PartyIdentified::named("Dr A Nurse").unwrap().into(),
        en(),
        CodePhrase::new("ISO_3166-1", "GB").unwrap(),
    )
    .unwrap();
    let json = serde_json::to_string(&composition).unwrap();
    assert!(!json.contains("null"), "{json}");
    assert!(
        !json.contains("content"),
        "an empty content list is omitted: {json}"
    );
    assert!(!json.contains("context"), "{json}");
}

// ---------------------------------------------------------------------------
// §10 Validation
// ---------------------------------------------------------------------------

/// `L10.7` — fails if violation order stops being document order. A report
/// diffed between two runs should show real change, not reordering.
#[test]
fn violations_are_reported_in_document_order_and_that_order_is_stable() {
    // Three broken elements at known positions, distinguishable by path.
    let broken = |node: &str| -> openehr::rm::data_structures::Item {
        serde_json::from_str::<Element>(&format!(
            r#"{{"name":{{"value":"x"}},"archetype_node_id":"{node}"}}"#
        ))
        .unwrap()
        .into()
    };
    let tree = ItemTree::new(
        at("t", "at0001"),
        vec![
            broken("at0002"),
            Cluster::new(at("c", "at0003"), vec![broken("at0004")])
                .unwrap()
                .into(),
            broken("at0005"),
        ],
    );
    let structure = ItemStructure::Tree(tree);

    let paths: Vec<String> = structure
        .validate()
        .violations()
        .iter()
        .map(|v| v.path.clone())
        .collect();
    assert_eq!(paths, ["/items[0]", "/items[1]/items[0]", "/items[2]"]);

    // Stable across runs: the same input gives the same order.
    for _ in 0..3 {
        let again: Vec<String> = structure
            .validate()
            .violations()
            .iter()
            .map(|v| v.path.clone())
            .collect();
        assert_eq!(again, paths);
    }
}

/// `L10.6` — fails if any of the checks that have no test of their own stops
/// firing. Each line below is one openEHR invariant, and each is the only
/// thing standing between a malformed received document and storage.
#[test]
fn every_validation_check_fires_on_a_document_that_breaks_it() {
    let cases: Vec<(&str, &str)> = vec![
        (
            "Archetype_node_id_valid",
            r#"{"name":{"value":"x"},"archetype_node_id":"","value":{"_type":"DV_COUNT","magnitude":1}}"#,
        ),
        // An *empty* name breaks `DV_TEXT.Value_valid`, not
        // `LOCATABLE.Name_valid` — openEHR's `Name_valid` is only
        // `name /= Void`. Attributing it to the wrong class would send a reader
        // to the wrong definition (`L10.4`).
        (
            "Valid_value",
            r#"{"name":{"value":""},"archetype_node_id":"at0001","value":{"_type":"DV_COUNT","magnitude":1}}"#,
        ),
        (
            "Inv_null_flavour_indicated",
            r#"{"name":{"value":"x"},"archetype_node_id":"at0001"}"#,
        ),
        (
            "Inv_null_flavour_valid",
            r#"{"name":{"value":"x"},"archetype_node_id":"at0001","null_flavour":{"value":"invented","defining_code":{"terminology_id":{"value":"openehr"},"code_string":"9999"}}}"#,
        ),
        (
            "Units_valid",
            r#"{"name":{"value":"x"},"archetype_node_id":"at0001","value":{"_type":"DV_QUANTITY","magnitude":1.0,"units":""}}"#,
        ),
        (
            "Precision_valid",
            r#"{"name":{"value":"x"},"archetype_node_id":"at0001","value":{"_type":"DV_QUANTITY","magnitude":1.0,"units":"mg","precision":-7}}"#,
        ),
        (
            "Valid_denominator",
            r#"{"name":{"value":"x"},"archetype_node_id":"at0001","value":{"_type":"DV_PROPORTION","numerator":1.0,"denominator":0.0,"type":0}}"#,
        ),
        (
            "Fraction_validity",
            r#"{"name":{"value":"x"},"archetype_node_id":"at0001","value":{"_type":"DV_PROPORTION","numerator":1.5,"denominator":2.0,"type":3}}"#,
        ),
        (
            "Precision_validity",
            r#"{"name":{"value":"x"},"archetype_node_id":"at0001","value":{"_type":"DV_PROPORTION","numerator":1.5,"denominator":2.5,"type":0,"precision":0}}"#,
        ),
        (
            "Value_is_rubric",
            r#"{"name":{"value":"x"},"archetype_node_id":"at0001","value":{"_type":"DV_CODED_TEXT","value":"deletion","defining_code":{"terminology_id":{"value":"openehr"},"code_string":"249"}}}"#,
        ),
        (
            "Not_empty",
            r#"{"name":{"value":"x"},"archetype_node_id":"at0001","value":{"_type":"DV_MULTIMEDIA","media_type":{"terminology_id":{"value":"IANA_media-types"},"code_string":"image/png"}}}"#,
        ),
        (
            // openEHR's name for it: `DV_TEXT.Valid_value`, not `Value_valid`
            // (`L10.4`, `A-20`).
            "Valid_value",
            r#"{"name":{"value":"x"},"archetype_node_id":"at0001","value":{"_type":"DV_TEXT","value":""}}"#,
        ),
    ];

    for (invariant, json) in cases {
        let element: Element =
            serde_json::from_str(json).unwrap_or_else(|e| panic!("{invariant}: {e}"));
        let report = element.validate();
        assert!(
            report.violations().iter().any(|v| v.invariant == invariant),
            "{invariant} did not fire; got {report}"
        );
    }

    // And the well-formed control passes every one of them.
    let clean = Element::new(at("x", "at0001"), count(1));
    assert!(clean.validate().is_empty());
}

// ---------------------------------------------------------------------------
// §11 Security
// ---------------------------------------------------------------------------

/// `X11.15` — fails if a chain becomes constructible with pre-existing entries,
/// which is backfilling: a chain assembled after the fact attests only that the
/// rows look consistent now.
#[test]
fn a_chain_begins_where_it_begins_and_says_when_it_began_late() {
    let fresh = Chain::new();
    assert!(fresh.is_empty());
    assert_eq!(fresh.len(), 0);
    assert!(fresh.genesis_note().is_none());

    let late = Chain::genesis_after("chain added 2026-08-01; 412 prior versions unchained");
    assert!(late.is_empty());
    assert_eq!(
        late.genesis_note(),
        Some("chain added 2026-08-01; 412 prior versions unchained")
    );

    // The note survives serialization, so the fact that history predates the
    // chain travels with the chain rather than living in someone's memory.
    let json = serde_json::to_string(&late).unwrap();
    let back: Chain = serde_json::from_str(&json).unwrap();
    assert_eq!(back.genesis_note(), late.genesis_note());
}

/// `A-18`: an original version can carry a signature, and it survives canonical
/// JSON.
///
/// openEHR declares `signature` on `VERSION`, so `ORIGINAL_VERSION` inherits it.
/// This crate modelled it only on `IMPORTED_VERSION`, which meant a locally
/// created version could not be signed at all — only one that had arrived from
/// somewhere else. Found by reading the RM 1.1.0 BMM rather than the code.
#[test]
fn an_original_version_can_be_signed_and_the_signature_round_trips() {
    let signed = OriginalVersion::new(
        version_id(1),
        None,
        version_lifecycle_state::COMPLETE,
        Some("signed locally".to_owned()),
        audit(audit_change_type::CREATION, "2026-07-31T09:00:00Z"),
        owner(),
    )
    .unwrap()
    .with_signature("-----BEGIN PGP SIGNATURE-----");

    assert_eq!(
        signed.signature(),
        Some("-----BEGIN PGP SIGNATURE-----"),
        "an original version must be able to carry a signature"
    );

    let version: Version<String> = signed.into();
    let json = serde_json::to_string(&version).unwrap();
    assert!(
        json.contains("signature"),
        "the signature must reach canonical JSON, or it is not stored anywhere"
    );
    let back: Version<String> = serde_json::from_str(&json).unwrap();
    assert_eq!(back, version, "the signature must survive a round trip");
    assert_eq!(back.signature(), Some("-----BEGIN PGP SIGNATURE-----"));
}

/// `E6.3` — `EHR.Ehr_status_valid` and `Ehr_access_valid`.
///
/// An `OBJECT_REF` carries its target's type as a string, so nothing in Rust
/// stops an EHR pointing its status at a versioned composition. openEHR states
/// both as invariants and this crate checked neither; the shared fixture built
/// both references as `"EHR"` from the day it was written, and the `SQLite` store
/// read them back as `VERSIONED_EHR_STATUS` — so a round trip silently changed
/// the type and only `ehr_id` was ever compared (`A-21`).
#[test]
fn an_ehr_must_reference_the_versioned_containers_openehr_names() {
    let reference = |ty: &str| {
        ObjectRef::new(ty, ty, ObjectId::HierObjectId(record_uid())).unwrap()
    };
    let build = |status: &str, access: &str| {
        Ehr::new(
            record_uid(),
            record_uid(),
            reference(status),
            reference(access),
            DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
        )
    };

    assert!(build("VERSIONED_EHR_STATUS", "VERSIONED_EHR_ACCESS").is_ok());

    let wrong_status = build("EHR", "VERSIONED_EHR_ACCESS").unwrap_err();
    assert_eq!(wrong_status.reason, "Ehr_status_valid");

    let wrong_access = build("VERSIONED_EHR_STATUS", "VERSIONED_COMPOSITION").unwrap_err();
    assert_eq!(wrong_access.reason, "Ehr_access_valid");
}
