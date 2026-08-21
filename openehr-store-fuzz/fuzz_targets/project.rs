#![no_main]
//! Projection: a `COMPOSITION` that arrived as JSON, turned into rows.
//!
//! Drives `openehr_store::conformance::check_projection` (`W0.26`).

use libfuzzer_sys::fuzz_target;
use openehr::rm::ehr::Composition;

fuzz_target!(|data: &[u8]| {
    let Ok(composition) = serde_json::from_slice::<Composition>(data) else {
        return;
    };
    openehr_store::conformance::check_projection(
        "8849182c-82ad-4088-a07f-48ead4180515::ehr1.example.org::1",
        openehr_store::conformance::RECORD,
        &composition,
    );
});
