//! A repository abstraction for retrieving archetypes: `K15.24`–`K15.27`.
//!
//! # `K15.24`: the abstraction, not an implementation
//!
//! [`ArchetypeRepository`] is one method: resolve an archetype by identifier,
//! or answer why not. **`openehr` performs no network or filesystem I/O**
//! (`K15.25`) — no implementation of this trait lives in this crate, the same
//! way `openehr::rm` defines what a `COMPOSITION` is without deciding how one
//! reaches a database. A caller supplies an implementation backed by a CKM
//! client, a filesystem cache of published archetypes, or, in a test, a fixed
//! in-memory map. [`crate::am::validate::validate_with_repository`] is the
//! entry point that takes one.
//!
//! # `K15.26`: verified, and carrying its provenance
//!
//! [`Resolved`] pairs an [`Archetype`] with [`Provenance`] — source,
//! revision, retrieval time, content digest — or states plainly that
//! provenance could not be established
//! ([`Resolved::without_provenance`]). The caller who implements
//! [`ArchetypeRepository::resolve`] decides which one it can honestly build;
//! this crate does not fabricate a digest for retrieval it did not perform.
//! Using an unestablished-provenance result for validation needs the caller
//! to opt in explicitly (`RepositoryOptions::allow_unestablished_provenance`
//! in `validate.rs`), and the verdict records that it did.
//!
//! # `K15.27`: a retrieval failure is a refusal, never a pass
//!
//! [`RepositoryError`] enumerates the ways retrieval can fail — not found,
//! unreachable, a digest mismatch, an ambiguous revision — each naming what
//! happened. Nothing here falls back to "validate what could be reached":
//! [`crate::am::validate`] reports a retrieval failure as
//! [`crate::am::validate::Unchecked`], which keeps
//! [`crate::am::validate::ArchetypeReport::is_conformant`] `false`, exactly
//! as any other unchecked node does.

use crate::am::archetype::Archetype;
use crate::base::ArchetypeId;
use core::fmt;

/// Where a retrieved archetype came from, and enough to tell it is the one
/// asked for (`K15.26`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    source: String,
    revision: String,
    retrieved_at: String,
    content_digest: String,
}

impl Provenance {
    /// Builds a provenance record.
    ///
    /// `retrieved_at` is carried as the caller's own timestamp text, never
    /// generated here (`K15.23`): this crate does not read the wall clock,
    /// so the moment a retrieval actually happened can only come from
    /// whichever crate performed it.
    #[must_use]
    pub fn new(
        source: impl Into<String>,
        revision: impl Into<String>,
        retrieved_at: impl Into<String>,
        content_digest: impl Into<String>,
    ) -> Self {
        Self {
            source: source.into(),
            revision: revision.into(),
            retrieved_at: retrieved_at.into(),
            content_digest: content_digest.into(),
        }
    }

    /// Where the archetype was retrieved from — a CKM URL, a file path, a
    /// package name, whatever identifies the repository implementation used.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The revision retrieved, in whatever scheme the source uses.
    #[must_use]
    pub fn revision(&self) -> &str {
        &self.revision
    }

    /// When retrieval happened, as the caller recorded it.
    #[must_use]
    pub fn retrieved_at(&self) -> &str {
        &self.retrieved_at
    }

    /// A content digest of the bytes retrieved, so a later reader can tell
    /// whether a cached copy still matches what the source holds.
    #[must_use]
    pub fn content_digest(&self) -> &str {
        &self.content_digest
    }
}

/// An archetype together with where it came from, or a stated absence of
/// that (`K15.26`).
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    archetype: Archetype,
    provenance: Option<Provenance>,
}

impl Resolved {
    /// Builds a resolved artefact with established provenance.
    #[must_use]
    pub const fn new(archetype: Archetype, provenance: Provenance) -> Self {
        Self {
            archetype,
            provenance: Some(provenance),
        }
    }

    /// Builds a resolved artefact whose provenance could not be established
    /// — the repository has the archetype in hand but cannot honestly say
    /// where it came from or verify it. Using this for validation needs the
    /// caller to opt in explicitly (`K15.26`); fabricating a `Provenance` to
    /// avoid that opt-in would defeat the entire point of recording one.
    #[must_use]
    pub const fn without_provenance(archetype: Archetype) -> Self {
        Self {
            archetype,
            provenance: None,
        }
    }

    /// The archetype retrieved.
    #[must_use]
    pub const fn archetype(&self) -> &Archetype {
        &self.archetype
    }

    /// Its provenance, if established.
    #[must_use]
    pub const fn provenance(&self) -> Option<&Provenance> {
        self.provenance.as_ref()
    }
}

