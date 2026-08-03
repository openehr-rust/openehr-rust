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

// ---------------------------------------------------------------------------
// The premise `X11.24` rests on

/// `A-10` says redaction's fail-closed path cannot be provoked, because "every
/// `Composition` this crate can construct serializes". That is the reason
/// `X11.24` is recorded as **?** rather than **•**, so it is worth more than a
/// belief.
///
/// The reason is sharper than "documents are well formed", and worse.
/// `serde_json` does **not** refuse a non-finite float: it writes `null`. So a
/// `NaN` magnitude that ever reached serialization would not fail — it would
/// silently become an absent value, in the canonical form the content digest
/// is taken over.
///
/// Every `f64` a document can carry is therefore refused **at construction**,
/// and those constructors are the only barrier. This asserts each one holds.
/// If somebody relaxes one — an `Accuracy_finite` that starts accepting `NaN` —
/// this test fails and names it, and what follows is silent data loss rather
/// than an error anybody sees.
#[test]
fn no_document_this_crate_can_build_carries_a_non_finite_float() {
    use openehr::rm::data_types::{DvCodedText, DvProportion, DvScale, ProportionKind};

    let symbol = || {
        DvCodedText::new("symbol", CodePhrase::new("local", "at0001").unwrap()).unwrap()
    };
    for (name, value) in [
        ("NaN", f64::NAN),
        ("+inf", f64::INFINITY),
        ("-inf", f64::NEG_INFINITY),
    ] {
        assert!(
            DvQuantity::new(value, "mm[Hg]").is_err(),
            "DV_QUANTITY accepted a magnitude of {name}"
        );
        assert!(
            DvScale::new(value, symbol()).is_err(),
            "DV_SCALE accepted a value of {name}"
        );
        assert!(
            DvQuantity::new(1.0, "mm[Hg]")
                .unwrap()
                .with_accuracy(value, false)
                .is_err(),
            "DV_AMOUNT accepted an accuracy of {name}"
        );
        assert!(
            DvProportion::new(value, 1.0, ProportionKind::Ratio).is_err(),
            "DV_PROPORTION accepted a numerator of {name}"
        );
        assert!(
            DvProportion::new(1.0, value, ProportionKind::Ratio).is_err(),
            "DV_PROPORTION accepted a denominator of {name}"
        );
    }

    // The fact that makes the above load-bearing, asserted rather than
    // assumed. Serialization does not fail here; it loses the value.
    assert_eq!(serde_json::to_string(&f64::NAN).unwrap(), "null");
    assert_eq!(
        openehr::security::to_canonical_string(&f64::INFINITY).unwrap(),
        "null"
    );
}

/// `X11.12`: tag comparison is constant-time, and the *natural* wrong way to
/// write it does not compile.
///
/// Timing is not measured — a timing assertion in a unit test is a flake
/// generator, and the matrix records `X11.12` as **?** for that reason. The
/// The structural half is not pinned by a test either, and `Mac`'s own
/// documentation says why: a `compile_fail` doctest for it passes whether or
/// not the derive is there, because the constructor it would need is private.
/// What this test pins is the behaviour that matters — a tag from the wrong
/// key is refused, and refused *as a tag mismatch* rather than as a missing or
/// unknown key (`X11.13`).
///
/// The rule was previously kept by one `ct_eq` call and the discipline not to
/// replace it, and `==` is what anyone simplifying that line would reach for.
#[test]
fn a_forged_tag_is_refused() {
    let key = ChainKey::new("k1", vec![7u8; 32]).unwrap();
    let other = ChainKey::new("k1", vec![9u8; 32]).unwrap();

    let mut chain = openehr::security::Chain::new();
    chain.append("v1", &"content", Some(&key)).unwrap();
    assert!(matches!(
        chain.verify(&[&key]),
        openehr::security::ChainStatus::Verified
    ));

    // A key with the same id and different material is the forgery this is
    // for: the entry names `k1`, so verification finds a key and the tag has
    // to do the work.
    assert!(matches!(
        chain.verify(&[&other]),
        openehr::security::ChainStatus::Broken {
            reason: openehr::security::BreakReason::TagMismatch,
            ..
        }
    ));
}

