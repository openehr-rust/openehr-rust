//! Decide access against an `EHR_ACCESS` policy, then disclose a filtered
//! composition.
//!
//! ```sh
//! cargo run --example 05_access_and_redaction
//! ```
//!
//! Two things worth watching:
//!
//! **Everything defaults to deny.** An `EHR_ACCESS` with no settings, and a
//! policy in a scheme this process cannot evaluate, both refuse — and they
//! refuse with different reasons, because one is a data problem and the other
//! a deployment problem.
//!
//! **Redaction masks; it does not delete.** A withheld element becomes
//! `272｜masked｜`, which tells the reader *there is a value here and you are
//! not being shown it*. Deleting it instead would turn "the patient has
//! withheld their HIV status" into "the patient has no HIV status", which is a
//! clinical statement nobody made.

use openehr::rm::common::{Archetyped, LocatableAttrs, PartyIdentified};
use openehr::rm::data_structures::{Element, ItemTree};
use openehr::rm::data_types::{CodePhrase, DataValue, DvText};
use openehr::rm::ehr::{Composition, EntryAttrs, Evaluation};
use openehr::security::{
    AccessControlSettings, AccessRequest, EhrAccess, GroupSettings, Operation, RedactionRule,
    Redactor,
};
use openehr::terminology::composition_category;
use openehr::validation::Validate;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ---- access ----------------------------------------------------------
    let bare = EhrAccess::new(LocatableAttrs::named(
        "EHR Access",
        "openEHR-EHR-EHR_ACCESS.generic.v1",
    )?);
    let anybody = AccessRequest {
        operation: Operation::Read,
        groups: &["care-team".to_owned()],
        is_subject: false,
    };
    println!("no settings recorded   -> {:?}", bare.decide(&anybody));

    let configured = bare.clone().with_settings(
        GroupSettings::new()
            .permit(Operation::Read, "care-team")
            .permit(Operation::Write, "care-team")
            .permit(Operation::Audit, "information-governance")
            .permit_subject_read()
            .into(),
    );
    let care = ["care-team".to_owned()];
    let ig = ["information-governance".to_owned()];
    let none: [String; 0] = [];

    for (label, groups, operation, is_subject) in [
        ("care team, read     ", &care[..], Operation::Read, false),
        ("care team, write    ", &care[..], Operation::Write, false),
        ("care team, delete   ", &care[..], Operation::Delete, false),
        ("gov, audit trail    ", &ig[..], Operation::Audit, false),
        ("gov, clinical read  ", &ig[..], Operation::Read, false),
        ("the patient, read   ", &none[..], Operation::Read, true),
        ("the patient, write  ", &none[..], Operation::Write, true),
    ] {
        let decision = configured.decide(&AccessRequest {
            operation,
            groups,
            is_subject,
        });
        println!("{label} -> {decision:?}");
    }

    // A policy in a scheme this crate does not implement is carried unchanged
    // and denied — never evaluated as though it were the reference scheme, and
    // never silently dropped on a read-modify-write.
    let foreign: AccessControlSettings = serde_json::from_str(
        r#"{"scheme":"nl.nictiz.opt-out.v2","register":"national","withdrawn":false}"#,
    )?;
    println!("\nforeign scheme {}", foreign.scheme());
    println!("  decide -> {:?}", foreign.decide(&anybody));
    println!(
        "  round-trips unchanged: {}",
        openehr::security::to_canonical_string(&foreign)?
    );

    // ---- redaction -------------------------------------------------------
    let composition = record()?;
    let full = serde_json::to_string(&composition)?;
    println!(
        "\nunredacted contains the sensitive value: {}",
        full.contains("Positive")
    );

    let (filtered, count) = Redactor::new()
        .with_rule(RedactionRule::node_id("at0011"))
        .with_reason("Withheld under the patient's recorded consent preferences")
        .redact_counting(&composition)?;
    let redacted = serde_json::to_string(&filtered)?;

    println!("{count}");
    println!(
        "  sensitive value present : {}",
        redacted.contains("Positive")
    );
    println!(
        "  masked flavour present  : {}",
        redacted.contains("masked")
    );
    println!("  other content preserved : {}", redacted.contains("70 kg"));

    // The redacted document is still a valid COMPOSITION. A filter that
    // produced something the receiving system rejects has achieved nothing.
    let report = filtered.validate();
    assert!(report.is_empty(), "{report}");
    println!("  still valid             : true");

    // The count is what goes in the access log. The *names* of what was
    // withheld do not: an audit trail that records "HIV status withheld" has
    // disclosed the category it was protecting.
    println!("\naccess log line: disclosed 1 composition, {count}");
    Ok(())
}

fn record() -> Result<Composition, Box<dyn std::error::Error>> {
    let at = |name: &str, node: &str| LocatableAttrs::named(name, node).expect("literal attrs");
    let text = |v: &str| -> Result<DataValue, Box<dyn std::error::Error>> {
        Ok(DataValue::Text(DvText::new(v)?))
    };

    let data = ItemTree::new(
        at("tree", "at0001"),
        vec![
            Element::new(at("HIV status", "at0011"), text("Positive")?).into(),
            Element::new(at("Weight", "at0012"), text("70 kg")?).into(),
        ],
    );
    let evaluation = Evaluation::new(
        at("Problem", "openEHR-EHR-EVALUATION.problem_diagnosis.v1").with_archetype_details(
            Archetyped::new("openEHR-EHR-EVALUATION.problem_diagnosis.v1", "1.1.0")?,
        ),
        EntryAttrs::about_subject(
            CodePhrase::new("ISO_639-1", "en")?,
            CodePhrase::new("IANA_character-sets", "UTF-8")?,
        ),
        data.into(),
    );
    Ok(Composition::new(
        at("Encounter", "openEHR-EHR-COMPOSITION.encounter.v1").with_archetype_details(
            Archetyped::new("openEHR-EHR-COMPOSITION.encounter.v1", "1.1.0")?,
        ),
        composition_category::EVENT,
        PartyIdentified::named("Dr A Nurse")?.into(),
        CodePhrase::new("ISO_639-1", "en")?,
        CodePhrase::new("ISO_3166-1", "GB")?,
    )?
    .with_content(evaluation.into()))
}
