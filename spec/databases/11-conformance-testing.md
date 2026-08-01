# 11. Conformance testing

**Rewritten 2026-08-01.** Most of this section survived: its subject is testing
discipline, which is not engine-specific and was largely right. What changed is
everything that assumed a shredder, a search-parameter compiler, a REST surface,
or a committed FHIR example corpus. See [`spec/audit.md`](../audit.md) **W-04**.

Withdrawn requirements keep their numbers (`C0.5`); new ones start at `T11.15`
(`C0.19`).

Requirement prefix: `T11`.

## What every engine crate must test

- **T11.15** Every dialect MUST have **golden DDL tests**: the exact SQL it
  emits, so a change to a type mapping appears as a diff in a test rather than as
  a migration that fails in someone's staging environment.
- **T11.16** Every dialect MUST be checked by `conformance::check_dialect`, which
  fails a dialect that declares `Guard` without wrapping (`G2.15`) or leaves an
  append-only table unenforced (`M3.36`).
- **T11.17** Every dialect MUST be compared against **every other** dialect, and
  the comparison's coverage MUST itself be asserted (`X15.15`, `X15.16`).
- **T11.18** A golden test MUST assert what the dialect emits **is not** another
  engine's SQL, naming the specific spellings that would indicate a copy.

  This class of test is cheap and catches the expensive failure. It is also not
  sufficient on its own: `openehr-mariadb` passed tests of exactly this shape
  while emitting MySQL's script, because its assertions named MySQL's spellings —
  which were, correctly, present (**W-01**).

## What a golden test cannot tell you

- **T11.2** *(amended)* A dialect's DDL MUST be executed against a real server of
  the engine it names, in CI, before that crate claims level **Schema**
  (`C0.8`). The check MUST confirm the engine parses the script, that re-running
  it is a no-op, and that the append-only tables refuse `UPDATE` and `DELETE`
  **with a row present** (`C0.12`).

  A golden test compares an emitter against its author's belief. Only an engine
  compares it against the engine. Three of three crates that took this step were
  wrong at Dialect level and had passed every golden test while being wrong
  (`A-13`, `A-15`, **W-01**).

- **T11.19** The live check MUST invoke the same script a contributor runs
  locally, not a parallel implementation in CI configuration. Two ways of doing
  one check drift, and the one that drifts is always the one nobody runs.
- **T11.13** A test that self-skips without its database MUST NOT be the only
  evidence for a conformance level (`C0.9`). Where a suite requires a database
  its pipeline never provides, that suite is not a gate, and the
  [conformance matrix](conformance-matrix.md) MUST record the requirement as
  unverified rather than as passing.

## Tests that must exist and do not

Listed as requirements rather than omitted, so the gap is visible (`C0.20`).

- **T11.6** *(amended — **implemented for SQLite**)* Concurrency MUST be tested
  adversarially, not assumed: a reader looping against a writer MUST never
  observe a torn read (`R4.5`), and N racing commits to one container MUST
  produce one success and N−1 refusals, with the version tree intact (`H5.4`).

  A concurrency test MUST use a database the threads genuinely share. An
  in-memory SQLite database is private to its connection, so a test against one
  runs N independent databases and passes without testing anything — the same
  shape as a guard whose input list is incomplete (`T11.20`).

  Implemented in `openehr-sqlite/tests/concurrency.rs`. The `H5.4` half failed on
  first run and produced `D-06`: the guarantee held, but the refusal was reported
  as an engine error rather than a commit refusal, which `H5.9` forbids. Only
  SQLite has a `Store`, so only SQLite is covered.

