//! Validate a composition that arrived as JSON.
//!
//! ```sh
//! cargo run --example 02_validate_incoming
//! ```
//!
//! This is the case the constructors do not cover. `serde` writes fields
//! directly, so a document read from a wire has never been through
//! `Composition::new`, and it can carry combinations no constructor in this
//! crate will produce. [`Validate`](openehr::validation::Validate) is the
//! second gate, and it is the only gate on received data.
//!
//! The payload below has four separate defects, each one plausible:
//!
//! 1. an `ELEMENT` with both a value and a null flavour,
//! 2. an `ELEMENT` with neither,
//! 3. a `DV_CODED_TEXT` whose rubric contradicts its openEHR code,
//! 4. an archetype id constraining `OBSERVATION` on an `EVALUATION` node.
//!
//! None of them is a JSON error, and none would be caught by a schema.

use openehr::rm::ehr::Composition;
use openehr::validation::Validate;

const INCOMING: &str = r#"{
  "_type": "COMPOSITION",
  "name": {"value": "Encounter"},
  "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
  "language": {"terminology_id": {"value": "ISO_639-1"}, "code_string": "en"},
  "territory": {"terminology_id": {"value": "ISO_3166-1"}, "code_string": "GB"},
  "category": {
    "value": "deletion",
    "defining_code": {"terminology_id": {"value": "openehr"}, "code_string": "433"}
  },
  "composer": {"_type": "PARTY_IDENTIFIED", "name": "Dr A Nurse"},
  "content": [
    {
      "_type": "EVALUATION",
      "name": {"value": "Problem"},
      "archetype_node_id": "openEHR-EHR-OBSERVATION.blood_pressure.v2",
      "archetype_details": {
        "archetype_id": {"value": "openEHR-EHR-OBSERVATION.blood_pressure.v2"},
        "rm_version": "1.1.0"
      },
      "language": {"terminology_id": {"value": "ISO_639-1"}, "code_string": "en"},
      "encoding": {"terminology_id": {"value": "IANA_character-sets"}, "code_string": "UTF-8"},
      "subject": {"_type": "PARTY_SELF"},
      "data": {
        "_type": "ITEM_TREE",
        "name": {"value": "tree"},
        "archetype_node_id": "at0001",
        "items": [
          {
            "_type": "ELEMENT",
            "name": {"value": "Contradictory"},
            "archetype_node_id": "at0002",
            "value": {"_type": "DV_COUNT", "magnitude": 1},
            "null_flavour": {
              "value": "unknown",
              "defining_code": {"terminology_id": {"value": "openehr"}, "code_string": "253"}
            }
          },
          {
            "_type": "ELEMENT",
            "name": {"value": "Empty"},
            "archetype_node_id": "at0003"
          }
        ]
      }
    }
  ]
}"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Deserialization succeeds. That is the point: the document is
    // well-formed JSON that satisfies the type structure and violates the
    // Reference Model.
    let composition: Composition = serde_json::from_str(INCOMING)?;

    let report = composition.validate();
    println!("{} invariant violation(s):\n", report.len());
    for violation in report.violations() {
        println!(
            "  {:<28} {}.{}",
            if violation.path.is_empty() {
                "/"
            } else {
                &violation.path
            },
            violation.class,
            violation.invariant
        );
        println!("  {:<28} {}", "", violation.detail);
    }

    // Nothing in the report repeats a clinical value — only paths, class
    // names, and invariant names. That is what makes a validation report safe
    // to log at the point where it is most useful.
    assert!(!report.to_string().contains("Dr A Nurse"));

    // `validate_ok` turns the report into a `Result`, so a service can reject
    // the payload with one `?`.
    match composition.validate_ok() {
        Ok(()) => println!("\naccepted"),
        Err(report) => println!("\nrejected: {} violation(s)", report.len()),
    }
    Ok(())
}
