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
  On the trunk, absent `branch_number` and `branch_version` MUST be encoded as
  zero in their required SQL columns and decoded back to absence at the Rust
  boundary. Version-tree components are positive integers, so zero is outside
  the identifier domain and is an unambiguous storage sentinel.

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

  All four indexed columns MUST be non-null. A conventional unique index does
  not make two `NULL` values equal on SQLite or PostgreSQL, so nullable branch
  components allow any number of competing trunk versions at the same position
  while appearing to satisfy this requirement.

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

## Deactivation gates new content

- **H5.17** A commit adding a content version to an `EHR` — a `COMPOSITION`,
  a `FOLDER` — MUST be refused when the EHR's current `EHR_STATUS` has
  `is_modifiable = false`.

  **The check MUST re-read `is_modifiable` fresh for every commit, never a
  value cached from earlier in the same request or the same
  `CONTRIBUTION`.** This is the sequencing bug FerroEHR's own tracker
  records (`#2673`): an implementation that reads `is_modifiable` once, at
  the start of processing a contribution, and reuses that answer for every
  version the contribution adds, refuses content a caller committed
  *after* reactivating the same record earlier in the same contribution —
  because the cached answer is still the pre-reactivation one. A caller
  reactivating and adding content in one contribution MUST commit the
  reactivating `EHR_STATUS` version before the content it is meant to
  admit; this rule does not — cannot, without committing every version of
  a contribution as a single atomic unit, which nothing here does — look
  ahead to a reactivation the same contribution has not committed yet.

  A related but distinct defect the same investigation reports,
  `VERSIONED_OBJECT.versions` typed a `List` but described as a `Set`, is
  FerroEHR's `#2674` — see
  [`openehr/spec/ambiguities.md`](../../openehr/spec/ambiguities.md).

  This binds new *content*, not every commit: an `EHR_STATUS` version is
  itself always admitted regardless of the record's current
  `is_modifiable` — otherwise a deactivated record could never be
  reactivated at all, which is not what deactivation is for.

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

  An as-of read MUST exclude a later commit even when it falls within the
  query's UTC second. With versions committed at `09:05:00.100Z` and
  `09:05:00.900Z`, a query at `09:05:00.500Z` on that date returns the
  first version. The second becomes eligible at its commit time. The cutoff
  uses all authored fractional digits. Existing databases already hold them
  in the authoritative text. Use the UTC index to bound the candidates to
  the query second and the most recent occupied earlier second, then compare
  their parsed instants. For equal eligible instants prefer the higher trunk
  version, as in the whole-second lookup.

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

  **Verified 2026-08-01**, and strengthened 2026-11-19, by
  `openehr-sqlite/tests/concurrency.rs::racing_commits_to_one_position_produce_one_winner`:
  eight writers from eight distinct `creating_system_id`s race for one position
  in a version tree and exactly one wins. Distinct version UIDs are essential
  evidence for `H5.10`: otherwise the primary key can reject every loser before
  the unique tree-position index is exercised. The same test runs against a
  live PostgreSQL server.

  It **failed** the first time it ran, and that failure is `D-06` — the seven
  losers were refused, correctly, but the refusal surfaced as an engine error
  rather than as a commit-rule refusal, so a caller could not tell "you lost a
  race, retry" from "the database is broken". The guarantee this requirement
  states held; the way it was reported did not. That is the argument for writing
  the test rather than reasoning from the unique index.

## Preconditions on a write

Added 2026-08-02, when `openehr-axa` gained an update endpoint. These bind a
**service**; the core has no notion of a request.

- **H5.15** A service offering an update MUST require a precondition naming the
  version being replaced, and MUST refuse the write when it does not name the
  current one. It MUST answer `412 Precondition Failed` — not `409` — because
  the two are different instructions to the caller: `412` says *the statement
  you made about the world is false, re-read and decide*, and `409` says *the
  store refused this commit*. A caller that conflates them retries the wrong
  one.

  `If-Match: *` MUST be refused. It is satisfied by any current representation,
  so it would let a caller overwrite a version they have never seen — the exact
  loss the precondition exists to prevent.

  **A declared departure from RFC 9110** (`C0.16`). §13.1.1 requires *strong*
  comparison for `If-Match`, under which a weak entity tag never matches
  anything, so a service emitting `W/"…"` and honouring it is not conforming.
  This departs deliberately: the tag names a **version**, and "is the head
  still version N" is exactly the question optimistic concurrency asks. Strong
  comparison would fail on two byte-different serialisations of one version,
  which are the same clinical fact — rejecting correct requests to protect
  against nothing.

- **H5.16** A service MUST accept the precondition both as `W/"<version-uid>"`
  and as the bare `<version-uid>`.

  The first is what an HTTP client round-trips from the `ETag` it was given.
  The second is what the openEHR REST API specifies, and refusing it would mean
  a client written against openEHR could not talk to the service. Accepting
  both costs one `strip_prefix`.