- **T11.9** *(amended — **implemented**)* Every parser and every function that
  accepts untrusted input MUST be fuzzed, with the fuzz targets **run, not merely
  committed**, on a bounded time budget with a committed seed corpus. A crash,
  panic, abort, or stack overflow MUST fail the build.

  A stack overflow is not unwindable: `catch_unwind` does not catch it, a worker
  thread cannot contain it, and the process ends. For a component holding
  clinical data, one document ending the process is a denial of service that
  requires no cleverness.

  **Implemented for the dialects.** Six `openehr-<engine>-fuzz` crates drive two
  properties per engine — `check_quote` and `check_col_sql` — with a committed
  seed corpus, and CI runs each for a bounded time on every push. The properties
  live in `openehr_store::conformance`, shared by all six, because six copies of
  one assertion is the arrangement that produced **W-01**.

  **Implemented for the parsers**, which is where the untrusted surface actually
  is. `openehr-fuzz` drives five targets — ISO 8601, the identifier grammars,
  AQL, openEHR paths, and canonical-JSON deserialization — each asserting a
  property beyond "did not panic": lexical fidelity, `Display` round-trip,
  idempotent AQL normalisation, and canonical round-trip through `validate()`.
  That closes `lib:A-09`.

  A seed corpus is not optional for a structured target. `canonical_json` seeded
  with a real composition reaches roughly 4,800 covered edges; unseeded it would
  exercise the JSON lexer and stop, because random bytes are never a valid
  `COMPOSITION`. A target that runs millions of iterations against nothing
  reports the same green as one that works.

  Deep nesting is **not** treated as a finding: `lib:S1.15` states unbounded
  recursion as a documented limitation a caller must bound, and a fuzzer pointed
  at it would produce an impressive-looking result that means nothing.

- **T11.7** *(amended — **not implemented**)* A redaction test MUST assert that
  no log line emitted during a full write-and-read cycle over a record containing
  a distinctive marker value ever contains that marker, and that no error
  surfaced to a caller echoes a submitted value (`M3.38`).
- **T11.8** *(amended — partially implemented)* An audit test MUST assert that
  every write records its committing system and change type (`M3.15`), and that a
  direct `UPDATE`/`DELETE` on an append-only table is refused **by the database**
  (`M3.17`).

  The second half is tested, by `verify-schema.sh`, against three engines. The
  tamper-evidence half of the original requirement — that a chain verifies under
  each algorithm, that a truncated chain still verifies while the checkpoint
  moves, that a retired key leaves history verifiable — has nothing to test,
  because no chain exists (`M3.16`).

## Rules about tests themselves

These are the requirements that make the rest worth anything, and none of them
is engine-specific.

- **T11.10** A test asserting a defect is fixed MUST be shown to fail **without**
  the fix. Reverting the fix, or mutating the code it guards, MUST make the test
  fail; a test not verified this way is presumed decorative until it is.

  A test that cannot fail is indistinguishable from a control that works, and the
  distinction is the entire value of the control.

- **T11.11** A regression MUST be pinned by the **narrowest** assertion that
  catches it. Prefer an exact value or a named set over a threshold: a floor of
  "at least 20" tolerates losing four of twenty-four, and "more than zero"
  tolerates losing all but one.
- **T11.12** Coverage MUST NOT degrade silently. A check that skips — because a
  corpus is absent, a database is unreachable, or a path could not be resolved —
  MUST say so, and MUST fail if it ends up checking nothing. A skip is
  indistinguishable from a pass in a CI summary.
- **T11.20** A guard over a **list** MUST assert the list's completeness, not
  only its contents.

  This is `T11.12` sharpened by an actual failure. `dialects_are_distinct` did
  not skip and did not degrade; it ran, compared everything it was given, and
  passed. It was given five of six dialects. A partial guard and a complete one
  report the same green, so the count is now asserted (`X15.16`).

- **T11.14** A test disabled with `#[ignore]` MUST be accompanied by an entry in
  the conformance matrix. An ignored test is a known gap; an ignored test nobody
  tracks is a forgotten one.
- **T11.21** A documentation example MUST be compiled and run. Rustdoc examples
  are claims; `no_run` or `ignore` converts a checked claim into an unchecked
  one, and MUST NOT be used to make a failing example pass.

## Withdrawn

Withdrawn 2026-08-01. Numbers are retained and MUST NOT be reused (`C0.5`).

| Id | Was | Why withdrawn |
| --- | --- | --- |
| `T11.1` | round-trip property tests over every committed example resource | no example corpus is committed here; round-trip fidelity is required by `R4.2` and exercised by the `openehr` crate's own property tests |
| `T11.3` | search-semantics tests per parameter type | no search parameters (`P6.10`) |
| `T11.4` | a published conformance description generated from the relational map | no relational map and no published capability document |
| `T11.5` | load benchmarks in `doc/benchmarks.md` with a regression gate | no benchmarks exist and none are claimed; `W16.10` governs any that are added |

---

Part of the [openEHR persistence specification](index.md).
