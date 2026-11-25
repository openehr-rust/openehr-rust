//! Build a blood-pressure `COMPOSITION` and write it as openEHR canonical
//! JSON.
//!
//! ```sh
//! cargo run --example 01_build_composition
//! ```
//!
//! The shape to notice is that an `OBSERVATION`'s data is a `HISTORY` even for
//! a single reading. openEHR treats every observation as a time series of
//! length one or more, so a one-off blood pressure and a continuous arterial
//! trace have the same structure and no consumer has to special-case the
//! second reading.

use openehr::rm::common::{Archetyped, LocatableAttrs, PartyIdentified};
use openehr::rm::data_structures::{Element, History, ItemTree, PointEvent};
use openehr::rm::data_types::{CodePhrase, DataValue, DvCodedText, DvDateTime, DvQuantity};
use openehr::rm::ehr::{Composition, EntryAttrs, EventContext, Observation};
use openehr::terminology::{composition_category, setting};
use openehr::validation::Validate;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let at = |name: &str, node: &str| LocatableAttrs::named(name, node).expect("literal attrs");

    // The two readings. `mm[Hg]` is the UCUM code, not the display form: the
    // display form belongs in `units_display_name`, and mixing the two is how a
    // units string stops being machine-comparable.
    let systolic = Element::new(
        at("Systolic", "at0004"),
        DataValue::Quantity(
            DvQuantity::new(184.0, "mm[Hg]")?
                .with_units_system(DvQuantity::UCUM)
                .with_units_display_name("mmHg")
                .with_precision(0)?,
        ),
    );
    let diastolic = Element::new(
        at("Diastolic", "at0005"),
        DataValue::Quantity(
            DvQuantity::new(96.0, "mm[Hg]")?
                .with_units_system(DvQuantity::UCUM)
                .with_precision(0)?,
        ),
    );

    // `state` is not decoration. A blood pressure of 184/96 standing and the
    // same value lying down are different findings, and openEHR keeps `state`
    // separate from `data` so the distinction survives into every consumer.
    //
    // The position is **coded**, not free text. In the published archetype
    // (CKM `openEHR-EHR-OBSERVATION.blood_pressure.v2`, revision 2.0.17, the
    // `local` terminology) `at0008` constrains its value to `DV_CODED_TEXT`
    // with a defining code from `at1000`-`at1003` and `at1014`; sitting is
    // `at1001`. A `DV_TEXT` reading "Sitting" looks identical to a person and
    // is invisible to any system searching for the coded posture. Note that
    // `validate()` below would accept either: it checks the Reference Model's
    // invariants, not the archetype's constraints, so a successful commit is
    // not evidence of archetype conformance.
    let position = ItemTree::new(
        at("state structure", "at0007"),
        vec![
            Element::new(
                at("Position", "at0008"),
                DataValue::CodedText(DvCodedText::new(
                    "Sitting",
                    CodePhrase::new("local", "at1001")?,
                )?),
            )
            .into(),
        ],
    );

    let event = PointEvent::new(
        at("any event", "at0006"),
        DvDateTime::new("2026-07-31T09:15:00Z")?,
        ItemTree::new(
            at("blood pressure", "at0003"),
            vec![systolic.into(), diastolic.into()],
        )
        .into(),
    )
    .with_state(position.into());

    let history = History::new(
        at("Event Series", "at0001"),
        DvDateTime::new("2026-07-31T09:00:00Z")?,
        vec![event.into()],
        None,
    )?;

    let observation = Observation::new(
        at(
            "Blood pressure",
            "openEHR-EHR-OBSERVATION.blood_pressure.v2",
        )
        .with_archetype_details(Archetyped::new(
            "openEHR-EHR-OBSERVATION.blood_pressure.v2",
            "1.1.0",
        )?),
        EntryAttrs::about_subject(
            CodePhrase::new("ISO_639-1", "en")?,
            CodePhrase::new("IANA_character-sets", "UTF-8")?,
        ),
        history,
    );

    let composition = Composition::new(
        at("Encounter", "openEHR-EHR-COMPOSITION.encounter.v1").with_archetype_details(
            Archetyped::new("openEHR-EHR-COMPOSITION.encounter.v1", "1.1.0")?,
        ),
        composition_category::EVENT,
        PartyIdentified::named("Dr A Nurse")?.into(),
        CodePhrase::new("ISO_639-1", "en")?,
        CodePhrase::new("ISO_3166-1", "GB")?,
    )?
    .with_context(
        EventContext::new(
            DvDateTime::new("2026-07-31T09:00:00Z")?,
            setting::PRIMARY_MEDICAL_CARE,
        )?
        .with_location("Consulting room 3")?,
    )?
    .with_content(observation.into());

    // Building it through the constructors already enforced the invariants;
    // validating proves it, and costs one traversal.
    let report = composition.validate();
    assert!(report.is_empty(), "{report}");

    println!("{}", serde_json::to_string_pretty(&composition)?);
    println!(
        "\n-- {} entr(y|ies), category {}, {} invariant violations",
        composition.entries().count(),
        composition.category().value(),
        report.len()
    );
    Ok(())
}
