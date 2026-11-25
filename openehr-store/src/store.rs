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
//!
//! [`check_commit_rules`] is the check itself, factored out so a second
//! versioned class — [`Store::commit_ehr_status`], today — calls the same
//! function rather than re-deriving the same three-way match beside it. Two
//! copies of one rule are exactly the shape that let `openehr-mariadb`
//! diverge from `openehr-mysql` unnoticed (`W-01`), one level down from DDL
//! to logic.

use crate::error::{Result, StoreError};
use crate::record::{CompositionIndexRow, VersionRow};
use openehr::base::{HierObjectId, ObjectVersionId};
use openehr::rm::common::{CommitError, Contribution, Version};
use openehr::rm::data_types::DvDateTime;
use openehr::rm::ehr::{Composition, Ehr, EhrStatus};

/// `Ok` only when `ehr` owns the container and it holds `rm_type`; otherwise
/// the **same** `NotFound` an absent container gets, carrying `kind` and `id`
/// (`db:H5.18`). A different `EHR`, a different `rm_type` and no such
/// container are deliberately one outcome.
///
/// # Errors
///
/// `NotFound` as above, or any engine failure from the owner lookup.
pub fn require_owned<S: Store + ?Sized>(
    store: &S,
    ehr: &HierObjectId,
    container: &HierObjectId,
    rm_type: &str,
    kind: &'static str,
    id: &str,
) -> Result<()> {
    match store.versioned_object_owner(container) {
        Ok((owner, actual)) if owner.to_string() == ehr.to_string() && actual == rm_type => Ok(()),
        Ok(_) | Err(StoreError::NotFound { .. }) => Err(StoreError::NotFound {
            kind,
            id: id.to_owned(),
        }),
        Err(other) => Err(other),
    }
}

/// Selects the metadata needed to resolve a historical read (`db:H5.13`).
///
/// Bind the container UID and whole UTC second twice, in that order. Only the
/// query second and the latest occupied earlier second can contain the answer.
/// The latter matters when every commit in the query second is still in the future.
#[must_use]
pub fn version_at_time_candidates_sql(placeholder: crate::Placeholder) -> String {
    format!(
        "SELECT uid, audit_time_committed_text, trunk_version FROM openehr_version \
         WHERE versioned_object_uid = {} AND audit_time_committed_utc IN ({}, \
           (SELECT MAX(audit_time_committed_utc) FROM openehr_version \
            WHERE versioned_object_uid = {} AND audit_time_committed_utc < {}))",
        placeholder.render(1),
        placeholder.render(2),
        placeholder.render(3),
        placeholder.render(4)
    )
}

/// Resolves an inclusive cutoff over candidate (UID, commit time, trunk version)
/// rows from [`version_at_time_candidates_sql`]. Only the winning immutable
/// version needs to be fetched with its content.
///
/// # Errors
///
/// Returns a driver error or a parsing error for malformed stored metadata.
pub fn version_uid_at_time(
    at: &DvDateTime,
    candidates: impl IntoIterator<Item = Result<(String, String, i64)>>,
) -> Result<Option<ObjectVersionId>> {
    let mut candidates = candidates.into_iter();
    version_uid_at_time_iter(at, &mut candidates)
}

/// Non-generic implementation, so closure coverage reflects one source body
/// rather than one independently instrumented copy per driver iterator.
fn version_uid_at_time_iter(
    at: &DvDateTime,
    candidates: &mut dyn Iterator<Item = Result<(String, String, i64)>>,
) -> Result<Option<ObjectVersionId>> {
    use openehr::base::iso8601::DateTime;
    use std::cmp::Ordering;

    let mut latest: Option<(DateTime, i64, String)> = None;
    for candidate in candidates {
        let (uid, text, trunk) = candidate?;
        let time: DateTime = text.parse().map_err(|_| {
            openehr::ParseError::invariant("DV_DATE_TIME", "invalid stored commit time")
        })?;
        if !time.semantic_cmp(at.value()).is_some_and(Ordering::is_le) {
            continue;
        }
        let replaces = latest.as_ref().is_none_or(|(best_time, best_trunk, _)| {
            // Eligible instants at the same position can have different precision.
            time.semantic_cmp(best_time)
                .unwrap_or(Ordering::Equal)
                .then_with(|| trunk.cmp(best_trunk))
                .is_gt()
        });
        if replaces {
            latest = Some((time, trunk, uid));
        }
    }
    latest
        .map(|(_, _, uid)| {
            uid.parse().map_err(|_| {
                StoreError::Parse(openehr::ParseError::invariant(
                    "OBJECT_VERSION_ID",
                    "invalid stored version uid",
                ))
            })
        })
        .transpose()
}

