//! Address nodes by openEHR path, and parse the AQL query that would fetch
//! them.
//!
//! ```sh
//! cargo run --example 03_paths_and_aql
//! ```
//!
//! The two are the same addressing scheme seen from two places: an AQL
//! `SELECT` column is an alias plus a path, and the path is what
//! [`Pathable`](openehr::path::Pathable) resolves against a composition in
//! memory. Parsing the query and resolving the path with one crate means the
//! two cannot drift.

use openehr::aql::AqlQuery;
use openehr::path::{Node, Pathable, Scalar};
use openehr::rm::common::{Archetyped, LocatableAttrs, PartyIdentified};
use openehr::rm::data_structures::{Element, History, ItemTree, PointEvent};
use openehr::rm::data_types::{CodePhrase, DataValue, DvDateTime, DvQuantity};
use openehr::rm::ehr::{Composition, EntryAttrs, Observation};
use openehr::terminology::composition_category;

const QUERY: &str = "
    SELECT
        o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude AS systolic,
        o/data[at0001]/events[at0006]/data[at0003]/items[at0005]/value/magnitude AS diastolic
    FROM EHR e[ehr_id/value=$ehrUid]
        CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1]
            CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.blood_pressure.v2]
    WHERE o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude >= 140
    ORDER BY c/context/start_time DESC
    LIMIT 5
";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let composition = blood_pressure()?;

    // ---- paths -----------------------------------------------------------
    let systolic = composition.item_at_path(
        "/content[openEHR-EHR-OBSERVATION.blood_pressure.v2]\
         /data/events[at0006]/data/items[at0004]/value/magnitude",
    )?;
    println!("systolic  = {}", scalar(&systolic));

    // The same node reached by runtime name rather than by node id. Templates
    // rename nodes; archetype node ids do not change. Both forms are useful and
    // they answer differently after a template revision.
    let diastolic =
        composition.item_at_path("/content/data/events/data/items['Diastolic']/value/magnitude")?;
    println!("diastolic = {}", scalar(&diastolic));

    // Drop the predicate and the path becomes ambiguous. openEHR asks
    // `path_exists` and `path_unique` separately, and so does this crate:
    // taking the first of two would silently return systolic for diastolic.
    let ambiguous = "/content/data/events/data/items/value/magnitude";
    println!(
        "\n{ambiguous}\n  exists: {}  unique: {}  matches: {}",
        composition.path_exists(ambiguous)?,
        composition.path_unique(ambiguous)?,
        composition.items_at_path(ambiguous)?.len(),
    );
    println!(
        "  item_at_path: {}",
        match composition.item_at_path(ambiguous) {
            Ok(_) => "resolved".to_owned(),
            Err(e) => e.to_string(),
        }
    );

    // ---- AQL -------------------------------------------------------------
    let query: AqlQuery = QUERY.parse()?;
    println!("\nAQL");
    println!("  archetypes : {:?}", query.archetype_ids());
    println!("  aliases    : {:?}", query.aliases());
    println!("  parameters : {:?}", query.parameters());
    println!("  limit      : {:?}", query.limit);

    // The static check that no runtime reports usefully: a path rooted at an
    // alias FROM does not bind. Such a query parses, executes, and returns
    // nothing at all.
    query.check()?;
    let typo: AqlQuery = "SELECT obs/value FROM COMPOSITION c CONTAINS OBSERVATION o".parse()?;
    println!(
        "  typo check : {}",
        match typo.check() {
            Ok(()) => "clean".to_owned(),
            Err(e) => e.reason,
        }
    );

    // This crate parses and checks AQL; it does not execute it. Executing
    // means resolving paths against a repository, and there is none here.
    println!("\nnormalised:\n  {query}");
    Ok(())
}

fn scalar(node: &Node<'_>) -> String {
    match node {
        Node::Scalar(s @ (Scalar::Number(_) | Scalar::Integer(_))) => s.to_display_string(),
        other => format!("<{}>", other.type_name()),
    }
}

fn blood_pressure() -> Result<Composition, Box<dyn std::error::Error>> {
    let at = |name: &str, node: &str| LocatableAttrs::named(name, node).expect("literal attrs");
    let quantity = |v: f64| -> Result<DataValue, Box<dyn std::error::Error>> {
        Ok(DataValue::Quantity(DvQuantity::new(v, "mm[Hg]")?))
    };

    let readings = ItemTree::new(
        at("blood pressure", "at0003"),
        vec![
            Element::new(at("Systolic", "at0004"), quantity(184.0)?).into(),
            Element::new(at("Diastolic", "at0005"), quantity(96.0)?).into(),
        ],
    );
    let history = History::new(
        at("Event Series", "at0001"),
        DvDateTime::new("2026-07-31T09:00:00Z")?,
        vec![
            PointEvent::new(
                at("any event", "at0006"),
                DvDateTime::new("2026-07-31T09:15:00Z")?,
                readings.into(),
            )
            .into(),
        ],
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
    Ok(Composition::new(
        at("Encounter", "openEHR-EHR-COMPOSITION.encounter.v1"),
        composition_category::EVENT,
        PartyIdentified::named("Dr A Nurse")?.into(),
        CodePhrase::new("ISO_639-1", "en")?,
        CodePhrase::new("ISO_3166-1", "GB")?,
    )?
    .with_content(observation.into()))
}
