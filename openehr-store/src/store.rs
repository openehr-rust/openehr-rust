//! The store trait.
//!
//! # Every engine enforces the same commit rules
//!
//! [`Store::commit_composition`] is specified to refuse exactly what
//! `VersionedObject::commit` refuses — a version belonging to another container,
//! a duplicate, a missing or stale predecessor. Those are not database
//! constraints that happen to be convenient; they are the difference between a
//! history that connects and one that reads back and does not
//! (`V8.1`–`V8.5`).
//!
//! An engine is free to enforce them with a unique index instead of a query, and
//! [`crate::conformance`] does not care how — only that the refusal happens and
//! is distinguishable.

use crate::error::Result;
use crate::record::{CompositionIndexRow, VersionRow};
use openehr::base::{HierObjectId, ObjectVersionId};
use openehr::rm::common::{Contribution, Version};
use openehr::rm::data_types::DvDateTime;
use openehr::rm::ehr::{Composition, Ehr};

/// What a successful commit produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitOutcome {
    /// The version identifier now at the head of the container.
    pub version_uid: ObjectVersionId,
    /// Whether the container was created by this commit.
    pub created_container: bool,
}

/// A persistent openEHR repository.
///
/// # What this trait is not
///
/// It is not an `AQL` engine. Executing AQL means resolving archetype paths
/// against stored content, and neither this crate nor `openehr` implements
/// archetypes (`S1.4`, `S1.5`). What the schema offers instead is an index over
/// the attributes the Reference Model fixes, which is what an `AQL` `FROM` clause
/// filters on before it reaches into content.
pub trait Store {
    /// Which engine backs this store, for error messages.
    fn engine(&self) -> &'static str;

    /// Installs the schema.
    ///
    /// # Errors
    ///
    /// Returns [`crate::StoreError::Engine`] if the engine rejects the DDL.
    fn install(&mut self) -> Result<()>;

    /// Creates an EHR.
    ///
    /// # Errors
    ///
    /// Returns [`crate::StoreError::Conflict`] if the record already exists.
    fn create_ehr(&mut self, ehr: &Ehr) -> Result<()>;

    /// Reads an EHR.
    ///
    /// # Errors
    ///
    /// Returns [`crate::StoreError::NotFound`] if there is no such record.
    fn get_ehr(&self, ehr_id: &HierObjectId) -> Result<Ehr>;

    /// Records a contribution.
    ///
    /// # Errors
    ///
    /// Returns [`crate::StoreError::Conflict`] if it already exists.
    fn create_contribution(
        &mut self,
        ehr_id: &HierObjectId,
        contribution: &Contribution,
    ) -> Result<()>;

    /// Commits one version of a composition.
    ///
    /// Validates before writing: a store that accepted an invalid composition
    /// would make every later reader's `validate()` fail on data it cannot fix.
    ///
    /// # Errors
    ///
    /// - [`crate::StoreError::Invalid`] if the composition breaks a Reference
    ///   Model invariant.
    /// - [`crate::StoreError::Commit`] if the version does not belong at the
    ///   head of its container — see the module header.
    /// - [`crate::StoreError::NotFound`] if the EHR does not exist.
    fn commit_composition(
        &mut self,
        ehr_id: &HierObjectId,
        version: &Version<Composition>,
        contribution_uid: &str,
    ) -> Result<CommitOutcome>;

    /// Reads one version by identifier.
    ///
    /// # Errors
    ///
    /// Returns [`crate::StoreError::NotFound`] if there is no such version.
    fn get_version(&self, uid: &ObjectVersionId) -> Result<VersionRow>;

    /// The latest version of a container.
    ///
    /// # Errors
    ///
    /// Returns [`crate::StoreError::NotFound`] if the container has no
    /// versions.
    fn latest_version(&self, versioned_object_uid: &HierObjectId) -> Result<VersionRow>;

    /// The version current at a given time.
    ///
    /// Ordering uses the derived UTC column, so a version whose commit time is
    /// not an established instant is **skipped**, exactly as
    /// `VersionedObject::version_at_time` skips it (`V8.6`). A store that
    /// ordered on the lexical form would sort `2026-07-31T09:00:00+02:00` after
    /// `2026-07-31T08:30:00Z`, which is the wrong way round.
    ///
    /// # Errors
    ///
    /// Returns [`crate::StoreError::NotFound`] if no version was current then.
    fn version_at_time(
        &self,
        versioned_object_uid: &HierObjectId,
        at: &DvDateTime,
    ) -> Result<VersionRow>;

    /// Every version of a container, **oldest first**.
    ///
    /// Oldest-first because that is the order `REVISION_HISTORY` requires
    /// (`V8.7a`) — openEHR contradicts itself about this in prose, and the
    /// `most_recent_version` postcondition settles it.
    ///
    /// # Errors
    ///
    /// Returns [`crate::StoreError::Engine`] on an engine failure.
    fn all_versions(&self, versioned_object_uid: &HierObjectId) -> Result<Vec<VersionRow>>;

    /// A checkpoint over a container's chain, for an external witness.
    ///
    /// The chain of `M3.16` links each version to its predecessor, so altering
    /// or removing a version in the *middle* invalidates everything after it.
    /// It cannot detect **truncation**: delete the newest version and what
    /// remains is a shorter chain that verifies perfectly.
    ///
    /// That is the gap this closes, and only if the checkpoint is published
    /// somewhere the database administrator does not control (`M3.16c`). A
    /// checkpoint stored beside the data it attests to is worth nothing — an
    /// attacker who can truncate the history can rewrite the checkpoint too.
    ///
    /// Carries a count, a head digest, and the last version's identifier, and
    /// **no clinical content**, so it is safe to ship to a log or a witness that
    /// must never hold patient data.
    ///
    /// # Errors
    ///
    /// Returns [`crate::StoreError::Engine`] on an engine failure.
    fn chain_checkpoint(&self, versioned_object_uid: &HierObjectId) -> Result<String>;

    /// Compositions in a record whose archetype matches.
    ///
    /// The one query the index exists for: `AQL`'s
    /// `CONTAINS COMPOSITION c[`openEHR-EHR-COMPOSITION.encounter.v1`]`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::StoreError::Engine`] on an engine failure.
    fn find_compositions_by_archetype(
        &self,
        ehr_id: &HierObjectId,
        archetype_id: &str,
    ) -> Result<Vec<CompositionIndexRow>>;
}
