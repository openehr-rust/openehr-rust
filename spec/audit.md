# Repository audit findings

**Non-normative.** This is the register of known divergences that span crates or
sit above either domain specification. Domain-local findings belong in
[`openehr/spec/audit.md`](../openehr/spec/audit.md) and
[`databases/audit.md`](databases/audit.md).

A finding stays here until it is fixed or a specification is amended to match
reality. Deleting one because it is inconvenient, or because the text that stated
it was rewritten, is the failure mode this file exists to prevent (`W0.4`).

**Audit date:** 2026-08-01. **Scope:** the whole monorepo — eight crates, three
specification trees, and every documentation file. **Method:** every crate built
and tested; every conformance claim traced to the evidence cited for it; the
generated DDL of all six dialects compared byte for byte; the three engines that
can be provisioned locally actually provisioned and the DDL run against them;
crates.io queried for what is already published.

Nineteen findings: three High (**W-01**, **W-02**, **W-04**), ten Medium
(**W-03**, **W-05**, **W-06**, **W-09**, **W-11**, **W-12**, **W-13**,
**W-14**, **W-16**, **W-18**), six Low (**W-07**, **W-08**, **W-10**,
**W-15**, **W-17**, **W-19**). **Eighteen are fixed**; **W-03** is fixed going forward only, because the
published `openehr` 0.1.0 is immutable and keeps its wrong `repository` field.

**W-09 through W-18 were added on 2026-08-20 and -21**, outside the original audit date
below, and are marked as such. They were found the same way as the rest — by
running something — and three of them are the same defect in three places: a
**guard whose input list was written by hand**. The MSRV was declared in
seventeen manifests and compiled in none (**W-09**); the layering check compared
nine crates of seventeen (**W-13**); the licence check compared the same nine,
and the eight it skipped were the eight with no licence (**W-14**).

That is `W-01` recurring — *a guard is only as wide as its input list* — and the
fix is the same in all three: derive the list from the tree, and assert the
derived count, so that a crate the guard cannot see fails the job instead of
being silently exempt.

**W-08** was found by reviewing a requirement against the openEHR terminology
rather than against the code that inspired it, and is the smallest finding here
with the largest lesson attached.

W-04's fix opened four persistence-local findings, `db:D-01`–`db:D-04`, which is
the usual pattern: rewriting a specification against the code is what makes the
gaps between them visible.

The pattern across them is one thing, stated four ways: **a claim was written
once and never re-checked, and nothing in the tree could tell the difference
between a verified claim and a plausible one.** Every finding below was found by
running something rather than by reading something.

## Severity

| Level | Means |
| --- | --- |
| **High** | A published or user-visible claim is false, or a guarantee is unenforced. |
| **Medium** | A claim is unverifiable, or a defect is possible and undetectable. |
| **Low** | Metadata or cosmetic inconsistency with no behavioural consequence. |

---

## W-01 — `openehr-mariadb` was `openehr-mysql` under another name — **High, fixed**

**Claimed.** The crate documented **Conformance level: Schema**: "the DDL has
been executed against **MariaDB 8.4**, re-applied as a no-op, and its append-only
triggers observed refusing `UPDATE` and `DELETE`. Reproduce with
`openehr-store/scripts/verify-schema.sh mariadb`."

**Found.** Every part of that was false, and each part was independently
checkable:

1. **The DDL was byte-identical to `openehr-mysql`'s.** Both scripts hashed to
   `40f32f64e5015f8640830a67aecb9c72`. The source files were identical modulo the
   engine name — verified by normalizing `MariaDB`/`MySQL` to a placeholder and
   diffing, which produced no output.
2. **The crate exported a struct named `MysqlDialect`.** Two crates in the
   workspace defined a public type by that name.
3. **MariaDB 8.4 has never existed.** MariaDB versions run 10.x then 11.x. The
   number is MySQL's, carried across by the substitution — as was the crate's
   claim that "MariaDB rejects `CREATE INDEX IF NOT EXISTS`", which is true of
   MySQL and false of MariaDB, which has accepted it since 10.0.5.
4. **The cited reproducer refused to run.** `verify-schema.sh` accepted
   `postgresql|mysql` and exited `FAIL: unknown engine 'mariadb'`. Nobody had
   ever run the command the documentation told them to run.
5. **The guard that exists for exactly this did not cover it.**
   `openehr-sqlite/tests/dialects.rs` compares dialects and fails if any two emit
   the same DDL — the sibling monorepo's **F-08** made into a test. Its `all()`
   listed five dialects. `openehr-mariadb` was the sixth and was absent, as it was
   from that crate's dev-dependencies and from the engine list in
   `openehr-store/src/lib.rs`. A comparison that does not include a dialect
   cannot find it identical to another.

This is **F-08 reproduced inside the repository whose architecture exists to
prevent F-08**, and it survived because the guard was incomplete in exactly the
place the defect was.

**Fixed.** The dialect is now genuinely MariaDB: the struct is `MariadbDialect`,
`index_idempotence` is `IfNotExists` (MariaDB supports it; MySQL does not), and
append-only uses `CREATE OR REPLACE TRIGGER` (MariaDB 10.1.4+), which — unlike
MySQL's drop-then-create — never leaves an interval in which an append-only table
would accept an `UPDATE`. The two scripts now hash differently. `verify-schema.sh`
has a `mariadb` branch using `mariadb:11.4` and the `mariadb` client binary, and
**it was run: `parses idempotent append-only enforced row intact`.** The Schema
claim is now earned rather than asserted.

`dialects.rs` covers all six dialects, and a new test ties `all().len()` to the
number of engine crates so that adding a seventh and forgetting to list it fails
rather than silently reducing coverage (`W0.15`).

**Residual.** The MariaDB verification is a local run, not CI — so the level is
Schema, not Verified, and **W-02** applies to it as to the others.

---

