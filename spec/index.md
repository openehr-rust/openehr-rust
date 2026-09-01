# The openehr-rust specification

This directory is the **root of the specification tree** for this monorepo. It is
normative for the repository as a whole: what the crates are, how they relate,
which requirement identifiers mean what, what a conformance claim means, and how
a change is allowed to land.

It does **not** restate the two domain specifications. Those are:

| Specification | Governs | Directory |
| --- | --- | --- |
| **Library** | the openEHR Reference Model in Rust — types, validation, paths, AQL, security | [`openehr/spec/`](../openehr/spec/index.md) |
| **Persistence** | storing openEHR in SQL — storage model, dialects, commit rules, conformance suite | [`spec/databases/`](databases/index.md) |

Requirements use RFC 2119 keywords (MUST, SHOULD, MAY). Every normative statement
carries a stable identifier. The trace — openEHR specification → requirement →
code → test → matrix — is what makes a claim about this repository checkable
years later by someone who was not here.

Requirement prefix for this document: `W0`.

## The single source of truth rule

- **W0.1** A normative statement MUST exist in exactly **one** place in this
  repository. Where two files state the same rule, one of them is a copy, and a
  copy is a future divergence.
- **W0.2** A README, a rustdoc comment, a tutorial, `CLAUDE.md`, `AGENTS.md`, or
  a commit message is **descriptive**. None of them is normative. Where such a
  file and a specification disagree, the specification governs and the other file
  has a defect — to be fixed, not reconciled by amending the specification to
  match (`W0.15`).
- **W0.3** Documentation MUST NOT claim more than is verified. "The same code
  path works elsewhere" is not evidence. This is the rule this repository has
  broken most often; see [`audit.md`](audit.md).
- **W0.4** A gap that is not written down reads as a pass. Every known divergence
  between specification, documentation, and code MUST appear in an audit register
  with evidence a reader can check.

## Identifier namespaces, and the collision this resolves

The two domain specifications were written independently and **both** allocate
the prefixes `C0`, `S1`, `R4`, and others. They do not mean the same things:

| Id | In [`openehr/spec/`](../openehr/spec/index.md) | In [`spec/databases/`](databases/index.md) |
| --- | --- | --- |
| `S1.4` | the crate MUST NOT implement the Archetype Model — *withdrawn 2026-08-26, reversed by `lib:S1.21`; the identifier is permanent and still resolves* | a port MUST declare a minimum engine version |
| `R4.x` | Data structures — `ITEM_STRUCTURE`, `CLUSTER`, null flavours | Shredding and reconstruction |
| `C0.x` | conformance framework for the library | conformance framework for the ports |

Both directories declare their identifiers permanent and never reused. Both are
right to; the mistake was assuming one flat namespace.

- **W0.5** Requirement identifiers are scoped to the specification that defines
  them. A citation that could be ambiguous MUST be qualified with its domain:
  `lib:S1.4` for the library, `db:S1.4` for persistence. An unqualified id inside
  a domain's own directory means that domain's.
- **W0.6** The qualified form is the one to use in code comments, commit
  messages, test names, and audit findings — anywhere the reader may not know
  which directory an id came from. Unqualified citations already written into the
  code stay valid where the surrounding crate makes the domain unambiguous, and
  MUST NOT be mass-rewritten: a citation's whole value is that it does not change.
- **W0.7** *(amended 2026-08-20)* Neither domain may allocate identifiers in the
  other's sections. `W0.x` is reserved for this file and MUST NOT be defined by
  either domain.

  A repository-level document other than this one takes its **own** prefix,
  registered here, rather than extending `W0`: `W0.1` puts each normative
  statement in exactly one place, and a prefix shared across files is the
  arrangement in which two files can both look like its home. The register:

  | Prefix | Document |
  | --- | --- |
  | `W0` | this file |
  | `RV` | [`rust-msrv-n-minus-2/index.md`](rust-msrv-n-minus-2/index.md) — the Rust MSRV |

