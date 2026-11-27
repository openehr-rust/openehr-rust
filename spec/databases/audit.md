# Persistence audit findings

**Rewritten 2026-08-01.** This file previously held ~70 KB of findings imported
from a FHIR monorepo and mechanically renamed. They described defects in a
different codebase — a shredder, a search-parameter compiler, six copied `map/`
trees — and included claims about this software that were never true of it, such
as "all **7,399 official OpenEHR example resources** (R3 + R4 + R5) round-trip".
Retaining them would have meant a findings register whose findings were fiction.
See [`spec/audit.md`](../audit.md) **W-04**.

**Non-normative.** This is the register of known gaps between what
[`spec/databases/`](index.md) requires, what the persistence documentation
claims, and what the code does. Repository-wide findings — anything spanning
crates or sitting above either domain — live in [`spec/audit.md`](../audit.md)
as `W-xx`. Library findings live in
[`openehr/spec/audit.md`](../../openehr/spec/audit.md) as `A-xx`.

A finding stays here until it is fixed or a requirement is amended to match
reality. Deleting one because it is inconvenient, or because the text that stated
it was rewritten, is the failure this file exists to prevent (`C0.20`).

## Numbering

- **D-xx** is this register's prefix, chosen because `F-xx` is taken (below) and
  `A-xx` belongs to the library.
- Identifiers are permanent and never reused (`C0.5`).

## `F-xx` refers to a different project

The code and documentation in this repository cite findings **F-01**, **F-06**,
**F-07**, **F-08**, **F-25**, and **F-26**. Those are **not** findings against
this software. They belong to the sibling FHIR monorepo, and they are cited here
because this repository's architecture is a direct response to them.

The citations are preserved rather than rewritten, because a citation's value is
that it does not change (`C0.5`), and because the lessons are the reason several
requirements exist. What they refer to:

| Id | The sibling monorepo's defect | What it caused here |
| --- | --- | --- |
| **F-01** | Six READMEs claiming a working store, a CLI, and 7,399 losslessly round-tripped resources — in ports where two had no store at all and none had ever had a CLI. | `C0.9`, `C0.11`: a crate states its level in the first screenful and may not describe a capability above it. |
| **F-06** | Two ports whose database CI jobs invoked a test target that did not exist, so they could not have passed and did not say so. | `C0.13`, `T11.13`: a check must fail rather than skip, and a self-skipping test is not evidence. |
| **F-07** | One port derived its hash-chain pre-image from `jsonb`, so no other port could verify its chains. | `X15.2`, `R4.11`: canonical form is computed in Rust, never delegated to the database. |
| **F-08** | An Oracle DDL emitter that produced MySQL types for the fork's entire life, because six ports each owned a full copy of the generator and nothing compared them. | The whole architecture: `X15.13`, `X15.14`, `M3.22`, `G2.8`. And `X15.15` — the comparison that catches it. |
| **F-25**, **F-26** | A migration path that could never have executed, in a port with no store to notice. | `C0.8`: **Dialect** level explicitly does not mean the DDL runs. |

**F-08 was reproduced in this repository anyway.** See `W-01`. That is the most
useful thing in this table: an architecture designed against a specific defect
still admitted it, because the guard that would have caught it was incomplete.

## Findings recorded elsewhere

Two sets of findings are persistence findings that live in another register, and
are **not** duplicated here. Renumbering them would break citations (`C0.5`).

| Ids | Register | Subject |
| --- | --- | --- |
| `A-13`, `A-14`, `A-15` | [`openehr/spec/audit.md`](../../openehr/spec/audit.md) | Found by running generated DDL against PostgreSQL 18 and MySQL 8.4: MySQL rejects `CREATE INDEX IF NOT EXISTS`, and three dialects enforced append-only nowhere. Recorded in the library register because that was the only register that existed at the time. |
| `W-01`, `W-02`, `W-04` | [`spec/audit.md`](../audit.md) | `openehr-mariadb` as a copy of `openehr-mysql`; the claimed-but-absent CI; this directory's import. All three span crates, so they belong to the repository register. |

They are listed rather than moved. A finding that changes number is a finding
whose citations in commit messages, test names, and code comments quietly stop
resolving.

## Open findings

### D-31 — The `EHR_STATUS` read answers in this service's version view, not as the resource ITS-REST defines — **Medium, fixed** (2026-11-27, not yet committed)

**Found 2026-11-27 by the conformance run** (`openehr-axa/spec/conformance-runs.md`,
Run 2). `GET /ehr/{ehr_id}/ehr_status` answers `{"uid", "versioned_object_uid",
"lifecycle_state_code", "is_deleted", "time_committed", "chain_digest",
"committer", "data": {…EHR_STATUS…}}`. ITS-REST 1.1.0 answers with the
`EHR_STATUS` itself, with the version identified in headers. EHRbase 2.38.0
does that, and the same runner reads `.is_modifiable` there and `null` here.
The composition routes use the same view, so they diverge the same way.

**Open until** a decision is made between two shapes. Option one: answer ITS-REST
routes with the resource, and move the version metadata (uid, chain digest,
committer) into headers or a separate versioned resource. Option two: keep the
view and declare the departure. Either way the runner's read cases must pass
or name the departure.

**Fixed 2026-11-27**, option one, by the maintainer's decision, against the new
`S1.23`.
- **The ITS-REST routes** answer with the resource: the current and versioned
  `EHR_STATUS`, the composition read, and the composition create and update.
  The composition read accepts ITS-REST's `uid_based_id`, so the version
  `Location` it returns can itself be read. `ETag` and `Location` name the
  version.
- **This service's own routes** keep the version view, with the chain digest
  and the committer (`PR12.25`): `_history`, `…/version/{version_uid}`, and
  search.
- **Evidence.** HTTP tests check the resource shape and headers, the `Location`
  read back, a deletion read by its version id (`410`), and a commit answering
  with its `COMPOSITION`. Run 3 (`conformance-runs.md`): 9/9 here and on
  EHRbase 2.38.0.
- **Lost from the status response:** the committer. It is in the store
  (`PR12.25`), but no route returns an `EHR_STATUS` version's audit yet.
  ITS-REST's `versioned_ehr_status/revision_history` would be that route.

### D-32 — A minimal ITS-REST `EHR_STATUS` body is refused as not an archetype root — **Low, closed by decision** (2026-11-27)

**Found 2026-11-27 by the conformance run.** `POST /ehr` with
`{"_type":"EHR_STATUS","archetype_node_id":"openEHR-EHR-EHR_STATUS.generic.v1",
"name":…,"subject":{"_type":"PARTY_SELF"},"is_modifiable":false,
"is_queryable":true}` answers `422`:
`EHR_STATUS.Is_archetype_root — an EHR_STATUS has no archetype_details`.
EHRbase 2.38.0 accepts the same body (`201`).

This service is enforcing the RM's own invariant: `EHR_STATUS` is an archetype
root, so its `archetype_details` is required. So this is not a defect against
the RM. It is a strictness difference a client will meet. **Open until** a
decision is made: keep refusing, as now (documented in `S1.22`, with the
`422` naming the invariant), or fill `archetype_details` from
`archetype_node_id` for this one case, as a declared, specified leniency.

**Closed 2026-11-27, by the maintainer's decision: keep refusing.** Written into
`S1.23`: a supplied `EHR_STATUS` must carry `archetype_details`, and is refused
with `422` naming the invariant. The runner's supplied-status case now sends a
valid RM body, which both implementations accept.

### D-23 — Current and historical EHR status follow an unverified reference — **High, fixed** (2026-11-26, not yet committed)

**Live PostgreSQL, 2026-11-27.** `openehr-postgresql/scripts/verify-store.sh`
against PostgreSQL 18 over verified TLS: 28 of 28 tests pass. That includes
`run_ehr_status_binding` (this finding), `run_ehr_scope` (`D-19`),
`run_contribution_membership` (`D-26`), `run_paging` (`P6.25`), both
concurrency races, and the four TLS cases, all on schema 6.

**Claimed.** `db:H5.18` requires reads under an EHR path to establish the
owning EHR and the expected RM type. The composition routes and the status
version-by-ID route have scoped lookups.

**Found by source inspection, 2026-11-25, at `ec029cf`.** In
`openehr-axa/src/controllers/ehr_status.rs::read`, the EHR row supplies a
container identifier, then the current and as-of branches call
`latest_version` and `version_at_time`. Neither call checks the owner or
type. `SqliteStore::create_ehr` persists the submitted reference, so reading
it from a stored EHR establishes which container the EHR names but does
not establish who owns that container. The same distinction applies when
the two stores resolve status for the modifiability gate. The ordinary
fixture deliberately aligns the references (`conformance::sample_ehr_status_version`).
No cross-chart disclosure was exercised in this audit.

**Acceptance.** Use owner- and type-scoped reads for both branches. Cover
matching, absent, foreign-owner, and wrong-class references in shared store
tests and at the HTTP boundary. Check the access outcome as well as the
response. Define how the write gate distinguishes an uninitialized status
from an inconsistent stored reference, and cover both on SQLite and PostgreSQL.

**Reproduced 2026-11-26 over HTTP.** An `EHR` created naming the first
chart's status container got `200` and that chart's `EHR_STATUS` from
`GET /ehr/{second}/ehr_status`. **Fixed** against the new `H5.20`. Both
branches of the handler use the scoped lookups (`latest_version_in` and the
new default method `version_at_time_in`). The write gate is one shared
`openehr_store::ehr_is_modifiable` in place of three private copies (SQLite,
PostgreSQL, and the owner memory store), and it tells the three states apart:
absent means open, owned means read fresh, and foreign or wrong-class means
`Conflict`. Evidence: `conformance::run_ehr_status_binding` on the owner
store and SQLite. HTTP tests assert the `404` with no content, and a
`not_found` access record. Owner-crate tests cover an engine failure in the
gate's owner lookup (it stays an engine error) and a deleted status (not
open). **Not run:** the same story against PostgreSQL, which is wired in
`openehr-postgresql/tests/store.rs` and needs a live server.

### D-24 — A composition update checks a different source of identity from the commit — **High, fixed** (2026-11-26, not yet committed)

**Claimed.** `db:H5.15` requires “a precondition naming the version being
replaced”. **Found by source inspection at `ec029cf`.**
`openehr-axa/src/controllers/composition.rs::update` resolves the container
from the URL and compares its head with the header. It passes the submitted
version to `commit_composition`, which derives the destination from
`version.uid().object_id()`. The handler never establishes that the two
container identifiers agree. EHR ownership checks permit multiple compositions
within one EHR.

**Acceptance.** Bind the addressed container, submitted version, predecessor,
and precondition to the same update. Same-EHR fixtures with two containers
must show unchanged heads, index, and history on disagreement. Preserve the
documented stale-header and successful-amendment cases.

**Reproduced 2026-11-26 over HTTP.** `PUT …/composition/LIVE`, with
`If-Match` naming `LIVE`'s current version, committed `GONE::3`.
**Fixed** against the new `H5.21`. `bind_update` refuses a body for another
container before the store is touched, and `bind_predecessor` refuses a body
whose `preceding_version_uid` is not the `If-Match` version. The second runs
after the `412` check, so the existing stale-header (`412`) and wrong-EHR
(`404`) tests pass unchanged, and so do the successful amendments. Both
refusals are `400`, with `GONE`'s history and `LIVE`'s head compared before
and after. The index is not compared separately: nothing reaches the store.

### D-25 — Contribution creation omits the committer comparison — **High, fixed** (2026-11-26, not yet committed)

**Claimed.** `db:PR12.19` applies wherever a write accepts an `AUDIT_DETAILS`.
**Found by source inspection at `ec029cf`.**
`openehr-axa/src/controllers/contribution.rs::create` discards its verified
principal before persisting the contribution's own audit. Checking the
version's separate audit later cannot establish who declared the change set.
The HTTP test named `a_contribution_can_be_declared_and_is_required_before_a_commit`
asserts creation and duplicate declaration.

**Acceptance.** Apply the shared party/subject comparison before persisting
either kind of audit. Cover agreement, disagreement (403), and a missing
identifier (422) for each route. Assert that refusals leave the database
unchanged. Durable attribution after an accepted write is tracked in `D-22`.

