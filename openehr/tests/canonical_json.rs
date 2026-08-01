//! Round-trip fidelity through openEHR canonical JSON (ITS-JSON).
//!
//! Every openEHR implementation exchanges canonical JSON, so a type that
//! serializes but does not read back is a type that cannot leave this process.
//! The tests here build values through the constructors, write them, read them,
//! and compare — which catches the whole class of `#[serde(rename)]` and
//! `#[serde(flatten)]` mistakes that unit tests on one struct do not.

use openehr::base::{HierObjectId, Interval, ObjectId, ObjectRef, ObjectVersionId, UidBasedId};
use openehr::rm::common::{
    Archetyped, Attestation, AuditDetails, Contribution, LocatableAttrs, OriginalVersion,
    PartyIdentified, PartyRelated, Version, VersionedObject,
};
use openehr::rm::data_structures::{
    Cluster, Element, History, IntervalEvent, ItemList, ItemSingle, ItemTable, ItemTree, PointEvent,
};
use openehr::rm::data_types::{
    CodePhrase, DataValue, DvBoolean, DvCodedText, DvCount, DvDate, DvDateTime, DvDuration,
    DvIdentifier, DvMultimedia, DvOrdinal, DvParsable, DvProportion, DvQuantity, DvText, DvTime,
    DvUri, MappingMatch, ProportionKind, TermMapping, Text,
};
use openehr::rm::ehr::{
    Action, Activity, AdminEntry, Composition, ContentItem, EntryAttrs, Evaluation, EventContext,
    Instruction, IsmTransition, Observation, Section,
};
use openehr::terminology::{
    audit_change_type, composition_category, event_math_function, instruction_state,
    instruction_transition, null_flavour, setting, subject_relationship, version_lifecycle_state,
};
use openehr::validation::Validate;

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