## Conformance levels

Two ladders existed: `Dialect / Schema / Store / Verified` in the persistence
crate's documentation, and `Scaffold / Schema / Store / Reference` in the
imported core. They overlapped in name and disagreed in meaning, which is worse
than either alone.

- **W0.8** This repository has **one** ladder, defined in
  [`databases/00-conformance.md`](databases/00-conformance.md):

  <!-- shared: conformance-ladder (copy) -->
  | Level | Means | Evidence required |
  | --- | --- | --- |
  | **Dialect** | Emits DDL for the shared schema. | The golden DDL tests, and `conformance::check_dialect`. |
  | **Schema** | The engine itself has executed that DDL. | A transcript against that engine's own server: the script applied cleanly, applied *again* cleanly, and the append-only tables were observed refusing `UPDATE` and `DELETE` **with a row present**. |
  | **Store** | Implements `Store` against a real database. | `conformance::run` passing against that engine. |
  | **Verified** | Store level, run in CI against the engine's own server on every commit. | A CI job that provisions the engine and fails — not skips — without it. |
  <!-- /shared: conformance-ladder -->

- **W0.32** *(added 2026-08-02)* The ladder above describes **engine** crates.
  Every rung is defined by DDL, by a `Store` implementation, or by a database
  server, so a crate that is none of those — `openehr-loco` — cannot occupy one.

  Such a crate MUST NOT borrow the nearest-looking rung, and MUST NOT be
  described as unverified when it is tested. It states, in the first screenful,
  **what has been demonstrated and what has not**, in those terms. A ladder
  invented so that every crate has a rung would give the word "Verified" two
  meanings, and the one that lost would be the one carrying evidence about a
  database.

- **W0.9** *(amended 2026-08-02)* A crate MUST NOT claim a level it has not
  earned, and its README and crate documentation MUST state its level — or, for
  a crate outside the ladder, its evidence (`W0.32`) — in the first screenful.
- **W0.10** A level is a claim about the present, not about an afternoon in the
  past. A level whose evidence is a one-off local run MUST say so rather than
  imply continuous verification.
- **W0.11** A crate is at **Verified** only once CI has actually run green on
  `main`. A committed workflow is not a working one, and a crate MUST NOT be
  promoted on the strength of one existing.

  As of green run 30713623082, 2026-08-01, `openehr-sqlite` is at **Verified** — the only crate
  eligible, being the only one at Store level. See [`audit.md`](audit.md)
  **W-02**, which was closed by the run rather than by the commit that added the
  workflow.
- **W0.12** The "with a row present" clause in **Schema** is not pedantry. A
  `FOR EACH ROW` trigger on an empty table never fires, so a `DELETE` matching
  zero rows reports refusal it never performed. A check whose subject is absent
  reports the silence as success.

## What is in this repository

**Eighteen crates**, each its own Cargo workspace: eight that are published
together at one version, `openehr-loco` published separately since
2026-09-01 at its own (`db:W16.1`, amended), and nine that declare
`publish = false` — `openehr-assets` and eight fuzz harnesses.

### The published crates

| Crate | Role | Level |
| --- | --- | --- |
| [`openehr`](../openehr) | the Reference Model, validation, paths, AQL, security | library — n/a |
| [`openehr-store`](../openehr-store) | engine-agnostic persistence: schema, projection, commit rules, conformance suite | library — n/a |
| [`openehr-sqlite`](../openehr-sqlite) | SQLite dialect **and a complete store** | **Verified** |
| [`openehr-postgresql`](../openehr-postgresql) | PostgreSQL 18 dialect | **Schema** |
| [`openehr-mysql`](../openehr-mysql) | MySQL 8.4 dialect | **Schema** |
| [`openehr-mariadb`](../openehr-mariadb) | MariaDB 11.4 dialect | **Schema** |
| [`openehr-mssql`](../openehr-mssql) | SQL Server dialect | **Dialect** |
| [`openehr-oracle`](../openehr-oracle) | Oracle Database dialect | **Dialect** |