**Fixed 2026-11-26.** `POST …/contribution` applies
`controllers::check_committer`, the rule version commits use, extracted from
`check_attribution` so the two cannot drift, before the store is called.
`a_contribution_is_declared_only_by_the_verified_caller` covers three cases:
another clinician's identifier (`403`), a committer with no identifier
(`422`), and the caller (`201`) with the same uid. The last shows that
neither refusal wrote a row. **Behaviour change:** a contribution whose
committer carries no identifier used to be accepted, and is now `422`.

### D-26 — Version commits do not establish membership in the declared contribution — **High, fixed** (2026-11-26, not yet committed)

**Claimed.** The contribution controller documents “the caller declares the act,
then commits versions into it” (`db:H5.14`, `db:PR12.10`).
**Found by source and DDL inspection at `ec029cf`.**
`VersionRow::project` assigns `contribution_uid` from its separate argument.
Both real stores call it without resolving that contribution or checking the
version's declared reference against it. In `assets/ddl/sqlite.sql` the
version table references the versioned object but has no foreign key on
`contribution_uid`. The declaration test cited in `D-25` makes no version
commit. Consequently the current evidence does not establish a reconstructible
change set for each committed version.

**Acceptance.** Specify agreement between the version reference, API argument,
and recorded contribution in the same EHR. Enforce it atomically on both
write paths in both stores. Exercise declared, missing, mismatched, and
wrong-EHR change sets with a positive multi-version case. Inspect existing
rows before changing constraints. If the schema changes, follow `db:O10.14`
through `db:O10.17`.

**Fixed 2026-11-26** against the new `H5.22`, in schema 6.
- **Schema.** `openehr_contribution.versions_json` stores the declared
  versions, and `openehr_version.contribution_uid` is a foreign key. The
  contribution table now emits first (`G2.12`).
- **Commit check.** Both commit paths in SQLite, PostgreSQL and the owner
  memory store call `openehr_store::check_membership` after the commit rules
  and before writing: the contribution exists, it is this `EHR`'s, and it
  declared this version. Otherwise the commit fails with
  `StoreError::NotInContribution`, which `openehr-axa` maps to `409`.
- **Evidence.** `conformance::run_contribution_membership`, on the owner
  store and SQLite, covers: one contribution committing a composition and a
  status; then an undeclared composition, an undeclared status, an absent
  contribution, and another record's contribution, each refused with no row
  written. HTTP: `a_version_is_committed_only_into_a_contribution_that_declared_it`.
- **Existing rows** were not inspected against the new constraint: no
  deployed database is known, and schema 5 is refused anyway.
- **Fixtures changed with it.** Every test commit across the tree now commits
  into a contribution that declares the version (`sample_contribution_of`,
  `sample_contribution_declaring`). Before, most did not.
- **Not run:** PostgreSQL.

### D-27 — Backup inspection creates databases, and an absent checkpoint succeeds — **High, fixed** (2026-11-26, not yet committed)

**Claimed.** `openehr-axa/src/tasks.rs::checkpoint_line` documents an error
if “the database cannot be opened or the container does not exist”.
**Observed at `ec029cf`, 2026-11-25.** The built CLI's `task checkpoint`
and `task verify` were each given a valid synthetic container ID and a
distinct nonexistent SQLite path in a temporary directory. Both created a
six-table database. Checkpoint exited 0 with `entries=0`, a zero head, and
`last_version=-`. Verify exited 1 with `Empty`. `store_for` calls the same
open-and-install path as service startup. A scheduled witness can therefore
record success against a mistyped backup path.

**Acceptance.** Open existing databases read-only for both tasks and check
schema compatibility without installing it. Refuse an absent requested
container at the operator boundary. Exercise a missing file, missing
container, incompatible schema, a populated read-only backup, and a damaged
history. Check exit status and verify that inspection leaves the input bytes
unchanged and creates no database on a missing path.

**Reproduced 2026-11-26** on the working tree: a mistyped
`OPENEHR_SQLITE_PATH` produced an 86 KB database even though the task then
refused its arguments. **Fixed 2026-11-26** against the new requirement
`O10.20`. `SqliteStore::open_read_only` opens with `SQLITE_OPEN_READ_ONLY`
and without `CREATE`, installs nothing, and refuses a database recording no
schema version (`found: 0`) or another one. `openehr-axa`'s
`open_store_read_only` is the only opener the two tasks use, and
`checkpoint` refuses a container the database does not hold instead of
printing `entries=0`. Evidence: `openehr-sqlite/tests/read_only.rs` covers a
missing path that stays absent, an empty file left empty, another schema
version, and a populated store whose bytes are identical after reads, with
writes refused. `openehr-axa/tests/tasks.rs` runs the real binary: both tasks
exit non-zero on a missing path and create nothing, an absent container is
refused, and both tasks leave a populated backup's bytes unchanged. The
existing tamper tests cover the damaged history. **Not covered:** an
incompatible schema through the binary (it is covered at the store level only),
and the restore drill, which is a separate `tasks.md` item and not part of
this finding.

### D-28 — Undecodable stored JSON becomes absent clinical content in the HTTP view — **High, fixed** (2026-11-26, not yet committed)

**Claimed.** `openehr-axa/src/views.rs::VersionView::data` describes “the
version's content, or `null` for a deletion”. **Found by source inspection
at `ec029cf`.** `composition::view` discards
`serde_json::from_str` errors with `.ok()`. A stored nondeleted row whose
content fails decoding therefore takes the successful response path with
`data: null`. Current, historical, and status responses share this view.

**Acceptance.** Make view construction fallible. Preserve a genuine tombstone
and a valid record. For an unreadable content-bearing row, return a
content-free server error, emit the corresponding audit outcome, and fail a
whole history page that contains the row. Cover each caller using synthetic
fixtures, including the readback that follows an already committed write.

**Fixed 2026-11-26** (`PR12.23` amended: outcome `error`). `composition::view`
returns a `Result`. A row with no document is still `data: null`. A
document that does not decode is a `500` that names the version, quotes none
of the bytes, and is recorded `error`. One such row fails a whole history
page. After a commit, the readback's `500` says the commit succeeded and names
the version, so a caller does not retry into a duplicate. Evidence, in
`openehr-axa/tests/http.rs`: read, vread and history each `500` with
`error` recorded; a tombstone in a history page is still `data: null`; the
readback case uses a trigger that corrupts the row as it is inserted. Negative
control run: restoring `.ok()` made the unreadable-read test answer `200`.
**Not exercised:** an unreadable `EHR_STATUS` row. It uses the same `view`,
but has no test of its own.

### D-29 — Some read outcomes are recorded before the outcome is known — **Medium, fixed** (2026-11-26, not yet committed)

**Claimed.** `db:PR12.23` requires distinguishable outcomes for failed reads.
**Found by source inspection at `ec029cf`.** `composition::history` and
`composition::search` record the store lookup before `Paging::resolve`
can refuse the request. `ehr_status::read` returns directly on a missing
parent EHR before reaching `record_read`. The audit can therefore show a
success for a rejected page or omit an authenticated missing-record lookup.

**Acceptance.** Resolve the response outcome before appending its audit record
and release clinical content only after the append succeeds. Exercise a
successful read, a rejected page, a missing parent, an unreadable row, and
an unavailable log. Compare response and recorded outcome on each route.

**Fixed 2026-11-26.** The composition read, vread, history and search
handlers, and both status handlers, now build the whole response first: the
lookup, paging, and decoding. `controllers::recorded` then derives the
outcome from the response's own status (`outcome_of_status`: `ok`,
`not_found`, `gone`, `refused`, `error`), appends it, and only then
releases the response. If the append fails, the caller still gets `503` and
nothing is released. `ehr` and `query` keep `outcome_of`, because nothing
after their record step can refuse. Evidence: a refused `_count` on history
and on search is recorded `refused` (it was `ok`), a status lookup under a
missing `EHR` is recorded `not_found` (it was not recorded at all), the
unreadable cases above are recorded `error`, and the existing unavailable-log
test still passes. The status read's access-record target is now the literal
`ehr_status` rather than the container its reference names. A refused read
must not log a container the caller never named or could see. **Behaviour
change for log consumers.**

### D-30 — Invalid optional environment values disable configured protections — **Medium, fixed** (2026-11-26, not yet committed; Compose not run)

**Claimed.** Startup must initialize a configured recorder before opening
the store (`db:PR12.6`). `db:PR12.17` describes binding verification to a
configured audience. **Found by source inspection at `ec029cf`.**
`AccessLog::from_env` treats every environment read error as auditing off.
`PasetoVerifier::from_env` uses `.ok()` on the issuer, audience, and implicit
assertion. A present value with invalid Unicode is treated as absent.
There is also a deployment mismatch in the opposite direction. Compose's
default substitution for the optional audience sets an empty string, which
the verifier passes on as a configured audience. The documented throwaway
token supplies no audience.

**Fixed 2026-11-26** against the new `PR12.24`. A new module,
`openehr_axa::settings`, has one reader per kind of setting. `claim` treats
absent or empty as unset and refuses a value that is not Unicode, naming the
setting without quoting it. `path` treats absent as off and refuses an empty
value; otherwise it is any OS path. `AccessLog::from_lookup` and
`PasetoVerifier::from_lookup` take the lookup, and `from_env` passes
`std::env::var_os`. Evidence:
- Unit tests of the four states.
- The Compose default (an empty audience) accepts a token with no audience.
- A configured audience read from settings still refuses another audience.
- A non-Unicode issuer, audience or implicit assertion is `ConfigError::Setting`.
- `tests/startup.rs` runs the real binary: an empty `OPENEHR_ACCESS_LOG`
  and a non-Unicode `OPENEHR_PASETO_AUDIENCE` each exit non-zero, naming the
  setting.

Negative control run: with an empty path read as "off", the service started
and served until it was killed. **Not run:** the documented Compose stack with
a protected route (needs podman).

**Acceptance.** Distinguish absent, empty, invalid, and configured values.
Read paths as OS paths or explicitly refuse unusable values at startup.
Keep the optional claim constraints active when configured. Test environment
parsing through child processes and exercise a protected route with the
documented Compose defaults and with an explicitly bound audience.

### D-15 — PostgreSQL connected without TLS — **High, fixed**

**Found 2026-11-19.** `PostgresqlStore::connect` calls
`Client::connect(params, NoTls)`. A remote server that accepts plaintext
can receive record contents over an unencrypted connection. The earlier
`O10.7` and `O10.18` matrix entries said that no crate opened a network
connection. The PostgreSQL constructor now does. This is a source finding.
The store's CI conformance run establishes database behavior and supplies
no evidence of transport encryption.

**Fixed and verified 2026-11-19.** Both public constructors now force TLS and
verify the configured server name and certificate chain through Rustls.
`PostgresqlStore::connect` uses platform roots; `connect_with_ca` replaces them
with a caller-supplied PEM CA and rejects an empty or malformed bundle before
opening a socket. `scripts/verify-store.sh` provisions a TLS-enabled PostgreSQL
18 server, an unrelated CA, and a plaintext-only server. The public connector
successfully queries the correctly named trusted server and rejects the wrong
hostname, unrelated issuer, and plaintext-only endpoint. The same live run
passes all 17 store tests and both concurrency tests.

### D-16 — A historical read can return a later commit — **High, fixed**

**Reproduced 2026-11-19 on SQLite and PostgreSQL 18.7.** Create a fresh store, create the
sample EHR and a contribution, then commit two successive versions at
`2026-08-01T09:05:00.100Z` and `2026-08-01T09:05:00.900Z`. Calling
`Store::version_at_time` at `09:05:00.500Z` returned version 2 with the
original `.900Z` timestamp. Only version 1 was current (`H5.13`).

`StoredInstant::from_date_time` projects both times to identical whole
seconds. SQLite breaks the resulting tie by descending trunk version.
PostgreSQL uses the same projection and tie-breaker. The same calls through
the public `Store` trait returned version 2 on both an in-memory SQLite
database and a disposable PostgreSQL server.
The original lexical times are available for a precision-preserving fix.

**Fixed 2026-11-19 without a schema change.** The UTC column still bounds the
query through `ix_version_time`. The stores fetch candidates in the query
second and the latest occupied earlier second, then the shared selector parses
the authoritative text and applies the exact inclusive cutoff. Existing rows
already retain every fractional digit in that text, so they need no migration.

