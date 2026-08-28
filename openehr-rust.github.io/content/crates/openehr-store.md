# openehr-store

The engine-agnostic half of openEHR persistence: the storage model, the
projection from openEHR objects onto rows, the commit rules, and the conformance
suite every engine runs.

## Why a sixth crate exists when five were asked for

Because the alternative is documented, in this repository, as a failure.

The sibling FHIR monorepo here has six ports, each carrying a byte-identical
copy of one core, a shell script written to police the copies, and an audit
finding for the copy that drifted anyway — an Oracle DDL emitter quietly
producing MySQL types for as long as the fork existed. One crate that five
depend on cannot drift from itself.

So the five engine crates own exactly four things each: type spellings,
identifier quoting, placeholder style, and append-only enforcement. Everything
else is here.

## The storage model

openEHR is archetype-driven: a `COMPOSITION` contains whatever its archetype
says, and archetypes are authored long after the software ships. A schema
shredded from the Reference Model alone would have one column per RM attribute
and a key/value table for everything clinically interesting — a document store
with extra joins.

So **the canonical JSON is the record**, and the relational part indexes only
the attributes the Reference Model itself fixes: who committed, when, which
archetype, which category, which setting. Those are exactly what an AQL `FROM`
clause filters on before it reaches into content.

| Table | Holds |
| --- | --- |
| `openehr_ehr` | one row per record |
| `openehr_versioned_object` | one row per version container |
| `openehr_version` | one row per version — **append-only** |
| `openehr_contribution` | one row per change set |
| `openehr_composition_index` | the RM-level projection of a composition |

## Two columns for every time

openEHR times are ISO 8601 **strings** with deliberate partial precision.
`2024-05` is a date known to the month — a birth date on a refugee's record, a
diagnosis recalled as "sometime in May" — and it is not `2024-05-01`. A native
timestamp column silently completes it, which fabricates a clinical fact, and
normalises the lexical form, which breaks round-tripping.

| Column | Type | Role |
| --- | --- | --- |
| `…_text` | text | **authoritative** — the exact lexical form |
| `…_utc` | native timestamp | derived, nullable, for ordering |

The derived column is `NULL` whenever the instant is not established — a local
time with no offset, a date with no time — because that is the same answer the
library gives. A column that guessed would make SQL disagree with Rust about the
same record.

## The conformance suite

Written once, here, and called by each engine's own tests against a real
connection. A test copied five times agrees with itself four times and drifts
once: the sibling monorepo's concurrency and redaction suites existed only for
PostgreSQL, and porting them to two more engines immediately found three defects
that had been shipping.

`conformance::dialects_are_distinct` compares all five dialects' DDL and fails
if any two match — the F-08 defect, made impossible to reintroduce.

## Conformance levels

Defined in [`spec/conformance.md`](spec/conformance.md), with a table of where
each engine crate stands. Short version: `openehr-sqlite` is at **Store** and
verified against a real database; the other four are at **Dialect** and have
never had a statement parsed by the engine they name.

## Licence

MIT OR Apache-2.0.