The [conformance matrix](databases/conformance-matrix.md) is the detailed
version of that last column and is the one to trust.

All eight are on crates.io at **0.8.0**, published 2026-08-29, and local matches
published. [`agents/publishing.md`](../agents/publishing.md) holds the
per-crate table and is the file to read before the next release.

This sentence said **0.2.0** for sixteen days after 0.3.0 went out, because the
release commit updated the file that exists to track releases and not the four
others that also state the version. See [`audit.md`](audit.md) **W-10**.

### The service and tooling crates

Neither is on the conformance ladder — see `W0.32` above. `openehr-assets` is
not published (`publish = false`). `openehr-loco` is published, but
separately from the eight above and at its own version: `0.8.1` since
2026-09-01 (`db:W16.1`, amended). `0.8.0` also shipped that day and is
immutable, but carries two RUSTSEC advisories permanently
(`agents/publishing.md`, `spec/audit.md` **W-20**) — not the version anything
should depend on.

| Crate | Role |
| --- | --- |
| [`openehr-loco`](../openehr-loco) | HTTP API server over `openehr-sqlite`, on Axum and Loco |
| [`openehr-assets`](../openehr-assets) | regenerates the committed schema/DDL assets; fails the build if one is stale |

### The fuzz crates

One per dialect — `openehr-<engine>-fuzz` — driving two properties each: that an
identifier cannot escape its own quoting, and that every logical column type maps
to something usable.

Plus two crates over the layers a dialect does not reach:

- `openehr-fuzz`, seven targets over the Reference Model parsers, which is where
  the untrusted surface actually is: ISO 8601, the identifier grammars, AQL,
  openEHR paths, canonical-JSON deserialization, `DV_URI`/`DV_EHR_URI` through
  **both** gates (`lib:A-36`), and every `DATA_VALUE` variant.
- `openehr-store-fuzz`, two targets over the layer that is not a parser at all:
  the projection from a composition onto rows, and `verify_versions` — the check
  that answers *has this record been tampered with*, whose input is by
  definition a row somebody may have edited.

Twenty-one targets in total.

- **W0.25** A fuzz crate MUST declare `publish = false`. It is a test harness; a
  registry release would claim it as part of the library's surface.
- **W0.26** *(amended 2026-08-20)* A property driven by **more than one** fuzz
  crate MUST live in `openehr-store`, shared rather than copied. Six copies of
  one assertion is the arrangement that produced **W-01**, and a fuzz harness
  repeating the mistake it exists to catch would be worse than none.

  A property driven by exactly one crate MAY live in the crate it is a property
  *of* — which for `openehr-store-fuzz` is `openehr-store` again, and for
  `openehr-fuzz` is the target file. The rule was written absolutely, and read
  literally it required a property about ISO 8601 lexical fidelity to live in
  the persistence crate, which knows nothing about ISO 8601 and would have had
  to be told. That is `W0.14`'s layering inverted to satisfy a rule aimed at
  duplication, and no duplication was involved.

  The test is not *where* but *how many*: one copy, in the crate that owns the
  thing being asserted about.
- **W0.27** A fuzz target MUST be **run**, not merely committed, with a committed
  seed corpus and a bounded budget in CI. A committed target nobody executes is a
  claim rather than a check (`db:T11.9`).
- **W0.28** A fuzz property MUST be shown to **fail** against a deliberately
  broken implementation. A check that cannot fail is indistinguishable from a
  control that works (`db:T11.10`).
