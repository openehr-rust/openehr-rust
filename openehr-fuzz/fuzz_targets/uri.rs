#![no_main]
//! `DV_URI` and `DV_EHR_URI`, through **both** gates.
//!
//! This target exists because of `lib:A-36`, and it was written before the fix
//! rather than after: it finds the defect from an empty corpus in seconds.
//!
//! The two gates are different code (`lib:L10.1`, `lib:L10.1a`) and the whole
//! finding was that only one of them was checked, so this drives both and then
//! checks that they **agree**:
//!
//! 1. **Parsing is total** — `str::parse` returns `Ok` or `Err`, never panics.
//! 2. **Reading is total** (`lib:D3.30a`). `scheme()` and `rest()` are called
//!    on every value that exists, however it was built. They used to `expect`
//!    a colon "the constructor guarantees", which a deserialized value does
//!    not have; that is the panic.
//! 3. **The gates agree.** A value that `DvUri::new` accepts must validate
//!    clean, and a value it rejects must produce a violation. If those ever
//!    disagree, one of them is enforcing a rule the other does not, and which
//!    rule a document is held to depends on how it arrived.
//! 4. **A `DV_EHR_URI` never carries a foreign scheme without saying so**
//!    (`lib:D3.31`). Whatever route it took in, either its scheme is `ehr` or
//!    validation reports `Scheme_valid`.

use libfuzzer_sys::fuzz_target;
use openehr::rm::data_types::{DataValue, DvEhrUri, DvUri};
use openehr::validation::Validate;

/// Does the report name this class and invariant anywhere?
fn reports(value: DataValue, class: &str, invariant: &str) -> bool {
    value
        .validate()
        .violations()
        .iter()
        .any(|v| v.class == class && v.invariant == invariant)
}

fuzz_target!(|text: &str| {
    // Gate one: the constructor.
    let constructed = DvUri::new(text).ok();
    if let Some(ref uri) = constructed {
        // Total, and for a constructed value also *correct*: there is a colon.
        assert!(
            !uri.scheme().is_empty(),
            "a constructed DV_URI must have a non-empty scheme"
        );
        assert_eq!(
            format!("{}:{}", uri.scheme(), uri.rest()),
            uri.value(),
            "scheme and rest must reassemble the value"
        );
    }

    // Gate two: the same text arriving as JSON, which runs no constructor.
    let json = serde_json::json!({ "value": text }).to_string();
    let Ok(deserialized) = serde_json::from_str::<DvUri>(&json) else {
        // A `&str` is always a valid JSON string, so this cannot happen — but
        // asserting it would be an assertion about serde rather than about
        // this crate, so the target simply stops.
        return;
    };

    // (2) Reading is total. No `unwrap`, no guard: this is the line that
    // panicked before `A-36`.
    let scheme = deserialized.scheme().to_owned();
    let rest = deserialized.rest().to_owned();
    assert_eq!(deserialized.value(), text, "Deserialize must be lossless");

    // (3) The gates agree, in both directions.
    let clean = DataValue::Uri(deserialized.clone()).validate().is_empty();
    assert_eq!(
        constructed.is_some(),
        clean,
        "the constructor and validation disagree about `{text:?}`: \
         constructor accepted = {}, validation clean = {clean}",
        constructed.is_some(),
    );

    // A value with no colon has no scheme and no rest. Stated as an assertion
    // rather than left implicit, because "" is the answer that makes a caller
    // dispatching on the scheme fail closed.
    if !text.contains(':') {
        assert!(scheme.is_empty() && rest.is_empty());
    }

    // (4) The EHR URI, by both routes.
    let ehr_constructed = DvEhrUri::new(text).is_ok();
    if let Ok(ehr) = serde_json::from_str::<DvEhrUri>(&json) {
        let is_ehr_scheme = ehr.scheme() == DvEhrUri::SCHEME;
        assert_eq!(
            ehr_constructed,
            is_ehr_scheme && clean,
            "DvEhrUri::new and the deserialized value disagree about `{text:?}`"
        );
        if !is_ehr_scheme {
            assert!(
                reports(DataValue::EhrUri(ehr), "DV_EHR_URI", "Scheme_valid"),
                "a DV_EHR_URI whose scheme is not `ehr` must be reported: `{text:?}`"
            );
        }
    }
});