## W-02 — The repository claims CI it does not have — **High, fixed**

**Claimed.** `openehr-store/spec/conformance.md`:

> A level is a claim about the present, not about an afternoon in the past. The
> `schema` job in `.github/workflows/openehr.yml` runs `scripts/verify-schema.sh`
> for PostgreSQL and MySQL on every change under `openehr*/`, so both crates'
> Schema claims are continuously checked rather than attested once.

It goes on to require that the job "**must fail, never skip**", citing the sibling
monorepo's **F-06** for database jobs that invoked a test target that did not
exist.

**Found.** There is no `.github/` directory anywhere in the repository, and no
workflow file of any kind — `find . -name '*.yml'` outside `target/` returns
nothing. Nothing is continuously checked. The paragraph asserting continuous
verification is itself the failure mode the paragraph is about.

**Consequence.** Every Schema-level claim in this repository rests on a local run
by one person on one afternoon — which is precisely what that text says a level
must not be. The three Schema claims are *true today* (all three were re-run
during this audit and all three passed), but nothing would report it if a change
broke them tomorrow.

**Fix committed.** The false claim of continuity was withdrawn from
`openehr-store/spec/conformance.md` — the policy it stated was right, only the
assertion that it was in force was not — and `.github/workflows/ci.yml` now
exists. It runs, on every push and pull request:

| Job | Covers |
| --- | --- |
| `test` | `clippy --all-targets`, `test`, and `doc` for each of the ten buildable crates separately, because each is its own workspace and a single `--workspace` invocation would silently cover one of them |
| `msrv` | that the MSRV is N−2, declared identically everywhere, and **builds** — see [`rust-msrv-n-minus-2/index.md`](rust-msrv-n-minus-2/index.md) and **W-09** (fixed when the floor was N−3; the offset since raised to N−2, `RV1`) |
| `examples` | the five runnable tutorials the README points at, plus `openehr-sqlite`'s persistence tutorial |
| `bench` | every criterion benchmark runs once (`--test`); nothing is gated on wall-clock (`W0.35`, `W0.36`) |
| `schema` | `verify-schema.sh` against real PostgreSQL 18, MySQL 8.4, and MariaDB 11.4 containers |
| `assets` | that the committed `assets/` files are what the code renders — a stale generated artifact is a lie that reviews are read against |
| `fuzz` | a bounded run of every fuzz target (`W0.27`); a crash, panic, or abort fails the build |
| `layering` | that `openehr` and `openehr-store` depend inward only, dev-dependencies included, against a crate list **derived** from the tree (**W-13**) |
| `claims` | that `openehr-mssql` and `openehr-oracle` still claim only **Dialect**; that the **library** matrix covers every requirement exactly once, and that both conformance matrices do not contradict themselves — the databases matrix has no exactly-once check: it is five topic tables in which one requirement can legitimately appear more than once, and 144 of its 221 requirements have never been assessed at all (`db:D-11`); that this file's summary paragraph counts itself correctly; that the licence expression is harmonized across every crate (`W0.22`); that no requirement marked satisfied calls itself unverified (**W-17**); and that the documentation's countable claims match the tree (`scripts/check-docs.py`) |
| `trademarks` | `scripts/check-trademarks.py`: every root document and every published crate's rustdoc that uses the openEHR mark in prose carries the professionalization rule 5 notice verbatim (added 2026-08-26, once the changes it was deferred behind had landed) |
| `mutants` | `cargo-mutants --in-diff` over the lines a push or a pull request changed, scoped to the diff because a full run is hours per crate. Pull-request-only until **W-18** |

The `schema` jobs **fail rather than skip** when no container runtime is present
(`C0.13`), and invoke the same script a contributor runs locally rather than a
parallel implementation in YAML. There is deliberately **no** schema job for SQL
Server or Oracle: no server has parsed their DDL, and a job that skipped would
convert an honest gap into a false green.

