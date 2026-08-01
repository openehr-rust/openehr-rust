//! Security: access control, tamper evidence, and withholding content.
//!
//! openEHR's security story is deliberately incomplete — `EHR_ACCESS` names a
//! scheme and stops — because access control in health is jurisdictional. What
//! the standard *does* fix is the audit model: every change is a version,
//! every version carries an [`AuditDetails`](crate::rm::common::AuditDetails),
//! and versions are append-only.
//!
//! This module supplies what a library can supply on top of that, and is
//! explicit about the rest.
//!
//! | Module | Provides |
//! | --- | --- |
//! | [`access`] | `EHR_ACCESS`, a default-deny decision, a reference scheme, lossless carriage of others |
//! | [`audit_chain`] | a tamper-evident chain over committed versions, keyed or unkeyed |
//! | [`canonical`] | the deterministic byte form a digest is taken over |
//! | [`redact`] | masking content as `272｜masked｜`, and a wrapper that never logs |
//!
//! # The trust boundary
//!
//! | This crate guarantees | The deployment must provide |
//! | --- | --- |
//! | invariants hold on data it constructs | authentication |
//! | invariants are checked on data it receives ([`crate::validation`]) | a principal's group membership |
//! | a place to record who committed what, when, why | transport security |
//! | tamper evidence over the version history | key storage and rotation policy |
//! | content withheld as masked, not deleted | consent capture and withdrawal |
//! | no PHI in `Display`, and none in errors | log retention and shipping policy |
//!
//! Authentication is outside on purpose. **Recording who acted is not**: a
//! perimeter knows the identity and only the record knows which nodes were
//! touched, so neither can answer an access complaint alone.
//!
//! # What this module does not do
//!
//! - It does not verify `OpenPGP` signatures on
//!   [`Attestation`](crate::rm::common::Attestation). That needs a key ring and
//!   a trust policy belonging to the deployment, and a `proof` that is present
//!   is therefore **not** evidence of anything here.
//! - It does not encrypt. At-rest and in-transit encryption are deployment
//!   concerns, and a library that encrypted would be choosing a key management
//!   story on the deployment's behalf.
//! - It does not implement break-the-glass, consent registries, or
//!   legitimate-relationship models. It carries their settings unchanged and
//!   denies when it cannot evaluate them.

pub mod access;
pub mod audit_chain;
pub mod canonical;
pub mod redact;

pub use access::{
    AccessControlSettings, AccessRequest, Decision, DenyReason, EhrAccess, GroupSettings,
    OpaqueSettings, Operation,
};
pub use audit_chain::{BreakReason, Chain, ChainEntry, ChainKey, ChainStatus, Digest256, Tag};
pub use canonical::{to_canonical_bytes, to_canonical_string};
pub use redact::{RedactionCount, RedactionError, RedactionRule, Redactor, Sensitive};