The shared conformance test runs against live SQLite and PostgreSQL stores. It
covers `.099`/`.100`/`.500`/`.900` boundaries, fractions beyond milliseconds,
comma and full-stop decimal signs, equivalent offsets across midnight, equal
instants with different lexical precision, a higher trunk committed earlier,
and an unestablished local query. A read-only probe over the database used to
reproduce the defect returned version 1 at `.500` after the fix. Mutation
testing of the shared SQL builder and selector caught all four viable mutants;
one replacement was unbuildable, with no survivors or timeouts.

### D-17 — Nullable branch columns defeated tree-position uniqueness — **High, fixed**

**Found 2026-11-19 by closing `D-14`'s two-system race gap.** Eight writers
with distinct `creating_system_id`s all committed trunk version 2 to the same
container. Their full version UIDs differed, so the primary key admitted them.
The unique `ix_version_container_trunk` index also admitted all eight because
`branch_number` and `branch_version` were `NULL`, and conventional unique
indexes do not treat two `NULL` values as equal on SQLite or PostgreSQL.

This violated `H5.4`, `H5.8`, and `H5.10`: the schema allowed multiple versions
at one position in the version tree precisely when independent systems raced.
The earlier concurrency test used one creating system, so every losing writer
reused the same full UID and exercised only the primary key. Its passing result
was not evidence for the tree-position index it claimed to cover.

**Fixed 2026-11-19.** The two SQL branch columns are required and encode an
absent trunk branch as zero, outside the positive-integer version-tree domain.
Store reads decode zero back to `None`, preserving the Rust model. Schema
version 5 refuses older databases rather than silently retaining the ineffective
constraint. The generated DDL for all six engines carries the non-null columns.
The schema verifier passed on PostgreSQL 18, MySQL 8.4, MariaDB 11.4, and
Oracle Free; SQLite's generated DDL and complete store suite passed locally.
SQL Server's generated DDL is golden-checked but was not live-run on this arm64
host, where the repository already documents SQL Server 2022's qemu crash.

The exact regression failed before the fix with eight winners. It now produces
one winner and seven `CommitError::NotLatest` refusals on SQLite and PostgreSQL
18, with both complete store suites passing against live databases. A focused
SQLite mutation run over conflict classification and branch decoding caught all
five viable mutants; two replacements were unbuildable and none survived.

### D-18 — AQL treated different scalar kinds as equal — **Medium, fixed**

**Found 2026-11-19 while adding owner-crate coverage to `openehr-store::aql`.**
The internal sort comparator deliberately returns `Equal` for different scalar
kinds to provide a stable total ordering. `cells_eq` reused that comparator
without first checking type compatibility, so a string such as `"1"` compared
equal to integer `1` and boolean `true`. The same leak made mixed-kind `>=` and
`<=` predicates true. A bounded AQL query could therefore return rows that did
not satisfy its `WHERE` clause.

**Fixed and verified 2026-11-19.** `compare` now recognizes strings, booleans,
and the numeric family explicitly. Integers and real numbers compare by numeric
value; different families produce false for equality and ordering, true only
for inequality. Direct owner-crate tests exercise all six comparison operators,
both integer/real directions, every same-kind family, null, and the unsupported
operator refusal. The mixed string/integer assertion fails against the old
implementation.

### D-21 — AQL results are served without a read-audit record, and metadata says AQL is absent — **High, fixed**

**Found 2026-11-25 by an external clinical-safety audit (source inspection),
reproduced here.** With read auditing on (`records_reads: true`),
`POST /openehr/v1/query/aql` against the `openehr-axa` test fixture answered
**200 with the composition's rows** and the access log held **0 lines**. The
handler binds its principal as `_principal` and never calls `record_read`; its
own doc comment concedes "a query's *result* is content" and then excludes it
from the record on the reasoning that the query *text* is not. The promise
(`db:PR12.5`, `PR12.6`: an access event before clinical data is returned) is
about what is returned.

The same run's `GET /openehr/v1/metadata` still lists **"AQL execution" under
`not_implemented`** (`db:S1.6`) while the executor is served, so the capability
statement is wrong in the opposite direction at the same moment.

`every_read_route_records_who_read_what` iterates a fixed list of five routes
that omits the query route, so the guard that was written to catch exactly this
omission has the same blind spot as the code (rule 3 of `CLAUDE.md`: a guard is
only as wide as its input list).

**Open until** the query handler records resolved EHR, actor, action and
outcome *before* releasing rows, without logging literals or result values; a
log failure refuses the query as it does for every other read; the guard test
enumerates routes from the router rather than from a list; and the metadata
capability list is derived from what is served.

