# openehr-store

openEHR® store is the engine-agnostic half of openEHR persistence: the storage model, the
projection from openEHR objects onto rows, the commit rules, and the conformance
suite every engine runs.

> openEHR® is the registered trademark of the openEHR Foundation and is used
> with the permission of openEHR International. Use of the trademark does not
> constitute endorsement of this product by openEHR International or openEHR
> Foundation.

## Why this crate exists

Because the alternative is documented, in this repository, as a failure.

The sibling FHIR monorepo here has six ports, each carrying a byte-identical
copy of one core, a shell script written to police the copies, and an audit
finding for the copy that drifted anyway — an Oracle DDL emitter quietly
producing MySQL types for as long as the fork existed. One crate that six
depend on cannot drift from itself.

So the six engine crates own exactly four things each: type spellings,
identifier quoting, placeholder style, and append-only enforcement. Everything
else is here.

That boundary is necessary and was not sufficient. `openehr-mariadb` was a
name-substituted copy of `openehr-mysql` — byte-identical DDL, a struct still
called `MysqlDialect` — and the cross-dialect comparison that exists to catch
precisely this compared five dialects while that one was the sixth. Both are
fixed; the lesson kept is that a guard is only as wide as its input list. See
[`spec/audit.md`](../spec/audit.md) **W-01**.

## Install

```toml
[dependencies]
openehr-store = "0.9"
openehr = "0.9"
```

Requires Rust 1.96+ (edition 2024). This crate emits DDL and defines the traits;
it opens no connections and depends on no driver.

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

## Seeing it work

This crate has no `Store` of its own — it defines the trait, the schema, the
projection, and the commit rules, and an engine crate implements against them.
The runnable demonstration therefore lives one crate over:

```sh
cd ../openehr-sqlite && cargo run --example 01_store_a_record
```

## The conformance suite

Written once, here, and called by each engine's own tests against a real
connection. A test copied five times agrees with itself four times and drifts
once: the sibling monorepo's concurrency and redaction suites existed only for
PostgreSQL, and porting them to two more engines immediately found three defects
that had been shipping.

`conformance::dialects_are_distinct` compares all six dialects' DDL and fails if
any two match — the F-08 defect, made detectable. It is driven from
`openehr-sqlite/tests/dialects.rs`, which is also where the list of dialects to
compare lives, and a companion test ties that list's length to the number of
engine crates so a new engine cannot be added without being compared.

## Conformance levels

Defined in [`spec/conformance.md`](spec/conformance.md), with a table of where
each engine crate stands, under the ladder set out in
[`spec/index.md`](../spec/index.md) (`W0.8`).

| Crate | Level |
| --- | --- |
| `openehr-sqlite` | **Verified** — the full suite against a real in-process database, in CI on every push |
| `openehr-postgresql` | **Schema** — DDL executed against PostgreSQL 18 |
| `openehr-mysql` | **Schema** — DDL executed against MySQL 8.4 |
| `openehr-mariadb` | **Schema** — DDL executed against MariaDB 11.4 |
| `openehr-mssql` | **Dialect** — no server has parsed it |
| `openehr-oracle` | **Dialect** — no server has parsed it |

`openehr-sqlite` reached **Verified** on 2026-08-01: [run 30713623082](https://github.com/openehr-rust/openehr-rust/actions/runs/30713623082) is green across
all nineteen jobs. The three Schema claims are now checked by CI on every push
rather than attested once. See [`spec/audit.md`](../spec/audit.md) **W-02**.

## Specification

- [`spec/databases/`](../spec/databases/index.md) — the normative core: storage
  model (`M3.x`), dialect boundary (`X15.x`), commit rules (`H5.x`)
- [`spec/databases/conformance-matrix.md`](../spec/databases/conformance-matrix.md)
  — per-engine status
- [`spec/audit.md`](../spec/audit.md) — known gaps

## Benchmarks

```sh
cargo bench                     # criterion; results in target/criterion/
cargo bench -- --test           # one iteration each, which is what CI runs
```

`benches/store.rs` measures the two hot pure functions: the projection from a
composition onto rows, and `verify_versions`. Everything else in a commit is a
round trip to a database server, which no benchmark in this process can measure
honestly.

`verify_versions` is measured at 1, 10, and 100 versions, because the question
is not how fast one row is but whether the walk is **linear**. A check that
quietly became quadratic would pass every test in the suite and be noticed first
by whoever verified a record with ten years of history in it.

**A number here is not a conformance claim** (`W0.3`, `W0.34`). No requirement in
this repository is stated in seconds and no crate's conformance level depends on
a timing. CI runs the benchmarks with `--test` and gates on nothing: wall-clock
on a shared runner varies by more than most real regressions, and a threshold
that fails for unrelated reasons is one somebody silences.

## Licence

Any of these, at your option — MIT, Apache-2.0, BSD-3-Clause, GPL-2.0-only, or
GPL-3.0-only. See [`LICENSE.md`](LICENSE.md).
