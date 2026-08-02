//! The cross-cutting guarantees, tested where they can actually fail.
//!
//! Unit tests inside a module check one type. These check a **property that
//! must hold across the crate**, which is the kind that regresses when someone
//! adds a type and follows the shape of the one next to it without noticing
//! what that shape was for.
//!
//! Each test here has a stated failure mode. A test whose failure mode nobody
//! wrote down is a test nobody will maintain.

use openehr::aql::AqlQuery;
use openehr::base::{Interval, iso8601};
use openehr::path::Pathable;
use openehr::rm::common::{LocatableAttrs, PartyIdentified};
use openehr::rm::data_structures::{Element, ItemTree};
use openehr::rm::data_types::{
    CodePhrase, DataValue, DvDate, DvDateTime, DvIdentifier, DvMultimedia, DvQuantity, DvText,
};
use openehr::rm::ehr::{Composition, EntryAttrs, Evaluation};
use openehr::security::{ChainKey, RedactionRule, Redactor, Sensitive};
use openehr::terminology::{self, composition_category};
use openehr::validation::Validate;

/// A string that will not occur by accident, so that "does this output contain
/// patient data?" is answerable by substring search.
const MARKER: &str = "ZZ-DISTINCTIVE-MARKER-9999";

fn at(name: &str, node: &str) -> LocatableAttrs {
    LocatableAttrs::named(name, node).expect("literal attrs")
}

fn composition_containing(marker: &str) -> Composition {
    let data = ItemTree::new(
        at("tree", "at0001"),
        vec![
            Element::new(
                at("HIV status", "at0011"),
                DataValue::Text(DvText::new(marker).unwrap()),
            )
            .into(),
        ],
    );
    let evaluation = Evaluation::new(
        // An ENTRY is the root of an entry archetype (`ENTRY.Is_archetype_root`),
        // and this fixture said `at0000` — an interior node id — until that rule
        // was enforced.
        at("Problem", "openEHR-EHR-EVALUATION.problem.v1").with_archetype_details(
            openehr::rm::common::Archetyped::new("openEHR-EHR-EVALUATION.problem.v1", "1.1.0")
                .unwrap(),
        ),
        EntryAttrs::about_subject(
            CodePhrase::new("ISO_639-1", "en").unwrap(),
            CodePhrase::new("IANA_character-sets", "UTF-8").unwrap(),
        ),
        data.into(),
    );
    Composition::new(
        at("Encounter", "openEHR-EHR-COMPOSITION.encounter.v1").with_archetype_details(
            openehr::rm::common::Archetyped::new("openEHR-EHR-COMPOSITION.encounter.v1", "1.1.0")
                .unwrap(),
        ),
        composition_category::EVENT,
        PartyIdentified::named("Dr A Nurse").unwrap().into(),
        CodePhrase::new("ISO_639-1", "en").unwrap(),
        CodePhrase::new("ISO_3166-1", "GB").unwrap(),
    )
    .unwrap()
    .with_content(evaluation.into())
}

// ---------------------------------------------------------------------------
// Nothing prints protected health information
// ---------------------------------------------------------------------------

/// Fails if any `Display` implementation on a PHI-bearing type starts printing
/// its content — the change that turns one `tracing::info!("{id}")` into a
/// disclosure.
#[test]
fn display_never_reveals_an_identifier_or_a_media_blob() {
    let id = DvIdentifier::new(MARKER)
        .unwrap()
        .with_type("NHS number")
        .with_issuer("NHS England");
    assert!(!format!("{id}").contains(MARKER));
    assert_eq!(format!("{id}"), "NHS number issued by NHS England");

    let media = DvMultimedia::inline(
        CodePhrase::new("IANA_media-types", "image/png").unwrap(),
        MARKER.as_bytes().to_vec(),
    );
    // Multimedia has no Display at all — deliberately — so the reachable
    // rendering is Debug, which prints shape.
    assert!(!format!("{media:?}").contains(MARKER));

    let wrapped = Sensitive::new(MARKER.to_owned());
    assert!(!format!("{wrapped}").contains(MARKER));
    assert!(!format!("{wrapped:?}").contains(MARKER));
    // …and it is still there for storage and for an authorised recipient.
    assert_eq!(wrapped.expose(), MARKER);
}

/// Fails if a constructor starts echoing the value that broke an invariant.
///
/// The failure this prevents is specific: an error message is the one place a
/// value reaches a log, an HTTP response, and a support ticket at once.
#[test]
fn no_construction_error_echoes_a_submitted_value() {
    let failures: Vec<String> = vec![
        DvText::new("").unwrap_err().to_string(),
        DvQuantity::new(f64::NAN, "mg").unwrap_err().to_string(),
        DvQuantity::new(1.0, "").unwrap_err().to_string(),
        DvIdentifier::new("").unwrap_err().to_string(),
        Element::new_null(at("x", "at0001"), "999")
            .unwrap_err()
            .to_string(),
        DvDate::new(MARKER).unwrap_err().to_string(),
    ];
    // The one deliberate exception is a *lexical* rejection of design-time
    // vocabulary, which does echo — an identifier error that will not say which
    // identifier is unactionable. So the date failure above is expected to
    // repeat its input, and the invariant failures are not.
    for message in &failures[..5] {
        assert!(!message.contains(MARKER), "{message}");
        assert!(message.contains("invalid"), "{message}");
    }
    assert!(
        failures[5].contains(MARKER),
        "lexical errors do name their input"
    );
}