- **W0.30** *(amended 2026-08-20)* A fuzz target over a **structured** input
  MUST carry a seed corpus of real instances. Random bytes are never a valid
  `COMPOSITION`, so an unseeded target exercises the lexer and stops — and
  reports the same green as one that works. Coverage is the evidence: seeded,
  `canonical_json` reaches roughly 4,800 edges against 650 for `iso8601`.

  The corpus MUST itself be checked, and by something that runs. A committed
  seed that stopped deserializing is an unseeded target wearing a corpus: the
  target still runs, the directory is still there, the file contributes nothing,
  and a fuzz run's output cannot tell twenty-two seeds from twenty-two files the
  deserializer rejects. `openehr/tests/fuzz_seeds.rs` asserts that each
  structured target's corpus still spans **both** answers — at least one real
  instance, so the target gets past the lexer, and at least one refusal for the
  parser's error paths.

  Requiring *every* seed to parse would be wrong and was tried: five of
  `canonical_json`'s seven seeds are `null`, `{}`, `[]` and other deliberate
  malformations, and they are half of what the corpus is for.
- **W0.31** A fuzz target MUST NOT report a **documented limitation** as a
  finding. `lib:S1.15` states that recursion depth on deserialization is
  deliberately unbounded and that a caller must bound it; a fuzzer pointed at
  that produces a result in seconds that looks like a defect and is not.

### Dialect annexes

- **W0.29** Every engine crate MUST carry `spec/14-<engine>-dialect.md`
  (`db:X15.6`), stating what that dialect actually does and declaring every
  departure as a numbered `M14.x` requirement. All six exist and all six are
  **proposed** rather than ratified, so none counts as evidence for a level.

- **W0.13** `openehr` and `openehr-store` are libraries, not ports; the ladder
  does not apply to them. Their assurance is the requirement-level status in
  [`openehr/spec/conformance-matrix.md`](../openehr/spec/conformance-matrix.md)
  and [`databases/conformance-matrix.md`](databases/conformance-matrix.md).

### Why the engine crates are thin

- **W0.14** An engine crate MUST own exactly four things: how it spells a type,
  how it quotes an identifier, how it writes a bind placeholder, and how it
  enforces append-only. Everything else — which tables exist, which columns,
  which indexes, the projection from openEHR objects onto rows, the commit rules,
  the conformance suite — lives in `openehr-store` and is shared by all six.

  This is the whole architecture, and it is a response to a specific failure. The
  sibling FHIR monorepo gave each of six ports a full copy of the DDL generator,
  and one copy spent that fork's entire life emitting another engine's types
  because nothing ever compared them. A dialect that owns only spellings cannot
  do that.

  It is not hypothetical here either: this repository reproduced the same defect
  in `openehr-mariadb` — a name-substituted copy of `openehr-mysql`, emitting
  byte-identical DDL, exporting a struct still called `MysqlDialect`, and
  claiming a MariaDB server had accepted it. The cross-dialect guard existed and
  did not catch it, because the guard compared five dialects and this was the
  sixth. See [`audit.md`](audit.md) **W-01**.

- **W0.15** The guard against `W0.14` MUST cover **every** engine crate, and the
  coverage MUST itself be checked. A comparison that omits a dialect cannot find
  that dialect identical to another, and reports the same green as a complete one.

## Amending

- **W0.16** An amendment edits the requirement in place, keeps its identifier,
  and states the reason in the commit message.
- **W0.17** Amending a specification to match what the code already does is
  permitted and expected; doing it **silently** is not. The commit MUST say so, so
  a considered generalization and a rubber stamp stay distinguishable.
- **W0.18** Every amendment MUST be checked against the conformance matrix — does
  it change a status? — and against the audit register — does it close a finding,
  or open one?
- **W0.19** Behaviour is decided in a specification first. Discovering a
  requirement while implementing is normal; the fix is to write it down before the
  commit lands, not after.

## Publishing

- **W0.20** Every crate here is published to crates.io. A published version is
  immutable, so a conformance claim in a crate's documentation becomes permanent
  the moment it is published, and cannot be corrected in place — only superseded.
