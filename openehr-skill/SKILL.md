---
name: openehr-skill
description: Explains openEHR concepts, vocabulary, and ideas for someone using this repository's Rust library to work with electronic health record data — the Reference Model, archetypes, compositions, paths, versioning, why the model looks the way it does — with pointers to this repository's runnable examples. Use when asked what openEHR is, what a term (COMPOSITION, archetype, ELEMENT, DV_QUANTITY, AQL, node id, …) means, why openEHR is not a fixed-schema record format, or for a worked example of building, validating, querying, or storing an openEHR record in Rust. For this repository's own engineering conventions rather than openEHR itself, use openehr-rust-maintainer-skill instead.
---

# openehr-skill — openEHR concepts, for people using this library

This is a primer on **openEHR the specification**, using this repository's
Rust types and examples as the illustration. It is not this repository's own
engineering conventions — for how the crates here are built, tested, and
maintained, use `openehr-rust-maintainer-skill` instead.

**Not normative.** [`openehr/spec/index.md`](../openehr/spec/index.md) says
what this implementation decided; the specification itself is at
[specifications.openehr.org](https://specifications.openehr.org/).

## The one idea everything else follows from

**Most record formats fix a resource's shape in the specification. openEHR
does not.** A format with a fixed shape can say in advance exactly which
fields a "blood pressure reading" has. openEHR instead fixes a small, stable
**Reference Model** — around ninety general-purpose classes for structure,
data types, versioning, and audit — and leaves *clinical* content to
**archetypes**: separately authored, separately published definitions of what
a blood pressure reading, a diagnosis, or a discharge summary actually
contains. A hospital's software and a clinical modeller's archetype library
can therefore evolve independently, and a new clinical concept does not
require a schema migration.

The practical consequence: an openEHR record is not "rows in a table shaped
like the spec." It is a **`COMPOSITION`** — a general-purpose tree of RM
classes — whose specific clinical shape at each point is whatever
**archetype** that point declares, addressed by an **archetype node id**
(`at0004`, or the archetype's own id at the root) rather than by a column
name a schema author chose.

## Vocabulary

| Term | Means |
| --- | --- |
| **Reference Model (RM)** | The fixed, general-purpose class hierarchy — `COMPOSITION`, `SECTION`, `ENTRY` (`OBSERVATION`, `EVALUATION`, `INSTRUCTION`, `ACTION`, `ADMIN_ENTRY`), `ITEM_STRUCTURE`, `ELEMENT`, and the `DV_*` data types. This is what this crate's `openehr::rm` module implements. |
| **Archetype** | A clinical modeller's published constraint on the RM: which elements may appear at a point in the tree, how many times, with what value ranges, under what node ids. Written in ADL and governed through the openEHR Clinical Knowledge Manager (CKM). |
| **Template** | An archetype composed from several archetypes and specialised for a local purpose — a specific form for a specific department. |
| **`COMPOSITION`** | The top-level clinical document: one encounter's or one context's worth of content — `SECTION`s containing `ENTRY`s. |
| **`ELEMENT`** | A leaf node holding one clinical value (a `DV_*`) or an explicit reason it is absent (a null flavour). |
| **`DV_QUANTITY`, `DV_COUNT`, `DV_CODED_TEXT`, `DV_DATE_TIME`, …** | The data value types — a measured quantity with units, an integer count, a coded/terminology value, a date-time with partial precision, and about a dozen more. |
| **Archetype node id** | The `at`-code (ADL 1.4: `at0004`) or `id`-code (ADL 2: `id1.1`) identifying a specific constrained node — how a path, a query, or a template refers to "this specific field," independent of any column name. |
| **openEHR path (AQL path)** | A slash-separated address into a `COMPOSITION`, by RM attribute name and archetype node id — `/content[…]/data/events[at0006]/data/items[at0004]/value/magnitude`. |
| **AQL** | Archetype Query Language — SQL-like queries over openEHR data, addressed by archetype path rather than column name. |
| **`VERSION` / `CONTRIBUTION`** | openEHR's change-control model: every change is a new, appended `VERSION`; a `CONTRIBUTION` is one change set (possibly several versions) attributed to one committer and one audit record. Nothing is overwritten or deleted. |
| **Null flavour** | A structured reason a value is absent — not merely `null`. openEHR distinguishes "nobody looked," "somebody looked and could not find out," "the value is deliberately withheld," and "the question does not apply" — four different clinical facts, not one absence. |
| **`EHR`** | One subject's whole longitudinal record — the container for every `COMPOSITION` recorded about them over time. |

## Ideas that surprise people coming from a fixed-schema record

- **A partial date is a fact, not missing data.** `"2024-05"` means "known to
  the month" — a birth date on an incomplete record, a diagnosis recalled as
  "sometime in May." It is a different, real value from `"2024-05-01"`, and
  treating it as an incomplete `"2024-05-01"` fabricates a precision nobody
  recorded.
- **Comparisons can refuse to answer.** `5 mg` and `5 mL` are not
  comparable — different physical quantities — and a library that "coerces"
  one into the other is guessing at a clinical fact. openEHR — and this
  crate's types — return "not comparable" rather than a plausible-looking
  wrong answer.
- **A record only grows.** A correction is a *new version* alongside the one
  it corrects, never an edit in place. The full history — including what was
  believed before a correction — stays part of the record.
- **A path can legitimately match nothing.** Because an archetype, not the
  RM, decides what fields exist at a point in the tree, asking for a field an
  archetype doesn't define is a normal "no such data here," not an error —
  the same way asking a spreadsheet for a column past its last one returns
  nothing rather than crashing.

## Where this crate stands today

Honestly, not aspirationally — see
[`openehr/spec/conformance-matrix.md`](../openehr/spec/conformance-matrix.md)
for what is actually verified:

- The **Reference Model** — the classes above, paths, and RM-level
  validation — is implemented.
- **Validating an instance against a specific, already-built archetype** —
  does this `COMPOSITION` conform to *this* archetype's constraints — is
  implemented (`openehr::am::validate`), against an archetype already held
  in memory in this crate's own object-model form, not one parsed from the
  ADL text a clinical modeller actually publishes. A `C_ARCHETYPE_ROOT`
  filler (a slot a template already filled, naming the archetype) can be
  *resolved* too, through a repository you supply
  (`openehr::am::repository`; this crate performs no network or filesystem
  I/O itself) — a bare, unfilled `ARCHETYPE_SLOT` cannot, because which
  archetype fills it is recorded on the instance itself, and this crate's
  path machinery does not yet expose that attribute.
- **Parsing ADL itself, and applying a specialised archetype's inherited
  constraints,** is not implemented yet. **AQL parses and is statically
  checked but does not execute** — running a query needs a repository of
  versioned data this crate does not provide on its own.

If a claim here and the conformance matrix disagree, trust the matrix.

## Runnable examples in this repository

- [`README.md`](../README.md)'s six tutorials, in order: build a composition,
  validate what you received, paths and AQL, versioning and audit, access
  decisions and redaction, and store a record end to end.
- [`openehr/examples/`](../openehr/examples/) — the same six as standalone,
  runnable Rust files (`cargo run --example 01_build_composition`, and so
  on through `05_access_and_redaction`).
- [`openehr-sqlite/examples/01_store_a_record.rs`](../openehr-sqlite/examples/01_store_a_record.rs)
  — the one crate here with a working `Store`, end to end.

## Further reading

- [specifications.openehr.org](https://specifications.openehr.org/) — the
  authority on openEHR itself.
- [`openehr/spec/index.md`](../openehr/spec/index.md) — what this
  implementation decided, and where openEHR left something open.
- [`agents/openehr-concepts.md`](../agents/openehr-concepts.md) — the same
  ground, aimed at someone who already knows HL7 FHIR and needs the
  vocabulary mapped and the false cognates flagged.
- [`PHI.md`](../PHI.md) — what this software does with patient data, for a
  privacy or clinical-safety reader rather than a developer.
