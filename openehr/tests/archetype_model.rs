//! `K15.1`–`K15.4`: the AOM2 object model, and the four things it must do.
//!
//! Each test states what would break if the behaviour were removed, in the
//! style `tests/guarantees.rs` uses, because a test whose failure nobody can
//! interpret is a test that gets deleted (`T13.3`).

use openehr::am::{
    AM_RELEASE, Archetype, ArchetypeSlot, ArchetypeTerminology, CAttribute, CComplexObject,
    CObject, CPrimitive, CPrimitiveObject, Cardinality, MultiplicityInterval, TermDefinition,
};
use openehr::base::Interval;
use std::collections::{BTreeMap, BTreeSet};

fn terms(codes: &[&str]) -> BTreeMap<String, TermDefinition> {
    codes
        .iter()
        .map(|code| {
            (
                (*code).to_owned(),
                TermDefinition::new(format!("term {code}"), None).unwrap(),
            )
        })
        .collect()
}

/// An archetype using every node kind this crate models.
fn blood_pressure() -> Archetype {
    let systolic = CObject::Complex(
        CComplexObject::new(
            "ELEMENT",
            Some("at0004".to_owned()),
            MultiplicityInterval::MANDATORY,
            vec![
                CAttribute::single(
                    "value",
                    MultiplicityInterval::MANDATORY,
                    vec![CObject::Primitive(CPrimitiveObject::new(
                        "DV_QUANTITY",
                        MultiplicityInterval::MANDATORY,
                        CPrimitive::Real {
                            list: Vec::new(),
                            range: Some(Interval::closed("0".parse().unwrap(), "1000.0".parse().unwrap()).unwrap()),
                        },
                    ))],
                )
                .unwrap(),
            ],
        )
        .unwrap(),
    );

    let position = CObject::Primitive(CPrimitiveObject::new(
        "DV_CODED_TEXT",
        MultiplicityInterval::OPTIONAL,
        CPrimitive::TerminologyCode {
            constraint: Some("ac0001".to_owned()),
            code_list: Vec::new(),
        },
    ));

    let slot = CObject::Slot(
        ArchetypeSlot::new("CLUSTER", "at0007", MultiplicityInterval::new(0, None).unwrap())
            .unwrap()
            .including("archetype_id/value matches {/openEHR-EHR-CLUSTER\\..*/}"),
    );

    let items = CAttribute::container(
        "items",
        MultiplicityInterval::MANDATORY,
        Cardinality::new(MultiplicityInterval::at_least(1).unwrap()).ordered(),
        vec![systolic, position, slot],
    )
    .unwrap();

    let definition = CComplexObject::new(
        "OBSERVATION",
        Some("id1".to_owned()),
        MultiplicityInterval::MANDATORY,
        vec![CAttribute::single("data", MultiplicityInterval::MANDATORY, vec![CObject::Complex(
            CComplexObject::new(
                "ITEM_TREE",
                Some("at0003".to_owned()),
                MultiplicityInterval::MANDATORY,
                vec![items],
            )
            .unwrap(),
        )])
        .unwrap()],
    )
    .unwrap();

    let terminology = ArchetypeTerminology::new(
        "en",
        terms(&["id1", "at0003", "at0004", "at0007", "at0010"]),
    )
    .unwrap()
    .with_value_set("ac0001", BTreeSet::from(["at0010".to_owned()]))
    .unwrap()
    .with_binding("SNOMED-CT", "at0004", "271649006");

    Archetype::new(
        "openEHR-EHR-OBSERVATION.blood_pressure.v2".parse().unwrap(),
        definition,
        terminology,
    )
    .unwrap()
}