/// Redaction has three rule kinds. Two of them had no test at all.
///
/// Every existing redaction test used `RedactionRule::node_id`, so the arms
/// matching by **name** and by **archetype root** could each be inverted with
/// the suite green (`lib:A-09`). Redaction is the PHI-withholding mechanism
/// (`X11.24`, `X11.25`); two thirds of its vocabulary being unexercised is not
/// a coverage statistic, it is a rule nobody has watched work.
#[test]
fn every_redaction_rule_kind_withholds_what_it_names() {
    let masked = |rule: RedactionRule| {
        let (redacted, count) = Redactor::new()
            .with_rule(rule)
            .redact_counting(&composition_containing(MARKER))
            .unwrap();
        let json = serde_json::to_string(&redacted).unwrap();
        (json.contains(MARKER), count.masked)
    };

    // By node id — the one that was covered.
    assert_eq!(masked(RedactionRule::node_id("at0011")), (false, 1));

    // By runtime name. The fixture's element is named "HIV status", and a
    // deployment withholding by name is withholding what a clinician sees.
    assert_eq!(masked(RedactionRule::name("HIV status")), (false, 1));

    // By archetype root: everything under the entry, not one element.
    assert_eq!(
        masked(RedactionRule::archetype_root(
            "openEHR-EHR-EVALUATION.problem.v1"
        )),
        (false, 1)
    );

    // And a rule that names nothing withholds nothing — the direction that
    // catches an inverted comparison, which would mask every element *except*
    // the one asked for.
    assert_eq!(masked(RedactionRule::node_id("at9999")), (true, 0));
    assert_eq!(masked(RedactionRule::name("Blood pressure")), (true, 0));
    assert_eq!(
        masked(RedactionRule::archetype_root("openEHR-EHR-OBSERVATION.other.v1")),
        (true, 0)
    );
}

/// The count says how much, and the rules are the ones that were given.
#[test]
fn a_redaction_count_reports_numbers_and_the_rules_are_kept() {
    let redactor = Redactor::new()
        .with_rule(RedactionRule::node_id("at0011"))
        .with_rule(RedactionRule::name("something else"));
    assert_eq!(redactor.rules().len(), 2, "a rule was dropped");

    let (_, count) = redactor
        .redact_counting(&composition_containing(MARKER))
        .unwrap();
    assert_eq!(count.masked, 1);
    assert!(count.examined >= count.masked);

    // `Display` could render nothing at all: a report saying how much was
    // withheld is the point of counting, and an empty one reads as "none".
    let shown = count.to_string();
    assert!(shown.contains('1'), "no number in {shown:?}");
    assert!(!shown.contains("HIV"), "a count must not name what it withheld");
}