- **W0.21** A crate MUST NOT be published while any finding against its
  conformance claims is open. `openehr` 0.1.0 was published carrying a
  `repository` field pointing at an unrelated project; that field is now immutable
  for that version. See [`audit.md`](audit.md) **W-03**. This requirement binds
  regardless of who runs `cargo publish` — as of 2026-09-01 that may be an
  agent for `openehr-loco` specifically, under the conditions
  [`agents/publishing.md`](../agents/publishing.md) states; W0.21 does not
  relax for that case.

## Licensing

- **W0.22** Every crate MUST carry the same licence expression:

  ```
  MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR GPL-3.0-only
  ```

  `OR` is a grant of choice: a user may take the software under any **one** of
  the five. This matches the convention already established across these
  projects — `assertables` and the sibling FHIR monorepo declare the same five.

- **W0.23** Every crate MUST ship a `LICENSE.md` naming all five with their SPDX
  identifiers, and that file MUST be the only licence file in the crate. Shipping
  the full text of some licences and not others privileges those and understates
  the grant — which is the same defect class as claiming more than is verified
  (`W0.3`), pointed the other way.

  Before 2026-08-01 each crate shipped `LICENSE-MIT` and `LICENSE-APACHE` while
  declaring only those two. Both files were summaries rather than the full texts —
  `LICENSE-APACHE` was 705 bytes against roughly 11 KB for the real Apache 2.0 —
  so neither satisfied the "include a copy" expectation they appeared to.

- **W0.24** A crate's README MUST state the licence in the same terms as
  `LICENSE.md`, and MUST NOT name a subset.

## One fact, one owner, checked

`W0.1` says a normative statement exists in exactly one place. It is the oldest
rule here and the one this repository breaks by accident most often, because the
alternative — a reader who must open a second file to learn what a level means —
is genuinely worse. The rules below keep the copies and make them mechanical.

- **W0.38** *(added 2026-08-20)* Where the same statement genuinely must appear
  in more than one document, exactly **one** occurrence is the **owner** and the
  rest are marked copies:

  ```
  <!-- shared: NAME (owner) -->   …   <!-- /shared: NAME -->
  <!-- shared: NAME (copy)  -->   …   <!-- /shared: NAME -->
  ```

  [`scripts/check-docs.py`](../scripts/check-docs.py) fails when a copy diverges
  and rewrites it from the owner with `--fix`. A copy that is not marked is a
  copy nothing compares, which is the arrangement `W0.1` forbids.

  The conformance ladder is the first block bound this way. It was written out
  four times — `db:C0.8`, which owns it; `W0.8` below, which says the ladder is
  *defined* in `db:C0.8` and then reproduces it; `openehr-store/spec/conformance.md`;
  and `agents/conformance.md` — and two of the four had already drifted. See
  [`audit.md`](audit.md) **W-16**.

- **W0.39** *(added 2026-08-20)* A **countable** claim about this repository —
  how many crates, how many are published, how many fuzz targets, how many
  tutorials, which version is live, which CI jobs exist — MUST be checked against
  the tree by [`scripts/check-docs.py`](../scripts/check-docs.py), which the
  `claims` job runs.

  A count is the cheapest possible claim to check and the easiest to leave
  behind: **W-10** (the published version stated in five files, updated in one)
  and **W-11** (`db:W16.1` saying fourteen crates while five documents said
  seventeen) were both counts, and both survived review. The residual recorded
  against **W-11** was this script's absence.

- **W0.40** *(added 2026-08-20)* A crate's **conformance level** has one owner:
  the table in [`databases/conformance-matrix.md`](databases/conformance-matrix.md).
  Every other statement of a level — this file, `README.md`, `CLAUDE.md`,
  `AGENTS.md`, each crate's README and rustdoc first screenful (`W0.9`,
  `db:C0.9`) — is checked against it.

  These are not marked copies, because they are legitimately different shapes: a
  crate's README says `## Conformance level: Verified` and this file's table
  carries a Role column. What is checked is the **level**, wherever it is
  asserted, which is the part `W0.9` is about.

