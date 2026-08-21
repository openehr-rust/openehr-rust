//! Benchmarks for the persistence layer's two hot pure functions.
//!
//! # What a number here is, and is not
//!
//! **Not a conformance claim** (`W0.3`) and **not a CI gate**. See
//! `openehr/benches/rm.rs` for the reasoning; it applies unchanged. CI runs
//! these with `--test`, one iteration each, so that a benchmark cannot rot
//! unnoticed — the same standard `W0.27` sets for a fuzz target.
//!
//! # Why these two
//!
//! Projection and verification are the only parts of a commit that are *this
//! crate's* cost rather than the database's. Everything else in a write is a
//! round trip to a server, which no benchmark in this process can measure
//! honestly.
//!
//! `verify_versions` is measured at three lengths on purpose. It walks a chain,
//! and the interesting question is not how fast one row is but whether the walk
//! is linear — a check that quietly became quadratic would still pass every
//! test in the suite, and would first be noticed by whoever verified a record
//! with ten years of versions in it.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use openehr_store::conformance;
use openehr_store::integrity::verify_versions;
use openehr_store::record::{CompositionIndexRow, VersionRow};
use std::hint::black_box;

/// A chain of `n` versions, each hashed onto the one before.
fn history(n: u32) -> Vec<VersionRow> {
    let mut rows: Vec<VersionRow> = Vec::with_capacity(n as usize);
    for i in 1..=n {
        let previous = rows.last().map(|r: &VersionRow| r.chain.digest);
        let version = conformance::sample_version(i, (i > 1).then(|| i - 1), i % 60);
        rows.push(
            VersionRow::project(&version, "contribution-1", previous, None)
                .expect("the sample projects"),
        );
    }
    rows
}

fn project(c: &mut Criterion) {
    let composition = conformance::sample_composition("Encounter");
    let version = conformance::sample_version(1, None, 0);

    let mut group = c.benchmark_group("project");
    group.bench_function("composition_index", |b| {
        b.iter(|| {
            CompositionIndexRow::project(
                black_box("87284370-2D4B-4E3D-A3F3-F303D2F4F34B::ehr1.example.org::1"),
                black_box(conformance::RECORD),
                black_box(&composition),
            )
            .expect("the sample projects")
        });
    });
    // Not comparable to the row above: this one canonicalises the content and
    // takes a SHA-256 over it, which is most of what it costs. That is the
    // point — the digest is what makes the row tamper-evident, and its price
    // belongs where it can be seen.
    group.bench_function("version_row", |b| {
        b.iter(|| {
            VersionRow::project(black_box(&version), black_box("contribution-1"), None, None)
                .expect("the sample projects")
        });
    });
    group.finish();
}

fn integrity(c: &mut Criterion) {
    let mut group = c.benchmark_group("integrity");
    for n in [1_u32, 10, 100] {
        let rows = history(n);
        group.throughput(Throughput::Elements(u64::from(n)));
        group.bench_with_input(BenchmarkId::new("verify_versions", n), &rows, |b, rows| {
            b.iter(|| verify_versions(black_box(rows), black_box(&[])));
        });
    }
    group.finish();
}

criterion_group!(benches, project, integrity);
criterion_main!(benches);
