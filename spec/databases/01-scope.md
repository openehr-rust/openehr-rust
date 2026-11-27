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

- **S1.6** *(amended 2026-09-13)* The core MUST NOT execute AQL **beyond the
  bounded subset §6 states**, and MUST refuse — by naming the construct, not
  by silently ignoring it or partially executing it — anything outside that
  subset.

  This reverses the blanket prohibition, not its own original reasoning:
  "executing AQL means resolving archetype paths against stored content"
  was true, and still is, of the general case — an archetype-authored
  `WHERE` clause over arbitrary nested structure genuinely needs an
  archetype engine this crate does not have (`lib:A-40`). What changed is
  the discovery that a **narrower, real, useful** case does not: one
  `EHR`, one archetype-predicated containment level, comparisons against
  scalar leaves — needs only the archetype index this schema already
  builds (`P6.12`) to find candidates, and [`openehr::path`], already
  built for validation and RM navigation, to read the fields a `WHERE`
  clause and a `SELECT` list name. Nothing here parses ADL, resolves a
  specialisation, or checks a constraint — `am`'s own boundary (`lib:A-40`)
  is untouched, because none of that is what the bounded subset needs.

  `S1.5`'s own prohibition — no per-attribute shredding — is undisturbed:
  the database's own queryable, indexed surface stays exactly what
  `P6.10`/`P6.19` already say it is, index-only. What executes AQL is
  application code in `openehr-store`, reading a canonical-JSON document
  the index already narrowed the search to, the same way any caller of
  `Store::get_version` already could — this requirement is about a new
  *capability offered*, not a new thing the *database* does.
- **S1.7** *(amended 2026-08-02)* The **core** MUST NOT implement an HTTP
  service or a command-line tool. `openehr`, `openehr-store`, and the six engine
  crates MUST have no HTTP dependency, no async runtime, and no server.

  That boundary is the point, and it is what the original wording protected: a
  program wanting storage must not thereby acquire a web framework. It survives
  intact.

  What changed is the sentence "nothing in this repository builds either".
  `openehr-axa` now does — a service crate that depends *inward* on
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

- **S1.22** *(added 2026-11-27, ITS-REST 1.1.0 `EHR` API, the maintainer's
  decision, `tasks.md` triage item 8)* A service MUST create an `EHR` the way
  ITS-REST defines it: the **server** builds the record and commits its first
  `EHR_STATUS` in the same request.

  - `POST /ehr` mints the `ehr_id`. `PUT /ehr/{ehr_id}` uses the one the
    caller names, and answers `409` when it exists.
  - The body is an optional `EHR_STATUS`. Without one the server commits the
    default: `is_modifiable` and `is_queryable` true, an anonymous
    `PARTY_SELF` subject, archetype `openEHR-EHR-EHR_STATUS.generic.v1`. A
    supplied status that is not valid RM is refused (`422`) before anything
    is written.
  - The status version is version 1 of a server-minted container. It is
    committed by the **verified caller**: the committer's identifier is the
    token subject and its name the token's `name`, as `PR12.19` requires of
    every commit. It is committed inside a `CONTRIBUTION` the server creates
    for that one act and that declares only that version (`H5.22`). Its
    `system_id` is the service's own (`OPENEHR_SYSTEM_ID`, default
    `openehr-axa.local`).
  - **Grants** (`PR12.18`). `PUT` needs `write` on the named record. `POST`
    needs a `write` grant on `"*"`, because no per-record grant can name an
    id that does not exist yet.
  - The answer is `201` with the `EHR`, a `Location` naming it, and an
    `ETag` naming the record.
  - **Not atomic across the three writes.** The record, the contribution and
    the status are three store calls, and everything is validated before the
    first. A failure after the first leaves an `EHR` with no status. Such an
    `EHR` answers `404` for its status and stays modifiable (`H5.20`).

  Before this, `POST /ehr` took a caller-built `Ehr` and committed no status,
  so no shared request could create an `EHR` against this service and against
  EHRbase alike.

- **S1.23** *(added 2026-11-27, `D-31`, the maintainer's decision)* A route
  ITS-REST 1.1.0 defines MUST answer with the **resource** it defines, not a
  wrapper of this service's own. The version is identified in headers: `ETag`
  names the version uid, and `Location` names the version's own URL.

  - `GET /ehr/{ehr_id}/ehr_status` and `…/ehr_status/{version_uid}` answer
    with the `EHR_STATUS`.
  - `GET /ehr/{ehr_id}/composition/{uid_based_id}` answers with the
    `COMPOSITION`. As ITS-REST defines it, `{uid_based_id}` is a container
    id (the latest version) or a version id (that version), so the
    `Location` a create or read returns can be read.
  - `POST` and `PUT` of a composition answer with the `COMPOSITION`
    committed.
  - This service's own routes are not ITS-REST routes and keep its version
    view: `…/composition/{uid}/_history`, `…/composition/{uid}/version/{version_uid}`,
    and the archetype search. That view is where the version metadata ITS-REST
    puts elsewhere stays readable: uid, lifecycle, deletion, chain digest, and
    committer (`PR12.25`).
  - A version that records a deletion has no resource to answer with. Read by
    a version id it is `410`, as the latest one already is (`S1.20`).

  Found by the conformance run against EHRbase 2.38.0: a client reading
  `.is_modifiable` read `null` here.

  *(`D-32`, decided the same day.)* A supplied `EHR_STATUS` MUST still carry
  `archetype_details`. It is an archetype root (`EHR_STATUS.Is_archetype_root`),
  and a status without it is refused (`422`) naming that invariant. EHRbase
  accepts one. This service does not, deliberately.

- **S1.21** Sections 7 and 8 remain **retired** (`C0.6`). The requirements
  withdrawn there described a FHIR REST surface and are not restored by a
  service existing; reinstating text nobody wrote, and mapping it to regulatory
  obligations in §13, is the failure `C0.16` records. A service specification
  will be written when the service has behaviour worth specifying.
- **S1.8** *(amended 2026-08-02)* The **core** MUST NOT authenticate or
  authorize. It records who acted; establishing who they are belongs to the
  deployment (§12, `lib:X11.1`).

  A *service* crate may go one step further and **verify** an assertion the
  deployment signed — `openehr-axa` does (`PR12.13`–`PR12.15`). That is
  checking a signature, not establishing an identity, and it changes nothing
  above: `openehr`, `openehr-store`, and the six engine crates still contain no
  credential, no key, and no principal. The core does not authorize. Since
  2026-11-26 the service enforces the per-record grants the token's issuer
  wrote (`PR12.18`, amended). It decides none of them.
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

The engine floors come from the dialect annexes. Levels come from the
[conformance matrix](conformance-matrix.md).

| Crate | Engine | Declared floor | Level |
| --- | --- | --- | --- |
| `openehr-sqlite` | SQLite | 3.35+ (bundled) | **Verified** |
| `openehr-postgresql` | PostgreSQL | 18 | **Verified** |
| `openehr-mysql` | MySQL | 8.4 | **Schema** |
| `openehr-mariadb` | MariaDB | 11.4 | **Schema** |
| `openehr-mssql` | SQL Server | 2019+ | **Schema** |
| `openehr-oracle` | Oracle Database | 12.2+ | **Schema** |

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
