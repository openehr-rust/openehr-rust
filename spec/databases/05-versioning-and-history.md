# 5. Versioning and history

**Rewritten 2026-08-01.** The previous text specified FHIR's versioning model — a
monotonic `version_id` per resource id, a `<resource>_history` table, `op ∈ C/U/D`,
soft delete, and `vread`. openEHR versions differently and more strictly, and the
model is part of the Reference Model rather than a storage convention. See
[`spec/audit.md`](../audit.md) **W-04**.

Withdrawn requirements keep their numbers (`C0.5`); new ones start at `H5.5`
(`C0.19`).

Requirement prefix: `H5`.

## There is no separate history table

- **H5.5** Every version MUST be a row in `openehr_version`. There MUST NOT be a
  separate current-versus-history split.

  FHIR keeps a current row and archives superseded ones. openEHR does not have a
  "current row" to update: a `VERSIONED_OBJECT` **is** its sequence of versions,
  and the latest is a query over that sequence rather than a distinct object. A
  store that maintained both would have two representations of the same fact and
  a way for them to disagree.

- **H5.1** *(amended)* A commit MUST append one row and MUST NOT modify any
  existing row. `openehr_version` is append-only in the database (`M3.17`), so
  this is enforced by the engine rather than trusted to the caller.

## Version identity

- **H5.6** A version MUST be identified by a full `OBJECT_VERSION_ID` —
  `object_id::creating_system_id::version_tree_id` — and that identity MUST be
  stored **decomposed** into its parts as well as whole.

  The parts are stored because the commit rules are checked on them (`H5.8`), and
  a store that only kept the string would have to re-parse it on every check.

- **H5.7** `creating_system_id` MUST be stored and MUST NOT be dropped as
  redundant. It is what keeps two systems' "version 2" distinct when both were
  authored offline and later merged. A version tree without it is ambiguous
  exactly when it matters most.
- **H5.8** A commit MUST be refused when the version does not belong at the head
  of its container: a version belonging to another container, a duplicate
  position in the version tree, or a missing or stale predecessor
  (`lib:V8.1`–`lib:V8.5`).

  These are not database constraints that happen to be convenient. They are the
  difference between a history that connects and one that reads back and does
  not.

- **H5.9** Those refusals MUST be enforced by every engine, and MUST be
  distinguishable from one another by the caller. An engine MAY enforce a
  refusal with a unique index rather than a query; the conformance suite does not
  care how, only that it happens and can be told apart.
- **H5.10** A unique index over
  `(versioned_object_uid, trunk_version, branch_number, branch_version)` MUST
  exist, so a duplicate commit fails in the database and not only in the library.
  A rule enforced solely in application code is a rule that ends at the first
  process that does not use the library.

## Deletion is a version

- **H5.2** *(amended)* Deletion MUST be recorded as a **new version** whose
  lifecycle state marks it deleted, with `data_json` `NULL`. It MUST NOT remove
  or alter the version it supersedes.

  This is the same rule as `M3.17` seen from the model's side: a correction is a
  new version, and the version it corrects stays. A store permitting a hard
  delete would let a correction erase what it corrected.

- **H5.11** `is_deleted` MUST be stored as a column derived from the lifecycle
  state, and indexed, so that "current content" does not require a code
  comparison on every row.

## Reads over the sequence

- **H5.3** *(amended)* A store MUST offer: a version by identifier, the latest
  version of a container, the version current at a given time, and every version
  of a container.
- **H5.12** "Every version" MUST be returned **oldest first**.

  openEHR contradicts itself about this in prose; the `most_recent_version`
  postcondition on `REVISION_HISTORY` settles it (`lib:V8.7a`). Where a
  specification disagrees with itself, the resolution is recorded rather than
  silently chosen (`C0.2`).

- **H5.13** "The version current at a time" MUST order on the derived UTC column,
  and MUST therefore **skip** any version whose commit time is not an established
  instant — exactly as `version_at_time` skips it in the library (`lib:V8.6`,
  `M3.28`). A store ordering on the lexical form would sort
  `2026-07-31T09:00:00+02:00` after `2026-07-31T08:30:00Z`.

## Contributions

- **H5.14** A `CONTRIBUTION` MUST be recorded as its own row and MUST be
  append-only. It is the unit a user recognises as "I saved the consultation" —
  one change set spanning several versions — and it is what makes a record's
  change history readable in the order it was made rather than per object.

## Concurrency

- **H5.4** *(amended)* Concurrent commits to one container MUST NOT produce two
  versions at the same position in the version tree. The unique index of `H5.10`
  is sufficient for this and is the preferred mechanism, because it holds
  regardless of how many processes are writing.

  **Not verified.** Nothing in this repository exercises concurrent writers. The
  index exists and the constraint is declared; that a race actually loses in the
  database has not been demonstrated, and is recorded as unverified rather than
  assumed (`C0.20`, `T11`).

## Withdrawn

Withdrawn 2026-08-01. Numbers are retained and MUST NOT be reused (`C0.5`).

| Id | Was | Why withdrawn |
| --- | --- | --- |
| `H5.1` *(part)* | a monotonic integer `version_id` per resource id | openEHR identity is an `OBJECT_VERSION_ID` with a creating system and a version tree (`H5.6`, `H5.7`); the amended `H5.1` keeps the append obligation |
| `H5.2` *(part)* | `<resource>_history(id, version_id, last_updated, op, resource)` with `op ∈ C/U/D` | no separate history table (`H5.5`); the amended `H5.2` keeps the soft-delete obligation |
| `H5.3` *(part)* | `vread` and `read` as REST interactions | §7 is retired (`C0.6`); the amended `H5.3` keeps the read obligations as library operations |

---

Part of the [openEHR persistence specification](index.md).
