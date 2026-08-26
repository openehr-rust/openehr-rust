#![no_main]
//! Projection: a `COMPOSITION` that arrived as JSON, turned into rows.
//!
//! Drives `openehr_store::conformance::check_projection` (`W0.26`).

#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use openehr::rm::ehr::Composition;

fuzz_target!(|data: &[u8]| {
    let Ok(composition) = serde_json::from_slice::<Composition>(data) else {
        return;
    };
    // The result is deliberately ignored: a composition with no
    // `archetype_details` legitimately does not project, and refusing it is the
    // behaviour `CompositionIndexRow::project` documents. What the return value
    // is *for* is `db:D-10` — it makes the property observable to a test, which
    // is what stops it being deletable in silence.
    let _projected = openehr_store::conformance::check_projection(
        "8849182c-82ad-4088-a07f-48ead4180515::ehr1.example.org::1",
        openehr_store::conformance::RECORD,
        &composition,
    );
});
