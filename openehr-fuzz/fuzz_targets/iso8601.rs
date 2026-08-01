#![no_main]
//! ISO 8601 parsing: `Date`, `Time`, `DateTime`, `Duration`.
//!
//! Two properties, and the second is the one that matters:
//!
//! 1. **Totality.** Parsing arbitrary text returns `Ok` or `Err`, never panics.
//! 2. **Lexical fidelity** (`lib:D3.10`). If a value parses, its `as_str` is
//!    the *input* — byte for byte, not a normalised form. openEHR times carry
//!    deliberate partial precision, so `2024-05` must not come back as
//!    `2024-05-01`, and a parser that normalises has destroyed a clinical
//!    distinction before storage ever sees it.

use libfuzzer_sys::fuzz_target;
use openehr::base::iso8601::{Date, DateTime, Duration, Time};

fuzz_target!(|text: &str| {
    if let Ok(v) = text.parse::<Date>() {
        assert_eq!(v.as_str(), text, "Date did not preserve its lexical form");
    }
    if let Ok(v) = text.parse::<Time>() {
        assert_eq!(v.as_str(), text, "Time did not preserve its lexical form");
    }
    if let Ok(v) = text.parse::<DateTime>() {
        assert_eq!(
            v.as_str(),
            text,
            "DateTime did not preserve its lexical form"
        );
        // Comparison is partial and must never panic, including against itself.
        let _ = v.diff_seconds(&v);
    }
    if let Ok(v) = text.parse::<Duration>() {
        assert_eq!(
            v.as_str(),
            text,
            "Duration did not preserve its lexical form"
        );
    }
});