**Demonstrated 2026-08-01**, which is what closed this rather than the commit
that added the file. run [30713623082](https://github.com/openehr-rust/openehr-rust/actions/runs/30713623082) on `main` (`127b4df`) is green across all
nineteen jobs: eight `test`, `examples`, `claims`, three `schema`, and six
`fuzz`. The claim of continuity is now a fact about an observed run.

Getting there took three attempts at one job, and the two failed ones are worth
recording because both were **guesses**:

1. `schema / mysql` failed under docker. Diagnosed as a readiness race —
   correct as a defect, and not what was failing.
2. It failed again under podman at the 180s budget. Diagnosed as slowness —
   also wrong.
3. Only after the harness was made to dump the engine's own log did the cause
   appear: MySQL was ready, and the probe was connecting to nothing, because the
   runner's server served `/var/lib/mysql/mysql.sock` while the client defaulted
   to `/var/run/mysqld/mysqld.sock`. All connections moved to TCP, which is also
   immune to the temporary-init-server race that (1) aimed at and missed.

The lesson is this register's own, turned on the tooling: a check that cannot
report its evidence forces the next step to be a guess.

**Consequence.** `openehr-sqlite` moves to **Verified**: it is at Store level
and its conformance suite runs in CI on every commit against a bundled engine
that cannot be absent. It is the only crate eligible, and now the only one there.

**Residual.** `openehr-mssql` and `openehr-oracle` remain at **Dialect** — CI
cannot verify what no reachable server will parse.

---

## W-03 — `openehr` 0.1.0 is published with a wrong `repository` — **Medium, partially fixed**

**Found.** All eight `Cargo.toml` files carried
`repository = "https://github.com/joelparkerhenderson/fhir-databases"`. The actual
remote is `git@github.com:openehr-rust/openehr-rust.git`. The field pointed
readers at a different project.

`openehr` **0.1.0 was already published to crates.io** on 2026-07-31, carrying
that field. crates.io versions are immutable: the published 0.1.0 will point at
the wrong repository permanently, and can only be superseded.

**Fixed, forward.** All eight manifests now name the correct repository, and all
eight are bumped to **0.1.1** so the corrected metadata can ship. The published
0.1.0 cannot be corrected and is left as the historical record.

**Residual.** Anyone reading `openehr` 0.1.0 on crates.io is sent to the wrong
repository. Yanking would not help — a yanked version keeps its metadata — so the
remedy is to publish 0.1.1 promptly and let it become the version people see.

---

## W-04 — `spec/databases/` specifies a different system — **High, fixed**

**Found.** The persistence specification is a FHIR database specification with
`FHIR` replaced by `OpenEHR`. The substitution is complete at the string level —
zero literal occurrences of `FHIR` remain — which is what makes it dangerous: it
reads as an openEHR specification and requires a FHIR one.

Evidence, each independently checkable:

| Where | Says | Reality |
| --- | --- | --- |
| `db:S1.1` | ports MUST support "OpenEHR R5 (5.0.0), R4 (4.0.1), R3 (3.0.2)" | those are FHIR releases; openEHR's Reference Model is 1.0.2 / 1.1.0 (`lib:S1.16`) |
| `db:X15.1` | lists `map/src/shred.rs`, `map/src/reconstruct.rs`, `map/src/canon.rs`, `gen/src/**` as MUST-be-identical modules | none of those files or crates exist |
| `db:G2.5` | installing a version means "creating thousands of tables — 7,355 for R5" | the schema is **five** tables |
| `db:G2.3` | table names concatenate resource name and element path, `Patient.name.given` → `patient_name_given` | there are no resources and no shredding |
| `db:X15.6` | every port MUST have `spec/14-<engine>-dialect.md` | all six port `spec/` directories are empty |
| `db:S1.3`, `§9`, `§12` | `OperationOutcome`, `CapabilityStatement`, `Provenance`, `Bundle`, `_include`, `$export`, "a OpenEHRPath evaluation engine" | all FHIR artifacts; openEHR uses AQL and openEHR paths |

The conflict is not cosmetic. `spec/databases/02-schema-generation.md` **mandates
a shredded schema generated from specification packages**, and
`openehr-store/src/schema.rs` **argues at length against exactly that** — because
a `COMPOSITION` contains whatever its archetype says, archetypes are authored
after the software ships, and a schema shredded from the Reference Model alone is
"a document store with extra joins". The code's position is the correct one for
openEHR. The specification currently requires the code to be wrong.

`spec/databases/conformance-matrix.md` compounds it by recording six ports as
satisfying requirements — `R4.2` lossless round-trip, `R4.5` snapshot reads — that
no crate here implements, and `audit.md` there quotes a claim about "all 7,399
official OpenEHR example resources" round-tripping, which is the sibling
monorepo's own false-claim finding imported wholesale.

**Fixed.** The directory has been rewritten to describe this system. All
**fourteen** numbered sections, plus both status files:

| | |
| --- | --- |
| Rewritten | §0 Conformance, §1 Scope, §2 Schema generation, §3 Storage model, §4 Projection, §5 Versioning, §6 Search, §9 Validation, §10 Operations, §11 Conformance testing, §12 Trust and audit, §13 Compliance mapping, §15 Portability, §16 Repository and release |
| Withdrawn | `locale-accent-folding.md`, `unbounded-string-search-…md` — text-search machinery for a layer with no text search |
| Rebuilt | `conformance-matrix.md`, `databases/audit.md` |

Identifier discipline was kept throughout (`db:C0.5`, `db:C0.19`): withdrawn
requirements retain their numbers and are listed in a table at the foot of each
section with the reason; amended ones keep their numbers and are marked; new ones
take the next unused ordinal rather than reusing a vacated one, which is why §3
restarts at `M3.19` and §16 at `W16.16`. No identifier changed meaning, so
citations written before this pass still resolve.

The two `[service]` sections were the largest deletions — nine requirements
across §10 and §12 described liveness endpoints, connection pooling, TLS
termination, and trusted principal headers for a service this repository does not
build (`db:S1.7`).

**Residual, and it is real.** The rewrite was done by reading **the code**, not by
re-deriving requirements from the openEHR specifications. Where a requirement is
now a description of what the code already does, that is a rubber stamp rather
than a considered generalization, and `db:C0.21` requires the two to stay
distinguishable. For this pass they are not. Recorded as `db:D-05` residual.

That matters more here than it would elsewhere: the library's own register
records that **every** review against openEHR primary sources so far found the
source contradicting what had been implemented — four for four — and that the
rendered specification pages omit every class-definition table, so anything
written from the prose alone is a guess.

Four persistence-local gaps were surfaced by the rewrite and are open as
`db:D-01`–`db:D-04`: no crate has a dialect annex, two store requirements are
unverifiable as written, tamper evidence is built and unused, and read access is
not audited.

---

## W-05 — Documentation contradicted its own specification about levels — **Medium, fixed**

**Found.** `openehr-store/README.md` said:

> Short version: `openehr-sqlite` is at **Store** and verified against a real
> database; the other four are at **Dialect** and have never had a statement
> parsed by the engine they name.

`openehr-store/spec/conformance.md`, in the same crate, recorded `openehr-postgresql`
and `openehr-mysql` at **Schema**, with transcripts. Both could not be right, and
the README was understating two crates while the crate documentation for
`openehr-mariadb` was overstating one.

Separately, the README and `openehr-store/src/lib.rs` both enumerated "the five
engine crates" and listed five, in a workspace containing six.

**Fixed.** Levels are stated once, in `spec/index.md` (`W0.8`), and referenced
rather than restated. The engine count is six everywhere.

---

## W-06 — Two conformance ladders with overlapping names — **Medium, fixed**

**Found.** `openehr-store/spec/conformance.md` defined
`Dialect / Schema / Store / Verified`. `spec/databases/00-conformance.md` defined
`Scaffold / Schema / Store / Reference`. Both were in force, both used **Schema**
and **Store** for different evidence bars, and a crate claiming "Store" did not
resolve to one meaning.

**Fixed.** One ladder, `W0.8`, defined once in `databases/00-conformance.md` and
cited from `spec/index.md`. `Scaffold` and `Reference` are withdrawn as level
names; their identifiers are not reused.

---

## W-07 — Requirement identifiers collide across specifications — **Low, fixed**

**Found.** `openehr/spec/` and `spec/databases/` independently allocate `C0.x`,
`S1.x`, `R4.x`, and others, with different meanings — `lib:S1.4` excludes the
Archetype Model, `db:S1.4` requires a minimum engine version — while both declare
their identifiers permanent and never reused. An unqualified `S1.4` in a commit
message or a test name resolved to two different requirements.

**Fixed.** `W0.5`–`W0.7` scope identifiers per specification and define the
qualified forms `lib:` and `db:`. Existing unqualified citations inside a domain's
own directory stay valid and are deliberately **not** rewritten: a citation's
value is that it does not change.

---

## W-08 — "openEHR names two integrity-check algorithms" — **Low, fixed**

Found by the primary-source review that `db:D-05` called for, and it is the
finding that review existed to produce.

**Claimed.** Three places said openEHR's terminology names *two* integrity-check
algorithms, SHA-1 and SHA-256:

> `openehr/Cargo.toml` — "SHA-256 is one of the two integrity-check algorithms
> the openEHR terminology names (the other is SHA-1 …)"
>
> `openehr/src/security/audit_chain.rs` — "openEHR's terminology names **both**
> as integrity check algorithms."
>
> `spec/databases/03-storage-model.md` `M3.39` — "openEHR's own terminology names
> two integrity-check algorithms, SHA-1 and SHA-256."

**Found.** The `integrity_check_algorithms` group in
`openEHR/specifications-TERM` names **seven**: SHA-1, SHA-224, SHA-256, SHA-384,
SHA-512, SHA-512/224, SHA-512/256.

**The crate's own data was right the whole time.**
`openehr/src/terminology.rs` defines all seven, correctly, and a test exercises
them. Only the prose was wrong — in the manifest comment, in the module header,
and then in a specification requirement I wrote *from that prose*.

**Consequence.** Nil for behaviour: no code branches on the count, `SHA-256` is
still the right choice, and `M3.39` still requires it. The damage is to the
argument. "One of two" reads as *there was little to choose from*; "one of seven"
requires saying why this one, which the requirement now does — SHA-1 is broken,
the wider members buy nothing this use needs and double the stored width.

**Fixed** in all three places, with `M3.39` carrying a note about what it used to
say.

**Why it is worth a finding at all.** This is `db:D-05` made concrete. That
residual says the persistence specification was rewritten by reading the code
rather than the openEHR sources, so some requirements are descriptions wearing
the clothes of decisions. Here a *comment* became a *requirement's rationale*
without anyone rechecking the source, and the library's own register already
warned this would happen: it records four findings closed by reading a primary
source, and **all four times the source contradicted what had been implemented**.

Four of four, now five of five.

**Residual.** One requirement was checked this way. The rest of
`spec/databases/` has still not been re-derived from openEHR, so `db:D-05`
stays open with its scope narrowed rather than closed.

---

## W-09 — The declared MSRV was checked by nothing, and was false — **Medium, fixed**

**Claimed.** Seventeen manifests declared `rust-version = "1.90"`, and nine
READMEs plus [`agents/adding-an-engine.md`](../agents/adding-an-engine.md) said
"Requires Rust 1.90+ (edition 2024)". A floor stated eleven times is a floor a
reader is entitled to rely on.

**Found.** Nothing had ever compiled this repository with a 1.90 toolchain. The
`test` job installs `stable` and nothing else; there was no MSRV job. Running it
by hand — `cargo +1.90 check --all-targets --all-features`, per crate, which is
the whole method — six crates passed and **`openehr-loco` failed**:

```
error: rustc 1.90.0 is not supported by the following packages:
  loco-gen@1.0.0 requires rustc 1.94
  loco-rs@1.0.1 requires rustc 1.94
```

The crate's own floor was 1.90; its framework's was 1.94. Cargo refuses rather
than miscompiles, so the consequence was bounded — but the statement "Requires
Rust 1.90+" was false for that crate from the day it was written, and the
repository could not tell.

**When it was written matters.** `rust-version = "1.90"` arrived in `9a7ecf6`,
2026-08-01, when stable was 1.97. The number was seven releases old on the day it
was committed. It was not a floor that drifted; it was one nobody had picked.

**This is `W0.3` in its purest form.** Not a claim that became false — a claim
that was never in contact with anything that could make it true or false. It sat
next to `W-02` ("the repository claims CI it does not have") and `W-01` ("the
guard existed and did not cover the sixth dialect") and is the same defect: an
assertion with no check behind it, reporting the same green as a verified one.

**Fixed**, in three parts, because fixing only the number would have left the
next one just as unchecked:

1. **A rule instead of a number.**
   `rust-msrv-n-minus-3/index.md` (superseded 2026-08-29 by `rust-msrv-n-minus-2/index.md`) sets the MSRV at **N−3** —
   three Rust releases behind stable, an eighteen-week window. A number is
   consistent with itself no matter what it says; a formula is re-derived from
   the outside world and is either right or loudly wrong.
2. **The floor is compiled.** The `msrv` job in
   [`.github/workflows/ci.yml`](../.github/workflows/ci.yml) derives N from the
   stable toolchain it installs, installs N−3, and runs `cargo test` for all ten
   buildable crates under it (`RV3`). All eighteen manifests, the ten prose
   statements, and the specification's own headline sentence are checked against
   the same derived value (`RV2`, `RV4`).
3. **The number moved to 1.95** and `openehr-loco` builds on it, which was
   confirmed by running `cargo +1.95 test` rather than by observing that 1.95 is
   above 1.94.

**The check was shown to fail.** Deleting `rust-version` from one manifest, and
re-running with a fabricated stable version, both produce a red job naming the
files — the standard `W0.28` asks of a fuzz property, applied to a CI check.

**Residual.** Two, both stated in the specification rather than left implicit:
the eight fuzz crates are not built by the job, because `cargo fuzz` needs
nightly; and only N−3 and stable are built, so a break appearing only on an
intermediate release reaches a user before it reaches CI.

---

## W-10 — the published version was stated in five places and updated in one — **Low, fixed**

**Claimed.** [`index.md`](index.md), the file that says what this repository
*is*: "All eight are on crates.io at **0.2.0**." Likewise `AGENTS.md` ("live on
crates.io at **0.2.0**, and **the tree has moved past them**"), `CLAUDE.md`, and
a paragraph in each about what the next release would be.

**Found.** 0.3.0 was published on **2026-08-04**, sixteen days before this
finding. `agents/publishing.md` — the file whose whole job is tracking releases —
says so, correctly, with a per-crate table. The four other files that also state
the version were not touched by the release commit, so a reader of the
specification was told the tree was ahead of a release that had already gone
out, and told to plan a release that had already happened.

**Consequence.** Small and entirely of the "which file do I trust" kind, which
is the kind `W0.1` exists to prevent: *a normative statement MUST exist in
exactly one place, because where two files state one rule, one of them is a copy
and a copy is a future divergence.* The version is not a normative statement, but
it behaves like one — five copies, one maintained.

**Fixed** by making the other four defer rather than restate: each now names
`agents/publishing.md` as the file that tracks this. The version still appears in
them, because a reader needs the number in front of them; what changed is that
they say where it comes from.

**Why it is not Medium.** Nothing was published on the strength of it and no
behaviour depended on it. It is here because the *shape* is `W-02`'s and
`W-05`'s, and this repository's register is mostly a record of that shape
recurring in cheaper and cheaper forms.

---

## W-11 — one specification said fourteen crates, another said seventeen — **Medium, fixed**

**Found.** `db:W16.1` — a **normative** requirement, in the persistence
specification — read: "The repository holds **fourteen** crates: `openehr`,
`openehr-store`, six dialect crates, and six `openehr-<engine>-fuzz` harnesses."

There are seventeen. `openehr-loco`, `openehr-assets`, and `openehr-fuzz` were
added and the requirement was not amended, while `spec/index.md`, `README.md`,
`AGENTS.md`, `CLAUDE.md`, and `agents/index.md` all said seventeen.

**Why this is worse than a stale README.** `W0.2` settles a disagreement between
a specification and a descriptive file: the specification governs. It has nothing
to say when a specification disagrees with a *specification*, and a reader
following the rule as written would have concluded that three crates in the tree
should not exist. The requirement had already been amended once, when the layout
changed, and the amendment did not survive the next three additions.

**Fixed.** `W16.1` now says eighteen — seventeen plus `openehr-store-fuzz`, added
by the same change — and enumerates them, and states the eight that are
published rather than deriving the count. The note recording what it used to say
stays with it (`W0.16`).

**Residual — closed 2026-08-20 by `W0.39`.** This said: "a count in prose is
checkable and is not checked… the next count to go stale will go stale
silently." [`scripts/check-docs.py`](../scripts/check-docs.py) now derives every
countable claim from the tree and checks the documents against it, and the
`claims` job runs it. It found two stale counts on its first run —
`agents/index.md` still said nine unpublished crates and still said the released
version was **0.2.0**, a sixth file with the defect **W-10** records in five.

---

## W-12 — nine tests in `openehr-assets` that CI never ran — **Medium, fixed**

**Found.** The `test` matrix listed nine crates. `openehr-assets` was not one of
them, and the `assets` job runs `cargo run -- check` and nothing else. The crate
has nine `#[test]`s in `src/main.rs`, and none had ever been executed by CI.

Nor had clippy or `cargo doc`, on the crate whose job is to fail the build when a
generated artifact is stale — so the checker was the one component nothing
checked.

**Consequence.** Latent rather than realised: run by hand, all nine pass, clippy
is clean, and the docs build. What was wrong is that nobody could have known
that, and "nine tests" and "nine tests, three of which stopped compiling" report
the same green when neither is run. That is `W-02` at one remove — the workflow
existed, and covered nine crates of ten.

**Fixed.** `openehr-assets` is in the `test` matrix. Verified by running the same
three commands the job runs.

---

## W-13 — the layering guard's list omitted eight of seventeen crates — **Medium, fixed**

**Claimed.** [`index.md`](index.md) `W0.15`: "The guard against `W0.14` MUST
cover **every** engine crate, and the coverage MUST itself be checked. A
comparison that omits a dialect cannot find that dialect identical to another,
and reports the same green as a complete one."

**Found.** The `layering` job — which enforces that `openehr` and
`openehr-store` depend inward only, and which exists because a probe once added
`openehr-store` as a dev-dependency of `openehr` and nothing objected — checked
against a list of nine crate names written into the YAML. `openehr-assets` and
all seven fuzz crates were absent.

A dev-dependency from `openehr` onto `openehr-sqlite-fuzz` is a cycle:
`openehr-sqlite-fuzz` → `openehr-sqlite` → `openehr-store` → `openehr`. The guard
could not see it. Neither could it see one onto `openehr-assets`, which depends
on every dialect crate in the tree.

**This is `W-01` exactly**, and `W-01`'s own lesson stated as a rule in
`CLAUDE.md`: *a guard is only as wide as its input list.* The cross-dialect check
compared five dialects and the sixth was a copy of another. Here the guard
compared nine crates of seventeen, and the ones it skipped were the ones with the
most edges.

**Fixed.** The list is **derived** — `ls -d */Cargo.toml` — and the job asserts
the derived count is eighteen, so a crate the guard cannot see fails the job
rather than being silently exempt. The check also now recognises
`[dev-dependencies.x]` table syntax, which the inline-only `^x *=` pattern would
have missed.

**Shown to fail.** A `[dev-dependencies.openehr-sqlite-fuzz]` block added to
`openehr/Cargo.toml` produces `openehr/Cargo.toml depends on
openehr-sqlite-fuzz; dependencies run inward only`. Against the old list, it
produced nothing.

---

## W-14 — eight crates carried no licence at all — **Medium, fixed**

**Claimed.** [`index.md`](index.md) `W0.22`: "**Every** crate MUST carry the same
licence expression." `W0.23`: "**Every** crate MUST ship a `LICENSE.md` naming
all five."

**Found.** Nine did. The other eight — `openehr-assets` and the seven fuzz
harnesses — had **no `license` field in `Cargo.toml`**, and seven of the eight
had no `LICENSE.md` either. The `claims` job's licence check iterated a
hand-written list of the same nine.

The guard and the gap were the same list, which is why neither found the other.

**Consequence.** Real, if small. These crates are not published, but they are
distributed as source in this repository, and a file with no licence in a
licensed tree is precisely the ambiguity a five-way grant exists to remove.
`W0.23`'s own reasoning applies: shipping the licence for some files and not
others understates the grant, "which is the same defect class as claiming more
than is verified, pointed the other way".

**Fixed.** All eighteen crates declare the expression and ship `LICENSE.md`, and
the check derives its list from the tree and asserts the count — the same fix as
`W-13`, for the same reason.

**Shown to fail.** Deleting the `license` line from `openehr-mssql-fuzz`
produces a red job naming the file.

---

## W-15 — a fuzz seed corpus that nothing checked — **Low, fixed**

**Found** while writing the check, not by suspecting the corpus.
`corpus/canonical_json/` holds seven committed seeds; two are real compositions
and five are deliberate malformations. That mix is correct and useful. What was
missing was any way to tell it from the mix that is *not* correct — a seed that
used to be a real composition and quietly stopped parsing after a field became
required.

A fuzz run's output cannot distinguish them. libFuzzer reports how many files it
read, not how many got past `serde_json`, so seven seeds and seven files the
deserializer rejects produce the same line. `W0.30` requires the corpus and said
nothing about checking it, on the reasoning that a committed file does not
change — which is true of the file and not of the type it is a seed *for*.

**Fixed.** `openehr/tests/fuzz_seeds.rs` asserts that each structured target's
corpus still spans both answers: at least one instance the deserializer accepts,
so the target reaches past the lexer, and at least one it refuses, so the error
paths are driven by something other than mutation. `W0.30` is amended to require
it.

**The first version of the check was wrong**, and the way it was wrong is the
finding's other half. It required *every* seed to parse, which failed
immediately on `seed-307cfdd3` — `{"_type":"COMPOSITION","name":{}}`, a
deliberate malformation doing exactly its job. A check that would have deleted
half the corpus is not a stricter check; it is a different and worse one, and it
looked like a finding for about a minute.

---

## W-16 — the conformance ladder was written out four times, and two had drifted — **Medium, fixed**

**Found** by grepping for the table rather than by suspecting it. The four-level
ladder — the definition of what `Dialect`, `Schema`, `Store`, and `Verified`
mean, which every conformance claim in this repository is denominated in — was
stated in full in four documents:

| Where | Status |
| --- | --- |
| `spec/databases/00-conformance.md` `C0.8` | the owner; `W0.8` says so |
| [`index.md`](index.md) `W0.8` | says "This repository has **one** ladder, defined in `databases/00-conformance.md`", then reproduces it |
| `openehr-store/spec/conformance.md` | a third copy |
| [`../agents/conformance.md`](../agents/conformance.md) | a fourth |

**Two of the four had already drifted.** `W0.8`'s copy read "A transcript
against that engine's own server: applied cleanly…" where the owner reads "…the
script applied cleanly…", and `agents/conformance.md` had rewritten every cell
shorter — "Golden tests + `conformance::check_dialect`", "Store, run in CI on
every commit". None of the differences changed what a level means. That is the
point: they were four texts drifting apart at the rate prose drifts, and the
next difference would not have announced itself as the interesting one.

**Why the copies were not simply deleted.** `W0.1`'s literal remedy is one
occurrence and three links. Applied here it would make all four documents worse:
someone reading `agents/conformance.md` to decide whether a crate's claim is
honest should not have to open a second file to learn what the claim means, and
`W0.8` reproducing the ladder is why the repository index is readable on its own.

The rule `W0.1` is really enforcing is *one place where the statement can be
changed* — not one place where it can be read.

**Fixed** by marking the copies rather than removing them (`W0.38`):

```
<!-- shared: conformance-ladder (owner) -->  …  <!-- /shared: conformance-ladder -->
<!-- shared: conformance-ladder (copy)  -->  …  <!-- /shared: conformance-ladder -->
```

`scripts/check-docs.py` fails when a copy diverges from its owner and rewrites
it with `--fix`. The three copies are now byte-identical to `C0.8`.

**Shown to fail.** Shortening one cell in `agents/conformance.md` produces
`block 'conformance-ladder' differs from its owner
spec/databases/00-conformance.md`, and `--fix` restores it.

**Residual.** One block is bound this way. The ladder was the worst case and is
not the only duplicated passage — the "two gates, not one" argument, the
`W-01`/`F-08` story, and the "with a row present" explanation each appear in
three or four documents in their own words. Those are **rationale** rather than
normative statements (`db:C0.2`), so they are not covered by `W0.38` and are not
checked. Whether they should be is an open question, not a settled exemption.

---

## W-17 — a gap that was closed, and four documents that went on describing it — **Low, fixed**

**Found 2026-08-21** by cross-checking the register against the tree while
looking for something else: `README.md` listed "No concurrency testing of the
SQLite store" as a known gap and cited **`db:D-02`** for it. `db:D-02` is
**fixed**, and it is not about that — it is about two store requirements being
unverifiable as written. The citation was wrong and the gap was not a gap.

`openehr-sqlite/tests/concurrency.rs` has driven both races since 2026-08-01. It
passes. It found `db:D-06` when it was written.

**Four documents said otherwise**, and two of them are normative:

| Where | Said |
| --- | --- |
| `spec/databases/04-shredding-and-reconstruction.md` `R4.5` | "**Not verified.** … nothing in this repository exercises concurrent readers against writers" |
| `spec/databases/05-versioning-and-history.md` `H5.4` | "**Not verified.** Nothing in this repository exercises concurrent writers" |
| `openehr-store/spec/conformance.md` | "Nothing exercises concurrent writers." |
| [`index.md`](index.md) → this file's *What this audit did not cover* | "nothing exercises concurrent writers" |

Meanwhile [`databases/conformance-matrix.md`](databases/conformance-matrix.md)
marked both `•` with the evidence named. So the matrix and the requirements it
indexes disagreed, in one directory, for twenty days.

**This is `W0.3` pointed the other way.** Overclaiming is the dangerous
direction and is what most of this register is about. Underclaiming is the quiet
one and costs the same way twice: a reader redoes work that already exists, and
a reader distrusts a guarantee that actually holds. Neither is visible to
anybody who does not go and check.

**Why it survived.** A requirement that says "not verified" is *correct when
written* and becomes wrong by someone else's success. Nothing was watching that
direction — every check in this repository looks for a claim exceeding its
evidence, and none looked for evidence exceeding its claim. The "what this audit
did not cover" section has the same property: it is a list of things that are
true until they are fixed, and closing one does not touch it.

**Fixed**, and mechanically:

- All four now state what verifies each requirement, with the test's name and
  what it found. `H5.4` records that the test **failed first** — the guarantee
  held, the reporting did not, which is `db:D-06` and is the argument for
  writing the test rather than reasoning from the unique index.
- `scripts/check-docs.py` gained a check: **a requirement marked `•` in a
  conformance matrix must not describe itself as unverified.** It found exactly
  these two and nothing else, across 65 satisfied requirements in both matrices.

**Residual, stated because closing a gap is how this one started.** Concurrency
is exercised for **SQLite only**, and only for the two races `db:R4.5` and
`db:H5.4` name. No other engine has a `Store`, so there is nothing else to race;
that changes the moment one does.

---

## W-18 — the mutation gate never ran on a commit made to `main` — **Medium, fixed**

**Found 2026-08-21** by asking what had checked the previous three days' work,
rather than by anything failing.

The `mutants` job carried `if: github.event_name == 'pull_request'`, with the
reason written beside it: *"On a push to main there is no pull request and no
meaningful diff scope, so the job does not run rather than running over nothing
and reporting success."*

The premise is false. A push carries `github.event.before`, which is exactly the
diff scope, and the consequence was that **every commit made directly to `main`
bypassed the gate**. Nine in a row, on 2026-08-20 and -21, including:

- `Interval::contains` rewritten against `semantic_cmp` — the function that
  decides whether a clinical value falls inside a reference range;
- the whole `DV_URI`/`DV_EHR_URI` validation path (`lib:A-36`);
- the AQL lexer's string handling and the `FROM` renderer (`lib:A-37`);
- signed numeric literals and `Literal::Number` rendering (`lib:A-27`);
- three new properties in `openehr_store::conformance`.

Every one of those is exactly what `T13.2` exists for: changed lines that no
test notices when their meaning changes.

**What made it invisible.** The job reported **`skipped`**, and a skipped job in
a green summary reads as "nothing to do here". That is `C0.13` — *a skip is
indistinguishable from a pass* — the rule this repository wrote for database
jobs, working exactly as predicted against a job nobody had thought to apply it
to.

**Fixed.** The job runs on `push` as well, diffing `github.event.before..HEAD`
— two dots, because a push is not a merge and has no base to find, where a pull
request wants three. The one genuinely baseless case, the first push of a new
branch where `event.before` is forty zeros, is skipped **loudly in the step**
with a note saying there was nothing to check, rather than by an `if` that
cannot say why.

**Run by hand over the two files where a silent wrong answer would matter
most**, and it found three survivors — all of them in `A-36`'s own changes:

| Mutant | Meaning |
| --- | --- |
| `DvUri::rest -> ""` | nothing tested `rest` with a non-empty answer |
| `Display for DvUri -> Ok(())` | nothing asserted what a URI displays as |
| `Display for DvEhrUri -> Ok(())` | nor an EHR URI |

`Interval::contains` and the `SemanticOrd` machinery — the rewrite most likely
to change a clinical answer — had **none**: 24 caught, 0 missed.

**`rest` is the instructive one.** It *was* asserted, in
`guarantees::a_uri_that_never_saw_a_constructor_is_reported_rather_than_panicking`,
which checks the colon-less case — where the correct answer is `""`, the exact
value the mutant substitutes. A test that agrees with the bug is not a test, and
no amount of reading it would say so; mutating the function is what says so.

`Display` matters for a reason worth writing down: it is what a `ParseError`
embeds and what a caller shows whoever wrote the document. A `DV_EHR_URI` that
rendered as nothing would make a link error unreadable at exactly the moment
somebody needed to read it.

**Fixed** by two tests in `uri.rs`, and re-mutated to confirm all three are now
caught rather than assumed to be.

**Confirmed in CI, and the confirmation is weaker than it looks.** On
`7d91a9d` the job ran on a push for the first time — four crates, all green.
It tested **zero mutants**, correctly: the only changed lines were inside
`#[cfg(test)]`, which cargo-mutants does not mutate. A green run over nothing
is not evidence, and in a summary it looks identical to a green run over two
hundred mutants.

That is the same shape as the `skipped` job this finding is about, one level
down, so the job now says which zero it was: a `::notice::` when the diff
changed no mutable code, and a count when it did. The count excludes the
unmutated baseline from `outcomes.json` — including it reports 16 where
cargo-mutants says 15, and a count that is off by one is a count nobody trusts
twice.

**Residual — closed 2026-08-21/22.** The whole session diff was mutated by
hand: `--in-diff` over `d910470..HEAD` in both `openehr` (67 mutants) and
`openehr-store`. It found `lib:A-39` and `db:D-10`, both now fixed. What remains
is two timeouts in the AQL lexer, where mutating an index arithmetic stops the
loop advancing — detected by hanging, which is detection — and three survivors
in `dialects_are_distinct`, which pre-date this and are killed when measured
from `openehr-sqlite` as `lib:T13.2` records.

**A second residual, found by being asked whether tasks were being tracked.**
They were not, other than by the registers. Two `pub fn` signatures changed
after 0.5.0 shipped and `CHANGELOG.md` said nothing — nobody would have noticed
until an upgrade. `scripts/check-docs.py` now fails when library source has
changed since the last release commit and the newest changelog heading is not
`Unreleased`.

**The git tags, closed 2026-08-22.** This said: "`v0.2.0` and `openehr-v0.3.0`
use two naming schemes and nothing was tagged for 0.4.0 or 0.5.0, both of which
are on crates.io. Choosing the scheme is not this finding's to make."

The scheme is `v<version>`, annotated, on the release-record commit, and
[`../agents/publishing.md`](../agents/publishing.md) now owns it. `v0.3.0`,
`v0.4.0` and `v0.5.0` exist; `openehr-v0.3.0` stays as history. `v0.2.0` turned
out never to have been **pushed**, only created locally — so the remotes had no
release tags at all.

**And this paragraph was wrong for about ten minutes**, which is worth leaving
in. `W-17` is the finding that a closed gap goes on being described as open
because nothing watches that direction, and it recurred here within a day of
being written — in the very entry that records the tags as missing. It was
caught by re-reading the register after the work, not by any check. There is
still no mechanism for this direction beyond the one `W-17` added, which only
covers requirements marked satisfied in a conformance matrix.

---

## W-19 — every published crate's README told a reader to depend on the previous release — **Low, fixed**

**Found 2026-08-30**, by reading the rendered crate pages during the
"update, upgrade, harmonize, annotate, audit, fix" sweep, not by anything
failing — `check_versions` had been green throughout.

0.8.0 shipped 2026-08-29. Every published crate's README, plus the root
`README.md` and `INSTALL.md`, carried a fenced `Cargo.toml` snippet reading
`openehr = "0.7"` (or `openehr-sqlite`, `openehr-store`, and so on) — the
dependency line a reader copies, still pinning the release before the one
that had just gone out. `openehr-rust.github.io`'s `content/crates/*.md`,
vendored verbatim from those same READMEs, carried the same string.

**What made it invisible.** `check_versions` in `scripts/check-docs.py`
matches prose restatements — `live on crates.io at **X.Y.Z**` and its
variants — because that is the shape `W-10` was about. A fenced snippet
reading `openehr = "0.7"` matches none of those patterns: it is not prose,
it names no crate the regex looks for, and it is exactly the kind of claim a
reader trusts most, because it is the one they paste into their own project
without reading a sentence around it.

**Consequence.** Small: `^0.7` and `^0.8` overlap in no way that breaks a
build, so nobody's `cargo build` failed. But a reader who followed the
README got a crate two releases old with no indication a newer one existed,
which is the opposite of what a version-pin snippet is for.

**Fixed** in two parts. First, the eight READMEs, `README.md`, and
`INSTALL.md` now read `"0.8"`; `openehr-rust.github.io/bin/sync-content.mjs`
was re-run so the vendored copies match. Second, `check_dependency_snippets`
was added to `scripts/check-docs.py`: it scans every `*.md` file for a line
matching `openehr[a-z-]* = "X.Y"` and compares `X.Y` against the current
local version's `major.minor`, catching a patch release correctly (`0.8` and
`0.8.1` are the same caret range) while still failing on a stale minor.

**Why it is not Medium.** Nothing that depends on this repository broke, and
no behaviour changed. It is here because the *shape* is `W-10`'s and `W-17`'s
again — a value copied into documents nothing was watching — recurring in a
place none of those checks happened to look.

---

## What this audit did not cover

Stated so that "not examined" and "examined and sound" stay distinguishable
(`W0.3`):

- **The `openehr` crate's Reference Model conformance.** 150 unit tests, 144
  integration tests, and 8 doctests pass, and the crate has its own audit register
  with seventeen findings. This audit did not re-verify any of them against the
  openEHR primary sources.
- **SQL Server and Oracle DDL.** Neither has been parsed by the engine it names.
  SQL Server 2022 segfaults under qemu on arm64; the Oracle images require
  registry authentication. Both remain at **Dialect**, which is the correct level
  for "no server has seen it", and that is a gap in evidence rather than a
  judgement that the DDL is wrong.
- **The SQLite store under concurrency — no longer true, kept as a correction.**
  This said "nothing exercises concurrent writers". It was closed on 2026-08-01
  by `openehr-sqlite/tests/concurrency.rs` (`db:D-02`, and `db:D-06` which that
  test found), and four documents including two **normative requirements** went
  on saying otherwise until 2026-08-21. See **W-17**.

  What is still uncovered: concurrency on any engine other than SQLite, and any
  race other than the two that `db:R4.5` and `db:H5.4` name.
- **Fuzzing.** No parser in either crate is fuzzed; `openehr`'s own register
  carries this as **A-09**.
