#![no_main]
//! Canonical-JSON deserialization — the widest untrusted surface there is.
//!
//! This is the target that matters most. A `COMPOSITION` arriving as JSON has
//! been checked by **nothing** until `validate()` runs: serde writes fields
//! directly and never calls a constructor, which is the "two gates, not one"
//! rule (`db:V9.8`). So this drives the gate.
//!
//! Properties:
//!
//! 1. **Totality.** Deserializing arbitrary bytes returns `Ok` or `Err`.
//! 2. **Validation is total.** `validate()` on *any* deserialized value —
//!    including one that never passed a constructor — must report, not panic.
//! 3. **Canonical form round-trips.** A value that deserialized must
//!    re-serialize and deserialize again to an equal value (`db:R4.2`).
//!
//! # On recursion depth
//!
//! `lib:S1.15` says the crate MUST NOT bound recursion on deserialization, and
//! that a caller reading untrusted documents has to. That is a deliberate,
//! documented limitation, not a defect — so this target does **not** treat deep
//! nesting as a finding. `serde_json` applies its own default recursion limit
//! to the input, which is what keeps this target testing parser logic instead
//! of rediscovering a stated design decision.

#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use openehr::rm::ehr::Composition;
use openehr::validation::Validate;

fuzz_target!(|data: &[u8]| {
    let Ok(value) = serde_json::from_slice::<Composition>(data) else {
        return;
    };

    // Gate two: this value never saw a constructor.
    let report = value.validate();
    let _ = report.is_empty();
    for violation in report.violations() {
        // A report must never carry a submitted value (`db:M3.38`).
        let _ = (violation.path.len(), violation.class, violation.invariant);
    }

    // Canonical form must be stable.
    let Ok(canonical) = openehr::security::to_canonical_string(&value) else {
        return;
    };
    let again: Composition =
        serde_json::from_str(&canonical).expect("canonical form must re-parse");
    // Canonical form must be a **fixed point**, not merely re-parse. A digest is
    // taken over these bytes (`db:M3.43`), so "parses back to an equal value" is
    // not enough — the bytes have to be stable.
    //
    // This was weakened to "must re-parse" while `A-38` was open: `serde_json`'s
    // float parser was not the inverse of its own serializer, so a magnitude
    // drifted across round trips and the strong property was unassertable. That
    // was a missing cargo feature, not an upstream defect — `float_roundtrip` —
    // and the property is restored now that it is enabled.
    let twice = openehr::security::to_canonical_string(&again).expect("and re-canonicalise");
    assert_eq!(canonical, twice, "canonical JSON is not a fixed point");
    assert_eq!(value, again, "canonical JSON did not round-trip");
});
