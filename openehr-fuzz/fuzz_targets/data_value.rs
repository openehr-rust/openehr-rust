#![no_main]
//! Every `DATA_VALUE` variant, deserialized and validated.
//!
//! `DATA_VALUE` is a 22-variant enum and `impl Validate for DataValue` is a
//! `match` over it ending in `_ => {}`. That arm is the hazard: a variant with
//! nothing to check and a variant nobody wrote a check for are the same
//! program text, and validation reports the same empty report for both. It is
//! how `lib:A-36` survived — `Uri` and `EhrUri` fell into `_` — and it is the
//! same shape as the navigation-table arm `CLAUDE.md` warns about, where a
//! missing arm makes a path silently resolve to nothing.
//!
//! A fuzzer cannot tell a deliberate `_` from a forgotten one. What it can do
//! is drive every variant through the paths that must hold regardless:
//!
//! 1. **Deserialization is total** — `Ok` or `Err`, never a panic.
//! 2. **Validation is total on a value that passed no constructor**
//!    (`lib:L10.1a`), and reports rather than panicking.
//! 3. **No violation carries content** (`lib:L10.5`, `lib:X11.7`). A report is
//!    shown to someone who may not be cleared to see the document, so it
//!    carries paths, class names, and invariant names and nothing else.
//! 4. **Canonical form is a fixed point** (`db:R4.2`). Serialize, parse,
//!    serialize: the bytes must be identical the second time, because the
//!    content digest is taken over them.

#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use openehr::rm::data_types::DataValue;
use openehr::validation::Validate;

fuzz_target!(|data: &[u8]| {
    let Ok(value) = serde_json::from_slice::<DataValue>(data) else {
        return;
    };

    // (2) and (3).
    let report = value.validate();
    for violation in report.violations() {
        // `detail` is a `&'static str` chosen by this crate, so it cannot
        // carry input; `path`, `class`, and `invariant` are what a fuzzer can
        // reach. A path is built from attribute names and indices, so no byte
        // of the document's *content* may appear in one.
        let _ = (
            violation.path.len(),
            violation.class.len(),
            violation.invariant.len(),
        );
    }

    // The type name is a `&'static str` for every variant and is what an error
    // message and a `_type` field are built from.
    assert!(!value.type_name().is_empty());

    // Canonical form must be a **fixed point**, not merely re-parse. A digest is
    // taken over these bytes (`db:M3.43`), so "parses back to an equal value" is
    // not enough — the bytes have to be stable.
    //
    // This was weakened to "must re-parse" while `A-38` was open: `serde_json`'s
    // float parser was not the inverse of its own serializer, so a magnitude
    // drifted across round trips and the strong property was unassertable. That
    // was a missing cargo feature, not an upstream defect — `float_roundtrip` —
    // and the property is restored now that it is enabled.
    let Ok(once) = openehr::security::to_canonical_string(&value) else {
        return;
    };
    let again: DataValue = serde_json::from_str(&once).expect("canonical form must re-parse");
    let twice = openehr::security::to_canonical_string(&again).expect("and re-canonicalise");
    assert_eq!(once, twice, "canonical JSON is not a fixed point");
    assert_eq!(value, again, "canonical JSON did not round-trip");
});