/// `K15.4`: an archetype is reachable without going through ADL text.
///
/// If this broke, every test, generator, and tool that needs a constraint tree
/// would have to wait for a parser that does not exist — and the parser, when
/// it lands, would have no independently constructed artefact to be tested
/// against.
#[test]
fn an_archetype_is_constructible_without_a_parser() {
    let archetype = blood_pressure();
    assert_eq!(archetype.rm_type_name(), "OBSERVATION");
    assert_eq!(
        archetype.node_ids(),
        ["id1", "at0003", "at0004", "at0007"],
        "document order, root first"
    );
    assert!(archetype.terminology().defines("at0004"));
}

/// `K15.3`: every modelled type survives a JSON round trip byte for byte.
///
/// If this broke, an artefact read and written back would differ from the one
/// the author approved — the archetype equivalent of `db:D-08`, where a stored
/// magnitude of `1.10` came back as `1.1`.
#[test]
fn an_archetype_round_trips_through_json_unchanged() {
    let archetype = blood_pressure();
    let json = serde_json::to_string(&archetype).unwrap();
    let back: Archetype = serde_json::from_str(&json).unwrap();

    assert_eq!(back, archetype);
    assert_eq!(serde_json::to_string(&back).unwrap(), json);
    // And what came back is still a valid artefact by its own rules.
    back.check().unwrap();
}

/// `K15.3` with `K15.20`: a constraint kind this crate cannot represent is
/// carried, not dropped.
///
/// If this broke, an unrecognised primitive constraint would vanish on read and
/// the node would silently become unconstrained — "valid" meaning "the parts I
/// understood were satisfied", which is the failure the withdrawn `S1.4`
/// predicted and the reason §15 could replace it.
#[test]
fn a_constraint_this_crate_cannot_model_survives_rather_than_disappearing() {
    let exotic = CPrimitive::Unsupported {
        rm_type_name: "C_DURATION".to_owned(),
        source: "P0Y..P100Y".to_owned(),
    };
    let node = CObject::Primitive(CPrimitiveObject::new(
        "DV_DURATION",
        MultiplicityInterval::MANDATORY,
        exotic.clone(),
    ));

    let json = serde_json::to_string(&node).unwrap();
    assert!(json.contains("C_UNSUPPORTED"), "the kind is named, not erased");

    let CObject::Primitive(back) = serde_json::from_str::<CObject>(&json).unwrap() else {
        panic!("a primitive node deserialized as something else");
    };
    assert_eq!(back.constraint(), &exotic);
}

/// `K15.2`: the AM release these types are modelled against is named.
///
/// If this broke, a reader could not tell which version of AOM2 the model
/// follows, and `S1.16`'s discipline for the RM — name the release, carry what
/// the artefact declares — would hold on one side of the two-level model and
/// not the other.
#[test]
fn the_targeted_archetype_model_release_is_named() {
    assert_eq!(AM_RELEASE, "2.3.0");

    let declared = blood_pressure().with_versions(Some("2.4.0".to_owned()), Some("1.0.4".to_owned()));
    // Carried, not enforced: an artefact declaring another release is readable.
    let json = serde_json::to_string(&declared).unwrap();
    assert!(json.contains("2.4.0") && json.contains("1.0.4"));
    serde_json::from_str::<Archetype>(&json).unwrap().check().unwrap();
}

/// `K15.1`: the checks run at construction, and again on anything deserialized.
///
/// If this broke, an archetype that arrived as JSON would carry codes its own
/// terminology never defined — the two-gate failure `L10.1a` exists for, one
/// level up from the RM.
#[test]
fn an_artefact_that_arrived_as_json_is_checkable_by_the_same_rules() {
    let mut json: serde_json::Value = serde_json::to_value(blood_pressure()).unwrap();
    json["definition"]["rm_type_name"] = serde_json::Value::String("EVALUATION".to_owned());

    let smuggled: Archetype = serde_json::from_value(json).unwrap();
    // serde built it happily…
    assert_eq!(smuggled.rm_type_name(), "EVALUATION");
    // …and the artefact's own rules refuse it, naming the AOM2 validity code.
    assert_eq!(smuggled.check().unwrap_err().reason, "VARDT");
}
