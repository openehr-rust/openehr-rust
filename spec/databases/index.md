# The openEHR persistence specification

This directory is **normative** for storing openEHR in a SQL database. It covers
the storage model, the projection from openEHR objects onto rows, the commit
rules, the SQL dialect boundary, and the conformance suite every engine runs.

It is one of two domain specifications in this repository. The other,
[`openehr/spec/`](../../openehr/spec/index.md), governs the Reference Model
library itself. The root [`spec/index.md`](../index.md) governs both and resolves
what happens when their requirement identifiers collide.

Requirements use RFC 2119 keywords (MUST, SHOULD, MAY) as defined in
[§0 Conformance](00-conformance.md). Identifiers in this directory are written
unqualified — `M3.4`, `T11.2` — and are cited elsewhere as `db:M3.4` (`W0.5`).

Behaviour is defined here first, then implemented and verified. When code and
spec disagree, reconcile them — do not let them drift. Operational guidance for
contributors lives in [`AGENTS.md`](../../AGENTS.md); this directory defines
**what must be true**, not how to work.

---

## Provenance, and why identifiers look discontinuous

This directory was originally imported from a FHIR database specification and
mechanically text-substituted, `FHIR` → `OpenEHR`. The substitution was complete
at the string level — no literal `FHIR` remained — which is exactly what made it
hazardous: it read as an openEHR specification and required a FHIR one, including
a **shredded** schema of "7,355 tables", `map/` and `gen/` crates that do not
exist, and releases "R5 / R4 / R3" that openEHR does not publish.

**All fourteen numbered sections were rewritten on 2026-08-01**, and both status
files rebuilt. See [`spec/audit.md`](../audit.md) **W-04**.

Identifier discipline was kept through the rewrite (`C0.5`, `C0.19`), which is
why the numbering looks odd in places and MUST NOT be tidied:

- **Withdrawn requirements keep their numbers**, listed in a table at the foot of
  the section that held them, with the reason. A citation to `M3.4` resolves to
  "withdrawn: shredding" rather than to nothing.
- **Amended requirements keep their numbers** and are marked *(amended)*.
- **New requirements take the next unused ordinal**, never a vacated one — so §3
  restarts at `M3.19`, §4 at `R4.8`, §16 at `W16.16`.

No identifier changed meaning. A citation written before the rewrite still
resolves to the same subject, or to an explicit withdrawal.

| Section | State |
| --- | --- |
| [0. Conformance](00-conformance.md) | **Rewritten** |
| [1. Scope](01-scope.md) | **Rewritten** |
| [2. Schema generation](02-schema-generation.md) | **Rewritten** |
| [3. Storage model](03-storage-model.md) | **Rewritten** |
| [4. Projection and reconstruction](04-shredding-and-reconstruction.md) | **Rewritten** |
| [5. Versioning and history](05-versioning-and-history.md) | **Rewritten** |
| [6. Search](06-search.md) | **Rewritten** |
| [9. Validation](09-validation.md) | **Rewritten** |
| [10. Operations](10-operations.md) | **Rewritten** |
| [11. Conformance testing](11-conformance-testing.md) | **Rewritten** |
| [12. Trust, principal, and audit](12-trust-principal-and-audit.md) | **Rewritten** |
| [13. Compliance mapping](13-compliance-mapping.md) | **Rewritten** |
| [15. Portability and dialects](15-portability-and-dialects.md) | **Rewritten** |
| [16. Repository and release](16-repository-and-release.md) | **Rewritten** |
| [locale-accent-folding](locale-accent-folding.md) | **Withdrawn** — nothing here does text search (`P6.6`) |
| [unbounded-string-search…](unbounded-string-search-must-have-bounded-adjunct-and-checksum-adjunct.md) | **Withdrawn** — same reason (`P6.9`) |

The rewrite was done by reading **the code**, not by re-deriving requirements
from the openEHR specifications. Where a requirement here is now a description of
what the code already does, that is a rubber stamp rather than a considered
generalization, and `C0.21` requires the two to stay distinguishable. For this
pass they are not — recorded as [`audit.md`](audit.md) **D-05** residual.

---

## Why one core

