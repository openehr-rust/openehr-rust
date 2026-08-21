//! Every code fragment in `README.md`, compiled and run.
//!
//! A README is the first thing a reader tries and the last thing anyone
//! re-checks, so its examples rot silently. These tests are the fragments with
//! their surrounding context filled in — the bodies are otherwise identical,
//! and a change to either that is not made to both fails here.
//!
//! `T13.8` states the rule this file enforces: a documented example that does
//! not compile is worse than none, because it costs the reader the time to find
//! out.
//!
//! The JSON below carries `archetype_details` on its OBSERVATION as well as its
//! COMPOSITION. It did not until `ENTRY.Is_archetype_root` was enforced, and an
//! example of "a composition another implementation wrote" that would be
//! rejected by this crate's own validator is worse than no example at all —
//! `LOCATABLE.Archetyped_valid` makes `is_archetype_root` and having
//! `archetype_details` the same statement.

use openehr::base::iso8601::Date;
use openehr::path::Pathable;
use openehr::rm::data_structures::Element;
use openehr::rm::data_types::{DvOrdered as _, DvQuantity};
use openehr::rm::ehr::Composition;
use openehr::validation::Validate;

/// The composition the first fragment reads, as an openEHR canonical JSON
/// document. Deliberately minimal: the fragment is about the three calls, not
/// about the document.
const JSON: &str = r#"{
  "_type": "COMPOSITION",
  "name": {"value": "Encounter"},
  "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
  "archetype_details": {
    "archetype_id": {"value": "openEHR-EHR-COMPOSITION.encounter.v1"},
    "rm_version": "1.1.0"
  },
  "language": {"terminology_id": {"value": "ISO_639-1"}, "code_string": "en"},
  "territory": {"terminology_id": {"value": "ISO_3166-1"}, "code_string": "GB"},
  "category": {
    "value": "event",
    "defining_code": {"terminology_id": {"value": "openehr"}, "code_string": "433"}
  },
  "composer": {"_type": "PARTY_IDENTIFIED", "name": "Dr A Nurse"},
  "content": [{
    "_type": "OBSERVATION",
    "name": {"value": "Blood pressure"},
    "archetype_node_id": "openEHR-EHR-OBSERVATION.blood_pressure.v2",
    "archetype_details": {
      "archetype_id": {"value": "openEHR-EHR-OBSERVATION.blood_pressure.v2"},
      "rm_version": "1.1.0"
    },
    "language": {"terminology_id": {"value": "ISO_639-1"}, "code_string": "en"},
    "encoding": {"terminology_id": {"value": "IANA_character-sets"}, "code_string": "UTF-8"},
    "subject": {"_type": "PARTY_SELF"},
    "data": {
      "_type": "HISTORY",
      "name": {"value": "Event Series"},
      "archetype_node_id": "at0001",
      "origin": {"value": "2026-07-31T09:00:00Z"},
      "events": [{
        "_type": "POINT_EVENT",
        "name": {"value": "any event"},
        "archetype_node_id": "at0006",
        "time": {"value": "2026-07-31T09:15:00Z"},
        "data": {
          "_type": "ITEM_TREE",
          "name": {"value": "blood pressure"},
          "archetype_node_id": "at0003",
          "items": [{
            "_type": "ELEMENT",
            "name": {"value": "Systolic"},
            "archetype_node_id": "at0004",
            "value": {"_type": "DV_QUANTITY", "magnitude": 184.0, "units": "mm[Hg]"}
          }]
        }
      }]
    }
  }]
}"#;

/// README § *What it does*.
#[test]
fn what_it_does() -> Result<(), Box<dyn std::error::Error>> {
    let json = JSON;

    // Read a composition another openEHR implementation wrote.
    let composition: Composition = serde_json::from_str(json)?;

    // Check the Reference Model invariants. Deserialization never calls a
    // constructor, so this is the only gate on data that arrived from
    // elsewhere.
    composition.validate_ok()?;

    // Address a node by openEHR path.
    let systolic = composition.item_at_path(
        "/content[openEHR-EHR-OBSERVATION.blood_pressure.v2]\
         /data/events[at0006]/data/items[at0004]/value/magnitude",
    )?;

    assert_eq!(
        systolic,
        openehr::path::Node::Scalar(openehr::path::Scalar::Number(184.0))
    );
    Ok(())
}

/// README § *Refuse rather than guess*.
#[test]
fn refuse_rather_than_guess() -> Result<(), Box<dyn std::error::Error>> {
    let may: Date = "2024-05".parse()?;
    let may_17: Date = "2024-05-17".parse()?;
    assert_eq!(may.semantic_cmp(&may_17), None); // May which day?

    let mg = DvQuantity::new(5.0, "mg")?;
    let ml = DvQuantity::new(5.0, "mL")?;
    assert_eq!(mg.semantic_cmp(&ml), None); // not the same dose of anything
    Ok(())
}

/// README § *Two gates, not one*.
#[test]
fn two_gates_not_one() -> Result<(), Box<dyn std::error::Error>> {
    // No constructor in this crate produces this. A sender can still send it.
    let element: Element = serde_json::from_str(
        r#"{"name":{"value":"Systolic"},"archetype_node_id":"at0004",
            "value":{"_type":"DV_COUNT","magnitude":1},
            "null_flavour":{"value":"unknown","defining_code":
              {"terminology_id":{"value":"openehr"},"code_string":"253"}}}"#,
    )?;
    assert_eq!(
        element.validate().violations()[0].invariant,
        "Inv_null_flavour_indicated"
    );
    Ok(())
}

/// README § *Absence is structured* — the four flavours and their meanings, as
/// the table states them.
#[test]
fn the_null_flavour_table_is_accurate() {
    use openehr::terminology::null_flavour;
    for (code, rubric) in [
        (null_flavour::NO_INFORMATION, "no information"),
        (null_flavour::UNKNOWN, "unknown"),
        (null_flavour::MASKED, "masked"),
        (null_flavour::NOT_APPLICABLE, "not applicable"),
    ] {
        assert_eq!(null_flavour::GROUP.rubric(code), Some(rubric));
    }
    assert_eq!(null_flavour::GROUP.concepts.len(), 4);
}

/// README § *What it does* — the module table claims sixteen terminology
/// groups.
#[test]
fn the_module_table_counts_are_accurate() {
    assert_eq!(openehr::terminology::GROUPS.len(), 16);
}