/// What a successful commit produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitOutcome {
    /// The version identifier now at the head of the container.
    pub version_uid: ObjectVersionId,
    /// Whether the container was created by this commit.
    pub created_container: bool,
}

/// The commit rules every engine enforces, for every versioned class: refuse
/// a duplicate, a version belonging to another container, or one that does
/// not name the current head (`V8.1`–`V8.5`, see the module header).
///
/// Pure and connection-agnostic — `head` is the container's current head, if
/// any, as `(uid, trunk_version)`; `already_exists` is whether a version
/// with this exact uid is already stored; `preceding_version_uid` is what
/// the version being committed claims to follow. An engine resolves these
/// however it stores versions and then asks this one question.
///
/// # Errors
///
/// Returns [`StoreError::Commit`] naming which rule was broken.
pub fn check_commit_rules(
    head: Option<&(String, i64)>,
    already_exists: bool,
    preceding_version_uid: Option<&ObjectVersionId>,
) -> Result<()> {
    if already_exists {
        return Err(StoreError::Commit(CommitError::DuplicateVersion));
    }
    match (head, preceding_version_uid) {
        (None, None) => Ok(()),
        (None, Some(_)) | (Some(_), None) => {
            Err(StoreError::Commit(CommitError::PrecedingVersionMismatch))
        }
        (Some((latest, _)), Some(preceding)) => {
            if latest == &preceding.to_string() {
                Ok(())
            } else {
                Err(StoreError::Commit(CommitError::NotLatest))
            }
        }
    }
}

