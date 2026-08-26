//! Engine-agnostic openEHR persistence.
//!
//! This crate holds everything about storing openEHR that is **not** specific
//! to a SQL engine: the storage model, the projection from openEHR objects onto
//! rows, the commit rules, and a conformance suite every engine runs.
//!
//! The six engine crates — `openehr-postgresql`, `openehr-sqlite`,
//! `openehr-mysql`, `openehr-mariadb`, `openehr-mssql`, `openehr-oracle` — each
//! supply one [`Dialect`] and, where they have a driver, one [`Store`].
//!
//! # Trademarks
//!
//! openEHR® is the registered trademark of the openEHR Foundation. Use of
//! the trademark does not constitute endorsement of this product by openEHR
//! International or openEHR Foundation.
//!
//! # Why this crate exists at all
//!
//! Because the alternative is documented, in this repository, as a failure. The
//! sibling FHIR monorepo has six ports each carrying a byte-identical copy of
//! one core, a shell script written to police the copies, and an audit finding
//! for the copy that drifted anyway — an Oracle DDL emitter quietly producing
//! `MySQL` types. One crate the six depend on cannot drift from itself, and
//! [`conformance::dialects_are_distinct`] makes the specific failure that
//! finding describes detectable.
//!
//! "Detectable" rather than "impossible", because this repository reproduced
//! that exact failure anyway: `openehr-mariadb` was a name-substituted copy of
//! `openehr-mysql` emitting byte-identical DDL, and the comparison did not catch
//! it because the comparison listed five dialects and that was the sixth. A
//! guard is only as wide as its inputs. See `spec/audit.md` **W-01**.
//!
//! # The shape of the storage model
//!
//! openEHR is archetype-driven: a `COMPOSITION` contains whatever its archetype
//! says, and archetypes are authored long after the software ships. So the
//! canonical JSON **is** the record, and the relational part indexes only the
//! attributes the Reference Model itself fixes. See [`schema`] for the full
//! argument, including why every stored instant occupies two columns.
//!
//! # Verifying what came back
//!
//! [`integrity::verify_versions`] checks a container's history against what is
//! stored. It exists because verifying the chain is not enough: a chain entry
//! holds the *digest* of the content and never the content, so a row whose
//! document was edited and whose chain columns were left alone verifies
//! perfectly. Recomputing the content digest from the stored bytes is the one
//! check that needs a store, and it is why `M3.43` requires a column that
//! returns the bytes it was given.

#![forbid(unsafe_code)]

pub mod conformance;
pub mod dialect;
pub mod error;
pub mod integrity;
pub mod record;
pub mod schema;
pub mod store;

pub use dialect::{Dialect, Idempotence, ObjectKind, Placeholder, ddl_script};
pub use error::{Result, StoreError};
pub use integrity::{Breach, Integrity, verify_versions};
pub use record::{CompositionIndexRow, StoredInstant, VersionRow};
pub use schema::{ColTy, Column, Index, SCHEMA_VERSION, SCHEMA_VERSION_TABLE, TABLES, Table};
pub use store::{CommitOutcome, Store};
