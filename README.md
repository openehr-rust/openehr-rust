# openehr-rust

**[openehr-rust.github.io](https://openehr-rust.github.io)** · **openEHR® in
Rust**: the Reference Model, and persistence for six SQL engines.

> openEHR® is the registered trademark of the openEHR Foundation and is used
> with the permission of openEHR International. Use of the trademark does not
> constitute endorsement of this product by openEHR International or openEHR
> Foundation.

[openEHR](https://specifications.openehr.org/) is a specification for clinical
information — a small, stable **Reference Model** of about ninety classes, plus
**archetypes** that constrain it into clinical content. These crates implement
the Reference Model and the machinery around it, so a Rust program can read,
build, check, address, store, and safely disclose openEHR data without inventing
its own idea of what a health record is.

```rust
use openehr::rm::ehr::Composition;
use openehr::validation::Validate;
use openehr::path::Pathable;

// Build it, check it against the Reference Model's own invariants…
assert!(composition.validate().is_empty());

// …address it by openEHR path…
let systolic = composition.item_at_path(
    "/content[openEHR-EHR-OBSERVATION.blood_pressure.v2]\
     /data/events[at0006]/data/items[at0004]/value/magnitude",
).unwrap();

// …and round-trip it through openEHR canonical JSON.
let json = serde_json::to_string(&composition)?;
let back: Composition = serde_json::from_str(&json)?;
assert_eq!(back, composition);
```

## The crates

| Crate | What it does | Conformance level |
| --- | --- | --- |
| [`openehr`](openehr) | Reference Model types, validation, openEHR paths, AQL parsing, change-control security | library |
| [`openehr-store`](openehr-store) | Engine-agnostic persistence: storage model, projection onto rows, commit rules, conformance suite | library |
| [`openehr-sqlite`](openehr-sqlite) | SQLite dialect **and a complete embedded store** | **Verified** |
| [`openehr-postgresql`](openehr-postgresql) | PostgreSQL 18 dialect | **Schema** |
| [`openehr-mysql`](openehr-mysql) | MySQL 8.4 dialect | **Schema** |
| [`openehr-mariadb`](openehr-mariadb) | MariaDB 11.4 dialect | **Schema** |
| [`openehr-mssql`](openehr-mssql) | SQL Server dialect | **Dialect** |
| [`openehr-oracle`](openehr-oracle) | Oracle Database dialect | **Dialect** |
| [`openehr-loco`](openehr-loco) | An HTTP API server over `openehr-sqlite`, on Axum and Loco | not on the ladder |
| [`openehr-assets`](openehr-assets) | Regenerates the committed DDL/schema files; fails the build if they are stale | tooling |

**Levels are claims about what has been verified**, not about what has been
written:

- **Dialect** — emits DDL. No server has parsed it.
- **Schema** — an actual server executed that DDL, twice, and the append-only
  tables were observed refusing `UPDATE` and `DELETE` with a row present.
- **Store** — implements the full `Store` trait against a real database, with the
  shared conformance suite passing.
- **Verified** — Store, re-checked in CI on every commit. `openehr-sqlite` is
  here; it is the only crate at Store level and so the only one eligible.

`openehr-loco` is **outside this ladder entirely**: every rung on it is defined
by DDL, a `Store` implementation, or a database server, and an HTTP service is
none of those (`W0.32`). It states evidence instead of a level — see
[its own README](openehr-loco/README.md).

Full definitions in [`spec/index.md`](spec/index.md); current status in
[`openehr-store/spec/conformance.md`](openehr-store/spec/conformance.md).

## Install

```toml
[dependencies]
openehr = "0.8"

# and, if you want persistence:
openehr-store = "0.8"
openehr-sqlite = "0.8"
```

Requires Rust 1.96+ (edition 2024). There are no Cargo features to choose, and
the eight published crates are versioned in lockstep — take the same minor
version of each. Fuller instructions, including building from source and
verifying a dialect against a real engine: [`INSTALL.md`](INSTALL.md).

Published by `cargo publish` from the maintainer's own machine — no CI publish
workflow, no [Trusted Publishing](https://crates.io/docs/trusted-publishing)
yet, by stated policy rather than oversight
([`spec/trusted-publishing/index.md`](spec/trusted-publishing/index.md)):
adopted once it is production-ready across every forge this repository
mirrors to (GitHub, GitLab, Codeberg) and every destination it publishes to
(crates.io). Full publishing identities and what each can do:
[`MAINTAINERS.md`](MAINTAINERS.md#publishing-identities).

## Tutorial 1 — build a composition

A blood-pressure observation inside an encounter composition. Every constructor
checks the Reference Model's invariants, so an object that exists is an object
that is structurally valid.

```rust
use openehr::path::Pathable;
use openehr::rm::common::{Archetyped, LocatableAttrs, PartyIdentified};
use openehr::rm::data_structures::{Element, History, ItemTree, PointEvent};
use openehr::rm::data_types::{CodePhrase, DataValue, DvDateTime, DvQuantity};
use openehr::rm::ehr::{Composition, EntryAttrs, Observation};
use openehr::terminology::composition_category;
use openehr::validation::Validate;

let at = |name: &str, node: &str| LocatableAttrs::named(name, node).unwrap();
let quantity = |v: f64| DataValue::Quantity(DvQuantity::new(v, "mm[Hg]").unwrap());

let readings = ItemTree::new(at("blood pressure", "at0003"), vec![
    Element::new(at("Systolic", "at0004"), quantity(184.0)).into(),
    Element::new(at("Diastolic", "at0005"), quantity(96.0)).into(),
]);

let observation = Observation::new(
    at("Blood pressure", "openEHR-EHR-OBSERVATION.blood_pressure.v2")
        .with_archetype_details(
            Archetyped::new("openEHR-EHR-OBSERVATION.blood_pressure.v2", "1.1.0").unwrap(),
        ),
    EntryAttrs::about_subject(
        CodePhrase::new("ISO_639-1", "en").unwrap(),
        CodePhrase::new("IANA_character-sets", "UTF-8").unwrap(),
    ),
    History::new(
        at("Event Series", "at0001"),
        DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
        vec![PointEvent::new(
            at("any event", "at0006"),
            DvDateTime::new("2026-07-31T09:15:00Z").unwrap(),
            readings.into(),
        ).into()],
        None,
    ).unwrap(),
);

let composition = Composition::new(
    at("Encounter", "openEHR-EHR-COMPOSITION.encounter.v1")
        .with_archetype_details(
            Archetyped::new("openEHR-EHR-COMPOSITION.encounter.v1", "1.1.0").unwrap(),
        ),
    composition_category::EVENT,
    PartyIdentified::named("Dr A Nurse").unwrap().into(),
    CodePhrase::new("ISO_639-1", "en").unwrap(),
    CodePhrase::new("ISO_3166-1", "GB").unwrap(),
).unwrap().with_content(observation.into());

assert!(composition.validate().is_empty());
```

Runnable: `cd openehr && cargo run --example 01_build_composition`

## Tutorial 2 — validate what you received

There are **two gates, not one**. Constructors enforce invariants on data your
program *builds*. `validate()` enforces them on data your program *receives* —
serde writes fields directly and never calls a constructor.

A service that deserializes and stores without validating has no invariant
checking at all.

```rust
use openehr::validation::Validate;

let composition: Composition = serde_json::from_str(&incoming)?;

let report = composition.validate();
if !report.is_empty() {
    for violation in report.violations() {
        // Names the path, the class, and the invariant — never the value.
        eprintln!("{}: {} failed {}", violation.path, violation.class, violation.invariant);
    }
    return Err(/* … */);
}
```

Validation is **Reference-Model-level only**. It does not check archetypes —
those are a constraint language with their own parser, and a partial
implementation would let "valid" mean "the parts I understood were satisfied".

Runnable: `cargo run --example 02_validate_incoming`

## Tutorial 3 — paths and AQL

```rust
use openehr::path::Pathable;

let magnitude = composition.item_at_path(
    "/content[openEHR-EHR-OBSERVATION.blood_pressure.v2]\
     /data/events[at0006]/data/items[at0004]/value/magnitude",
)?;
```

A path matching **three** elements fails rather than returning the first. That is
the design commitment throughout: refuse rather than guess, wherever the wrong
answer is plausible and undetectable downstream.

AQL is **parsed and statically checked, never executed** — executing it means
resolving archetype paths against a repository, and these crates implement no
archetype engine:

```rust
use openehr::aql::AqlQuery;

let query: AqlQuery =
    "SELECT c/uid/value FROM EHR e \
     CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1]".parse()?;

println!("{:?}", query.archetype_ids());  // what it would filter on
println!("{:?}", query.aliases());
query.check()?;                            // static checks: unknown alias, …

// It returns a syntax tree, never rows — and says so.
```

An AQL `SELECT` column is an alias plus a path, and that path is what `Pathable`
resolves against a composition in memory. Parsing the query and resolving the
path with one crate means the two cannot drift.

Runnable: `cargo run --example 03_paths_and_aql`

## Tutorial 4 — versioning, audit, and storage

openEHR's change control is append-only: a correction is a **new version**, and
the version it corrects stays.

```rust
use openehr_sqlite::SqliteStore;
use openehr_store::{Store, conformance};

let mut store = SqliteStore::in_memory()?;
store.install()?;

let ehr = conformance::sample_ehr();
store.create_ehr(&ehr)?;

store.create_contribution(ehr.ehr_id(), &contribution)?;
let outcome = store.commit_composition(ehr.ehr_id(), &version, "contribution-uid")?;

// The one query the index exists for — AQL's `CONTAINS COMPOSITION c[...]`
let found = store.find_compositions_by_archetype(
    ehr.ehr_id(),
    "openEHR-EHR-COMPOSITION.encounter.v1",
)?;
```

The store **validates before it writes**. A store that accepted an invalid
composition would make every later reader's `validate()` fail on data it cannot
fix.

Append-only is enforced by database triggers, not application code — a guarantee
that a SQL console can walk around is not one.

Runnable: `cargo run --example 04_versioning_and_audit`

## Tutorial 5 — access decisions and redaction

Two different questions, asked in sequence: *may this request see this
record at all*, and then *which parts of what it may see should be withheld*.

```rust
use openehr::security::{AccessRequest, EhrAccess, GroupSettings, Operation};

// Everything defaults to deny: no settings recorded is not "wide open".
let bare = EhrAccess::new(attrs)?;
let request = AccessRequest { operation: Operation::Read, groups: &["care-team".to_owned()], is_subject: false };
assert!(!bare.decide(&request).is_permit());

let configured = bare.with_settings(
    GroupSettings::new().permit(Operation::Read, "care-team").into(),
);
assert!(configured.decide(&request).is_permit());
```

```rust
use openehr::security::{RedactionRule, Redactor};

// Redaction masks; it does not delete. A withheld element becomes
// `272｜masked｜` — there is a value here and you are not being shown it.
// Deleting it instead would turn "the patient withheld their HIV status"
// into "the patient has no HIV status", a clinical statement nobody made.
let (filtered, count) = Redactor::new()
    .with_rule(RedactionRule::node_id("at0011"))
    .with_reason("Withheld under the patient's recorded consent preferences")
    .redact_counting(&composition)?;

// The redacted document still validates: a filter that produces something
// the receiving system rejects has achieved nothing.
assert!(filtered.validate().is_empty());
```

A policy in a scheme this crate does not implement — an external opt-out
register, say — is carried unchanged and denied, never evaluated as though it
were the reference scheme. The access-log line records the **count** of
withheld elements, never their names: an audit trail that records "HIV status
withheld" has disclosed the category it was protecting.

Runnable: `cargo run --example 05_access_and_redaction`

## Tutorial 6 — store a record, and watch the guarantees hold

The five tutorials above build and check documents in memory. This one is the
other half — what happens when a document reaches a database — and it runs
against a real SQLite database in process, with nothing to install:

```sh
cd openehr-sqlite && cargo run --example 01_store_a_record
```

```
installed on SQLite
committed  87284370-…::ehr1.example.org::1
amended    version 2
latest     87284370-…::ehr1.example.org::2 (250), committed 2026-08-01T09:05:00Z
           text "2026-08-01T09:05:00Z", derived utc Some(1785575100)
history    2 versions, oldest first
at 09:02   87284370-…::ehr1.example.org::1
indexed    2 composition(s) of that archetype
refused    stale predecessor: preceding version is not the current latest
checkpoint entries=2 head=2a13e345… last_version=87284370-…::2
refused    direct UPDATE: openehr_version is append-only (openEHR V8.10)
intact     version 1 unchanged after the refused UPDATE
```

Eight steps, and the last four are the ones worth reading the source for:

- **A stale predecessor is refused.** Two clinicians amending one composition
  from the same starting point is a lost update in any store that does not
  check, and this one returns `StoreError::Commit`.
- **The checkpoint carries no clinical content** — a count, a head digest, and
  an identifier — so it can be published to a witness the database
  administrator does not control. That is the only thing that detects
  **truncation**: delete the newest version and the shorter chain still
  verifies perfectly (`db:M3.16c`).
- **Append-only is enforced by the database, not by this crate.** The tutorial
  goes around the `Store` with a raw `UPDATE` on the connection and the trigger
  refuses it. A rule enforced only in Rust is a rule an administrator with a
  SQL prompt does not have.
- **The row is checked afterwards.** A refusal alone proves nothing if nothing
  looks at the row: a `FOR EACH ROW` trigger on zero rows never fires
  (`db:C0.12`).

It lives in `openehr-sqlite` because that is the only crate with a `Store`. The
other five engine crates supply a `Dialect` and no more, so the same tutorial
there would be demonstrating code that does not exist (`db:C0.11`).

## Generating DDL for your engine

```rust
use openehr_postgresql::PostgresqlDialect;
use openehr_store::ddl_script;

println!("{}", ddl_script(&PostgresqlDialect));
```

Or from the command line:

```sh
cargo run --manifest-path openehr-postgresql/Cargo.toml --example ddl
```

To check the DDL against a real server — which is what separates level
**Dialect** from **Schema**:

```sh
sh openehr-store/scripts/verify-schema.sh postgresql   # or mysql, mariadb
```

Needs `podman` or `docker`. It provisions the engine, applies the DDL, applies it
**again** to prove idempotence, seeds a row, and confirms the append-only tables
refuse `UPDATE` and `DELETE` with that row present and intact afterwards.

## An HTTP service, if you want one

None of the above talks HTTP. [`openehr-loco`](openehr-loco) is a separate,
optional, **not published** crate that puts a RESTful API in front of
`openehr-sqlite`: PASETO `v4.public` bearer auth (verify-only — this service
never signs), `410 Gone` for a deleted composition against `404` for one that
never existed, `403`/`422` split for "not the committer" against "the committer
cannot be identified", and `If-Match` concurrency control. 53 tests, including
`tests/http.rs` serving real requests through Loco's own router.

It depends on the storage crates; nothing depends on it, and deleting it changes
nothing else (`S1.7`). Its job is narrow by design — translate HTTP to store
calls and get the status codes right — and it adds no clinical behaviour of its
own. Details, and what it does not yet demonstrate, are in
[its own README](openehr-loco/README.md).

```sh
cd openehr-loco
OPENEHR_SQLITE_PATH=openehr.sqlite3 \
OPENEHR_PASETO_PUBLIC_KEYS=k4.public.… \
  cargo run -- start
# fails closed with no server if the key is missing — see its README
```

## How openEHR is stored, and why it is not shredded

FHIR fixes the shape of a `Patient` at specification time, so you can generate a
schema from the specification and shred resources into typed columns.

**openEHR does not.** A `COMPOSITION` contains whatever its archetype says, and
archetypes are authored by clinicians long after the software ships. A schema
shredded from the Reference Model alone would have one column per RM attribute
plus a key/value table for everything clinically interesting — a document store
with extra joins, and a migration every time an archetype is published.

So the **canonical JSON is the record**, and the relational part is an *index*
over the attributes the Reference Model itself fixes:

| Table | Holds |
| --- | --- |
| `openehr_ehr` | one row per health record |
| `openehr_versioned_object` | one row per version container |
| `openehr_version` | one row per version — **append-only** |
| `openehr_contribution` | one row per change set — **append-only** |
| `openehr_composition_index` | the RM-level projection of a composition |

Those index columns — who committed, when, which archetype, which category,
which setting — are exactly what an AQL `FROM` clause filters on before it
reaches into content.

### Every instant is stored twice

openEHR times are ISO 8601 **strings** with deliberate partial precision.
`2024-05` is a date known to the month — a birth date on a refugee's record, a
diagnosis recalled as "sometime in May" — and it is **not** `2024-05-01`.

A native timestamp column silently completes it, which fabricates a clinical
fact, and normalises the lexical form, which breaks round-tripping. So:

| Column | Role |
| --- | --- |
| `…_text` | **authoritative** — the exact lexical form |
| `…_utc` | derived, **nullable**, for ordering and range scans |

The derived column is `NULL` whenever the instant is not established, because
that is the same answer the library gives. A column that guessed would make SQL
disagree with Rust about the same record.

## Design commitments

**Refuse rather than guess.** Comparison is partial throughout: a month-precision
date is not ordered against a day inside it, `5 mg` is not comparable with
`5 mL`, and an ambiguous path fails. Each has a plausible wrong answer that no
downstream reader could detect.

**Absence is structured.** openEHR's four null flavours — nobody looked, somebody
looked and could not find out, the value is withheld, the question does not arise
— are four different clinical facts, and these crates will not let them collapse.

**Nothing prints protected health information.** No `Display` renders an
identifier or a media blob; no error echoes a submitted value; redaction masks
rather than deletes, and counts rather than names what it withheld.

## What is deliberately not implemented

Saying this plainly is part of the design. A clinical library that implies
coverage it does not have is worse than a small one.

| Not implemented | Why |
| --- | --- |
| AQL **execution** | needs a repository of versioned data; AQL parses and checks, and returns no rows |
| UCUM unit conversion | a wrong conversion is a thousand-fold dosing error |
| External terminology lookup | needs a terminology server; codes are carried opaquely |
| HL7 `GTS`/`PIVL` timing evaluation | a partial timing engine is right most of the time |

**Archetypes: the object model is built, and nothing above it is.** They were
excluded outright until 2026-08-26, when `lib:S1.4` was withdrawn and
[`openehr/spec/15-archetypes.md`](openehr/spec/15-archetypes.md) took its place.
`openehr::am` now holds AOM2 as Rust types — archetypes, the constraint tree,
multiplicities, terminology, with the artefact-level validity conditions checked
at construction. **Twenty-eight of the thirty-two requirements have no code**:
no ADL parser, no flattening, no template expansion, no operational template, no
retrieval, and **no way to check that a composition conforms to the archetype it
names**. The conformance matrix marks each `spec`, and `lib:A-40` keeps the gap
countable.

This table is the **library's** scope. `openehr-loco` (above) is a separate,
optional, unpublished crate that puts a narrow REST API in front of the store —
it does not change what the library itself implements, and does not add
archetype validation, AQL execution, or any of the rest of this table.

Where openEHR defines an operation these crates do not implement, they return an
explicit `Unsupported` error naming the specification section that records the
exclusion. Never a plausible default.

## Specification-driven development

Every normative statement lives in a `spec/` directory, carries a stable
identifier, and is cited from the code and the tests. The trace — openEHR
specification → requirement → code → test → matrix — is what makes a claim
checkable years later by someone who was not here.

| Directory | Governs | Ids |
| --- | --- | --- |
| [`spec/`](spec/index.md) | the repository: crate map, id namespaces, ladder, publishing | `W0.x` |
| [`spec/databases/`](spec/databases/index.md) | storing openEHR in SQL | `db:` — `M3.x`, `T11.x`, … |
| [`openehr/spec/`](openehr/spec/index.md) | the Reference Model library | `lib:` — `D3.x`, `V8.x`, … |

Four rules carry most of the weight:

1. **Documentation must not claim more than is verified** (`W0.3`). "The same
   code path works elsewhere" is not evidence.
2. **A gap that is not written down reads as a pass** (`W0.4`). Known
   divergences go in an audit register with checkable evidence.
3. **Requirement identifiers are permanent** (`db:C0.5`). Never renumbered,
   never reused — a citation in a test name or a commit message is written once
   and read years later.
4. **One fact, one owner** (`W0.1`). Where two files state one rule, one is a
   copy, and a copy is a future divergence.

### One owner, and a script that enforces it

The fourth rule is the one that is easy to agree with and hard to keep, because
the literal remedy — one occurrence and three links — makes documents worse. A
reader deciding whether a crate's conformance claim is honest should not have to
open a second file to learn what the claim means.

So the copies stay and are made mechanical. `python3 scripts/check-docs.py`,
which the `claims` job runs, derives the facts from the tree and fails when the
prose disagrees:

| Fact | Owner |
| --- | --- |
| how many crates, how many published, how many fuzz targets, how many tutorials | the tree itself |
| which CI jobs exist | `.github/workflows/ci.yml` |
| which version is live on crates.io | [`agents/publishing.md`](agents/publishing.md) |
| each crate's conformance level | [`spec/databases/conformance-matrix.md`](spec/databases/conformance-matrix.md) |
| a passage that genuinely belongs in several documents | whichever copy is marked `(owner)` |

The last row is a marked block:

```html
<!-- shared: conformance-ladder (owner) -->  …  <!-- /shared: conformance-ladder -->
<!-- shared: conformance-ladder (copy)  -->  …  <!-- /shared: conformance-ladder -->
```

`--fix` rewrites every copy from its owner. This exists because three findings
here were nothing but drift of this kind: a version stated in five files and
updated in one (**W-10**), a requirement saying fourteen crates while five
documents said seventeen (**W-11**), and the conformance ladder written out four
times with two already diverged (**W-16**).

### Known gaps

Recorded rather than implied — see [`spec/audit.md`](spec/audit.md):

- **SQL Server and Oracle DDL has never been parsed by the engine it names.**
  SQL Server 2022 segfaults under qemu on arm64 and the Oracle images need
  registry authentication, so both crates stay at **Dialect**. That is a gap in
  evidence, not a judgement that the DDL is wrong.
- **`spec/databases/` was rewritten from an imported FHIR specification** on
  2026-08-01. It now describes this system, but the requirements were derived by
  reading the code rather than the openEHR sources, so some are descriptions
  rather than considered generalizations (`db:D-05`).
- **AQL cannot express a negative numeric literal** (`lib:A-27`).
  `WHERE o/value/magnitude > -2.5` is refused at the lexer, because `-` also
  separates the parts of an archetype id. Declared as `lib:Q12.9b` and open —
  refused rather than misread, which is the safe direction.
- **Only N−2 and current stable are compiled** (`RV3`). A break appearing only
  on an intermediate Rust release reaches a user before it reaches CI.

## Repository layout

```
spec/                      repository specification + audit register
  databases/               persistence specification (db:)
AGENTS.md, agents/         contributor and agent guides
CLAUDE.md                  guidance for Claude Code
openehr-rust-maintainer-skill/  AGENTS.md/CLAUDE.md, condensed into a Claude Code Skill
openehr/                   the Reference Model library
  spec/                    library specification, audit, conformance matrix
  examples/                five runnable tutorials (the RM)
openehr-store/             engine-agnostic persistence
  scripts/verify-schema.sh Dialect -> Schema verification
openehr-<engine>/          one Dialect each; sqlite also has a Store
  spec/14-<engine>-dialect.md   that dialect's annex and departures
openehr-loco/              HTTP API server, outside the conformance ladder; not published
openehr-assets/            regenerates committed DDL/schema files; not published
openehr-fuzz/              fuzz harness for the RM parsers; not published
openehr-store-fuzz/        fuzz harness for projection and integrity; not published
openehr-<engine>-fuzz/     fuzz harness per dialect; not published
  fuzz_targets/, corpus/   21 targets in all, committed seed corpora
.github/workflows/ci.yml   test, msrv, examples, bench, schema, fuzz, claims
```

Eighteen crates, **each its own Cargo workspace** — run cargo from inside a
crate directory. Eight are published (`openehr`, `openehr-store`, and the six
dialect crates); the other ten — `openehr-loco`, `openehr-assets`, and the
eight fuzz harnesses — are not.

## Project documents

| Document | What it answers |
| --- | --- |
| [`INSTALL.md`](INSTALL.md) | how to install it, run it, and build it from source |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | what would help most, the rules a patch is held to, and the honest answer about money |
| [`RFC.md`](RFC.md) | what this project does not know, and the kinds of feedback that change it |
| [`CHANGELOG.md`](CHANGELOG.md) | what changed in each release, and why it was that version number |
| [`NEWS.md`](NEWS.md) | the short form of the above, what is coming, and the press contact |
| [`COMPARISONS.md`](COMPARISONS.md) | how this differs from EHRbase, FerroEHR, Archie, and the commercial platforms — including when to choose one of those instead |
| [`BENCHMARKS.md`](BENCHMARKS.md) | what is measured, one dated measurement, and why nothing is gated on timing |
| [`MAINTAINERS.md`](MAINTAINERS.md) | who ships a fix, and what happens if they cannot |
| [`GOVERNANCE.md`](GOVERNANCE.md) | who decides, on what basis, and where each decision is recorded |
| [`SECURITY.md`](SECURITY.md) | how to report a vulnerability, what is in scope, and this project's own posture gaps |
| [`AI_STATEMENT.md`](AI_STATEMENT.md) | how this code was written, what checks it, and what that does not prove |
| [`CITATION.cff`](CITATION.cff) | how to cite this work |
| [`LICENSE.md`](LICENSE.md) | five licences, your choice of any one |
| [`help/outreach/index.md`](help/outreach/index.md) | where this project is being taken to its professional audience |

## Contributing

Read [`AGENTS.md`](AGENTS.md), and [`agents/`](agents/index.md) for topic guides:
[adding an engine](agents/adding-an-engine.md),
[conformance](agents/conformance.md), [publishing](agents/publishing.md),
[openEHR concepts](agents/openehr-concepts.md), and
[auditing](agents/auditing.md).
[`openehr-rust-maintainer-skill/SKILL.md`](openehr-rust-maintainer-skill/SKILL.md)
is the same operating guide condensed to a [Claude Code
Skill](https://docs.claude.com/en/docs/claude-code/skills) — the layout, the
five rules, and the traps that have already bitten someone here — for a
session or tool that has not loaded `CLAUDE.md`/`AGENTS.md`.

```sh
for d in openehr openehr-store openehr-sqlite openehr-postgresql \
         openehr-mysql openehr-mariadb openehr-mssql openehr-oracle \
         openehr-loco openehr-assets; do
  (cd "$d" && cargo test --quiet \
     && RUSTFLAGS="-D warnings" cargo clippy --all-targets --quiet) \
    || echo "FAIL $d"
done
```

The tree is at zero clippy warnings under `pedantic`, with `missing_docs`,
`missing_errors_doc`, and `missing_panics_doc` at `deny` and `unsafe_code` at
`forbid`. Please keep it there.

The MSRV is **N−2** — two Rust releases behind stable, currently 1.96 — and CI
re-derives that number from the toolchain rather than trusting a constant, so it
will fail on a stale one within six weeks of every Rust release. That is
deliberate; see [`spec/rust-msrv-n-minus-2/index.md`](spec/rust-msrv-n-minus-2/index.md).

## Licence

Any of these, at your option:

* [MIT](https://opensource.org/license/mit)
* [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0)
* [BSD 3-Clause](https://opensource.org/license/bsd-3-clause)
* [GNU General Public License v2.0](https://www.gnu.org/licenses/old-licenses/gpl-2.0-standalone.html)
* [GNU General Public License v3.0](https://www.gnu.org/licenses/gpl-3.0-standalone.html)

SPDX: `MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR GPL-3.0-only`.
See [`LICENSE.md`](LICENSE.md).