/// One composition touching every data type and every structure this crate
/// models, so that the round trip covers the whole surface rather than the
/// blood-pressure shape everybody tests.
// Long on purpose: the value of this fixture is that it is one readable place
// showing every class in one document. Splitting it into a dozen helpers would
// make the coverage claim unverifiable at a glance, which is the only thing it
// is for.
#[allow(clippy::too_many_lines)]
fn kitchen_sink() -> Composition {
    let element = |name: &str, node: &str, value: DataValue| Element::new(at(name, node), value);

    let values: Vec<DataValue> = vec![
        DataValue::Boolean(DvBoolean::new(true)),
        DataValue::Count(DvCount::new(3)),
        DataValue::Quantity(
            DvQuantity::new(184.0, "mm[Hg]")
                .unwrap()
                .with_precision(0)
                .unwrap()
                .with_units_system(DvQuantity::UCUM)
                .with_accuracy(2.5, true)
                .unwrap(),
        ),
        DataValue::Proportion(DvProportion::new(37.0, 100.0, ProportionKind::Percent).unwrap()),
        DataValue::Ordinal(DvOrdinal::new(
            2,
            DvCodedText::new("moderate", CodePhrase::new("local", "at0003").unwrap()).unwrap(),
        )),
        DataValue::Date(DvDate::new("1953-11").unwrap()),
        DataValue::Time(DvTime::new("09:15:00+01:00").unwrap()),
        DataValue::DateTime(DvDateTime::new("2026-07-31T09:15:00.250Z").unwrap()),
        DataValue::Duration(DvDuration::new("-P1Y2M3W4DT5H6M7.5S").unwrap()),
        DataValue::Uri(DvUri::new("https://example.org/guideline").unwrap()),
        DataValue::Identifier(
            DvIdentifier::new("943-476-5919")
                .unwrap()
                .with_type("NHS number")
                .with_issuer("NHS England"),
        ),
        DataValue::Parsable(DvParsable::new("<x/>", "XML").unwrap()),
        DataValue::Multimedia(
            DvMultimedia::inline(
                CodePhrase::new("IANA_media-types", "image/png").unwrap(),
                b"\x89PNG\r\n\x1a\n".to_vec(),
            )
            .with_sha256_integrity()
            .with_alternate_text("a tiny PNG"),
        ),
        DataValue::CodedText(
            DvCodedText::new(
                "Chest pain",
                CodePhrase::new("SNOMED-CT", "29857009").unwrap(),
            )
            .unwrap()
            .with_mapping(
                TermMapping::new(
                    CodePhrase::new("ICD10", "R07.4").unwrap(),
                    MappingMatch::Broader,
                )
                .with_purpose(
                    openehr::terminology::term_mapping_purpose::GROUP
                        .coded_text(openehr::terminology::term_mapping_purpose::REIMBURSEMENT)
                        .unwrap(),
                ),
            ),
        ),
        DataValue::Text(
            DvText::new("free text with *markdown*")
                .unwrap()
                .with_formatting(openehr::rm::data_types::formatting::MARKDOWN)
                .unwrap()
                .with_language(en()),
        ),
        DataValue::Interval(Box::new(
            Interval::closed(
                DataValue::Quantity(DvQuantity::new(130.0, "mm[Hg]").unwrap()),
                DataValue::Quantity(DvQuantity::new(140.0, "mm[Hg]").unwrap()),
            )
            .unwrap(),
        )),
    ];

    let tree_items: Vec<openehr::rm::data_structures::Item> = values
        .into_iter()
        .enumerate()
        .map(|(i, v)| element("value", &format!("at{i:04}"), v).into())
        .chain(core::iter::once(
            Cluster::new(
                at("nested", "at9000"),
                vec![
                    Element::new_null(at("withheld", "at9001"), null_flavour::MASKED)
                        .unwrap()
                        .into(),
                    Element::new_null(at("not asked", "at9002"), null_flavour::NO_INFORMATION)
                        .unwrap()
                        .with_null_reason(
                            Text::plain("outside the scope of this consultation").unwrap(),
                        )
                        .unwrap()
                        .into(),
                ],
            )
            .unwrap()
            .into(),
        ))
        .collect();

    // An OBSERVATION with both event kinds and a summary.
    let history = History::new(
        at("Event Series", "at0001"),
        DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
        vec![
            // On the period: origin 09:00 + 0 × PT8H. A "periodic" history whose
            // events are not on the period is not periodic (`R4.12a`), and the
            // validator says so.
            PointEvent::new(
                at("any event", "at0006"),
                DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
                ItemTree::new(at("tree", "at0003"), tree_items).into(),
            )
            .with_state(
                ItemSingle::new(
                    at("state", "at0007"),
                    element(
                        "Position",
                        "at0008",
                        DataValue::Text(DvText::new("Sitting").unwrap()),
                    ),
                )
                .into(),
            )
            .into(),
            // origin 09:00 + 1 × PT8H = 17:00, and the interval's trailing edge.
            IntervalEvent::new(
                at("8 hour total", "at0009"),
                DvDateTime::new("2026-07-31T17:00:00Z").unwrap(),
                ItemList::new(
                    at("list", "at0010"),
                    vec![element(
                        "Urine output",
                        "at0011",
                        DataValue::Quantity(DvQuantity::new(1250.0, "mL").unwrap()),
                    )],
                )
                .into(),
                DvDuration::new("PT8H").unwrap(),
                event_math_function::TOTAL,
            )
            .unwrap()
            .with_sample_count(8)
            .into(),
        ],
        Some(
            ItemTable::new(
                at("summary", "at0012"),
                vec![
                    Cluster::new(
                        at("row", "at0013"),
                        vec![element("cell", "at0014", DataValue::Count(DvCount::new(1))).into()],
                    )
                    .unwrap(),
                ],
            )
            .into(),
        ),
    )
    .unwrap()
    .with_period(DvDuration::new("PT8H").unwrap())
    .unwrap()
    .with_duration(DvDuration::new("PT16H").unwrap());

    let observation = Observation::new(
        at("Vitals", "openEHR-EHR-OBSERVATION.blood_pressure.v2")
            .with_archetype_details(
                Archetyped::new("openEHR-EHR-OBSERVATION.blood_pressure.v2", "1.1.0")
                    .unwrap()
                    .with_template("vitals_form.v1")
                    .unwrap(),
            )
            .with_uid(UidBasedId::HierObjectId(
                HierObjectId::from_uid_str("11111111-2222-3333-4444-555555555555").unwrap(),
            )),
        entry_attrs(),
        history,
    );

    // A family-history EVALUATION: the subject is somebody else, and the model
    // has to say so.
    let family_history = Evaluation::new(
        at("Family history", "openEHR-EHR-EVALUATION.family_history.v1"),
        EntryAttrs::about(
            en(),
            utf8(),
            PartyRelated::new(
                PartyIdentified::named("Mother").unwrap(),
                subject_relationship::MOTHER,
            )
            .unwrap()
            .into(),
        ),
        ItemSingle::new(
            at("data", "at0001"),
            element(
                "Condition",
                "at0002",
                DataValue::Text(DvText::new("Breast carcinoma").unwrap()),
            ),
        )
        .into(),
    );

    let instruction = Instruction::new(
        at(
            "Medication order",
            "openEHR-EHR-INSTRUCTION.medication_order.v3",
        ),
        entry_attrs(),
        Text::plain("Amoxicillin 500 mg three times daily for seven days").unwrap(),
        vec![
            Activity::new(
                at("Order", "at0001"),
                ItemSingle::new(
                    at("description", "at0002"),
                    element(
                        "Medication",
                        "at0003",
                        DataValue::Text(DvText::new("Amoxicillin 500 mg").unwrap()),
                    ),
                )
                .into(),
                "openEHR-EHR-ACTION.medication.v1",
            )
            .unwrap()
            .with_timing(DvParsable::new("R7/2026-07-31T09:00:00Z/PT8H", "HL7:PIVL").unwrap()),
        ],
    )
    .unwrap()
    .with_expiry_time(DvDateTime::new("2026-08-07T09:00:00Z").unwrap());

    let action = Action::new(
        at("Medication given", "openEHR-EHR-ACTION.medication.v1"),
        entry_attrs(),
        DvDateTime::new("2026-07-31T09:30:00Z").unwrap(),
        ItemSingle::new(
            at("description", "at0001"),
            element(
                "Dose",
                "at0002",
                DataValue::Quantity(DvQuantity::new(500.0, "mg").unwrap()),
            ),
        )
        .into(),
        IsmTransition::new(instruction_state::ACTIVE)
            .unwrap()
            .with_transition(instruction_transition::START)
            .unwrap()
            .with_reason(Text::plain("first dose").unwrap()),
    );

    let admin = AdminEntry::new(
        at("Admission", "openEHR-EHR-ADMIN_ENTRY.admission.v1"),
        entry_attrs(),
        ItemSingle::new(
            at("data", "at0001"),
            element(
                "Ward",
                "at0002",
                DataValue::Text(DvText::new("Ward 12").unwrap()),
            ),
        )
        .into(),
    );

    let section: ContentItem = Section::new(
        at("Assessment", "at0001"),
        vec![
            observation.into(),
            Section::new(at("Background", "at0002"), vec![family_history.into()]).into(),
        ],
    )
    .into();

    Composition::new(
        at("Encounter", "openEHR-EHR-COMPOSITION.encounter.v1").with_archetype_details(
            Archetyped::new("openEHR-EHR-COMPOSITION.encounter.v1", "1.1.0").unwrap(),
        ),
        composition_category::EVENT,
        PartyIdentified::new(
            Some("Dr A Nurse".to_owned()),
            vec![DvIdentifier::new("GMC-1234567").unwrap().with_issuer("GMC")],
            None,
        )
        .unwrap()
        .into(),
        en(),
        CodePhrase::new("ISO_3166-1", "GB").unwrap(),
    )
    .unwrap()
    .with_context(
        EventContext::new(
            DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
            setting::PRIMARY_MEDICAL_CARE,
        )
        .unwrap()
        .with_end_time(DvDateTime::new("2026-07-31T09:45:00Z").unwrap())
        .unwrap()
        .with_location("Consulting room 3")
        .unwrap(),
    )
    .unwrap()
    .with_content(section)
    .with_content(instruction.into())
    .with_content(action.into())
    .with_content(admin.into())
}