- **H5.18** *(added 2026-11-25, `D-19`)* A read that names an `EHR` and a
  container (or a version of one) MUST return content only if that `EHR` owns
  the container **and** the container holds the expected kind of content. A
  mismatch of either MUST be answered exactly as an absent container is —
  `NotFound`, the same kind and the same shape — so a caller learns nothing
  about records it is not entitled to name.

  The store exposes this as scoped lookups (`latest_version_in`,
  `all_versions_in`, `get_version_in`), each taking the owning `EHR` and the
  expected `rm_type`. A service reading under an `EHR` path MUST use them: the
  unscoped lookups remain for callers that have already resolved the owner by
  other means (the AQL executor, whose candidates come from an `EHR`-scoped
  index), and are not for a request path. Ownership and clinical subject are
  different things: a mother's condition recorded as family history in her
  child's chart is correctly in the child's `EHR`, and this requirement says
  nothing about it.

- **H5.19** *(added 2026-11-25, `D-19`)* A commit naming an `EHR` MUST be
  refused when the container it adds to already exists under a different `EHR`,
  or under a different `rm_type`. The refusal MUST NOT write a row.

  Found alongside `H5.18`: `commit_composition` consulted its `ehr_id`
  argument only when it created a container, so an amendment addressed to the
  wrong `EHR` was appended to the right container. The `EHR` argument was
  decoration for every commit but the first.

- **H5.20** *(added 2026-11-26, `D-23`)* An `EHR`'s status is the
  `EHR_STATUS` container its `ehr_status` reference names **only if that
  `EHR` owns the container and the container holds `EHR_STATUS`**
  (`H5.18`). The reference is caller-supplied when the `EHR` is created,
  so naming a container proves nothing about it.

  - A **read** of the current or as-of status MUST be scoped exactly as
    `H5.18` requires, and a mismatch answered as an absent status.
  - The **write gate** of `H5.17` MUST distinguish three states. No such
    container means the status was never committed, and the record is
    modifiable. A container this `EHR` owns holding `EHR_STATUS` is read
    fresh. A container that exists but belongs to another `EHR` or holds
    other content is an **inconsistent reference**, and the commit MUST be
    refused as a conflict, writing nothing. It MUST NOT be read as "never
    committed": that would let one chart's deactivation be bypassed, and
    another chart's be imposed, by naming its container.

  Found reproduced over HTTP: an `EHR` created naming another chart's status
  container was served that chart's `EHR_STATUS` with `200`.

- **H5.23** *(added 2026-11-27, `D-33`)* Creating an `EHR` with its first
  `EHR_STATUS` is **one change**. The store MUST offer an operation that
  writes the `EHR`, the `CONTRIBUTION` that declares the status, and the
  status version together, or writes none of them. Every refusal the three
  steps make on their own still applies, and any of them, or an engine
  failure part-way, leaves no row behind.

  `S1.22` requires a service to create the record and its status together,
  and nothing else repairs a record left without one: its status cannot be
  read, a retried creation is a conflict, and there is no route that creates
  a status for an existing `EHR`. Before this, a service made three calls,
  each its own transaction, and an engine failure after the first left
  exactly that record.

- **H5.22** *(added 2026-11-26, `D-26`)* A version commit names the
  `CONTRIBUTION` it belongs to, and the store MUST refuse it, writing
  nothing, unless:
  - that contribution exists;
  - it belongs to the same `EHR`;
  - it declared this version among its `versions`.

  A contribution's declared version list is stored with it, and a version
  row carries a foreign key to its contribution, so the change set is
  reconstructible from the database alone (`H5.14`, `PR12.10`). Before
  this, `contribution_uid` was copied from an argument nothing checked. A
  version could name a contribution that did not exist, one in another
  chart, or one that never mentioned it.

- **H5.21** *(added 2026-11-26, `D-24`)* An update addressed to a
  container MUST commit to that container. The submitted version's container
  MUST be the one the request names, and its `preceding_version_uid` MUST be
  the version the precondition (`H5.15`) names. A disagreement is the
  caller's error (`400`), checked before anything is written. Ownership
  checks alone cannot catch it, because both containers may belong to the
  same `EHR`.

  Found reproduced over HTTP: a `PUT` to one composition, with that
  composition's current version as `If-Match`, committed a new version of a
  different composition in the same chart.

## Withdrawn

Withdrawn 2026-08-01. Numbers are retained and MUST NOT be reused (`C0.5`).

| Id | Was | Why withdrawn |
| --- | --- | --- |
| `H5.1` *(part)* | a monotonic integer `version_id` per resource id | openEHR identity is an `OBJECT_VERSION_ID` with a creating system and a version tree (`H5.6`, `H5.7`); the amended `H5.1` keeps the append obligation |
| `H5.2` *(part)* | `<resource>_history(id, version_id, last_updated, op, resource)` with `op ∈ C/U/D` | no separate history table (`H5.5`); the amended `H5.2` keeps the soft-delete obligation |
| `H5.3` *(part)* | `vread` and `read` as REST interactions | §7 is retired (`C0.6`); the amended `H5.3` keeps the read obligations as library operations |

---

Part of the [openEHR persistence specification](index.md).
