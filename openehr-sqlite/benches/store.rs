//! Benchmarks for a commit and a read against a real database connection.
//!
//! # What a number here is, and is not
//!
//! **Not a conformance claim** (`W0.3`) and **not a CI gate**. See
//! `openehr/benches/rm.rs` for the reasoning; it applies unchanged. CI runs
//! this with `--test`, one iteration, so a benchmark cannot rot unnoticed.
//!
//! # Why these two, and why here rather than `openehr-store`
//!
//! `openehr-store/benches/store.rs` measures projection and chain
//! verification specifically *because* they are the only parts of a commit
//! that are that crate's own cost rather than the database's — it has no
//! connection to round-trip through, so anything claiming to measure a commit
//! there would not be measuring a commit. `openehr-sqlite` is the one crate
//! with a real connection behind [`Store`], so it is where "how long does a
//! commit actually take" can be answered honestly.
//!
//! # What this does not measure
//!
//! [`SqliteStore::in_memory`] — the connection every other test and benchmark
//! in this crate uses — not a file on disk. That is a deliberate, narrower
//! claim: this is the cost of validation, projection, the commit rules, and
//! `SQLite`'s own transactional bookkeeping, with page-cache-speed I/O rather
//! than a real `fsync`. A number that included disk latency would depend on
//! the filesystem and hardware of whoever ran it far more than on anything
//! this crate controls, and would answer a question this benchmark does not
//! ask.

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use openehr_sqlite::SqliteStore;
use openehr_store::{Store, conformance};
use std::hint::black_box;

/// The committer every benchmark here uses.
const CONTRIBUTOR: &str = "22222222-3333-4444-5555-666666666666";

/// A fresh, installed store with one EHR and one declared contribution —
/// everything a commit needs except the version itself.
fn ready_store() -> SqliteStore {
    let mut store = SqliteStore::in_memory().expect("open");
    store.install().expect("install");
    let ehr = conformance::sample_ehr();
    store.create_ehr(&ehr).expect("ehr");
    store
        .create_contribution(
            ehr.ehr_id(),
            &conformance::sample_contribution(CONTRIBUTOR, &[1]),
        )
        .expect("contribution");
    store
}

/// The one commit `ready_store` is set up for.
fn commit(c: &mut Criterion) {
    c.bench_function("commit/composition", |b| {
        // Fresh store per iteration, unmeasured: a second commit against the
        // same container would be a duplicate-version conflict, not a second
        // sample of the same cost. `iter_batched` is what separates that setup
        // from what is actually being timed.
        b.iter_batched(
            ready_store,
            |mut store| {
                let ehr_id = conformance::RECORD.parse().expect("literal");
                store
                    .commit_composition(
                        &ehr_id,
                        &conformance::sample_version(1, None, 5),
                        CONTRIBUTOR,
                    )
                    .expect("commit")
            },
            BatchSize::SmallInput,
        );
    });
}

/// `get_version` and `latest_version`, against a container already holding
/// one committed version — set up once, read many times, since neither
/// mutates anything a repeat would invalidate.
fn read(c: &mut Criterion) {
    let mut store = ready_store();
    let ehr_id = conformance::RECORD.parse().expect("literal");
    store
        .commit_composition(&ehr_id, &conformance::sample_version(1, None, 5), CONTRIBUTOR)
        .expect("commit");
    let uid = format!("{}::{}::1", conformance::RECORD, conformance::SYSTEM)
        .parse()
        .expect("literal");

    let mut group = c.benchmark_group("read");
    group.bench_function("get_version", |b| {
        b.iter(|| store.get_version(black_box(&uid)).expect("read"));
    });
    group.bench_function("latest_version", |b| {
        b.iter(|| store.latest_version(black_box(&ehr_id)).expect("read"));
    });
    group.finish();
}

criterion_group!(benches, commit, read);
criterion_main!(benches);
