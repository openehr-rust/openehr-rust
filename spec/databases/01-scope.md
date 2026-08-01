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
- **S1.7** The core MUST NOT implement an HTTP service or a command-line tool.
  Nothing in this repository builds either, and no documentation may suggest
  otherwise (`C0.11`). Sections 7 and 8 are retired for this reason (`C0.6`).
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
| `openehr-oracle` | Oracle Database | 12.2+, undeclared | **Dialect** |

Two rows are open. Oracle's floor has not been written into an annex —
identifiers were 30 bytes before 12.2 and 128 after, so the generator's naming is
only safe on 12.2+. And **no crate has a dialect annex at all**: all six
`spec/` directories are empty, which `X15.6` requires them not to be.

- **S1.13** `openehr-sqlite` pins its engine rather than discovering it: the
  `bundled` feature compiles a known SQLite instead of linking whatever the host
  ships. The generated DDL and the JSON functions the store uses are
  version-dependent, and a store whose semantics depend on the operating system's
  patch level is not portable.

---

Part of the [openEHR persistence specification](index.md).
