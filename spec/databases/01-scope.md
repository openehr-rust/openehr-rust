# 1. Scope

**Rewritten 2026-08-01** to describe this repository. The previous text required
"OpenEHR R5 (5.0.0), R4 (4.0.1), and R3 (3.0.2)" — those are FHIR releases —
along with per-version namespaces `r5`/`r4`/`r3` and support for "all resource
types". openEHR has no resources and no such releases. See
[`spec/audit.md`](../audit.md) **W-04**.

Requirement prefix: `S1`.

## What the persistence layer is

- **S1.1** The core MUST provide one engine-agnostic storage model for openEHR:
  the tables, the projection from openEHR objects onto rows, the commit rules, and
  a conformance suite. It MUST NOT be specific to any SQL engine.
- **S1.2** The core MUST target the openEHR **Reference Model 1.1.0**, matching
  the [`openehr`](../../openehr) crate (`lib:S1.16`). `ARCHETYPED.rm_version` is
  carried and not enforced: data authored against 1.0.2 is storable, and the
  version it declares is preserved so a caller can decide.
- **S1.3** The core MUST store every openEHR class the `openehr` crate models,
  without loss, and MUST NOT reject a document because it contains archetyped
  content the core does not understand. Understanding archetyped content is not a
  precondition for storing it, and treating it as one would make the store
  useless the first time somebody authored a new archetype.
- **S1.4** Each engine crate MUST declare a **minimum engine version** and MUST
  NOT silently work below it. The floor is a dialect decision because it is driven
  by dialect facts — an identifier length, a boolean type, a JSON type, a trigger
  syntax — and it MUST be stated in that crate's annex rather than inherited by
  assumption.

## What the persistence layer is not

The exclusions below are decisions, not omissions. Each names why, because a
reader deciding whether to use these crates needs the reason more than the fact.

- **S1.5** The core MUST NOT shred openEHR content into per-attribute tables.
  A `COMPOSITION` contains whatever its **archetype** says; archetypes are
  authored long after the software ships, and neither this core nor the `openehr`
  crate implements them (`lib:S1.4`). A schema shredded from the Reference Model
  alone would have one column per RM attribute and a generic key/value table for
  everything clinically interesting — which is a document store with extra joins,
  and a migration every time an archetype is published.

  This requirement exists because the imported specification required the
  opposite, at length and with a table count. It is the single largest difference
  between storing FHIR and storing openEHR, and getting it wrong is not a
  performance mistake but an architectural one.

- **S1.6** The core MUST NOT execute AQL. Executing AQL means resolving archetype
  paths against stored content, which needs an archetype engine. What the schema
  offers instead is an index over the attributes the Reference Model fixes, which
  is what an AQL `FROM` clause filters on before it reaches into content (§6).
- **S1.7** *(amended 2026-08-02)* The **core** MUST NOT implement an HTTP
  service or a command-line tool. `openehr`, `openehr-store`, and the six engine
  crates MUST have no HTTP dependency, no async runtime, and no server.

  That boundary is the point, and it is what the original wording protected: a
  program wanting storage must not thereby acquire a web framework. It survives
  intact.

  What changed is the sentence "nothing in this repository builds either".
  `openehr-loco` now does — a service crate that depends *inward* on
  `openehr-sqlite` and can be deleted without touching anything it depends on.
  Amending rather than quietly contradicting, because a specification that
  disagrees with the tree is worse than one that is narrower than expected
  (`C0.21`).

- **S1.19** A service crate MUST NOT implement clinical behaviour. Validation,
  the commit rules, the chain, ordering, and lexical fidelity belong to the
  store; a rule enforced at the HTTP edge is a rule that stops applying the
  moment somebody uses the store directly.

  What a service crate *does* own is the translation: which status code a store
  outcome earns. That is real work and nothing else does it.

- **S1.20** A service MUST distinguish **deleted** from **never existed**. A
  composition whose latest version carries a deleted lifecycle state answers
  `410 Gone`; one with no versions answers `404 Not Found`.

  openEHR removes nothing (`H5.2`), so a deleted record demonstrably existed and
  its history is still readable. `404` would tell a caller that a record it once
  held never was — and a clinician or auditor told "not found" stops looking.

- **S1.21** Sections 7 and 8 remain **retired** (`C0.6`). The requirements
  withdrawn there described a FHIR REST surface and are not restored by a
  service existing; reinstating text nobody wrote, and mapping it to regulatory
  obligations in §13, is the failure `C0.16` records. A service specification
  will be written when the service has behaviour worth specifying.
- **S1.8** The core MUST NOT authenticate or authorize. It records who acted;
  establishing who they are belongs to the deployment (§12, `lib:X11.1`).
- **S1.9** The core MUST NOT resolve external terminologies, convert units, or
  interpret timing expressions. Those exclusions belong to the `openehr` crate
  (`lib:S1.8`–`lib:S1.10`) and are inherited here unchanged: a store does not get
  to be more permissive than the model it stores.
- **S1.10** The core MUST NOT encrypt at rest or manage keys. Both belong to the
  deployment and the engine.

## How an exclusion behaves

- **S1.11** Where this specification defines an operation an engine crate does not
  implement, the operation MUST return `StoreError::Unsupported`, naming the
  engine, what was asked for, and where the exclusion is recorded. It MUST NOT
  return a plausible default, an empty result, or a silent success.
- **S1.12** An engine crate below **Store** level MUST NOT ship a partial `Store`
  implementation whose unimplemented methods return empty results. A crate that
  has no store has no store, and says so.

## Engine bindings

Non-normative summary; each crate's annex is authoritative for its own row.

| Crate | Engine | Declared floor | Level |
| --- | --- | --- | --- |
| `openehr-sqlite` | SQLite | 3.35+ (bundled) | **Store** |
| `openehr-postgresql` | PostgreSQL | 18 | **Schema** |
| `openehr-mysql` | MySQL | 8.4 | **Schema** |
| `openehr-mariadb` | MariaDB | 11.4 | **Schema** |
| `openehr-mssql` | SQL Server | 2019+ | **Dialect** |
| `openehr-oracle` | Oracle Database | 12.2+ | **Dialect** |

Oracle's floor is now stated, in that crate's annex, with the fact that sets
it: identifiers were 30 bytes before 12.2 and 128 after, and several generated
names here exceed 30, so the schema is not installable below 12.2.

All six crates now have a dialect annex (`X15.6`). Every one is **proposed**
rather than ratified (`X15.9`), so none counts as evidence for a level.

- **S1.13** `openehr-sqlite` pins its engine rather than discovering it: the
  `bundled` feature compiles a known SQLite instead of linking whatever the host
  ships. The generated DDL and the JSON functions the store uses are
  version-dependent, and a store whose semantics depend on the operating system's
  patch level is not portable.

---

Part of the [openEHR persistence specification](index.md).