## Benchmarks

`openehr` and `openehr-store` carry criterion benchmarks —
[`openehr/benches/rm.rs`](../openehr/benches/rm.rs) and
[`openehr-store/benches/store.rs`](../openehr-store/benches/store.rs).

- **W0.34** *(added 2026-08-20)* A benchmark result is **not** a conformance
  claim and MUST NOT be cited as one. No requirement in this repository is
  stated in seconds, no crate's level depends on a timing, and the conformance
  ladder's rungs are defined by DDL, a `Store`, and a database server (`W0.32`)
  — none of which a stopwatch can establish.

  This is `W0.3` pointed at the most persuasive kind of number there is. A
  benchmark is evidence about one machine on one afternoon; a level is a claim
  about the present (`W0.10`).

- **W0.35** *(added 2026-08-20)* A benchmark MUST NOT gate CI on wall-clock.
  A shared runner varies by more than most real regressions, so a threshold
  fails for reasons unrelated to the change, and a check that fails for
  unrelated reasons is a check that gets silenced. A silenced check reports the
  same green as a passing one, which is the defect class in
  [`audit.md`](audit.md).

- **W0.36** *(added 2026-08-20)* A benchmark MUST be **run** in CI, with
  `--test`: one iteration per benchmark, asserting nothing about the time. This
  is `W0.27` for benchmarks — a committed benchmark nobody executes stops
  compiling within a few refactors, and the first person to need it finds a
  build error instead of a baseline. Running it proves the only property CI can
  honestly check, which is that it still works.

- **W0.37** *(added 2026-08-20)* A benchmark SHOULD measure a path a whole
  document travels, rather than a function chosen for being easy to measure.
  The ones here are deserialization, validation, canonical JSON, path
  resolution, AQL parsing, ISO 8601 parsing, projection, and chain
  verification — each of which runs once per request or once per row.

  `verify_versions` is measured at 1, 10, and 100 versions for a reason worth
  stating: the question is not how fast one row is but whether the walk is
  **linear**. A check that quietly became quadratic would pass every test in the
  suite and would first be noticed by whoever verified a record with ten years
  of history in it.

## Toolchain

- **W0.33** *(added 2026-08-20, offset raised 2026-08-29)* The minimum
  supported Rust version is a repository-wide requirement, not a per-crate one,
  and is specified in
  [`rust-msrv-n-minus-2/index.md`](rust-msrv-n-minus-2/index.md): **N−2**, two
  stable releases behind current, declared identically by all eighteen crates
  and **compiled** by CI rather than asserted (`RV1`–`RV3`).

  It is a separate document because it is the one requirement here whose correct
  value changes on a schedule nobody in this repository controls. See
  [`audit.md`](audit.md) **W-09** for what the number was before it had a rule
  behind it.

## Status, not requirements

- [`audit.md`](audit.md) — the repository-level findings register: divergences
  that span crates or sit above either domain. Domain-local findings stay in
  [`openehr/spec/audit.md`](../openehr/spec/audit.md) and
  [`databases/audit.md`](databases/audit.md).

## Reading order

Someone new to the repository should read, in order:

1. The root [`README.md`](../README.md) — what this is and how to use it.
2. [`databases/index.md`](databases/index.md) — the persistence architecture, and
   why the schema is document-centric rather than shredded.
3. [`openehr/spec/index.md`](../openehr/spec/index.md) — the library, for work on
   the Reference Model rather than on storage.
4. [`audit.md`](audit.md) — what is known to be wrong today.

Anyone about to build the tree should also read
[`rust-msrv-n-minus-2/index.md`](rust-msrv-n-minus-2/index.md), which is one page and
explains why CI will one day fail on their unrelated pull request.

Contributors should also read [`AGENTS.md`](../AGENTS.md), which is operational
guidance and not normative (`W0.2`).