/// Redaction tells an ELEMENT from a CLUSTER by **shape**, not by `_type`.
///
/// This crate does not emit `_type` on an `ELEMENT` — measured, not assumed —
/// so the structural fallback in `is_element` is not a corner case for foreign
/// documents. It is the path every composition here takes.
///
/// Its three negative conditions were untested: a node with `items`, `rows` or
/// `content` is a container and not a leaf. Deleting any of them lets a
/// `CLUSTER` be treated as an element, which for a redactor means masking a
/// whole branch as though it were one value — or counting it as examined when
/// nothing looked inside (`lib:A-09`).
#[test]
fn redaction_distinguishes_a_leaf_from_a_branch_by_shape() {
    use openehr::rm::data_structures::{Cluster, Item, ItemTree};

    let leaf = |name: &str, node: &str, text: &str| {
        Item::Element(Element::new(
            at(name, node),
            DataValue::Text(DvText::new(text).unwrap()),
        ))
    };
    // A tree holding one element and one cluster of two elements: three leaves
    // and two branches.
    let tree = ItemTree::new(
        at("tree", "at0001"),
        vec![
            leaf("Top", "at0020", "top"),
            Item::Cluster(
                Cluster::new(
                    at("Group", "at0021"),
                    vec![leaf("Inner A", "at0022", "a"), leaf("Inner B", "at0023", "b")],
                )
                .unwrap(),
            ),
        ],
    );
    let evaluation = Evaluation::new(
        at("Problem", "openEHR-EHR-EVALUATION.problem.v1").with_archetype_details(
            openehr::rm::common::Archetyped::new("openEHR-EHR-EVALUATION.problem.v1", "1.1.0")
                .unwrap(),
        ),
        EntryAttrs::about_subject(
            CodePhrase::new("ISO_639-1", "en").unwrap(),
            CodePhrase::new("IANA_character-sets", "UTF-8").unwrap(),
        ),
        tree.into(),
    );
    let composition = Composition::new(
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
    .with_content(evaluation.into());

    // Three leaves, and only three. A cluster counted as examined means the
    // shape test let a branch through.
    let (_, count) = Redactor::new()
        .with_rule(RedactionRule::node_id("at9999"))
        .redact_counting(&composition)
        .unwrap();
    assert_eq!(count.examined, 3, "a branch was counted as a leaf");
    assert_eq!(count.masked, 0);

    // And a rule naming the cluster masks nothing: it is not an element, so
    // there is no value to withhold and no branch to flatten.
    let (redacted, count) = Redactor::new()
        .with_rule(RedactionRule::node_id("at0021"))
        .redact_counting(&composition)
        .unwrap();
    assert_eq!(count.masked, 0, "a cluster was masked as though it were a value");
    let json = serde_json::to_string(&redacted).unwrap();
    assert!(json.contains('a') && json.contains('b'), "the branch survived");

    // A rule naming a leaf inside the cluster masks exactly that one.
    let (_, count) = Redactor::new()
        .with_rule(RedactionRule::node_id("at0022"))
        .redact_counting(&composition)
        .unwrap();
    assert_eq!(count.masked, 1);
}

/// The shape test is reached for an `ITEM_SINGLE`, whose element is a bare
/// field and carries no `_type`.
///
/// `is_element` checks `_type` first and falls back to shape. Inside an
/// `Item` enum an element is tagged, so the fallback never runs; as
/// `ITEM_SINGLE`'s `item` field it is untagged, and the fallback is the only
/// thing that recognises it.
///
/// The condition tested here is `value` **or** `null_flavour`: an element
/// carrying a value and no null flavour is still an element. Turning that into
/// `and` makes redaction stop recognising ordinary values — it would withhold
/// nothing and report nothing, which is the worst failure a redactor has
/// (`lib:A-09`, `X11.24`).
#[test]
fn an_untagged_element_is_still_recognised_and_withheld() {
    use openehr::rm::data_structures::ItemSingle;

    let composition = Composition::new(
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
    .with_content(
        Evaluation::new(
            at("Problem", "openEHR-EHR-EVALUATION.problem.v1").with_archetype_details(
                openehr::rm::common::Archetyped::new("openEHR-EHR-EVALUATION.problem.v1", "1.1.0")
                    .unwrap(),
            ),
            EntryAttrs::about_subject(
                CodePhrase::new("ISO_639-1", "en").unwrap(),
                CodePhrase::new("IANA_character-sets", "UTF-8").unwrap(),
            ),
            ItemSingle::new(
                at("single", "at0030"),
                Element::new(
                    at("HIV status", "at0031"),
                    DataValue::Text(DvText::new(MARKER).unwrap()),
                ),
            )
            .into(),
        )
        .into(),
    );

    // Untagged in the JSON, so only the shape test can find it.
    let raw = serde_json::to_string(&composition).unwrap();
    assert!(raw.contains(MARKER));
    assert!(
        !raw.contains(r#""at0031","_type":"ELEMENT""#),
        "the element is expected to be untagged here"
    );

    let (redacted, count) = Redactor::new()
        .with_rule(RedactionRule::node_id("at0031"))
        .redact_counting(&composition)
        .unwrap();
    assert_eq!(count.masked, 1, "an untagged element was not recognised");
    assert!(!serde_json::to_string(&redacted).unwrap().contains(MARKER));
}