/// Why retrieval failed (`K15.27`): a refusal naming what happened, never a
/// silent absence.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RepositoryError {
    /// No artefact exists at this identifier.
    NotFound {
        /// The identifier requested.
        id: ArchetypeId,
    },
    /// The repository itself could not be reached — a network failure, a
    /// missing file, a closed connection.
    Unreachable {
        /// What went wrong, safe to state because it names the failure mode,
        /// never any content the archetype held.
        reason: &'static str,
    },
    /// The retrieved bytes did not match the digest the caller expected.
    DigestMismatch {
        /// The identifier requested.
        id: ArchetypeId,
    },
    /// More than one revision matched and none was requested specifically.
    AmbiguousRevision {
        /// The identifier requested.
        id: ArchetypeId,
    },
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { id } => write!(f, "no archetype found at {id}"),
            Self::Unreachable { reason } => write!(f, "repository unreachable: {reason}"),
            Self::DigestMismatch { id } => {
                write!(f, "content digest did not match for {id}")
            }
            Self::AmbiguousRevision { id } => {
                write!(f, "more than one revision matched {id}")
            }
        }
    }
}

impl core::error::Error for RepositoryError {}

/// A source of archetypes (`K15.24`).
///
/// See the module documentation: `openehr` implements no I/O, so no
/// implementation of this trait lives here.
pub trait ArchetypeRepository {
    /// Resolves an archetype by identifier.
    ///
    /// # Errors
    ///
    /// Returns [`RepositoryError`] naming what happened. An implementation
    /// MUST NOT return `Ok` for an identifier it cannot actually account
    /// for — [`Resolved::without_provenance`] exists for the case where the
    /// archetype is in hand but verification could not be attempted, so
    /// there is never a reason to invent a [`Provenance`] to get past this
    /// method's return type.
    fn resolve(&self, id: &ArchetypeId) -> Result<Resolved, RepositoryError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::am::{ArchetypeTerminology, CComplexObject, MultiplicityInterval, TermDefinition};
    use std::collections::BTreeMap;

    fn archetype(id: &str) -> Archetype {
        let mut terms = BTreeMap::new();
        terms.insert(
            "id1".to_owned(),
            TermDefinition::new("Concept", None).unwrap(),
        );
        Archetype::new(
            id.parse().unwrap(),
            CComplexObject::new(
                "EVALUATION",
                Some("id1".to_owned()),
                MultiplicityInterval::MANDATORY,
                Vec::new(),
            )
            .unwrap(),
            ArchetypeTerminology::new("en", terms).unwrap(),
        )
        .unwrap()
    }

    struct FixedRepository(Result<Resolved, RepositoryError>);

    impl ArchetypeRepository for FixedRepository {
        fn resolve(&self, _id: &ArchetypeId) -> Result<Resolved, RepositoryError> {
            self.0.clone()
        }
    }

    #[test]
    fn a_resolved_archetype_without_provenance_says_so() {
        let resolved = Resolved::without_provenance(archetype("openEHR-EHR-EVALUATION.test.v1"));
        assert!(resolved.provenance().is_none());
        assert_eq!(resolved.archetype().rm_type_name(), "EVALUATION");
    }

    #[test]
    fn a_resolved_archetype_with_provenance_carries_it() {
        let provenance = Provenance::new("ckm.openehr.org", "1.0.3", "2026-08-30T00:00:00Z", "abc123");
        let resolved = Resolved::new(archetype("openEHR-EHR-EVALUATION.test.v1"), provenance);
        assert_eq!(resolved.provenance().unwrap().source(), "ckm.openehr.org");
        assert_eq!(resolved.provenance().unwrap().revision(), "1.0.3");
    }

    #[test]
    fn each_failure_kind_names_what_happened() {
        let id: ArchetypeId = "openEHR-EHR-EVALUATION.test.v1".parse().unwrap();
        assert!(
            RepositoryError::NotFound { id: id.clone() }
                .to_string()
                .contains("no archetype found")
        );
        assert!(
            RepositoryError::Unreachable {
                reason: "connection refused"
            }
            .to_string()
            .contains("connection refused")
        );
        assert!(
            RepositoryError::DigestMismatch { id: id.clone() }
                .to_string()
                .contains("digest")
        );
        assert!(
            RepositoryError::AmbiguousRevision { id }
                .to_string()
                .contains("more than one revision")
        );
    }

    #[test]
    fn a_fixed_repository_can_stand_in_for_a_real_one_in_a_test() {
        let repo = FixedRepository(Err(RepositoryError::NotFound {
            id: "openEHR-EHR-EVALUATION.missing.v1".parse().unwrap(),
        }));
        let err = repo
            .resolve(&"openEHR-EHR-EVALUATION.missing.v1".parse().unwrap())
            .unwrap_err();
        assert!(matches!(err, RepositoryError::NotFound { .. }));
    }
}
