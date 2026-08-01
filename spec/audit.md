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

Eight findings: three High (**W-01**, **W-02**, **W-04**), three Medium
(**W-03**, **W-05**, **W-06**), two Low (**W-07**, **W-08**). **Seven are
fixed** — W-01, W-02, W-04, W-05, W-06, W-07, and W-08. **W-03** is fixed going
forward only: the published `openehr` 0.1.0 is immutable and keeps its wrong
`repository` field.

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
| `test` | `clippy --all-targets`, `test`, and `doc` for each of the eight crates separately, because each is its own workspace and a single `--workspace` invocation would silently cover one of them |
| `examples` | the five runnable tutorials the README points at |
| `schema` | `verify-schema.sh` against real PostgreSQL 18, MySQL 8.4, and MariaDB 11.4 containers |
| `claims` | that `openehr-mssql` and `openehr-oracle` still claim only **Dialect**, and that the licence expression is harmonized across all eight crates (`W0.22`) |

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
- **The SQLite store under concurrency.** `conformance::run` passes against a real
  in-process database, but nothing exercises concurrent writers.
- **Fuzzing.** No parser in either crate is fuzzed; `openehr`'s own register
  carries this as **A-09**.
