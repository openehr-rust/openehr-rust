//! Benchmarks for the paths a document actually travels.
//!
//! # What a number here is, and is not
//!
//! **It is not a conformance claim** (`W0.3`). Nothing in
//! [`spec/`](../../spec/index.md) is stated in seconds, no requirement here has
//! a performance clause, and no crate's conformance level depends on any figure
//! this file produces. A benchmark measures one machine on one afternoon; the
//! rung a crate stands on is a claim about the present, and those are different
//! kinds of statement (`W0.10`).
//!
//! **It is not a CI gate.** Wall-clock on a shared runner varies by more than
//! most real regressions, so a threshold here would fail for reasons unrelated
//! to the change and be silenced — and a silenced check is worse than none. CI
//! runs these with `--test`: one iteration each, to prove the benchmarks still
//! *compile and run*, which is the part that rots. A benchmark nobody runs is a
//! claim rather than a check, exactly as `W0.27` says of a fuzz target.
//!
//! What it is for is answering "did that get slower?" on one machine, across two
//! commits, with the noise held still.
//!
//! # Why these paths
//!
//! Each is a place where a whole document goes through one function, so the cost
//! is proportional to what a service actually does per request:
//!
//! * **Deserialization** is the widest untrusted surface (`db:V9.8`) and is on
//!   the path of every write.
//! * **Validation** is gate two (`lib:L10.1a`), one full traversal, and a
//!   service that skips it has no invariant checking at all — so its cost is
//!   the price of the guarantee, and worth knowing.
//! * **Canonical JSON** is taken over by a content digest (`db:M3.43`), so it
//!   runs on every commit and every integrity check, twice per round trip.
//! * **Path resolution** and **AQL parsing** are the query surface.
//! * **ISO 8601** parsing is the smallest and hottest: openEHR times are
//!   lexically preserved (`lib:D3.10`), so every instant in a document is
//!   parsed as text rather than as an integer.

use criterion::{Criterion, criterion_group, criterion_main};
use openehr::aql::AqlQuery;
use openehr::base::iso8601::{Date, DateTime, Duration};
use openehr::path::Pathable;
use openehr::rm::common::{Archetyped, LocatableAttrs, PartyIdentified};
use openehr::rm::data_structures::{Element, History, ItemTree, PointEvent};
use openehr::rm::data_types::{CodePhrase, DataValue, DvDateTime, DvQuantity, DvText};
use openehr::rm::ehr::{Composition, EntryAttrs, EventContext, Observation};
use openehr::terminology::{composition_category, setting};
use openehr::validation::Validate;
use std::hint::black_box;

/// The blood-pressure composition from `examples/01_build_composition`.
///
/// The same document the tutorial prints, deliberately: a benchmark over a
/// fixture nobody else uses measures the fixture. This one is an `OBSERVATION`
/// with a `HISTORY`, an event with `state`, two quantities and an
/// `EVENT_CONTEXT` — the shape a real encounter has, rather than the smallest
/// thing that type-checks.
fn fixture() -> Composition {
    let at = |name: &str, node: &str| LocatableAttrs::named(name, node).expect("literal attrs");

    let systolic = Element::new(
        at("Systolic", "at0004"),
        DataValue::Quantity(
            DvQuantity::new(184.0, "mm[Hg]")
                .unwrap()
                .with_units_system(DvQuantity::UCUM)
                .with_units_display_name("mmHg")
                .with_precision(0)
                .unwrap(),
        ),
    );
    let diastolic = Element::new(
        at("Diastolic", "at0005"),
        DataValue::Quantity(
            DvQuantity::new(96.0, "mm[Hg]")
                .unwrap()
                .with_units_system(DvQuantity::UCUM)
                .with_precision(0)
                .unwrap(),
        ),
    );
    let position = ItemTree::new(
        at("state structure", "at0007"),
        vec![
            Element::new(
                at("Position", "at0008"),
                DataValue::Text(DvText::new("Sitting").unwrap()),
            )
            .into(),
        ],
    );
    let event = PointEvent::new(
        at("any event", "at0006"),
        DvDateTime::new("2026-07-31T09:15:00Z").unwrap(),
        ItemTree::new(
            at("blood pressure", "at0003"),
            vec![systolic.into(), diastolic.into()],
        )
        .into(),
    )
    .with_state(position.into());
    let history = History::new(
        at("Event Series", "at0001"),
        DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
        vec![event.into()],
        None,
    )
    .unwrap();
    let observation = Observation::new(
        at(
            "Blood pressure",
            "openEHR-EHR-OBSERVATION.blood_pressure.v2",
        )
        .with_archetype_details(
            Archetyped::new("openEHR-EHR-OBSERVATION.blood_pressure.v2", "1.1.0").unwrap(),
        ),
        EntryAttrs::about_subject(
            CodePhrase::new("ISO_639-1", "en").unwrap(),
            CodePhrase::new("IANA_character-sets", "UTF-8").unwrap(),
        ),
        history,
    );
    Composition::new(
        at("Encounter", "openEHR-EHR-COMPOSITION.encounter.v1").with_archetype_details(
            Archetyped::new("openEHR-EHR-COMPOSITION.encounter.v1", "1.1.0").unwrap(),
        ),
        composition_category::EVENT,
        PartyIdentified::named("Dr A Nurse").unwrap().into(),
        CodePhrase::new("ISO_639-1", "en").unwrap(),
        CodePhrase::new("ISO_3166-1", "GB").unwrap(),
    )
    .unwrap()
    .with_context(
        EventContext::new(
            DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
            setting::PRIMARY_MEDICAL_CARE,
        )
        .unwrap()
        .with_location("Consulting room 3")
        .unwrap(),
    )
    .unwrap()
    .with_content(observation.into())
}