**Progress 2026-11-25 (not committed).** Three of the four conditions are met.
`POST /query/aql` now records `query` / actor / resolved EHR / outcome before
the rows are released, via `openehr_store::aql_scoped_ehr` (so the EHR is taken
from the query by the owner module's own rule, not re-parsed in the service);
it records a refused query as `refused`; it answers 503 and drops the rows when
the log cannot be written; and neither the query text nor a row value reaches
the record. `AQL execution` is gone from `not_implemented`. Four tests in
`openehr-axa/tests/http.rs` cover those, and the same conditions produced 0 log
lines before the change. **Still open:** `every_read_route_records_who_read_what`
still iterates a hand-written list, so the next route added without a
`record_read` is missed in the same way; it should enumerate the router. The
finding stays open until that is done.

**Closed 2026-11-25 (not committed).** The guard is now
`every_mounted_route_is_either_a_recorded_read_or_exempt_with_a_reason`: it
takes the set of routes from `openehr_axa::app::routes().mounted()`, requires
every one to be classified in a table (a content-returning `GET`/`POST` with the
action it records, or an exemption **with its reason**), fails naming any route
that is mounted and unclassified, and then drives every `Records` route and
asserts exactly one new access record with the right action. Two negative
controls were run: deleting the `record_read` call from the AQL handler fails
four tests (that guard, with "left no access record", and the three AQL tests);
deleting the AQL entry from the table fails it with "mounted but not
classified". It also covers `EHR_STATUS` and its vread, which the old five-route
list did not. The old test remains, narrower. The committer-display-name half
of the audit's finding is **not** part of this entry and is still open in
`tasks.md`.

**Related, not reproduced:** the audit also says the committer's stored row
keeps only an optional display name, so two clinicians with the same name are
indistinguishable later (`PHI.md` claims). Not checked here; it stays in
`tasks.md`.

### D-20 — An amended composition appears once per version in an AQL result — **High, open** (fix landed, CI pending)

**Found 2026-11-25 by an external clinical-safety audit, reproduced here on
SQLite.** One `COMPOSITION` container, committed and then amended twice,
queried with `SELECT c/name/value … CONTAINS COMPOSITION
c[openEHR-EHR-COMPOSITION.encounter.v1] ORDER BY c/name/value LIMIT 10`
(the same shape as `conformance::run_aql`):

| after | rows returned |
| --- | --- |
| version 1 | `Encounter 1` |
| version 2 | `Encounter 2`, `Encounter 2` |
| version 3 | `Encounter 3`, `Encounter 3`, `Encounter 3` |

**Cause.** `openehr_composition_index` is keyed by `version_uid` and gains a row
per content-bearing version. `aql::execute` takes every candidate from
`find_compositions_by_archetype`, resolves each to its *container's current
version*, and emits a row each time. The comment at that site says the
container "still has exactly that one row"; that is false once a second
content-bearing version exists. `run_aql` commits three single-version
containers and one deleted one, so the shared suite never saw it.

**Consequence.** A worklist counts one encounter several times, and repeated
rows displace other records from a `LIMIT` page. A second, separate defect
follows from the same shape and is **not yet reproduced**: a candidate whose
*current* content no longer has the queried archetype (an amendment that
changed it) still matches through its old index row, because the archetype is
checked against the index, not the current version.

**Progress 2026-11-25 (not committed): fixed on SQLite and the in-memory owner
store; PostgreSQL not run.** `aql::execute` now answers once per container (a
`HashSet` of container ids, **not** a value comparison) and checks the requested
archetype against the container's *current* content. The shared suite
(`conformance::run_aql`, appended last so earlier row counts are untouched) gains
the story below. It fails on real SQLite with the dedupe removed (`SQLite: a
container amended twice must answer once, as its current content`) and passes
with it. Three owner-crate unit tests cover a container amended twice, two
containers with one display name, and a stale index row. An existing test that
"proved" a two-row sort by pushing a clone of the *same* container twice was
itself the duplicate and was rewritten to use two containers. The coverage
exclusion pins in `coverage-exclusions.tsv` moved one line (hashes re-checked).

**Story, as it runs:** a container amended twice (`Amended 1/2/3`) → `WHERE =
'Amended 3'` is 1 row; `'Amended 1'` and `'Amended 2'` are 0; two containers
both named `Twin` → 2 rows. **Not verified:** PostgreSQL (needs a server;
`openehr-postgresql/tests/store.rs` runs `run_aql` and so will exercise it in
CI); the amendment that *changes archetype* against a real engine (only the
double covers it); a withdrawal of an amended container.

**Open until** each distinct container is resolved once, its *current* content
is checked against the requested archetype, and only then are filter, order and
paging applied — **not** by deduplicating on displayed value, which would
merge two real encounters with the same name. The shared suite gets a story
with an original, two amendments, a second encounter with the same display
name, and a withdrawal, and SQLite and PostgreSQL both pass it.

### D-22 — The verified committer's identity is checked and then not stored — **High, fixed** (2026-11-26, not yet committed)

**Found 2026-11-25 by an external clinical-safety audit (source inspection),
reproduced here.** `openehr-axa` refuses a commit whose `AUDIT_DETAILS`
committer does not match the verified token subject (`PR12.19`,
`check_attribution`). That check is the only place the committer's *identifier*
is used. What is persisted is `VersionRow::audit_committer_name`, filled by
`party_name(...)`: the party's optional display name, `NULL` for an anonymous
`PARTY_SELF`. No column holds the identifier, and `data_json` is the composition
alone.

**Reproduction.** A commit made as `clinician-4417` was answered 201; the read
of it returned keys `uid, versioned_object_uid, lifecycle_state_code,
is_deleted, time_committed, chain_digest, data` and no committer field of any
kind; the subject appeared nowhere in the body. In the store the only trace is
the display name. Two clinicians called "Dr A Nurse" with different identifiers
leave identical rows (inferred from `party_name`; the two-clinician case was not
run).

**Why High.** The log of reads names the actor (`PR12.22`); the record of
*writes* — the one an investigation of a wrong entry starts from — does not,
and a record that was never stored cannot be reconstructed later. The
`CONTRIBUTION` and the version chain are append-only and tamper-evident, which
makes the omission permanent for every commit made before a fix.

**Open until** a stable committer identifier is persisted with each version and
contribution (separately from the display name, with the verified actor and the
*declared* author kept distinct, since they can legitimately differ — a service
acting on a clinician's behalf), is returned by the version views, and a test
commits as two clinicians with the same display name and reads both back after a
restart. Treat the identifiers as sensitive in the audit store. Needs a schema
change and a migration story (`db:O10.14`), so it is a design decision, not a
patch; it also changes `SCHEMA_VERSION`.

**Fixed 2026-11-26** against the new `PR12.25`, as schema 6 (`SCHEMA_VERSION`
5 → 6). `audit_committer_json` holds the whole committer `PARTY_PROXY` as
canonical JSON, required on `openehr_version` and `openehr_contribution`.
The version view returns it as `committer`.
- **Verified actor and declared author** are one value here, deliberately.
  `openehr-axa` refuses every write in which they differ (`PR12.19`, and
  `D-25` below), so there is nothing distinct to keep. A service acting for
  someone else would need a second column, and `PR12.25` says so.
- **Evidence.** `openehr-sqlite/tests/attribution.rs` commits as two
  "Dr A Nurse"s with different identifiers, reopens the file, and reads back
  two distinct committers on both the versions and the contributions.
- **Migration.** A schema-5 database is refused (`O10.15`) and, before 1.0,
  exported and reloaded (`O10.14`). Its rows hold no identifier to recover,
  and `PHI.md` now says so.
- **Not run:** PostgreSQL (implemented, compiled), and `verify-schema.sh` on
  real engines for the new DDL (needs podman).

### D-19 — A read route serves content under an EHR that does not own it — **High, fixed** (PostgreSQL run pending)

**Scope clarification at `ec029cf`.** The current and as-of status routes follow a stored reference whose ownership still needs checking. That residual is tracked separately in `D-23`.

**Found 2026-11-25 by an external clinical-safety audit (source inspection),
then reproduced here.** `openehr-axa`'s composition read routes take
`{ehr_id}` in the path and use it only to write the access record. The lookup is
by container or version identifier alone (`latest_version(&container)`,
`all_versions(&container)`, the vread version id). Nothing establishes that the
returned row belongs to `{ehr_id}`.

**Reproduction (`openehr-axa/tests/http.rs` fixture, one EHR holding one live
composition).** `GET /openehr/v1/ehr/6BA7B810-9DAD-11D1-80B4-00C04FD430C8/…`,
an EHR id that exists nowhere, answered **200 with the composition** for all
three of `…/composition/{uid}`, `…/composition/{uid}/_history` and
`…/composition/{uid}/version/{version_uid}`. A scratch test printed the status
and the first bytes of each body, and was removed rather than committed
because it passes by asserting the defect.

**Why it matters.** A clinician authorised for one chart who opens a
composition id under it sees another chart's content, and the read audit
records the *requested* EHR, so the log says the wrong thing. Chart ownership
and the clinical subject of an entry are different (a mother's condition in
her child's chart is correct); this is about the former.

**The other routes, checked 2026-11-25 the same way** (scratch tests,
removed):

- `GET …/ehr/{nonexistent}/ehr_status/{version_uid}` with a **composition**
  version id answered **200 with the composition**: the `EHR_STATUS` vread does
  `get_version(&id)` with no EHR scope and **no resource-type check**, so it is
  also a type confusion, and the same request under the *real* EHR's
  `ehr_status` also served the composition. The `GET …/ehr_status` current
  read is different: it resolves the EHR's own status container first.
- `PUT …/ehr/{nonexistent}/composition/{uid}` with a correct `If-Match` was
  refused `404 no such ehr`, because `commit_composition` checks the EHR
  exists. **Not established:** whether it refuses a *second real EHR* that
  does not own the container — that needs a two-EHR fixture, which does not
  exist yet. The `If-Match` precheck itself reads the container unscoped.

**Fixed 2026-11-25 (not committed), design chosen by the maintainer:** scoped
lookups (`latest_version_in`, `all_versions_in`, `get_version_in`), built on one
new required primitive `Store::versioned_object_owner`, specified as `H5.18` and
`H5.19`. **A breaking change to the `Store` trait**, accepted for the staged
0.11.0. A mismatch of EHR or of `rm_type` is the same `NotFound` an absent
container gets (compared in the shared suite, and over HTTP).

**A worse hole than the audit reported, found while fixing it.**
`commit_composition` read its `ehr_id` argument *only when it created a
container*; for an existing one it was ignored. A commit addressed to the wrong
EHR was therefore appended to the right container. `H5.19` now refuses it before
anything is written. Negative control: with the commit check disabled the shared
suite fails on real SQLite with "a commit to a container another EHR owns must
be NotFound, got Ok(())".

**Service.** `openehr-axa` now uses the scoped lookups for composition read,
`_history`, vread and the `PUT` precondition, and for the `EHR_STATUS` vread
(which also had no type check). Composition vread additionally requires the
version to belong to the container the *path* names. Five new HTTP tests, one
negative control (reverting `read` to the unscoped lookup fails two).

**What was verified:** SQLite (real), the owner-crate store, and the HTTP layer.
**Not verified:** PostgreSQL (`run_ehr_scope`, `#[ignore]`d, needs a server;
the code compiles and lints); `openehr-mysql` and the other dialects have no
`Store`. **Not done:** a second *real* EHR over HTTP (the HTTP fixture has one;
the shared suite covers two). **Not addressed:** authorisation, meaning whether
a verified caller may read *this* EHR at all, which is still the deployment's
(`db:PR12.18`); this closes the case where the path names the wrong chart.

**Original open condition, kept for the record:** the store offers a lookup scoped by EHR, container and expected
type; every read and the amendment target use it; and a two-chart test shows a
mismatched selection returns no content (and the same `404` an absent record
gets), with a clinician authorised for both still served. See `tasks.md`,
"Audit intake 2026-11-25".

### D-01 — No engine crate has a dialect annex — **Medium, fixed**

**Required.** `X15.6` requires every engine crate to carry
`spec/14-<engine>-dialect.md`, addressing nine subjects explicitly and by name,
with "not applicable" acceptable and silence not.

**Found.** All six `spec/` directories are empty. Every dialect decision — the
`ColTy` bindings, the idempotence declarations, the append-only mechanism, the
engine floor — is documented only in rustdoc, where it is not reviewable as a
diff against the core and where a departure cannot be declared as a numbered
`M14.x` requirement (`C0.14`).

**Consequence.** Undeclared departures are indistinguishable from oversights
(`C0.16`). `openehr-oracle` has an undeclared engine floor — identifiers were 30
bytes before Oracle 12.2 and 128 after, so the schema's names are only safe on
12.2+ and nothing says so.

**Fixed 2026-08-01.** All six annexes written, each addressing the nine
subjects `X15.6` names, and each carrying status **proposed** (`X15.9`) — so
none may be cited as evidence for a conformance level.

Writing them surfaced things that had not been written down anywhere:

- **Oracle's engine floor is 12.2**, and now for a stated reason. Identifiers
  were 30 bytes before 12.2 and 128 after; several generated names here exceed
  30, so the schema is not installable below it. That closes the open row in §1.
- **Four departures** now exist as numbered `M14.x` requirements where before
  they were undeclared: PostgreSQL and SQLite discard the `Id`/`Text` length
  bound (`M14.1`, `M14.5`); MySQL's drop-then-create trigger leaves a window
  (`M14.3`); Oracle cannot `=`-compare a `CLOB` and would need
  `DBMS_LOB.COMPARE` to satisfy `M3.42` (`M14.8`). `C0.16` calls an undeclared
  departure a defect; these are now declarations.
- **SQL Server's and Oracle's missing live runs** are recorded as `M14.6` and
  `M14.7` rather than left as an absence.

**Residual.** All six are **proposed**, not ratified. `X15.9` requires a
ratified annex before it counts as evidence, and ratification for the two
Dialect-level crates needs a live run that no available machine can provide.

**Update, 2026-09-06.** That last sentence was true of the machine available
when it was written, not of every machine: `gvenzl/oracle-free` is a public,
unauthenticated image, and `openehr-oracle`'s live run now exists — `M14.7` is
met, and the crate is **Schema** (`spec/databases/conformance-matrix.md`, the
one file that owns a level, `W0.40`). Its annex stays **proposed**; ratifying
an annex and reaching Schema level turned out to be independent — the three
already-Schema engines' annexes are proposed too.

`openehr-mssql` remained Dialect past that point for a different reason than
this paragraph expected: not the missing arm64 image (real, and still true —
this machine still cannot run the branch locally), but a genuine defect the
attempt to get a live run on CI's x86_64 runners found once the verification
script itself stopped failing before the DDL could even be tried — see
**D-12**, now fixed and observed green. `openehr-mssql` is **Schema**.

### D-12 — SQL Server refused its own generated trigger — **High, fixed**

**Required.** `T11.2`/`M14.6`: the DDL a dialect emits must be executable by
the engine it names, not merely parseable by this crate's own golden tests —
exactly the gap `openehr-store/spec/conformance.md` names Dialect as
deliberately not closing.

**Found**, running `openehr-store/scripts/verify-schema.sh mssql` against a
real SQL Server 2022 container in CI: `Msg 111, Level 15, State 1 ... 'CREATE
TRIGGER' must be the first statement in a query batch.` `MssqlDialect` joins
every statement with a bare `;` (the shared default `Dialect::terminator`),
so the whole script is one batch, and `CREATE OR ALTER TRIGGER` — emitted
last, in `append_only_sql` — is never first in it.

The rule was already known and already handled for `CREATE TABLE`/`CREATE
INDEX`: `MssqlDialect::guard`'s own comment says so ("`CREATE TABLE` must be
the first statement in its batch, so it cannot appear directly inside `IF
... BEGIN`") and works around it by wrapping the statement inside
`EXEC('…')`, which sidesteps the batch-position rule entirely — the literal
`CREATE TABLE` text is only a string argument there, not a statement in the
outer batch. `append_only_sql` has no such wrapping, so the same rule that
was solved once was never applied to the one statement kind that still hits
it.

**Consequence.** Nothing about this defect could be seen without running the
DDL against a real server: `conformance::check_dialect` and every golden
test in `tests/main.rs` assert what the emitter produces, and every one of
them passed while the produced script was one SQL Server would reject
outright, at the very last statement, after every prior one succeeded.

This was masked for five rounds of `verify-schema.sh` fixes by an unrelated,
genuinely real bug in the script itself (`M14.6`'s CI attempt): `apply`/`seed`
delivered SQL to `sqlcmd` via `-i "$path"`, a path that exists on the CI
runner but not inside the container `sqlcmd` runs in via `podman exec`, so
the DDL was never actually submitted at all across four rounds, and the
resulting `Sqlcmd: '<path>': Invalid filename.` matched none of the error
patterns the script checked for. Once that was fixed — piping through stdin
instead, matching how the postgresql/mysql/mariadb branches already do it —
the DDL reached a real server for the first time and Msg 111 is what came
back.

**Fixed** by giving `MssqlDialect` its own `terminator()`, `"\nGO"` instead
of the shared default `";"` — the same category of override
`OracleDialect::terminator` already makes for its own batching rule
(`\n/` for SQL*Plus). `GO` is a `sqlcmd`/SSMS client directive, not T-SQL
proper, and harmless between statements that did not need separating; every
statement including the trigger is now the sole content of its own batch. A
regression test,
`the_trigger_is_the_first_statement_in_its_own_batch`
(`openehr-mssql/tests/ddl.rs`), asserts nothing but blank lines sit between
the last `GO` and `CREATE TRIGGER`.

**Confirmed** in CI run
[34045294037](https://github.com/openehr-rust/openehr-rust/actions/runs/34045294037),
job `schema / mssql`, 2026-09-06 — one further, unrelated script fix was
needed to get there (`sqlcmd`'s own column padding made a byte-exact JSON
comparison fail; fixed with `-W`, no dialect change). `openehr-mssql` is
**Schema** (`spec/databases/conformance-matrix.md`, the one file that owns a
level, `W0.40`).

This is the fourth real DDL defect the same gap has found across the four
crates that have now had DDL run against a real server for the first time:
`A-13`, `A-14`, `A-15` for PostgreSQL and MySQL, and this one for SQL
Server. `openehr-oracle` alone had none — its DDL held up on first real
contact, and every failure attempting to verify it was in the script.

### D-02 — Two store requirements were unverifiable as written — **Medium, fixed**

**Required.** `R4.5` (a multi-row read sees one snapshot) and `H5.4` (concurrent
commits produce one winner).

**Found.** `openehr-sqlite` reads inside a transaction and the unique index of
`H5.10` exists, so both are plausibly satisfied. **Nothing exercises either.**
No test in this repository runs two threads against one store.

Both are recorded as `?` in the [conformance matrix](conformance-matrix.md)
rather than `•`, which is the correct handling (`C0.20`) and not a fix.

**Fixed 2026-08-01.** `openehr-sqlite/tests/concurrency.rs` drives both, with a
**file** database and a connection per thread — an in-memory database is private
to its connection, so a concurrent test against `in_memory()` would run N
independent databases and pass without testing anything.

- `R4.5` — a reader loops against a writer committing 24 versions and asserts
  every version it can see has its index row visible too. **Passed first time**;
  the commit transaction of `R4.4` was already doing its job.
- `H5.4` — eight writers race for one position in a version tree. Exactly one
  wins. **This one failed**, and is recorded as `D-06`.

### D-06 — A concurrent commit refusal was reported as an engine error — **Medium, fixed**

Found by the test that closed `D-02`, which is the point of writing it.

**Found.** Eight writers racing for one position in a version tree produced
exactly one winner — so the guarantee held, enforced by the unique index of
`H5.10`. The seven losers received:

```
StoreError::Engine("SQLite: UNIQUE constraint failed: openehr_version.uid")
```

**Why that is a defect and not cosmetic.** `H5.9` requires the commit refusals
to be **distinguishable by the caller**. A caller told `Commit` knows another
writer took the position and can re-read the head and retry. A caller told
"UNIQUE constraint failed" knows only that something went wrong — and cannot
tell it from a corrupt schema, a disk error, or a bug. A version tree is
precisely where guessing is not allowed.

It is only reachable under concurrency: the single-threaded path checks the
commit rules before inserting, so both writers pass the check, and only the
database catches the second. Every existing test drove one store from one
thread, which cannot distinguish "the rules hold" from "the rules hold when
nothing else is happening".

**Fixed.** The SQLite store now translates a uniqueness violation into the
refusal it is, and the two indexes mean different things:

| Index | Means | Maps to |
| --- | --- | --- |
| `openehr_version.uid` | the same version identity committed twice | `CommitError::DuplicateVersion` |
| `ix_version_container_trunk` | a *different* identity took that position | `CommitError::NotLatest` |

**Residual.** Only SQLite is fixed and only SQLite is tested — it is the only
crate with a `Store`. The other five engines will need the same translation when
they gain one, and their drivers report constraint violations differently. The
requirement is `H5.9`; the shared conformance suite cannot check it until a
second store exists.

### D-07 — The store silently dropped four VERSION attributes — **High, fixed, properly**

Found by reading the openEHR **BMM** for RM 1.1.0 rather than the code, which is
what `D-05` asked for. The rendered specification pages omit every class table —
they `include::` them from a UML export — so
`specifications-ITS-BMM/components/RM/json/openehr_rm_1.1.0.bmm.json` is the
machine-readable source of record.

**Found.** RM 1.1.0 gives `VERSION`, `ORIGINAL_VERSION`, and `AUDIT_DETAILS`
attributes the `openehr` crate models as struct fields and the store persists
**nowhere**:

| Attribute | RM type | Modelled? | In the schema? |
| --- | --- | --- | --- |
| `AUDIT_DETAILS.description` | `DV_TEXT` | yes | no |
| `ORIGINAL_VERSION.other_input_version_uids` | `OBJECT_VERSION_ID` | yes | no |
| `ORIGINAL_VERSION.attestations` | `ATTESTATION` | yes | no |
| `VERSION.signature` | `String` | **only on `IMPORTED_VERSION`** | no |

All are optional in the BMM. **Optional is not droppable.**

The fourth row was a different defect and belonged to the library. A first pass
of this finding recorded `VERSION.signature` as modelled and dropped; checking
the field's owner before writing the fix showed it sat on `ImportedVersion`
alone, while the BMM puts `signature` on `VERSION`, which `ORIGINAL_VERSION`
inherits — so a locally created version could not be signed at all. Split out as
`lib:A-18` and since **fixed**: `OriginalVersion` now has the field, a
`with_signature` builder, an accessor, and a round-trip test.

Closing `A-18` made the fourth row real. A signature can now exist, and this
schema still has no column for it — so `refuse_unpersistable` rejects it too,
and the count here is four.

`data_json` holds the version's *content* — the `COMPOSITION` — not the `VERSION`
envelope, which is decomposed into columns (`M3.19`, `R4.8`). So an attribute
without a column has nowhere to go, and `commit_composition` returns `Ok`.

**Why this is High.** Two of the four are legally meaningful. An `ATTESTATION` is
a clinician asserting that content is what they signed off; `VERSION.signature`
is the signature over the version. A store that accepts an attested version,
returns success, and cannot give the attestation back has lost the part of the
record that made it evidence — silently, with no error and no documentation
saying so.

`AUDIT_DETAILS.description` is the free-text reason for a change: "corrected
after telephone call with the lab". It is often the only thing that explains why
a correction exists.

**What it does and does not violate.** `R4.2` requires an openEHR object written
and read back to equal the original. The `Store` trait does not offer read-back
as a `Version<T>` at all — `get_version` returns a `VersionRow` — so the store
never *claims* the round-trip for the envelope. The defect is therefore not a
broken promise so much as an **accepted input that can never be returned**,
undocumented, and reported as success. `S1.11` says an operation this layer does
not implement must refuse rather than return a silent success; that is the rule
being broken.

**Two remedies, and the choice is not mine to make quietly.**

1. **Refuse.** `VersionRow::project` rejects a version carrying any of the four,
   with `StoreError::Unsupported`. No schema change, no migration, and silent
   loss becomes an explicit refusal. It is a **behaviour change on published
   crates**: commits that succeed today would start failing.
2. **Persist.** Four columns, an `Attestation` needs its own table or a JSON
   column, and there is no migration mechanism (`O10.14`) for the eight crates
   already on crates.io at 0.1.1.

**Fixed by remedy 2 — the columns.** Refusal was the smaller evil and shipped
first, in 0.2.0. The schema now has the columns instead: `audit_description`,
`signature`, `attestations_json`, and `other_input_version_uids_json`. An empty
collection stores `NULL` rather than `[]`, because "not a merge" and "merged
nothing" are the same fact and SQL has one way to say it.

Verified against PostgreSQL 18, MySQL 8.4, and MariaDB 11.4: parses, idempotent,
append-only enforced, row intact. The seed in `verify-schema.sh` failed first —
it omitted the new `NOT NULL` chain columns — which is the check doing its job.

*Superseded text:* `VersionRow::project` refused a version carrying any
of the four, with `StoreError::Unsupported` naming the attribute and citing this
finding. It sits in the **shared** projection, so all six engines inherit it
rather than each needing the same check (`M3.35`).

Two tests: one asserting an audit description is refused, and a control
asserting a version without one still projects — because a refusal that rejected
everything would be indistinguishable from a broken projection (`T11.10`).

**This is a behaviour change on published crates.** Commits that succeeded at
0.1.1 and silently lost an attestation will now fail. That is the intended
direction: a caller told `Unsupported` can act, and a caller whose attestation
vanished cannot.

**Residual.** Refusing is the smaller evil, not a good outcome. openEHR permits
these attributes and this store still cannot hold them; remedy 2 — columns, and
a table or JSON column for attestations — remains the real fix, and needs a
migration mechanism this project does not have (`O10.14`).

### D-10 — three fuzz properties that could be deleted without failing anything — **Medium, fixed**

**Found 2026-08-21** by the retrospective mutation pass `spec/audit.md` **W-18**
left as a residual.

`check_projection`, `check_stored_instant`, and `check_verify_versions` are the
properties the `openehr-store-fuzz` targets drive. Each was **replaceable with
`()`** with nothing in this repository failing.

**Why that is worse than an ordinary coverage gap.** These functions are called
only from fuzz targets, and `cargo test` does not run fuzz targets. A property
that asserts nothing never crashes, so the `fuzz` job stays green — over three
properties that had been deleted. The gate and the thing it gates would both
have reported success.

That is `T11.10` exactly, and `W0.28` states it as a requirement: *a fuzz
property MUST be shown to fail against a deliberately broken implementation. A
check that cannot fail is indistinguishable from a control that works.* The
properties were written on 2026-08-20 and that demonstration was never done.

**Fixed, in three parts, and the third is a limitation rather than a fix:**

1. **`check_verify_versions` was too weak to fail at all.** Its assertions —
   empty means `Empty`, and the verdict is a function of its input — hold for
   any input given a correct `verify_versions`. It now **provokes** the answer
   it is about: for a history that verifies, editing each version's content in
   turn must make the chain report `ContentAltered` at that version. That is the
   entire purpose of `M3.16`, and it is falsifiable by construction rather than
   by imagination.

2. **`conformance::property_tests` now calls all three**, with input that is
   right and input that is not. `check_stored_instant`'s failing input is
   constructible — a `StoredInstant` whose derived UTC column disagrees with its
   authoritative text, which is the defect `M3.31` names — and the test builds
   one and asserts the property rejects it. That mutant is killed.

3. **The other two now report what they checked**, which is what makes them
   killable. `check_projection` returns whether the composition projected;
   `check_verify_versions` returns how many versions had their tamper detection
   provoked. `property_tests` asserts the count equals the history's length, so
   a stubbed-out property fails a real test.

   **A `mutants.toml` skip was written first and then deleted.** It was the
   weaker answer: a skip says *this cannot be checked*, and a return value makes
   it checkable. The counts are not an artifact of the testing either — the
   second distinguishes "verified a hundred-version history and re-broke every
   one of them" from "returned early", which the fuzz target could not otherwise
   tell apart.

   Asserting only the `true` case left `check_projection -> bool` replaceable
   with `true`, so the test asserts the other answer too: a composition with no
   `archetype_details` has nothing to put in `archetype_id` and is refused,
   which is a **result** and not a failure.

**Residual.** Three mutants in `dialects_are_distinct` and `check_dialect`
survive, unrelated to this finding and pre-dating it: those helpers are killed
when measured from `openehr-sqlite`, whose tests call them, and the library
conformance matrix's `T13.2` row already records that `openehr-store`'s own test
target does not.

### D-09 — The conformance matrix contradicted itself — **Medium, fixed**

Found by comparing the matrix against the specification it summarises, after a
day of changes had landed under it.

**Found.** The file that says *"Where it disagrees with a crate's documentation,
this file is the one to trust"* disagreed with **itself**:

- `M3.39` and `M3.42` were marked **•** in the store-level table — "digest is
  SHA-256, 32 raw bytes", with `ColTy::Digest` named as evidence — and listed
  under **Not implemented** as "no digest is stored anywhere yet, and adding one
  needs a new `ColTy` variant". Both statements were in the same file, and the
  second had been true when written.
- `O10.14`'s row said "no migration mechanism and **no applied-version
  metadata**" while `O10.15` was marked **•** for recording exactly that
  metadata and refusing a mismatch.
- `R4.2` was marked **~** because "four `VERSION`/`AUDIT_DETAILS` attributes are
  accepted and silently dropped — `D-07`", months after `D-07` was fixed. The
  matrix understated the crate.
- `PR12.5`/`PR12.6` were listed as absent after `openehr-axa` implemented read
  auditing above the store.

**Why it happened.** The matrix is prose. Every other claim in this repository
that drifted was prose too, and the ones that stopped drifting are the ones a
build reads — the disposition table, the invariant register, the licence check.
The matrix was assessed once, on 2026-08-01, and nothing has compared it to
anything since.

**Fixed.** Rows corrected, the session's new requirements assessed, and a CI
check added: **no requirement may be both marked satisfied and listed under Not
implemented**. That catches the exact defect and nothing else, which is the most
a check over prose can honestly do.

**Residual.** The matrix still records status by hand. The check proves it is
not self-contradictory; it cannot prove a **•** is deserved. `C0.20` already
says a mark is a claim about evidence, and the evidence is the test named in the
row — a reader who doubts a row should open it.

### D-08 — Two engines cannot return the bytes they were given — **High, fixed**

Found while building the tamper-detection test `PR12.12` demands, by asking what
`ColTy::Json` actually maps to on each engine rather than trusting that a column
called JSON stores JSON.

**Found.** The chain's content digest is `SHA-256` over the version's canonical
JSON (`M3.16`, `M3.23`). Canonical means *these bytes, in this order* — keys in
Unicode-scalar order, no whitespace, numbers as written. `ColTy::Json` mapped to
a **normalizing** column type on three of six engines, so what came back was not
what was hashed, and the digest was unverifiable from the database.

Verified against real engines, not from documentation:

| Engine | `ColTy::Json` was | Round-trips? | Observed |
| --- | --- | --- | --- |
| SQLite | `TEXT` | yes | — |
| **PostgreSQL 18** | `jsonb` | **no** | keys reordered, `": "` inserted |
| **MySQL 8.4** | `JSON` | **no** | keys reordered, **`1.10` → `1.1`** |
| MariaDB 11.4 | `JSON` | yes | alias for `LONGTEXT` |
| SQL Server | `nvarchar(max)` | yes | — |
| Oracle | `CLOB` | yes | — |

Two things make this worse than a digest bug.

**MySQL destroyed a clinical fact.** `1.10` became `1.1`. In openEHR a
`DV_QUANTITY` magnitude's trailing zero is precision — "1.10 mg" asserts
two decimal places and "1.1 mg" asserts one. That loss is independent of the
chain and would have corrupted the record whether or not anything hashed it.
No digest was needed to make this a defect; a digest is only what made it
visible.

**MySQL and MariaDB disagreed, and both spell it `JSON`.** These are the two
crates that `W-01` records as having once been byte-identical copies. The
cross-dialect guard was widened to catch that, and it did — it proves the two
now differ. Nothing checked whether the difference was *correct*. A guard that
asserts two things are not the same says nothing about which of them is right,
and here MariaDB happened to be right by inheritance while MySQL was wrong.

**The comments recorded the assumption that made it wrong.** All three crates
carried a variant of "nothing here relies on binary storage, since the canonical
bytes are regenerated from the parsed object rather than read back from the
column", citing `J9.12`. That was true when written and `D-03`/`D-07` made it
false — the chain now hashes bytes that must be reproducible *from storage* —
and no one revisited the comment, because a comment explaining a decision is not
a thing that fails.

The citation was wrong twice over. `J9.12` is `lib:J9.12` and was cited
unqualified against the `db:` tree, which `W0.5` forbids for exactly this reason
— there is no `db:J9.12`, so the citation resolved to nothing. And `lib:J9.12`
does not say what was claimed: it *defines* the canonical byte form and says
nothing about regeneration.

The requirement actually breached was next to it. **`lib:J9.13`: "Numbers MUST
NOT be renormalised in the canonical form. Measured precision is data."** MySQL
was renormalising them in the column. The rule was written, correct, and
unenforced at the only layer that could break it.

**Fixed.** `ColTy::Json` maps to a byte-preserving text type on every engine,
and `M3.43` now requires it. `SCHEMA_VERSION` is `4`.

The reasoning is the same as `M3.39`–`M3.42`, which forbid storing a digest as
hexadecimal text: a value whose bytes carry the meaning must not be handed to a
type whose contract permits reinterpretation. `jsonb` is the right type for a
document you intend to *query*; the relational columns are this schema's index
and the JSON is never queried as structure (`M3.20`), so nothing is given up.

**What this cost, stated plainly.** PostgreSQL loses `jsonb` operators and GIN
indexing over content. That is a real capability and it is not used here, but a
deployment that reached into `data_json` with `->>` will have to stop.

### D-03 — Tamper evidence is specified, built, and unused — **Low, fixed**

**Found.** `M3.16` requires a tamper-evident chain over committed versions. The
`openehr` crate implements the primitives in full — `Chain`, `ChainEntry`,
`ChainKey`, SHA-256 digests, HMAC tags compared in constant time, key material
zeroized on drop — and `openehr-store` references none of it. `openehr_version`
has no hash or tag column.

So the library carries the cost of a security capability that the persistence
layer does not use, and a reader who finds `security::audit_chain` may
reasonably assume stored history is chained. It is not.

**Why this is Low rather than High.** Nothing *claims* tamper evidence. `M3.16`
is marked unimplemented, `PR12.11` states explicitly that append-only is not
tamper evidence, and the conformance matrix lists it under "not implemented".
The gap is real; the misrepresentation is not.

**Disposition.** Open. Closing it means either wiring the chain into the schema
or recording a decision not to.

### D-04 — Read access is not audited — **Medium, fixed above the store**

**Found.** `PR12.5` requires that a complete audit of access to clinical data
record reads, not only writes. This layer records no reads at all.

The version history looks like an audit trail and is not one: it records what
changed, and an access investigation asks who *looked*. A deployment that
assumed the history served the purpose would discover the gap during the
investigation.

**Disposition.** *(2026-08-02 — **fixed above the store, still open in it**.)*
`PR12.5` always said a deployment must provide read auditing above the storage
layer. A layer above now exists: `openehr-axa` records every read before
returning it, fails the read when it cannot, and reports in its metadata which
of the two it is doing.

The store still records nothing and still cannot. A principal exists at the
service and nowhere below, so the finding stays open for anyone embedding
`openehr-store` directly — which is the case `PR12.5` was written for and the
one that has not changed.

### D-11 — Most requirements have never been assessed against the matrix — **Medium, fixed**

Found by writing the coverage check the library matrix already has
(`.github/workflows/ci.yml`, "Every requirement has exactly one row in the
library matrix") and pointing it at this file instead.

**Found.** This file's own header says *"Assessed 2026-08-02. Anything not in
this file is not claimed."* That sentence is safe against overclaiming — an
absent requirement asserts nothing false — but it is silent about how much is
absent, and the answer is most of it:
**144 of 221 requirements defined in `spec/databases/*.md` have zero mentions
anywhere in this file** — not a **•**, not a **✗**, not a **~**, not a **doc**,
not even a row in the "Not implemented" table. `scripts/check-databases-matrix-
coverage.py` reproduces the count.

By section:

| Prefix | Missing | Prefix | Missing | Prefix | Missing |
| --- | --- | --- | --- | --- | --- |
| `C0` | 21 of 22 | `M3` | 13 of 32 | `T11` | 14 of 17 |
| `G2` | 8 of 14 | `O10` | 13 of 15 | `W16` | 18 of 20 |
| `H5` | 7 of 16 | `P6` | 9 of 13 | `X15` | 10 of 18 |
| `S1` | 14 of 16 | `PR12` | 6 of 20 | `R4` | 4 of 11 |
| `V9` | 7 of 7 | | | | |

`C0` and `W16` are largely framework sections — normative language and
repository conventions — and the library matrix's precedent (`lib:C0.1`–`C0.18`
marked **doc**) suggests most of those 39 will resolve the same way once looked
at. That does not shrink the finding: the library matrix actually *has* a `doc`
row for each of its own `C0` requirements, stating that it is prose rather than
code. This file has never stated anything about 21 of its own 22. And the gap
is not confined to framework text — `M3.19`, *"The canonical JSON of a
version's content **is** the record"*, the sentence the whole storage model is
built on, has never been mentioned in the file that is supposed to record what
is true of the storage model today. Neither has `S1.1`, the core's founding
scope statement, nor `V9`, entirely — all seven of its requirements.

**Why it happened.** The library matrix is one linear walk through every
requirement in order, so a missing one is a visible gap in a sequence. This
file is five topic tables assembled by hand — per-engine, store-level, service,
cross-cutting, "not implemented" — and nothing ever walked the specification
section by section and asked, of each requirement, which table it belongs in.
`D-09` fixed this file *contradicting* itself; nothing has ever checked it for
*completeness*, because until `scripts/check-databases-matrix-coverage.py`
nothing could, cheaply, given the multi-table shape (`db:C0.20`'s "not thereby
verified" caveat is doing real work here, and so is
`scripts/check-databases-matrix-coverage.py`'s own docstring on why it checks a
floor rather than exact-once).

**Not fixed.** Closing this means reading each of the 144 requirements against
six engine crates and the store, and recording an honest mark — `•`, `~`, `?`,
`✗`, or `—`, this file's own five (its Legend has no `doc`; that mark belongs
to `openehr/spec/conformance-matrix.md`'s legend, a different file with a
different set, and citing it here before checking was itself an unchecked
claim, corrected in place 2026-09-09) — which is real assessment work, not
something this finding or a script can manufacture. Filing 144 guessed rows to
make a count look better would be a worse defect than the one being recorded
(`W0.3`). `plan.md`'s "Open decisions" names the recommended path: assess in
batches by section, starting with `M3` and `S1` since they are the
requirements most likely to already be silently satisfied by existing, tested
code.

**2026-09-09, first batch: `M3` and `S1`, 27 of 144.** Read each of the 13
missing `M3` requirements and 14 missing `S1` requirements against the actual
code — `openehr-store/src/schema.rs`, `record.rs`, `error.rs`, and every
`Cargo.toml` in the seven core crates — rather than assumed satisfied because
the surrounding architecture is sound. Fifteen landed `•` with a real test or
an unambiguous structural fact behind them (six-table schema for `S1.5`;
`R4.2`'s own round-trip for `S1.3`/`M3.15`; `M3.43`/`R4.8`/`R4.11` together
for `M3.19`; bounded-by-construction `ColTy::Id(n)`/`Text(n)` for `M3.29`;
the real `... audit_time_committed_utc IS NOT NULL ORDER BY ...` clause,
`H5.13`'s own test, for `M3.28`). Eleven landed `?` — true today, on inspection,
but not actively guarded: `S1.6`–`S1.10` (the core's own "MUST NOT" list —
no AQL executor, no HTTP dependency confirmed absent from all seven
`Cargo.toml`s by hand for this pass, no auth, no terminology resolution, no
encryption — every one of which is a real, checkable absence that nothing
currently checks by name), `S1.11` (`StoreError::Unsupported` exists with
the exact shape required and is constructed nowhere in the tree — dead code
waiting for an engine crate that needs it), `S1.12` (true while only
`openehr-sqlite` implements `Store` at all), `S1.19` (no controller calls
`validate()` — an absence, not a demonstrated presence), `M3.20`/`M3.21`
(the schema is exactly this shape today; nothing would catch a change away
from it), `M3.38` (`StoreError`'s fields are identifiers and rule names by
construction; no test asserts a stored value can never reach one). One, the
purely editorial `S1.21` ("sections 7 and 8 stay retired"), landed `—`: a
statement about the specification's own structure, not a claim this crate's
code can satisfy or fail. Zero landed `✗` — no gap this pass found was a
genuine unmet requirement rather than an untested true one, which is itself
worth recording rather than assumed typical of the remaining 117.
`spec/databases/conformance-matrix.md`'s own rows carry the per-requirement
evidence; this paragraph is the summary, not the citation.

**2026-09-09, batch 2: `C0`, 21 of 21.** The section the finding itself
predicted would resolve like the library matrix's own `C0` rows — "largely
framework sections... most of those 39 will resolve the same way once
looked at" — checked rather than assumed. It did not resolve the same way,
because it could not: **this matrix's own Legend has no `doc` mark** (five
only — `•`, `~`, `?`, `✗`, `—`; `doc` belongs to `openehr/spec/conformance-
matrix.md`'s legend, a different file, corrected above once already this
finding). Framework language that is purely a reading convention for the
specification itself — RFC 2119 keyword interpretation (`C0.1`), rationale
being non-normative (`C0.2`–`C0.3`), the definitional rules for what counts
as a departure (`C0.16`–`C0.17`) and how an amendment is conducted
(`C0.21`–`C0.22`) — 7 in all, landed `—`: not applicable to any crate's
code, because they are not claims about code at all. 9 landed `•`, and
every one against
a real, already-existing check rather than a new one written to make the
count look better: the ladder table in this same file (`C0.8`, `C0.13`),
`check-docs.py`'s existing "every conformance level restated in the tree
matches conformance-matrix.md" (`C0.9`, `C0.11`) and its shared-block check
(`C0.18`), the `schema` CI job running each of the five dialects against
its own server separately — the actual fix for `W-01`, the finding
`C0.10`'s own rationale names (`C0.10`), `verify-schema.sh` inserting a row
before testing `UPDATE`/`DELETE` refusal (`C0.12`), all six dialect annexes
carrying at least one `M14.x` entry (`C0.14`), and `spec/databases/`
genuinely having no `07-`, `08-`, or `14-` numbered file (`C0.6`). 5 landed
`?`: the identifier-format and no-reuse-and-next-ordinal disciplines
(`C0.4`, `C0.5`, `C0.19`) are followed throughout this tree but enforced by
no script — a malformed id would not be flagged, it would simply fail to
match the regex every id-aware tool including this matrix's own coverage
checker relies on, and vanish from tracking rather than erroring; `C0.7`'s
cross-directory qualification (`lib:`/`db:`) is likewise followed but
unchecked; `C0.15`'s "a departure must not weaken an engine-independent
invariant" is a review-time judgement on the six departures that exist
today, not something a script currently evaluates. 0 landed `✗`.

**2026-09-09, batch 3: `W16`, 18 of 18.** Repository and release conventions
— more concretely code/manifest-checkable than `C0`'s pure prose turned out
to be. 11 landed `•`, most against real, already-existing checks:
`check-docs.py`'s own derived counts for `W16.1` (eighteen crates, eight
published), its version-agreement check for `W16.11`/`W16.17`, its
level-consistency check for `W16.14`; `X15.15`'s cross-dialect DDL
comparison — the actual mechanism that found `W-01` — for `W16.2` and
`W16.6`; `agents/publishing.md`'s documented pre-publish checklist for
`W16.13` and `W16.18`; the `examples` CI job and rustdoc doctests for
`W16.9`; direct inspection for `W16.5` (every engine crate's `spec/` holds
exactly one dialect annex, nothing else) and `W16.16` (all eighteen crates
declare their own `[workspace]`, checked exhaustively for this pass). 6
landed `?` — true by inspection, none independently tested: `W16.3`/`W16.4`
(crate descriptions and driver dependencies read correctly today), `W16.7`
(no shared-behaviour logic duplicated in an engine crate), `W16.8` (the one
known violation, `W-01`, is fixed; nothing guards a recurrence but review),
`W16.10` (measured numbers checked this session carry provenance; no script
scans the whole tree for one that does not), `W16.20` (0.9.0's own two
breaking changes bumped minor, not patch, but no automated semver-diff
check enforces the discipline). 1 landed `~`: `W16.12`, once `CHANGELOG.md`
was found to now exist — see below. 0 landed `✗`.

**Found and fixed in passing, twice, in the same section this time.**
`16-repository-and-release.md`'s own text had gone stale in two places, both
surfaced only by checking rather than reading past them: `W16.11`'s
rationale said the crates "currently share `0.2.0`" — six releases behind
the real `0.9.0`, and invisible to `check-docs.py`'s version check because
that check looks for four specific fixed phrasings ("crates.io at",
"published at") and "currently share `X`" matches none of them, so the
drift passed through the file unflagged rather than the file going
unscanned. `W16.12` said "No crate here has one yet" about a changelog;
`CHANGELOG.md` exists, covering the eight published crates as a declared
set rather than strictly one-per-crate, which is why the requirement lands
`~` and not `•` — real, not a substitution (`W16.8`), but not literally the
requirement's own words either, so the requirement's own text is corrected
to describe the fact rather than deny it, pending a fuller amendment.

**2026-09-10, batch 5: `O10`, 13 of 13.** Operations — logging, install
idempotence, schema versioning, connection security, backup, release
evidence. 7 landed `•`: `O10.16` against a real test
(`openehr-sqlite/tests/conformance.rs` deletes the schema-version row from
a database holding data and asserts `SchemaVersionMismatch { found: 0,
.. }` — exactly the scenario the requirement names); `O10.2`/`O10.13`
against an exhaustive, direct finding — `openehr-store` and
`openehr-sqlite` emit **no log line at all**, no `tracing`, `log`,
`println!`, or `eprintln!` anywhere in either crate, so neither stored
content nor a bound parameter can reach one; `O10.4` against `G2.13` and
`verify-schema.sh`'s own twice-run requirement; `O10.10` against the
`supply-chain` CI job and `Cargo.lock` committed in every crate; `O10.11`
against `agents/publishing.md`'s documented tag-as-part-of-publishing
step; `O10.12` against `T11.19`, which this requirement's own text already
cites. 5 landed `?`: `O10.6` (no backup code exists, but nothing guards
against a bespoke one being added), `O10.7`/`O10.18` (true today — SQLite
is exempt by construction, no other engine crate opens a live connection
yet for either requirement to bind), `O10.17` (the schema-version
mechanism is real and tested; remembering to bump it is a discipline, not
machine-enforced), `O10.19` (a true logical consequence of `M3.17`'s own
tested guarantee, with no backup/restore test in this tree to exercise it
directly). 1 landed `—`: `O10.4a`, already self-classified *(amended — not
applicable)* in its own text. 0 landed `✗`.

**2026-09-10, batch 6: `X15`, 10 of 10.** Portability and the dialect
boundary — largely a section restating, from a different angle, facts
already established and evidenced elsewhere in this file. 9 landed `•`:
`X15.1`/`X15.14` restate `S1.1`/`M3.22`; `X15.2` restates `R4.11`; `X15.3`
cites `M3.22` (names come from the one shared schema declaration; a
dialect's `col_sql`/`quote` affect spelling, never naming); `X15.7`/`X15.8`
restate `C0.14`/`W16.5` — the annex-citation pattern read directly while
investigating `db:D-13`'s own `M14.8`; `X15.12` cites `X15.15`/`X15.19`,
which this requirement's own text names as exactly the tests it asks for;
`X15.13` cites the `Dialect` trait's own method list, which structurally
cannot own more than the four methods the requirement names; `X15.20`
restates `M3.35`. 1 landed `?`: `X15.17` (no engine crate created by
copying one — true today, after `W-01`'s own fix, but a process
discipline nothing machine-enforces against a future copy). 0 landed `✗`.

**Two more copies of `D-13`'s stale premise, found the same way.**
Assessing this section against `D-11` surfaced two further instances of
the "no chain exists" claim `D-13` already found and fixed twice: `X15.11`
itself, and `openehr-oracle`'s own `M14.8` departure. Both corrected in
place, dated; `D-13`'s own account extended to record all four. Full
detail there.

**2026-09-10, batch 7: `P6`, 9 of 9 — the first two genuine `✗`.** Every
prior batch found `0` landed `✗`: eleven of the ids checked already worked,
untested, or inapplicable — never simply unmet. This batch breaks that
run, twice, for real:

- **`P6.16`** requires every search target to declare its kind (identity,
  prefix, range, membership, containment). `search-adjuncts.md`'s
  `AD1`–`AD2` define the framework in full, but no declaration exists for
  any of the seven real indexed columns `P6.4` names — written in advance
  of the columns it would apply to, and never applied to them. `✗`.
- **`P6.7`** forbids an operation whose result grows without limit in the
  size of a record unless the caller asked for exactly that, named so they
  know — `all_versions` is the example the requirement's own text gives.
  `find_compositions_by_archetype` (`openehr-store::Store`) is not:
  `Vec<CompositionIndexRow>`, no `LIMIT`, no page parameter, in the trait
  signature or in `openehr-sqlite`'s own SQL. `openehr-axa` bounds it
  above the store (`_count`/`_offset`, capped at 100) — the same shape as
  `PR12.5`'s read-auditing gap, so it joins that table rather than getting
  a bare `✗`: satisfied by the service, absent from the store a program
  embedding it directly would get.

Neither is a documentation defect like `D-13`; both are genuinely unmet —
exactly what assessing 144 previously-silent requirements was for.

The rest of the batch: 4 landed `•` (`P6.4`, all seven named indexes
confirmed in the one shared declaration; `P6.11`, all five required query
capabilities exercised in `openehr-store/src/conformance.rs`; `P6.14`
restates `M3.28`; `P6.19` restates `P6.18` — `Json`/`LongText` cannot be
indexed at all, structurally, so neither can be given a path index
specifically). 3 landed `?` (`P6.10`, true by inspection of the schema;
`P6.15`, vacuously true since the store applies no bound at all to
truncate silently — the real cap is `openehr-axa`'s; `P6.17`, vacuous
while `P6.18` already establishes nothing needs an adjunct yet).

**2026-09-10, batch 8: `G2`, 8 of 8.** Schema generation — emission order,
idempotence, identifiers. The section the schema is declared in, not
generated, so most of this is structural fact rather than runtime
behaviour. 7 landed `•`: `G2.7` against `schema::TABLES`'s own const
declaration; `G2.10`/`G2.11` against the shared `Dialect::ddl` default,
confirmed with no dialect overriding it — tables, then all indexes, then
append-only, in that order; `G2.12` against the same `M3.23` evidence;
`G2.14` against the `Idempotence` enum's own three-variant shape; `G2.17`
against `openehr-mysql`'s own `index_idempotence` returning `Inline` while
`openehr-mariadb` is deliberately left at the default — read directly,
not assumed from the requirement's own worked example; `G2.18` against the
same `X15.3` evidence. 1 landed `?`: `G2.19` — true by inspection (the
longest identifier today is 29 bytes, under Oracle's 30-byte pre-12.2
floor) but nothing would catch a longer one being added. 0 landed `✗`.

**2026-09-10, batch 9: `V9`, 7 of 7 — every one a bullet.** Validation:
validate before writing, the two-gate discipline, what a refusal reports,
what validation does not claim. The first batch of this whole assessment
where every requirement landed `•`, because `lib:A-23`'s own fix — validate
the whole `Version` a caller sent, not just the `Composition` inside it,
precisely because deserialization skips the constructor JSON always does —
already built exactly what this section asks for, months before this
assessment read it. `V9.1` restates `R4.13`. `V9.4`: no dialect emits
`CHECK` anywhere in the schema — code columns are plain `Id(n)`, so there
is no value set to freeze at DDL time even if one were tempted to. `V9.5`:
`commit_composition`'s `version.validate_ok()` is literally
`Validate::validate().into_result()`, the same trait method every other
caller in the library uses. `V9.6`/`V9.7`: `error::Violation`'s own fields
are `path`, `class`, `invariant`, and `detail: &'static str` — the last one
load-bearing for `V9.7` specifically, since a `&'static str` is a
compile-time constant and cannot embed a runtime-submitted value,
structurally rather than by discipline. `V9.8`: the same `lib:A-23` fix,
read directly in `commit_composition`'s own comment. `V9.9`: `/metadata`'s
own `not_implemented` list names "archetype and template validation"
(`lib:S1.4`) by name, so the crate never has to be trusted not to overclaim
— it states the boundary in the one response every caller can read. 0
landed `?`, `~`, or `✗`.

**2026-09-10, batch 10: `H5`, 7 of 7 — a third genuine gap, this time in
the code's own comment.** Versioning and history — no separate history
table, version identity, commit refusals, deletion as a version,
contributions. 6 landed `•`: `H5.3` restates `P6.11`; `H5.5` against the
six-table schema having exactly one version table, so there is no second
to split into; `H5.6`/`H5.7` against `openehr_version`'s own columns —
`uid` whole, `versioned_object_uid`/`creating_system_id`/`trunk_version`/
`branch_number`/`branch_version` decomposed; `H5.9` against
`openehr::rm::common::CommitError`'s four named, distinguishable variants
(`WrongObject`, `DuplicateVersion`, `PrecedingVersionMismatch`,
`NotLatest`), wrapped by `StoreError::Commit` rather than restated; `H5.14`
against `openehr_contribution` (`M3.21`) plus the same `M3.17` append-only
mechanism.

**`H5.11` is the third genuine `✗`, and the first found in the code's own
words rather than a separate document.** It requires `is_deleted` to be
"stored as a column derived from the lifecycle state, **and indexed**."
`openehr_version`'s `is_deleted` column exists and is derived — its own
doc comment said "indexed so 'current content' does not need a code
comparison" — but none of the table's three actual indexes name it. The
comment was asserting something about its own file that the same file did
not do. Corrected in place (`openehr-store/src/schema.rs`), dated;
`assets/schema.json` regenerated to match (`openehr-assets check` was red
until it was). No query in `openehr-sqlite` currently filters on
`is_deleted`, which is presumably why the gap went unfelt: the comment
described a query pattern that has not been written yet, and the "indexed"
half of the sentence was true of no version of this file, not stale from
a change — closer to `P6.16`'s shape (written in advance, never applied)
than `D-13`'s (true once, then drifted).

**2026-09-11, batch 11: `PR12` and `R4`, 10 of 10 — the last batch.**
Trust/principal/audit and projection/round-trip. 9 of 10 landed `•`:
`PR12.3a`/`PR12.4` restate `M3.34`/`M3.15`; `PR12.9` against
`AuditDetails.committer: PartyProxy` being a required field with no
`Default` impl — structurally impossible to synthesize; `PR12.10` against
`openehr_contribution`'s own `audit_*` columns, distinct from
`openehr_version`'s; `R4.3` against the same `V9.9` evidence (no
archetype-validation code path exists to reject on); `R4.6` against every
`Store` method taking typed `HierObjectId`/`ObjectVersionId`, never a bare
string; `R4.9` restates `M3.35`. Two found their evidence in the code's
own words rather than a test: `PR12.8` — `openehr-axa/src/auth.rs`'s
module doc cites this requirement by number, "the practical consequence
is that `db:PR12.8` still holds"; `PR12.11` — `PHI.md`'s own text draws
exactly the append-only/tamper-evidence distinction the requirement asks
for, citing it by number too. One landed `?`: `R4.10` — plausible by
construction (`project()`'s only input is the same object that gets
canonicalized to JSON, so nothing it derives can come from elsewhere) but
no test re-projects from stored JSON and compares. 0 landed `✗` or `~`.

**All 144 of the originally-unassessed 221 requirements now carry a mark.**
`scripts/check-databases-matrix-coverage.py` reports zero missing, for the
first time since it was written. Eleven dated batches, `M3`/`S1` through
`PR12`/`R4`, 2026-09-09 through 2026-09-11. Three genuinely unmet
requirements found along the way (`P6.7`, `P6.16`, `H5.11`) and one stale
claim traced to four separate places and fixed (`db:D-13`) — real findings
this assessment surfaced rather than manufactured, and the reason
`plan.md`'s own recommendation against "filing 144 guessed rows" mattered:
an assessment that found nothing wrong across 144 requirements would have
been the less credible result, not the more reassuring one.

**Fixed**, per this item's own stated condition: the diagnostic script is
now wired into CI (`.github/workflows/ci.yml`, the `claims` job) — it
would have passed on day one, because this is the day it does.
`AGENTS.md` and this file's own `claims`-job row updated to describe what
the check now asserts, in place of the 144-of-221 figure that described
the gap this closes.

### D-13 — The Digests section described a chain that had already shipped — **Medium, fixed**

**Required.** `C0.18`: a change to a normative statement changes its text in
this directory, in one commit, with the reason stated. `M3.16`'s own bullet
already says *(amended — implemented)*.

**Found** while assessing `T11` against `db:D-11`, tracing `T11.8`'s "no
chain exists (`M3.16`)" back to its source: the Digests section of
[`03-storage-model.md`](03-storage-model.md) closed with **"Not
implemented. No digest is stored anywhere in this schema today: the chain
of `M3.16` does not exist... Implementing either requires a new `ColTy`
variant."** All three clauses are false of the tree as it stands, and have
been since `f951556` ("Wire the tamper-evidence chain, and give `D-07` its
columns"), 2026-08-02 — five weeks before this was noticed.
`openehr_version` carries `chain_previous`, `chain_content`,
`chain_digest`, `chain_tag_key_id`, and `chain_tag_mac`
(`openehr-store/src/schema.rs`), all `ColTy::Digest` — the variant the
paragraph said would need adding, already added, already the store-level
table's own evidence for `M3.39`–`M3.42` two sections above the one that
said it did not exist.

**Why it happened.** `M3.16`'s own bullet was updated the same commit that
wired the chain in — the requirement text changed. The *rationale*
paragraph beneath a different, later section did not, and nothing checks
that a fact restated in two places moved together (`W0.38`'s own concern,
one level down: a duplicated *claim*, not a duplicated *passage*, so the
shared-block mechanism `check-docs.py` already runs would not have caught
it either). `db:D-11`'s own assessment of this same section is what
surfaced it — reading `T11.8` against the code it cites led straight to
the sentence contradicting both.

**Consequence.** A reader trusting this paragraph over the store-level
table's own `M3.16`/`M3.39`–`M3.42` rows — the two disagree, and the file
never said which one to believe — would conclude tamper evidence needs
building from nothing, when `openehr-sqlite` has shipped it since 2026-08-02
and `db:D-06`, `db:PR12.11`, and `PHI.md`'s own "in the process, not in the
database" distinction all already depend on it existing.

**Fixed.** The paragraph now says what is actually true: the chain is
implemented, using the `ColTy::Digest` variant that already exists; only
the checksum adjunct for `U4`'s unbounded-string search remains not
implemented, the same gap `db:P6.18` already records and the one true
clause in the original three.

**The stale claim had a second copy.** `T11.8`'s own text repeated "no
chain exists (`M3.16`)" as the reason its tamper-evidence half had
"nothing to test" — also false by the same date, since `M3.16c`/`M3.16d`
already test exactly the truncation and content-verification behaviour
this text said had no test to run. Also stale in the same paragraph,
independently: "tested... against three engines" — six dialects reached
Schema by 2026-09-06, not three — and "verifies under each algorithm",
which `M3.39` had already made moot by fixing the algorithm to exactly
one. All three corrected in
[`11-conformance-testing.md`](11-conformance-testing.md); the one gap that
survives the correction is real: no test in `openehr-sqlite`'s own suite
exercises a retired key leaving prior history verifiable, though the
library-level guarantee it depends on (`lib:X11.14`) is tested elsewhere.

**Two more copies, found assessing `db:X15` the same way, 2026-09-10.**
`X15.11` said the same "no chain exists (`M3.16`)" as its reason
cross-engine chain verification was "not implemented" — the chain is real;
what is actually untestable is *cross-engine* verification specifically,
because only `openehr-sqlite` has a `Store` to verify against, the same
reason `X15.10` already gives one bullet above it. And
`openehr-oracle/spec/14-oracle-dialect.md`'s own `M14.8` departure said "no
digest is stored anywhere" as why its `DBMS_LOB.COMPARE` requirement was
"presently theoretical" — true for a different reason: this crate has no
`Store` at all, so nothing in it ever executes the comparison, regardless
of whether `openehr-sqlite` writes digests through the same shared schema,
which it does. Both corrected in place, dated; the matrix's own
"Not implemented in the store" row for `X15.11` corrected to match. Four
copies of the same stale premise, across three files, found across two
assessment passes — the same root cause each time: `M3.16`'s own
requirement text was updated the day the chain shipped, and every other
place that had restated its old conclusion in different words was not.

## Closed

### D-05 — The specification required the architecture the code rejects — **High, fixed**

**Residual narrowed twice.** The rewrite was derived from the code rather than
from openEHR. Two reviews against primary sources have since run:

- **Terminology**, against `specifications-TERM`: five groups checked code for
  code — `audit_change_type`, `version_lifecycle_state`, `event_math_function`,
  `composition_category`, `setting` — all exact. One prose error found and fixed
  (`W-08`).
- **The Reference Model**, against the RM 1.1.0 BMM: `VERSION`,
  `ORIGINAL_VERSION`, `CONTRIBUTION`, and `AUDIT_DETAILS` attribute lists
  checked. RM 1.1.0 confirmed as the current release, so `S1.2` is correct. One
  **High** finding: `D-07`.

- **The RM invariants**, also from the BMM. The earlier note here said these
  would need the published PDF because the BMM only *names* them. That was
  wrong: the BMM carries the **expressions** —
  `Owner_id_valid: owner_id.value.is_equal (uid.object_id.value)` — for 68 of
  155 classes. No PDF was needed.

  Nineteen invariants across the nine classes this layer depends on were checked
  against the code. The result is not a simple pass count, because three
  categories behave differently:

  | Category | Invariants | Status |
  | --- | --- | --- |
  | Enforced, named in the code | `Lifecycle_state_valid`, `System_id_valid`, `Change_type_valid`, `Category_validity`, `Is_archetype_root`, `Setting_valid`, `Items_valid`, `Reason_valid` | ✅ |
  | **Vacuous in Rust** | `Attestations_valid`, `Other_input_version_uids_valid`, `Participations_validity`, `Items_valid` (partly) — all of the form `X /= Void implies not X.is_empty` | not applicable: Rust has no Void-versus-empty distinction for a `Vec`, so an empty vector *is* the absent case and the rule cannot be violated |
  | **Unenforced and undeclared** | `Territory_valid`, `Language_valid` | **`lib:A-19`** |

  `A-19` is the finding. `COMPOSITION` requires `territory` to be a member of
  `Code_set_id_countries` and `language` of `Code_set_id_languages`. The crate
  checks `CODE_PHRASE` well-formedness only, so `ISO_639-1::zz` is accepted
  although `zz` is not a language. These are code sets **openEHR names**, so
  `lib:S1.10` — which excludes external terminologies like SNOMED CT — does not
  cover them.

  Now declared as `lib:S1.18` with the reason: both code sets are mutable, and a
  table compiled into a library is wrong from the day a country changes.
  Validating against a stale copy would reject conformant data, which the
  crate's own `D3.5` reasoning calls the worse failure. Enforcement stays open.

  Also noted, not findings: `VERSION.owner_id` is a derived function in the BMM
  and the crate does not expose it, so `Owner_id_valid` has nothing to violate;
  and `VERSIONED_OBJECT.Uid_validity` (`extension.is_empty`) is satisfied by
  `HierObjectId` construction, which rejects an empty extension outright.

**The rest is now tracked rather than promised.** `assets/invariant-coverage.md`
is regenerated by `openehr-assets` and lists all 155 RM 1.1.0 invariants against
whether the crate names each one: 60 named, 95 not. `rm-1.1.0-invariants.json`
commits the expressions themselves, so the check needs no network and no PDF.

The 95 are **not** 95 findings. They mix three things — out of scope by a
declared exclusion, vacuous in Rust, and genuinely unenforced — and separating
them needs a human, which the report says rather than guessing. A first attempt
at that triage was itself wrong: a shell glob read only one directory level, so
`CODE_PHRASE.Code_string_valid` showed as missing while sitting in
`text.rs:52`. The tool now walks the tree, and that is why it is a committed
tool and not a one-off command.

What remains is the human triage of those 95, class by class.

**Found.** Every numbered section in this directory was imported from a FHIR
specification and text-substituted, and §2 and §3 therefore required a **shredded
schema** generated from specification packages — "7,355 tables for R5", child
tables per repeating element, `Patient.name.given` → `patient_name_given` — while
`openehr-store/src/schema.rs` argues at length against exactly that, correctly,
because archetypes are authored after the software ships.

A specification requiring the code to be wrong is worse than no specification: it
makes every conformance statement meaningless and invites someone to "fix" the
code toward it.

**Fixed.** All fourteen numbered sections rewritten against the code, with
withdrawn requirements keeping their numbers and listed per section (`C0.5`).
Two cross-cutting documents specifying text-search machinery this layer does not
have are marked withdrawn rather than deleted, because their reasoning would
apply directly if text search were ever added.

**Residual.** The rewrite was done by reading the code, not by re-deriving the
requirements from the openEHR specifications. Where a requirement here is now
merely a description of what the code does, that is a rubber stamp rather than a
considered generalization, and `C0.21` requires the distinction to be visible.
It is not, for this pass. A future review against the primary sources is the way
to close that — and the library register records that every such review so far
found the primary source contradicting what had been implemented.

### D-14 — The shared conformance suite never exercises a keyed chain — **Low, open**

**Found 2026-09-12**, mutation-testing `openehr-postgresql::PostgresqlStore`
once it gained a real implementation
(`openehr-postgresql/scripts/verify-store.sh`, `cargo mutants --in-place`
against a live server): four of its 54 diff mutants survived. Three trace to
the keyed-chain fixture gap that remains here. The fourth traced to the missing
two-system race and exposed the real schema defect now recorded as `D-17`.

**The remaining gap in the fixtures every engine's suite shares:**

1. **No fixture ever commits a keyed chain.** `VersionRow::project`'s `key:
   Option<&ChainKey>` parameter is `None` in every call
   `openehr_store::conformance` makes, so `chain_tag_key_id`/`chain_tag_mac`
   are `NULL` in every row any conformance test reads back. A mutant that
   makes the tag-reading path return the wrong thing — three of the four
   survivors, all on `PostgresqlStore::optional_digest_column` — cannot be
   caught by a suite that never exercises the `Some` case at all.
The two-system half of this finding was exercised on 2026-11-19 and exposed an
actual High-severity defect, `D-17`, rather than merely increasing coverage.
The shared fixture now accepts a creating system, and the SQLite and PostgreSQL
race tests use eight distinct systems. That path is fixed and verified.

**Why the residual is Low.** The keyed read path remains a test-coverage gap.
The chain implementation itself has keyed unit tests, but no `Store` API accepts
a key when committing, so no shared engine suite can persist and read one back.
Closing it needs a specified key-supply API, not an engine-specific test that
reaches around `Store` and makes the shared blind spot harder to see.

## What this audit did not cover

Stated so "not examined" and "examined and sound" stay distinguishable (`W0.3`):

- **SQL Server and Oracle DDL** has never been parsed by the engine it names.
  Both crates are at **Dialect**, which is the correct level for that, and the
  gap is in evidence rather than a judgement that the DDL is wrong.
- **The `openehr` crate's Reference Model conformance** was not re-verified; it
  has its own register, tracked and counted separately (its own summary
  paragraph is checked against its own table by CI) — see
  [`openehr/spec/audit.md`](../../openehr/spec/audit.md).
- **Performance.** No benchmark exists and none is claimed (`T11.5`, withdrawn).
- **The rewritten requirements themselves** have not been reviewed by a second
  reader.

---

Part of the [openEHR persistence specification](index.md).
