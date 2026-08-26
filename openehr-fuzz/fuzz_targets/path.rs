#![no_main]
//! openEHR path parsing and navigation.
//!
//! Driven against a real composition, because a path parser that never
//! navigates anything is only half tested. The composition is the conformance
//! suite's sample, so this target exercises the same shape the store indexes.
//!
//! The property is refusal rather than a guess: an ambiguous path — one
//! matching several nodes — must fail rather than return the first
//! (`lib:Q12.x`). That is checked by the crate's own tests; here the obligation
//! is totality, since a panic while resolving a caller-supplied path is
//! reachable by anyone who can ask a question.

#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use openehr::path::Pathable;
use openehr::rm::common::{Archetyped, LocatableAttrs, PartyIdentified};
use openehr::rm::data_types::CodePhrase;
use openehr::rm::ehr::Composition;
use openehr::terminology::composition_category;
use std::sync::OnceLock;

fn subject() -> &'static Composition {
    static SUBJECT: OnceLock<Composition> = OnceLock::new();
    SUBJECT.get_or_init(|| {
        Composition::new(
            LocatableAttrs::named("Encounter", "openEHR-EHR-COMPOSITION.encounter.v1")
                .expect("literal")
                .with_archetype_details(
                    Archetyped::new("openEHR-EHR-COMPOSITION.encounter.v1", "1.1.0")
                        .expect("literal"),
                ),
            composition_category::EVENT,
            PartyIdentified::named("Dr A Nurse").expect("literal").into(),
            CodePhrase::new("ISO_639-1", "en").expect("literal"),
            CodePhrase::new("ISO_3166-1", "GB").expect("literal"),
        )
        .expect("literal")
    })
}

fuzz_target!(|path: &str| {
    // Ok or Err, never a panic, for any path a caller can write.
    let _ = subject().item_at_path(path);
});
