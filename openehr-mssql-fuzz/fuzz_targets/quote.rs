#![no_main]
//! Drives `check_quote` with arbitrary identifiers.
//!
//! The property with a security consequence: an identifier that escapes its
//! own delimiter is SQL injection, and archetype ids reach a `WHERE` clause
//! from caller input (`P6.12`, `G2.20`).

#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|identifier: &str| {
    openehr_store::conformance::check_quote(&openehr_mssql::MssqlDialect, identifier);
});
