#![no_main]
//! Drives `check_col_sql` with arbitrary column types and bounds.

#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use openehr_store::ColTy;

#[derive(arbitrary::Arbitrary, Debug)]
enum Ty {
    Id(u16),
    Text(u16),
    LongText,
    Json,
    Instant,
    InstantUtc,
    Int,
    Bool,
}

fuzz_target!(|ty: Ty| {
    let ty = match ty {
        Ty::Id(n) => ColTy::Id(n),
        Ty::Text(n) => ColTy::Text(n),
        Ty::LongText => ColTy::LongText,
        Ty::Json => ColTy::Json,
        Ty::Instant => ColTy::Instant,
        Ty::InstantUtc => ColTy::InstantUtc,
        Ty::Int => ColTy::Int,
        Ty::Bool => ColTy::Bool,
    };
    openehr_store::conformance::check_col_sql(&openehr_oracle::OracleDialect, ty);
});