fn document(c: &mut Criterion) {
    let composition = fixture();
    let canonical =
        openehr::security::to_canonical_string(&composition).expect("the fixture canonicalises");

    let mut group = c.benchmark_group("document");
    // Throughput is in bytes so that the three are comparable to each other and
    // stay comparable if the fixture grows.
    group.throughput(criterion::Throughput::Bytes(canonical.len() as u64));

    group.bench_function("deserialize", |b| {
        b.iter(|| {
            let parsed: Composition =
                serde_json::from_str(black_box(&canonical)).expect("round-trips");
            black_box(parsed)
        });
    });
    group.bench_function("canonical_json", |b| {
        b.iter(|| {
            openehr::security::to_canonical_string(black_box(&composition)).expect("canonicalises")
        });
    });
    // Gate two, on a value that came off a wire rather than out of a
    // constructor — which is the only case where validation can find anything.
    let received: Composition = serde_json::from_str(&canonical).expect("round-trips");
    group.bench_function("validate", |b| {
        b.iter(|| black_box(&received).validate());
    });
    group.finish();
}

fn query(c: &mut Criterion) {
    let composition = fixture();
    let path = "/content[openEHR-EHR-OBSERVATION.blood_pressure.v2]\
                /data/events/data/items[at0004]/value/magnitude";
    let aql = "SELECT c/uid/value FROM COMPOSITION c \
               WHERE c/archetype_node_id = 'openEHR-EHR-COMPOSITION.encounter.v1'";

    let mut group = c.benchmark_group("query");
    group.bench_function("resolve_path", |b| {
        b.iter(|| black_box(&composition).item_at_path(black_box(path)));
    });
    group.bench_function("parse_aql", |b| {
        b.iter(|| black_box(aql).parse::<AqlQuery>().expect("the literal parses"));
    });
    group.finish();
}

fn iso8601(c: &mut Criterion) {
    let mut group = c.benchmark_group("iso8601");
    // A full instant, a date known only to the month, and a duration. The
    // partial one is not an edge case here: openEHR carries deliberate partial
    // precision, so a real document is full of them.
    group.bench_function("date_time", |b| {
        b.iter(|| {
            black_box("2026-07-31T09:15:00Z")
                .parse::<DateTime>()
                .expect("parses")
        });
    });
    group.bench_function("date_to_the_month", |b| {
        b.iter(|| black_box("2026-07").parse::<Date>().expect("parses"));
    });
    group.bench_function("duration", |b| {
        b.iter(|| black_box("P1Y2M3DT4H5M6S").parse::<Duration>().expect("parses"));
    });
    group.finish();
}

criterion_group!(benches, document, query, iso8601);
criterion_main!(benches);
