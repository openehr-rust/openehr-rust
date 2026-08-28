# 8. Versioning and change control

Requirement prefix: `V8`.

Nothing in an openEHR record is updated in place. This section is what makes
that true in code rather than by convention.

## `VERSIONED_OBJECT`

- **V8.1** A commit MUST be refused when the version's `uid.object_id` does not
  match the versioned object's own identifier.
- **V8.2** A commit MUST be refused when a version with the same identifier is
  already present.
- **V8.3** A commit MUST be refused when the first version claims a predecessor,
  or a later version claims none. Either produces a history with two roots and
  no way to tell which is wrong.
- **V8.4** A commit MUST be refused when `preceding_version_uid` is not the
  current latest version. This is the concurrent-write case: openEHR's answer is
  a branch, not a silent overwrite, so the refusal is the point at which a
  caller must decide.
- **V8.5** Each refusal MUST be distinguishable from the others, because they
  call for different responses.
- **V8.6** `version_at_time` MUST return the latest version committed at or
  before the given time, and MUST skip versions whose commit time is not
  comparable with it (`D3.14`) rather than assuming they precede it.
- **V8.7** `revision_history` MUST return nothing at all for an object with no
  versions. An empty history object would assert that the audit trail exists and
  is blank.
- **V8.7a** `REVISION_HISTORY.items` MUST be ordered **oldest first**, and
  `most_recent_version` MUST return the **last** item.

  openEHR contradicts itself here. The class table's *Purpose* says
  "most-recent-first"; the `items` attribute's *Meaning* says
  "most-recent-last"; and the `most_recent_version` postcondition reads
  `Result.is_equal(items.last.version_id.value)`. Two of the three agree, and
  one of those two is executable — a postcondition is a statement a conformant
  implementation can be tested against, a Purpose line is prose. This crate
  follows the postcondition and records the contradiction, because a caller
  rendering an audit trail from the other sentence gets it backwards.
- **V8.7b** `most_recent_version_time_committed` MUST be available, defined as
  openEHR defines it: `items.last.audits.first.time_committed`.

## `VERSION`

- **V8.8** `lifecycle_state` MUST come from the openEHR
  `version_lifecycle_state` group.
- **V8.9** A version with no `data` MUST be refused unless its lifecycle state
  is `deleted`. A version claiming `complete` and supplying nothing claims the
  content is finished and then does not supply it.
- **V8.10** Deletion MUST be a version, not a removal. The record of the
  deletion is itself a record.
- **V8.11** `IMPORTED_VERSION` MUST delegate its identity to the wrapped
  `ORIGINAL_VERSION`. Minting a local identifier would make the same clinical
  fact appear twice the next time the two systems exchange data.
- **V8.12** `IMPORTED_VERSION` MUST carry its own commit audit as well as the
  wrapped version's, so that an authored record and a received one stay
  distinguishable.

## `AUDIT_DETAILS` and `CONTRIBUTION`

- **V8.13** `AUDIT_DETAILS` MUST require `system_id`, `time_committed`,
  `change_type`, and `committer`.
- **V8.14** `change_type` MUST come from the openEHR `audit_change_type` group,
  and the constructor MUST take a code, making a rubric that disagrees with its
  code unconstructible.
- **V8.15** `CONTRIBUTION.audit.change_type` MUST be restricted to `creation`,
  `amendment`, and `deleted`, as openEHR requires — `synthesis` is a valid
  `AUDIT_DETAILS` change type and not a valid `CONTRIBUTION` one.
- **V8.16** `CONTRIBUTION.versions` MUST be non-empty.

## `ATTESTATION`

- **V8.17** `ATTESTATION` MUST carry `reason`, `is_pending`, and optionally
  `proof`, `attested_view`, and the specific nodes attested to.
- **V8.17a** Where `ATTESTATION.reason` is coded from openEHR's own
  terminology, it MUST be in the `attestation_reason` group (`Reason_valid`),
  and MUST NOT be checked when coded from another terminology — the same shape
  as `M5.18a`.
- **V8.18** `proof` MUST NOT be verified (`S1.11`), and the documentation MUST
  say at the point of use that a present `proof` is therefore **not evidence of
  anything** in this crate.
- **V8.19** `attested_view` MUST be modelled. A clinician signs what was on the
  screen, and the screen is a template's rendering of the data; keeping the
  rendering is what makes the signature meaningful after the template changes.
