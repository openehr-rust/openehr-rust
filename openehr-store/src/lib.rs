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

pub mod conformance;
pub mod dialect;
pub mod error;
pub mod record;
pub mod schema;
pub mod store;

pub use dialect::{Dialect, Idempotence, ObjectKind, Placeholder, ddl_script};
pub use error::{Result, StoreError};
pub use record::{CompositionIndexRow, StoredInstant, VersionRow};
pub use schema::{ColTy, Column, Index, TABLES, Table};
pub use store::{CommitOutcome, Store};
