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
| `S1.4` | the crate MUST NOT implement the Archetype Model | a port MUST declare a minimum engine version |
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
- **W0.7** Neither domain may allocate identifiers in the other's sections.
  `W0.x` is reserved for this file and MUST NOT be defined by either domain.

## Conformance levels

Two ladders existed: `Dialect / Schema / Store / Verified` in the persistence
crate's documentation, and `Scaffold / Schema / Store / Reference` in the
imported core. They overlapped in name and disagreed in meaning, which is worse
than either alone.

- **W0.8** This repository has **one** ladder, defined in
  [`databases/00-conformance.md`](databases/00-conformance.md):

  | Level | Means | Evidence required |
  | --- | --- | --- |
  | **Dialect** | Emits DDL for the shared schema. | The golden DDL tests, and `conformance::check_dialect`. |
  | **Schema** | The engine itself has executed that DDL. | A transcript against that engine's own server: applied cleanly, applied *again* cleanly, and the append-only tables observed refusing `UPDATE` and `DELETE` **with a row present**. |
  | **Store** | Implements `Store` against a real database. | `conformance::run` passing against that engine. |
  | **Verified** | Store level, run in CI against the engine's own server on every commit. | A CI job that provisions the engine and fails — not skips — without it. |

- **W0.9** A crate MUST NOT claim a level it has not earned, and its README and
  crate documentation MUST state its level in the first screenful.
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

**Fourteen crates**, each its own Cargo workspace: eight that are published, and
six fuzz harnesses that are not.

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

All eight are on crates.io at **0.1.1**.

### The fuzz crates

One per dialect — `openehr-<engine>-fuzz` — driving two properties: that an
identifier cannot escape its own quoting, and that every logical column type maps
to something usable.

- **W0.25** A fuzz crate MUST declare `publish = false`. It is a test harness; a
  registry release would claim it as part of the library's surface.
- **W0.26** The properties a fuzz target drives MUST live in `openehr-store`,
  shared by every fuzz crate. Six copies of one assertion is the arrangement that
  produced **W-01**, and a fuzz harness repeating the mistake it exists to catch
  would be worse than none.
- **W0.27** A fuzz target MUST be **run**, not merely committed, with a committed
  seed corpus and a bounded budget in CI. A committed target nobody executes is a
  claim rather than a check (`db:T11.9`).
- **W0.28** A fuzz property MUST be shown to **fail** against a deliberately
  broken implementation. A check that cannot fail is indistinguishable from a
  control that works (`db:T11.10`).

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
  for that version. See [`audit.md`](audit.md) **W-03**.

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

Contributors should also read [`AGENTS.md`](../AGENTS.md), which is operational
guidance and not normative (`W0.2`).
