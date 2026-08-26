#![no_main]
//! The integrity check, over rows that may have been edited in the database.
//!
//! Drives `openehr_store::conformance::check_verify_versions` (`W0.26`).
//!
//! `VersionRow` derives `Deserialize`, so this reaches `verify_versions` with
//! rows no `Store` would ever have written — which is the point: the function's
//! subject is a row somebody changed.

#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use openehr_store::record::VersionRow;

fuzz_target!(|data: &[u8]| {
    let Ok(rows) = serde_json::from_slice::<Vec<VersionRow>>(data) else {
        return;
    };
    // Ignored for the reason `project.rs` gives: arbitrary rows rarely form a
    // history that verifies, so zero provocations is the common and correct
    // answer here (`db:D-10`).
    let _provoked = openehr_store::conformance::check_verify_versions(&rows);
});
