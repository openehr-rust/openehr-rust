//! Response shapes.
//!
//! Separate from the controllers so that what goes over the wire is visible in
//! one place. A response assembled inline in a handler is one nobody reviews.

use serde::Serialize;

/// A page of results.
///
/// `total` is the count *before* paging, which is what a caller needs to know
/// whether to ask again. Omitting it and returning a short page would be
/// indistinguishable from having reached the end.
#[derive(Debug, Serialize)]
pub struct Page<T> {
    /// The results for this page.
    pub items: Vec<T>,
    /// How many matched in total, before `_count` and `_offset`.
    pub total: usize,
    /// The offset this page started at.
    pub offset: usize,
    /// The maximum this page could hold.
    pub count: usize,
}

/// What the service is and what it does **not** do.
///
/// The absences are the useful part. A caller reading this should not have to
/// discover by experiment that erasure and read auditing are absent.
#[derive(Debug, Serialize)]
pub struct Metadata {
    /// The service's version.
    pub version: &'static str,
    /// The openEHR Reference Model release this speaks.
    pub rm_version: &'static str,
    /// The storage engine behind it.
    pub engine: &'static str,
    /// The schema version the database is installed under.
    pub schema_version: i64,
    /// Capabilities this service does **not** provide, and why.
    pub not_implemented: Vec<Absence>,
}

/// One thing the service does not do, named rather than left to be discovered.
#[derive(Debug, Serialize)]
pub struct Absence {
    /// What is missing.
    pub capability: &'static str,
    /// The requirement that records the exclusion.
    pub spec_ref: &'static str,
}

/// One version, as returned over HTTP.
///
/// Carries the chain digest so a caller can compare it against a checkpoint
/// held elsewhere. It carries no key material and no tag: an HMAC tag is
/// verifiable only by a holder of the key, and shipping it to every reader
/// would publish the thing the key exists to protect.
#[derive(Debug, Serialize)]
pub struct VersionView {
    /// Full `OBJECT_VERSION_ID`.
    pub uid: String,
    /// The container this version belongs to.
    pub versioned_object_uid: String,
    /// openEHR lifecycle-state code.
    pub lifecycle_state_code: String,
    /// Whether this version marks a logical deletion.
    pub is_deleted: bool,
    /// When it was committed, in its exact lexical form (`db:M3.25`).
    pub time_committed: String,
    /// This entry's chain digest, lower-case hex.
    pub chain_digest: String,
    /// The version's content, or `null` for a deletion.
    pub data: Option<serde_json::Value>,
}
