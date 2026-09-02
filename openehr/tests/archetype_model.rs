//! `K15.1`–`K15.4`: the AOM2 object model, and the four things it must do.
//!
//! Each test states what would break if the behaviour were removed, in the
//! style `tests/guarantees.rs` uses, because a test whose failure nobody can
//! interpret is a test that gets deleted (`T13.3`).

use openehr::am::{
    AM_RELEASE, Archetype, ArchetypeSlot, ArchetypeTerminology, CArchetypeRoot, CAttribute,
    CComplexObject, CObject, CPrimitive, CPrimitiveObject, Cardinality, MultiplicityInterval,
    TermDefinition,
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
            constraint_status: None,
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

/// Every accessor on every AOM2 type returns what was put into it.
///
/// If this broke, an accessor could return `""`, `None`, or an empty slice and
/// no test would notice — which is `lib:A-28` one level up, and exactly what
/// `cargo mutants` reported against the first version of this module: 43
/// surviving mutants, nearly all of them accessors nothing asserted. An
/// archetype whose `rm_attribute_name()` answers `""` addresses no attribute,
/// and the failure is silent at every layer above it.
#[test]
fn every_accessor_returns_what_was_constructed() {
    // --- MultiplicityInterval, and the three questions it answers ----------
    let bounded = MultiplicityInterval::new(0, Some(4)).unwrap();
    assert_eq!(bounded.lower(), 0);
    assert_eq!(bounded.upper(), Some(4));
    assert!(!bounded.is_open());
    assert!(!bounded.is_mandatory());

    let open = MultiplicityInterval::at_least(2).unwrap();
    assert_eq!(open.lower(), 2);
    assert_eq!(open.upper(), None);
    assert!(open.is_open());
    assert!(open.is_mandatory());

    assert!(MultiplicityInterval::MANDATORY.is_mandatory());
    assert!(!MultiplicityInterval::OPTIONAL.is_mandatory());

    // --- Cardinality: ordered and unique are off unless asked for ----------
    let plain = Cardinality::new(bounded.clone());
    assert_eq!(plain.interval(), &bounded);
    assert!(!plain.is_ordered());
    assert!(!plain.is_unique());
    assert!(Cardinality::new(bounded.clone()).ordered().is_ordered());
    assert!(Cardinality::new(bounded.clone()).unique().is_unique());

    // --- TermDefinition ----------------------------------------------------
    let with_description =
        TermDefinition::new("Systolic", Some("Peak pressure".to_owned())).unwrap();
    assert_eq!(with_description.text(), "Systolic");
    assert_eq!(with_description.description(), Some("Peak pressure"));
    assert_eq!(TermDefinition::new("Diastolic", None).unwrap().description(), None);

    // --- ArchetypeTerminology ---------------------------------------------
    let terminology = ArchetypeTerminology::new("en", terms(&["id1", "at0004"])).unwrap();
    assert_eq!(terminology.original_language(), "en");
    assert_eq!(terminology.definition("at0004").unwrap().text(), "term at0004");
    assert_eq!(terminology.definition("at9999"), None);
    let mut codes: Vec<&str> = terminology.codes().collect();
    codes.sort_unstable();
    assert_eq!(codes, ["at0004", "id1"]);

    // --- CAttribute: the name, and the cardinality only a container has ----
    let leaf = CObject::Primitive(CPrimitiveObject::new(
        "DV_TEXT",
        MultiplicityInterval::MANDATORY,
        CPrimitive::String { list: Vec::new() },
    ));
    let single = CAttribute::single("value", MultiplicityInterval::MANDATORY, vec![leaf.clone()])
        .unwrap();
    assert_eq!(single.rm_attribute_name(), "value");
    assert_eq!(single.cardinality(), None);
    assert_eq!(single.existence(), &MultiplicityInterval::MANDATORY);

    let container = CAttribute::container(
        "items",
        MultiplicityInterval::MANDATORY,
        Cardinality::new(MultiplicityInterval::at_least(1).unwrap()).ordered(),
        vec![leaf.clone()],
    )
    .unwrap();
    assert_eq!(container.rm_attribute_name(), "items");
    assert!(container.cardinality().unwrap().is_ordered());
    assert_eq!(container.children().len(), 1);

    // --- CObject and its variants -----------------------------------------
    assert_eq!(leaf.rm_type_name(), "DV_TEXT");
    let CObject::Primitive(primitive) = &leaf else {
        panic!("built a primitive and got something else");
    };
    assert_eq!(primitive.rm_type_name(), "DV_TEXT");

    let slot = ArchetypeSlot::new("CLUSTER", "at0007", MultiplicityInterval::OPTIONAL)
        .unwrap()
        .including("archetype_id/value matches {/.*/}")
        .excluding("archetype_id/value matches {/nothing/}");
    assert_eq!(slot.node_id(), "at0007");
    assert_eq!(slot.includes(), ["archetype_id/value matches {/.*/}"]);
    assert_eq!(slot.excludes(), ["archetype_id/value matches {/nothing/}"]);
    let slot_object = CObject::Slot(slot);
    assert_eq!(slot_object.rm_type_name(), "CLUSTER");
    assert_eq!(slot_object.node_id(), Some("at0007"));

    let root = CArchetypeRoot::new(
        "CLUSTER",
        "openEHR-EHR-CLUSTER.device.v1",
        MultiplicityInterval::MANDATORY,
    )
    .unwrap();
    assert_eq!(root.archetype_ref(), "openEHR-EHR-CLUSTER.device.v1");
    assert_eq!(CObject::ArchetypeRoot(root).rm_type_name(), "CLUSTER");

    // --- Archetype: the specialisation parent, and the template flag -------
    let plain_archetype = blood_pressure();
    assert_eq!(plain_archetype.parent_archetype_id(), None);
    assert!(!plain_archetype.is_template());

    let parent = "openEHR-EHR-OBSERVATION.blood_pressure.v2".parse().unwrap();
    let specialised = blood_pressure().specialising(parent);
    assert_eq!(
        specialised.parent_archetype_id().map(ToString::to_string),
        Some("openEHR-EHR-OBSERVATION.blood_pressure.v2".to_owned())
    );
    assert!(blood_pressure().as_template().is_template());
}

/// `K15.8`: the two node-id syntaxes are told apart at their boundaries.
///
/// If this broke, an ADL 2 code would be read as an ADL 1.4 one or the reverse,
/// and a converted archetype would record the wrong provenance for every node
/// in it. The cases here are the ones where the rule actually decides something
/// — one digit against several, a leading zero against none, and a trailing
/// empty segment, which is malformed rather than either syntax.
#[test]
fn the_node_id_syntaxes_are_distinguished_at_their_boundaries() {
    use openehr::am::NodeIdSyntax;

    // A single digit is ADL 2 even when it is zero: padding is what marks 1.4.
    assert_eq!(NodeIdSyntax::of("at0"), Some(NodeIdSyntax::Adl2));
    assert_eq!(NodeIdSyntax::of("id1"), Some(NodeIdSyntax::Adl2));
    // Several digits with no leading zero is still ADL 2.
    assert_eq!(NodeIdSyntax::of("at10"), Some(NodeIdSyntax::Adl2));
    // A leading zero and more than one digit is 1.4's four-digit spelling.
    assert_eq!(NodeIdSyntax::of("at0004"), Some(NodeIdSyntax::Adl14));
    assert_eq!(NodeIdSyntax::of("ac0001"), Some(NodeIdSyntax::Adl14));
    // A trailing or empty specialisation segment is neither syntax.
    assert_eq!(NodeIdSyntax::of("id1."), None);
    assert_eq!(NodeIdSyntax::of("id1..2"), None);
    assert_eq!(NodeIdSyntax::of("id1.x"), None);
    assert_eq!(NodeIdSyntax::of("id"), None);
}

/// A container whose children need exactly the cardinality it offers is legal.
///
/// If this broke — `>` becoming `>=` in `CAttribute::container` — every
/// archetype whose children fill their container exactly would be refused, and
/// that is the ordinary case rather than an edge one: two mandatory elements
/// under a `0..2` container is what a blood pressure looks like.
#[test]
fn children_may_fill_a_container_exactly() {
    let element = |code: &str| {
        CObject::Complex(
            CComplexObject::new(
                "ELEMENT",
                Some(code.to_owned()),
                MultiplicityInterval::MANDATORY,
                Vec::new(),
            )
            .unwrap(),
        )
    };

    // Two mandatory children, cardinality 0..2: exactly filled, and accepted.
    assert!(
        CAttribute::container(
            "items",
            MultiplicityInterval::MANDATORY,
            Cardinality::new(MultiplicityInterval::new(0, Some(2)).unwrap()),
            vec![element("at0004"), element("at0005")],
        )
        .is_ok()
    );

    // One more than it can hold is refused.
    assert!(
        CAttribute::container(
            "items",
            MultiplicityInterval::MANDATORY,
            Cardinality::new(MultiplicityInterval::new(0, Some(2)).unwrap()),
            vec![element("at0004"), element("at0005"), element("at0006")],
        )
        .is_err()
    );
}