#[test]
fn a_composition_covering_every_modelled_class_round_trips() {
    let original = kitchen_sink();
    let json = serde_json::to_string(&original).expect("serialize");
    let back: Composition = serde_json::from_str(&json).unwrap_or_else(|e| {
        panic!("deserialize failed: {e}\n{json}");
    });
    assert_eq!(back, original);
}

#[test]
fn round_tripping_twice_is_byte_identical() {
    // A round trip that changes the bytes on the second pass means the first
    // pass normalised something, and normalising a clinical record silently is
    // exactly what must not happen.
    let json_1 = serde_json::to_string(&kitchen_sink()).unwrap();
    let back: Composition = serde_json::from_str(&json_1).unwrap();
    let json_2 = serde_json::to_string(&back).unwrap();
    assert_eq!(json_1, json_2);
}

#[test]
fn the_kitchen_sink_satisfies_every_invariant_it_should() {
    let report = kitchen_sink().validate();
    assert!(report.is_empty(), "{report}");
}

#[test]
fn every_polymorphic_attribute_carries_its_type_tag() {
    let json = serde_json::to_string(&kitchen_sink()).unwrap();
    for tag in [
        "COMPOSITION",
        "SECTION",
        "OBSERVATION",
        "EVALUATION",
        "INSTRUCTION",
        "ACTION",
        "ADMIN_ENTRY",
        "ITEM_TREE",
        "ITEM_LIST",
        "ITEM_TABLE",
        "ITEM_SINGLE",
        "CLUSTER",
        "ELEMENT",
        "POINT_EVENT",
        "INTERVAL_EVENT",
        "DV_QUANTITY",
        "DV_CODED_TEXT",
        "DV_MULTIMEDIA",
        "DV_INTERVAL",
        "PARTY_IDENTIFIED",
        "PARTY_RELATED",
    ] {
        assert!(
            json.contains(&format!("\"_type\":\"{tag}\"")),
            "missing _type {tag}"
        );
    }
}