/// Fails if a validation report starts carrying node content.
#[test]
fn a_validation_report_names_paths_and_never_values() {
    let json = format!(r#"{{"name": {{"value": "{MARKER}"}}, "archetype_node_id": "at0004"}}"#);
    let element: Element = serde_json::from_str(&json).unwrap();
    let report = element.validate();
    assert!(!report.is_empty());
    assert!(!report.to_string().contains(MARKER), "{report}");
}

/// Fails if a chain checkpoint starts including anything from the versions it
/// covers. The whole value of a checkpoint is that it can be shipped to a
/// long-retention log where clinical data must not go.
#[test]
fn a_chain_checkpoint_carries_no_patient_data() {
    let key = ChainKey::new("k1", vec![3u8; 32]).unwrap();
    let mut chain = openehr::security::Chain::new();
    chain
        .append("uid::sys::1", &composition_containing(MARKER), Some(&key))
        .unwrap();
    let checkpoint = chain.checkpoint();
    assert!(!checkpoint.contains(MARKER), "{checkpoint}");
    assert!(checkpoint.contains("entries=1"));
}

/// Fails if redaction starts deleting rather than masking, or starts naming
/// what it withheld.
#[test]
fn redaction_masks_and_reports_a_count_not_a_category() {
    let (redacted, count) = Redactor::new()
        .with_rule(RedactionRule::node_id("at0011"))
        .redact_counting(&composition_containing(MARKER))
        .unwrap();
    let json = serde_json::to_string(&redacted).unwrap();

    assert!(!json.contains(MARKER), "the value survived redaction");
    assert!(json.contains(terminology::null_flavour::MASKED));
    // The boundary, stated: redaction rules address ELEMENTs. The composer,
    // the participations, and the audit trail are *not* clinical content and
    // are not withheld by an element rule — a deployment that must also strip
    // those is doing de-identification, which is a different operation with
    // different rules.
    assert!(json.contains("Dr A Nurse"));
    // Masked, not deleted: the reader must be able to tell "withheld" from
    // "never recorded".
    let element = redacted
        .item_at_path("/content/data/items[at0011]")
        .expect("the element is still there");
    assert_eq!(element.type_name(), "ELEMENT");
    // And the count says how much without saying what.
    assert_eq!(count.masked, 1);
    assert!(!count.to_string().contains("HIV"));

    // The result is still a valid composition, so the receiving system can
    // read it.
    assert!(redacted.validate().is_empty());
}

// ---------------------------------------------------------------------------
// Refuse rather than guess
// ---------------------------------------------------------------------------

/// Fails if any of the partial orders quietly becomes total.
///
/// Each pair below has a plausible wrong answer that nothing downstream could
/// detect: a month ordered before a day inside it, five milligrams equal to
/// five millilitres, a local time compared with a UTC one.
#[test]
fn every_undecidable_comparison_answers_none() {
    let month: iso8601::Date = "2024-05".parse().unwrap();
    let day: iso8601::Date = "2024-05-17".parse().unwrap();
    assert_eq!(month.partial_cmp(&day), None);

    let local: iso8601::Time = "11:00:00".parse().unwrap();
    let utc: iso8601::Time = "11:00:00Z".parse().unwrap();
    assert_eq!(local.partial_cmp(&utc), None);

    let twelve_months: iso8601::Duration = "P12M".parse().unwrap();
    let one_year: iso8601::Duration = "P1Y".parse().unwrap();
    assert_eq!(twelve_months.partial_cmp(&one_year), None);

    let mg = DataValue::Quantity(DvQuantity::new(5.0, "mg").unwrap());
    let ml = DataValue::Quantity(DvQuantity::new(5.0, "mL").unwrap());
    assert_eq!(mg.partial_cmp(&ml), None);
    assert!(!mg.is_strictly_comparable_to(&ml));

    let count = DataValue::Count(openehr::rm::data_types::DvCount::new(5));
    assert_eq!(mg.partial_cmp(&count), None);

    // And the decidable cases are still decided, so the refusals above are not
    // just "comparison is broken".
    let april: iso8601::Date = "2024-04".parse().unwrap();
    assert!(april < day);
    let more = DataValue::Quantity(DvQuantity::new(6.0, "mg").unwrap());
    assert!(mg < more);
}

/// Fails if an ambiguous path starts resolving to its first match.
#[test]
fn an_ambiguous_path_refuses_instead_of_choosing() {
    let data = ItemTree::new(
        at("tree", "at0001"),
        vec![
            Element::new(
                at("Systolic", "at0004"),
                DataValue::Quantity(DvQuantity::new(184.0, "mm[Hg]").unwrap()),
            )
            .into(),
            Element::new(
                at("Diastolic", "at0005"),
                DataValue::Quantity(DvQuantity::new(96.0, "mm[Hg]").unwrap()),
            )
            .into(),
        ],
    );
    // Rooted at `ItemStructure`, which is the type a path addresses; the four
    // concrete structures convert into it.
    let data: openehr::rm::data_structures::ItemStructure = data.into();
    assert!(data.path_exists("/items/value/magnitude").unwrap());
    assert!(!data.path_unique("/items/value/magnitude").unwrap());
    assert_eq!(
        data.items_at_path("/items/value/magnitude").unwrap().len(),
        2
    );
    assert!(data.item_at_path("/items/value/magnitude").is_err());
    // With a predicate it resolves, and to the right one.
    assert!(
        data.item_at_path("/items['Diastolic']/value/magnitude")
            .is_ok()
    );
}

/// Fails if an interval starts accepting bounds whose order is not established.
#[test]
fn an_interval_refuses_bounds_it_cannot_order() {
    // Two dates of different precision whose known components agree: nothing
    // establishes which is earlier, so this is not a usable range.
    let month = DvDate::new("2024-05").unwrap();
    let day = DvDate::new("2024-05-17").unwrap();
    assert!(Interval::closed(month, day).is_err());

    let ok = Interval::closed(
        DvDate::new("2024-04").unwrap(),
        DvDate::new("2024-05-17").unwrap(),
    );
    assert!(ok.is_ok());
}

/// Fails if an unimplemented openEHR operation starts returning a plausible
/// value instead of refusing.
#[test]
fn unimplemented_operations_refuse_and_cite_the_spec() {
    use openehr::rm::data_structures::{IntervalEvent, ItemSingle};

    let data = ItemSingle::new(
        at("d", "at0001"),
        Element::new(
            at("v", "at0002"),
            DataValue::Count(openehr::rm::data_types::DvCount::new(1)),
        ),
    );
    // A calendar-month width has no fixed length in seconds; a month before
    // 31 March is 28 February, which no number of seconds produces.
    let event = IntervalEvent::new(
        at("monthly", "at0006"),
        DvDateTime::new("2026-03-31T08:00:00Z").unwrap(),
        data.into(),
        openehr::rm::data_types::DvDuration::new("P1M").unwrap(),
        terminology::event_math_function::TOTAL,
    )
    .unwrap();
    let err = event.interval_start_time().unwrap_err();
    assert!(matches!(err, openehr::Error::Unsupported { .. }));
    assert!(err.to_string().contains("spec/"), "{err}");
}

// ---------------------------------------------------------------------------
// Absence stays structured
// ---------------------------------------------------------------------------

/// Fails if the four null flavours ever become interchangeable.
#[test]
fn the_four_null_flavours_remain_four() {
    let flavours = [
        terminology::null_flavour::NO_INFORMATION,
        terminology::null_flavour::UNKNOWN,
        terminology::null_flavour::MASKED,
        terminology::null_flavour::NOT_APPLICABLE,
    ];
    let mut codes = std::collections::HashSet::new();
    for code in flavours {
        let element = Element::new_null(at("x", "at0001"), code).unwrap();
        assert!(element.is_null());
        assert_eq!(element.null_flavour_code(), Some(code));
        assert_eq!(
            element.is_masked(),
            code == terminology::null_flavour::MASKED
        );
        codes.insert(code);

        // Each survives a round trip as itself.
        let json = serde_json::to_string(&element).unwrap();
        let back: Element = serde_json::from_str(&json).unwrap();
        assert_eq!(back.null_flavour_code(), Some(code));
    }
    assert_eq!(codes.len(), 4);

    // A fifth cannot be invented.
    assert!(Element::new_null(at("x", "at0001"), "999").is_err());
}

// ---------------------------------------------------------------------------
// The AQL front end tells the truth about itself
// ---------------------------------------------------------------------------

/// Fails if AQL parsing starts accepting a construct it cannot represent, which
/// would make a partially-understood query look fully understood.
#[test]
fn aql_refuses_what_it_does_not_model_and_says_where_that_is_recorded() {
    for text in ["SELECT * FROM COMPOSITION c", "SELECT c/uid FROM VERSION v"] {
        let err = text.parse::<AqlQuery>().unwrap_err();
        assert!(err.reason.contains("Q12.9"), "{err}");
    }
}

/// Fails if the alias check stops catching the rename bug — a query that parses,
/// executes, and returns nothing.
#[test]
fn aql_catches_a_path_rooted_at_an_unbound_alias() {
    let query: AqlQuery = "SELECT o/value FROM COMPOSITION c CONTAINS OBSERVATION obs"
        .parse()
        .unwrap();
    assert!(query.check().is_err());
    let fixed: AqlQuery = "SELECT obs/value FROM COMPOSITION c CONTAINS OBSERVATION obs"
        .parse()
        .unwrap();
    assert!(fixed.check().is_ok());
}
