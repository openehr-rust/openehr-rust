//! openEHR persistence for **`SQLite` 3**, with a complete embedded store.
//!
//! # Trademarks
//!
//! openEHR® is a registered trademark of openEHR International (the openEHR
//! Foundation). This project is an independent implementation: it is not
//! affiliated with, endorsed by, or certified by openEHR International.
//!
//! # Conformance level: **Verified**
//!
//! Store level, re-checked in CI on every commit. `SQLite` is bundled and
//! compiled in, so the engine cannot be absent and the job cannot silently
//! skip — which is what the level requires (`C0.8`).
//!
//! Unlike the other five engine crates, this one contains a working
//! [`SqliteStore`] and runs [`openehr_store::conformance::run`] against a real
//! database in its own test suite. `SQLite` is the only one of the six that can
//! be verified without provisioning a server, so it is the one where the shared
//! logic is actually exercised — every commit rule, every read, every index.
//!
//! That matters beyond `SQLite`: the store logic lives in `openehr-store` and is
//! shared, so verifying it here verifies it for all six. What remains
//! unverified for the others is their driver glue and their DDL against a real
//! parser, which is exactly what `spec/conformance.md` says.
//!
//! This crate also hosts the cross-dialect comparison in `tests/dialects.rs`,
//! because that comparison needs to see every dialect and `openehr-store`
//! cannot — they depend on it.
//!
//! ```
//! use openehr_sqlite::SqliteStore;
//! use openehr_store::{Store, conformance};
//!
//! let mut store = SqliteStore::in_memory().unwrap();
//! store.install().unwrap();
//!
//! let ehr = conformance::sample_ehr();
//! store.create_ehr(&ehr).unwrap();
//! assert_eq!(
//!     store.get_ehr(ehr.ehr_id()).unwrap().ehr_id().to_string(),
//!     ehr.ehr_id().to_string()
//! );
//! ```

#![forbid(unsafe_code)]

mod dialect;
mod store;

pub use dialect::SqliteDialect;
pub use store::SqliteStore;

/// The driver this crate is built on, re-exported.
///
/// `libsqlite3-sys` declares `links = "sqlite3"`, so exactly one version of it
/// may exist in a dependency graph. A dependent that named `rusqlite` itself
/// would have to keep its version constraint in step with this crate's or fail
/// to resolve — a coupling that is invisible until it breaks, and that breaks
/// at the next bump rather than at the change that caused it.
///
/// Re-exporting removes the choice. A caller needing raw SQL — to verify a
/// backup, or to demonstrate that tampering is detected — reaches it through
/// here and cannot mismatch.
pub use rusqlite;