/// A persistent openEHR repository.
///
/// # What this trait is not
///
/// It is not an `AQL` engine, though this crate now has one:
/// [`crate::aql::execute`], one `EHR`, one archetype-predicated containment
/// level, over exactly the trait methods below (`db:S1.6`, amended
/// 2026-09-13, `P6.20`–`P6.24`). General AQL execution — an
/// archetype-authored `WHERE` clause over arbitrary nested structure —
/// still needs an archetype engine neither this crate nor `openehr`
/// implements (`lib:A-40`). What the schema offers a query beyond that
/// bounded subset is only an index over the attributes the Reference Model
/// fixes, which is what an `AQL` `FROM` clause filters on before it reaches
/// into content.
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

    /// Commits one version of an `EHR_STATUS`.
    ///
    /// The same commit rules as [`Store::commit_composition`]
    /// ([`check_commit_rules`]), against the same `openehr_version` table —
    /// openEHR versions every class the same way (module header). The
    /// caller names the container by the version's own `uid`, exactly as
    /// for a composition; `EHR.ehr_status` conventionally names it by the
    /// EHR's own `ehr_id` (`EHR.Ehr_status_valid`), and
    /// [`conformance::sample_ehr`](crate::conformance::sample_ehr) already
    /// builds one this way, but this method does not itself check the
    /// convention was followed. There is no per-`EHR_STATUS` index row to
    /// project — `openehr_composition_index` exists for archetype search,
    /// and an `EHR_STATUS` is not archetyped content (`M3.32`).
    ///
    /// # Errors
    ///
    /// - [`crate::StoreError::Invalid`] if the status breaks a Reference
    ///   Model invariant.
    /// - [`crate::StoreError::Commit`] if the version does not belong at the
    ///   head of its container — see the module header.
    /// - [`crate::StoreError::NotFound`] if the EHR does not exist.
    fn commit_ehr_status(
        &mut self,
        ehr_id: &HierObjectId,
        version: &Version<EhrStatus>,
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

    /// The `EHR` that owns a container, and the kind of content it holds
    /// (`COMPOSITION`, `EHR_STATUS`, …). The one primitive the scoped lookups
    /// and the commit-time ownership check are built on (`db:H5.18`,
    /// `db:H5.19`).
    ///
    /// # Errors
    ///
    /// Returns [`crate::StoreError::NotFound`] (`versioned_object`) if there is
    /// no such container, or [`crate::StoreError::Engine`] on an engine failure.
    fn versioned_object_owner(
        &self,
        versioned_object_uid: &HierObjectId,
    ) -> Result<(HierObjectId, String)>;

    /// The latest version of a container **that `ehr` owns and that holds
    /// `rm_type`** (`db:H5.18`).
    ///
    /// A container owned by another `EHR`, or holding another kind of content,
    /// is answered with the same [`crate::StoreError::NotFound`] an absent one
    /// gets, so the answer reveals nothing about records the caller did not
    /// name correctly. Use this, not [`Store::latest_version`], for anything
    /// reached through an `EHR` path.
    ///
    /// # Errors
    ///
    /// [`crate::StoreError::NotFound`] as above, or an engine failure.
    fn latest_version_in(
        &self,
        ehr: &HierObjectId,
        versioned_object_uid: &HierObjectId,
        rm_type: &str,
    ) -> Result<VersionRow> {
        require_owned(
            self,
            ehr,
            versioned_object_uid,
            rm_type,
            "versioned_object",
            &versioned_object_uid.to_string(),
        )?;
        self.latest_version(versioned_object_uid)
    }

    /// Every version of a container `ehr` owns and that holds `rm_type`,
    /// oldest first (`db:H5.18`). Same refusal shape as
    /// [`Store::latest_version_in`].
    ///
    /// # Errors
    ///
    /// [`crate::StoreError::NotFound`] when not owned or not of that type, or
    /// an engine failure.
    fn all_versions_in(
        &self,
        ehr: &HierObjectId,
        versioned_object_uid: &HierObjectId,
        rm_type: &str,
    ) -> Result<Vec<VersionRow>> {
        require_owned(
            self,
            ehr,
            versioned_object_uid,
            rm_type,
            "versioned_object",
            &versioned_object_uid.to_string(),
        )?;
        self.all_versions(versioned_object_uid)
    }

    /// One version, by identifier, **only if `ehr` owns its container and the
    /// container holds `rm_type`** (`db:H5.18`). A mismatch is
    /// [`crate::StoreError::NotFound`] of kind `version`, exactly what an
    /// absent version gets.
    ///
    /// # Errors
    ///
    /// [`crate::StoreError::NotFound`] as above, or an engine failure.
    ///
    /// # Panics
    ///
    /// Never: an `OBJECT_VERSION_ID` parses only if its object part is a `UID`,
    /// and a bare `UID` is always a valid `HIER_OBJECT_ID`.
    fn get_version_in(
        &self,
        ehr: &HierObjectId,
        uid: &ObjectVersionId,
        rm_type: &str,
    ) -> Result<VersionRow> {
        // A version identifier names its container, so the container is not
        // an extra input a caller could get wrong.
        let container = HierObjectId::new(uid.object_id().clone(), None)
            .expect("an OBJECT_VERSION_ID's object part is always a valid HIER_OBJECT_ID");
        require_owned(self, ehr, &container, rm_type, "version", &uid.to_string())?;
        self.get_version(uid)
    }

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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::{check_commit_rules, version_at_time_candidates_sql, version_uid_at_time};
    use crate::StoreError;
    use openehr::base::ObjectVersionId;
    use openehr::rm::common::CommitError;

    fn uid(v: u32) -> ObjectVersionId {
        format!("87284370-2D4B-4E3D-A3F3-F303D2F4F34B::ehr1.example.org::{v}")
            .parse()
            .expect("literal")
    }

    #[test]
    fn historical_candidates_bind_the_container_and_cutoff_for_both_index_lookups() {
        let pg = version_at_time_candidates_sql(crate::Placeholder::Dollar);
        assert!(pg.contains("versioned_object_uid = $1 AND audit_time_committed_utc IN ($2,"));
        assert!(pg.contains("versioned_object_uid = $3 AND audit_time_committed_utc < $4"));
        let sqlite = version_at_time_candidates_sql(crate::Placeholder::Question);
        assert_eq!(sqlite.matches('?').count(), 4);
    }

    #[test]
    fn historical_selection_is_independent_of_row_order() {
        let cutoff = openehr::rm::data_types::DvDateTime::new("2026-08-01T00:00:00.5Z").unwrap();
        let candidates = [
            (1, "00:00:00.5000Z"),
            (2, "00:00:00.50000001Z"),
            (3, "00:00:00.5Z"),
            (4, "00:00:00.4Z"),
        ];
        for indices in [[0, 1, 2, 3], [3, 2, 1, 0]] {
            let rows = indices.map(|i| {
                let (n, time) = candidates[i];
                Ok((
                    uid(n).to_string(),
                    format!("2026-08-01T{time}"),
                    i64::from(n),
                ))
            });
            assert_eq!(version_uid_at_time(&cutoff, rows).unwrap(), Some(uid(3)));
        }
    }

    #[test]
    fn malformed_historical_metadata_is_refused_without_echoing_it() {
        let cutoff = openehr::rm::data_types::DvDateTime::new("2026-08-01T00:00:00Z").unwrap();
        for (id, time) in [
            (uid(1).to_string(), "private-marker"),
            ("private-marker".into(), "2026-08-01T00:00:00Z"),
        ] {
            let error = version_uid_at_time(&cutoff, [Ok((id, time.into(), 1))]).unwrap_err();
            assert!(matches!(error, StoreError::Parse(_)));
            assert!(!error.to_string().contains("private-marker"));
        }
    }

    #[test]
    fn historical_candidate_engine_errors_are_propagated() {
        let cutoff = openehr::rm::data_types::DvDateTime::new("2026-08-01T00:00:00Z").unwrap();
        let error = version_uid_at_time(
            &cutoff,
            [Err(StoreError::Engine {
                engine: "fixture",
                message: "candidate read failed".to_owned(),
            })],
        )
        .unwrap_err();
        assert!(matches!(
            error,
            StoreError::Engine {
                engine: "fixture",
                ..
            }
        ));
    }

    /// `check_commit_rules` is a pure function with no test of its own in
    /// this crate — every crate that calls it (`openehr-sqlite`, today) is
    /// what actually exercises it, which means a mutation here would go
    /// unnoticed by `cargo mutants` run against this crate alone. Every
    /// branch, directly.
    #[test]
    fn the_first_version_of_a_new_container_is_admitted() {
        assert!(check_commit_rules(None, false, None).is_ok());
    }

    #[test]
    fn a_first_version_naming_a_predecessor_is_refused() {
        assert!(matches!(
            check_commit_rules(None, false, Some(&uid(1))),
            Err(StoreError::Commit(CommitError::PrecedingVersionMismatch))
        ));
    }

    #[test]
    fn a_successor_naming_no_predecessor_is_refused() {
        let head = (uid(1).to_string(), 1);
        assert!(matches!(
            check_commit_rules(Some(&head), false, None),
            Err(StoreError::Commit(CommitError::PrecedingVersionMismatch))
        ));
    }

    #[test]
    fn a_successor_naming_the_current_head_is_admitted() {
        let head = (uid(1).to_string(), 1);
        assert!(check_commit_rules(Some(&head), false, Some(&uid(1))).is_ok());
    }

    #[test]
    fn a_successor_naming_a_stale_predecessor_is_refused() {
        let head = (uid(2).to_string(), 2);
        assert!(matches!(
            check_commit_rules(Some(&head), false, Some(&uid(1))),
            Err(StoreError::Commit(CommitError::NotLatest))
        ));
    }

    /// Checked before either match arm, and independently of both: a
    /// duplicate is refused even when it would otherwise look like the
    /// first version of a new container.
    #[test]
    fn a_duplicate_uid_is_refused_even_as_a_first_version() {
        assert!(matches!(
            check_commit_rules(None, true, None),
            Err(StoreError::Commit(CommitError::DuplicateVersion))
        ));
    }
}