The engine crates share one implementation of everything that is not a SQL
spelling. That is the architecture, and it is a response to a specific failure:
the sibling FHIR monorepo gave each of six ports a full copy of the DDL
generator, and one copy spent that fork's entire life emitting another engine's
types because nothing compared them.

One crate that six depend on cannot drift from itself. A port states only where
it *departs*, and a departure has to be written down to exist (`C0.12`).

That is necessary and was not sufficient. This repository reproduced the same
defect anyway — see [`spec/audit.md`](../audit.md) **W-01** — because the guard
against it compared five of the six dialects. §15 is the section that exists to
keep the boundary honest.

## The shape of the storage model

openEHR is archetype-driven. The canonical JSON **is** the record; the relational
part is an *index* over the attributes the Reference Model itself fixes — who
committed, when, which archetype, which category, which setting. Five tables:

| Table | Holds |
| --- | --- |
| `openehr_ehr` | one row per health record |
| `openehr_versioned_object` | one row per version container |
| `openehr_version` | one row per committed version — **append-only** |
| `openehr_contribution` | one row per change set — **append-only** |
| `openehr_composition_index` | the RM-level projection of a composition |

Every stored instant occupies two columns: `…_text`, the authoritative exact
lexical form, and `…_utc`, derived and nullable, for ordering. §3 gives the
argument, which turns on partial precision being a clinical fact rather than a
missing value.

## Contents

### Framework

- **0.** [Conformance](00-conformance.md) — `C0.x`. Normative language,
  requirement-id grammar, the conformance ladder, departures, how to amend.

### The core requirements

- **1.** [Scope](01-scope.md) — `S1.x`
- **2.** [Schema generation](02-schema-generation.md) — `G2.x`
- **3.** [Storage model](03-storage-model.md) — `M3.x`
- **4.** [Projection and reconstruction](04-shredding-and-reconstruction.md) — `R4.x`
- **5.** [Versioning and history](05-versioning-and-history.md) — `H5.x`
- **6.** [Search](06-search.md) — `P6.x`
- **9.** [Validation](09-validation.md) — `V9.x`
- **10.** [Operations](10-operations.md) — `O10.x`
- **11.** [Conformance testing](11-conformance-testing.md) — `T11.x`
- **12.** [Trust, principal, and audit](12-trust-principal-and-audit.md) — `PR12.x`
- **13.** [Compliance mapping](13-compliance-mapping.md) — a table, not requirements

Sections **7** (REST API) and **8** (CLI) are retired: these are embeddable
libraries, and neither an HTTP server nor a command-line tool is in scope. The
numbering keeps the gap rather than renumbering, so an identifier still means
what it meant (`C0.6`).

### Monorepo framework

- **15.** [Portability and dialects](15-portability-and-dialects.md) — `X15.x`.
  What every engine crate shares by construction, what a dialect annex must
  contain, and what cross-engine agreement means.
- **16.** [Repository and release](16-repository-and-release.md) — `W16.x`.
  Layout, crate naming, versioning, and publishing.

### Status, not requirements

- [Conformance matrix](conformance-matrix.md) — which engine satisfies which
  requirement today. Non-normative; it records reality, not intent. Rewritten
  2026-08-01 against the current requirements.
- [Audit](audit.md) — the persistence findings register. Repository-wide findings
  live in [`spec/audit.md`](../audit.md).

## The engine crates

| Crate | Engine | Level |
| --- | --- | --- |
| [`openehr-sqlite`](../../openehr-sqlite) | SQLite 3 | **Verified** |
| [`openehr-postgresql`](../../openehr-postgresql) | PostgreSQL 18 | **Schema** |
| [`openehr-mysql`](../../openehr-mysql) | MySQL 8.4 | **Schema** |
| [`openehr-mariadb`](../../openehr-mariadb) | MariaDB 11.4 | **Schema** |
| [`openehr-mssql`](../../openehr-mssql) | SQL Server | **Dialect** |
| [`openehr-oracle`](../../openehr-oracle) | Oracle Database | **Dialect** |

Levels are defined in [§0](00-conformance.md). `openehr-sqlite` is at
**Verified** as of green run 30713623082, 2026-08-01; the three Schema claims are now checked on
every push rather than attested once ([`spec/audit.md`](../audit.md) **W-02**).
