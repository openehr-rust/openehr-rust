//! openEHR persistence for **`SQLite` 3**, with a complete embedded store.
//!
//! # Conformance level: **Store**
//!
//! Unlike the other four engine crates, this one contains a working
//! [`SqliteStore`] and runs [`openehr_store::conformance::run`] against a real
//! database in its own test suite. `SQLite` is the only one of the five that can
//! be verified without provisioning a server, so it is the one where the shared
//! logic is actually exercised — every commit rule, every read, every index.
//!
//! That matters beyond `SQLite`: the store logic lives in `openehr-store` and is
//! shared, so verifying it here verifies it for all five. What remains
//! unverified for the others is their driver glue and their DDL against a real
//! parser, which is exactly what `spec/conformance.md` says.
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

mod dialect;
mod store;

pub use dialect::SqliteDialect;
pub use store::SqliteStore;
