#![no_main]
//! AQL lexing, parsing, and static checking.
//!
//! AQL arrives from callers and is the largest grammar in the crate, so it is
//! the likeliest place for a parser defect. The crate parses and statically
//! checks AQL and never executes it (`lib:S1.5`), which bounds what a defect
//! here can do — but not to nothing: a panic in a query parser is a denial of
//! service reachable by anyone who can submit a query.

use libfuzzer_sys::fuzz_target;
use openehr::aql::AqlQuery;

fuzz_target!(|text: &str| {
    let Ok(query) = text.parse::<AqlQuery>() else {
        return;
    };

    // Every accessor must be total on a query that parsed.
    let _ = query.archetype_ids();
    let _ = query.aliases();
    let _ = query.parameters();
    let _ = query.limit;

    // Static checking must not panic on anything the parser accepted; it
    // reports, or it is clean.
    let _ = query.check();

    // Normalisation must round-trip: rendering a parsed query and reparsing it
    // must yield a query that renders identically. A normaliser that is not
    // idempotent silently rewrites a caller's query into a different one.
    let rendered = query.to_string();
    if let Ok(again) = rendered.parse::<AqlQuery>() {
        assert_eq!(
            again.to_string(),
            rendered,
            "AQL normalisation is not idempotent"
        );
    }
});