#[test]
fn partial_dates_and_negative_durations_survive_verbatim() {
    let json = serde_json::to_string(&kitchen_sink()).unwrap();
    // A month-precision date of birth and a negative duration are both things a
    // careless serializer completes or normalises.
    assert!(json.contains(r#""value":"1953-11""#), "{json}");
    assert!(json.contains(r#""value":"-P1Y2M3W4DT5H6M7.5S""#), "{json}");
    assert!(json.contains(r#""value":"09:15:00+01:00""#), "{json}");
    assert!(
        json.contains(r#""value":"2026-07-31T09:15:00.250Z""#),
        "{json}"
    );
}

#[test]
fn a_versioned_composition_round_trips_with_its_audit_trail() {
    let record = HierObjectId::from_uid_str("87284370-2D4B-4E3D-A3F3-F303D2F4F34B").unwrap();
    let owner = ObjectRef::new("local", "EHR", ObjectId::HierObjectId(record.clone())).unwrap();
    let commit = |change: &str, at: &str| {
        AuditDetails::new(
            "ehr1.example.org",
            DvDateTime::new(at).unwrap(),
            change,
            PartyIdentified::named("Dr A Nurse").unwrap().into(),
        )
        .unwrap()
    };
    let version_id = |n: u32| -> ObjectVersionId {
        format!("87284370-2D4B-4E3D-A3F3-F303D2F4F34B::ehr1.example.org::{n}")
            .parse()
            .unwrap()
    };

    let mut versioned: VersionedObject<Composition> = VersionedObject::new(
        record.clone(),
        owner.clone(),
        DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
    );
    let v1: Version<Composition> = OriginalVersion::new(
        version_id(1),
        None,
        version_lifecycle_state::COMPLETE,
        Some(kitchen_sink()),
        commit(audit_change_type::CREATION, "2026-07-31T09:05:00Z"),
        owner.clone(),
    )
    .unwrap()
    .with_attestation(Attestation::new(
        commit(audit_change_type::ATTESTATION, "2026-07-31T09:06:00Z"),
        Text::plain("signed").unwrap(),
        false,
    ))
    .into();
    versioned.commit(v1).unwrap();

    let v2: Version<Composition> = OriginalVersion::new(
        version_id(2),
        Some(version_id(1)),
        version_lifecycle_state::DELETED,
        None,
        commit(audit_change_type::DELETED, "2026-07-31T09:10:00Z"),
        owner.clone(),
    )
    .unwrap()
    .into();
    versioned.commit(v2).unwrap();

    let json = serde_json::to_string(&versioned).unwrap();
    let back: VersionedObject<Composition> = serde_json::from_str(&json).unwrap();
    assert_eq!(back, versioned);
    assert_eq!(back.version_count(), 2);
    assert!(back.latest_version().unwrap().is_deleted());
    assert!(back.latest_version().unwrap().data().is_none());

    // A CONTRIBUTION over the same two versions.
    let contribution = Contribution::new(
        HierObjectId::from_uid_str("22222222-3333-4444-5555-666666666666").unwrap(),
        vec![version_id(1), version_id(2)],
        commit(audit_change_type::CREATION, "2026-07-31T09:10:00Z"),
    )
    .unwrap();
    let json = serde_json::to_string(&contribution).unwrap();
    let back: Contribution = serde_json::from_str(&json).unwrap();
    assert_eq!(back, contribution);
}

#[test]
fn a_payload_written_by_another_implementation_reads() {
    // Two divergences that real openEHR payloads carry: identifiers written as
    // bare strings rather than `{"value": …}` objects, and `_type` omitted
    // where the declared type is concrete. Neither is what this crate emits,
    // and rejecting either would fail on data every other implementation reads.
    let json = r#"{
      "name": {"value": "Encounter"},
      "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
      "archetype_details": {
        "archetype_id": "openEHR-EHR-COMPOSITION.encounter.v1",
        "rm_version": "1.1.0"
      },
      "language": {"terminology_id": "ISO_639-1", "code_string": "en"},
      "territory": {"terminology_id": "ISO_3166-1", "code_string": "GB"},
      "category": {
        "value": "event",
        "defining_code": {"terminology_id": "openehr", "code_string": "433"}
      },
      "composer": {"name": "Dr A Nurse"},
      "content": []
    }"#;
    let composition: Composition = serde_json::from_str(json).expect("lenient read");
    assert_eq!(composition.category().value(), "event");
    assert_eq!(composition.composer().name(), Some("Dr A Nurse"));
    assert!(composition.validate().is_empty());

    // And what this crate writes back is the canonical form, so the leniency
    // does not propagate.
    let out = serde_json::to_string(&composition).unwrap();
    assert!(
        out.contains(r#""terminology_id":{"_type":"TERMINOLOGY_ID","value":"ISO_639-1"}"#),
        "{out}"
    );
}

/// Guards the stack cost of reading a composition, which is a real
/// user-visible property and not an implementation detail.
///
/// # Why this test exists
///
/// Deserialization is recursive, and serde's derived readers for
/// `#[serde(flatten)]` and internally tagged enums cost a lot of stack per
/// level in unoptimized builds. Before the boxing described in
/// `spec/audit.md` **A-03**, reading the ~10 KB fixture below needed more than
/// 2 MiB — which is exactly what Rust gives a spawned thread, and `cargo test`
/// runs every test on one. A caller's own test suite would have aborted on a
/// stack overflow the first time it read a composition.
///
/// Measured 2026-07-31 on `rustc 1.96.1` by bisecting an explicit
/// `stack_size`: the fixture reads inside **768 KiB** and does not inside
/// 512 KiB.
///
/// The ceiling below is 1 MiB — above the measurement so a small change does
/// not flake it, and far below the 2 MiB a test thread has so a large
/// regression fails here rather than in a user's CI.
#[test]
fn reading_a_composition_stays_within_a_small_stack() {
    let json = serde_json::to_string(&kitchen_sink()).unwrap();
    let handle = std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(move || serde_json::from_str::<Composition>(&json).is_ok())
        .expect("spawn");
    assert!(handle.join().expect("no overflow within 1 MiB"));
}
