# openehr-sqlite

openEHR® persistence for **SQLite 3** — a complete embedded store.

> openEHR® is the registered trademark of the openEHR Foundation and is used
> with the permission of openEHR International. Use of the trademark does not
> constitute endorsement of this product by openEHR International or openEHR
> Foundation.

## Install

```toml
[dependencies]
openehr-sqlite = "0.8"
openehr-store = "0.8"
openehr = "0.8"
```

Requires Rust 1.96+ (edition 2024). SQLite itself is compiled in — no system
library is needed, and none is used.

## Conformance level: Verified

Unlike the five dialect-only crates, this one contains a working `SqliteStore`
and runs the shared conformance suite against a **real database** in its own
tests: every commit rule, every read, the archetype index, the append-only
triggers, and DDL idempotence.

SQLite is the only one of the six engines that can be verified without
provisioning a server, so it is the one where the shared logic actually gets
exercised. See
[`openehr-store/spec/conformance.md`](../openehr-store/spec/conformance.md).

```rust
use openehr_sqlite::SqliteStore;
use openehr_store::{Store, conformance};

let mut store = SqliteStore::in_memory()?;
store.install()?;

let ehr = conformance::sample_ehr();
store.create_ehr(&ehr)?;
store.create_contribution(ehr.ehr_id(), &conformance::sample_contribution(uid, &[1]))?;
store.commit_composition(ehr.ehr_id(), &version, uid)?;

// The record as it stood at a moment in time.
let then = store.version_at_time(&container, &at)?;
```

**Verified** rather than Store because the suite runs in CI on every push
against a bundled engine that cannot be absent, so the job cannot silently skip.
It is the only crate at this level, being the only one with a `Store`.

## Tutorial

```sh
cargo run --example 01_store_a_record
```

The whole loop, against a real in-process database: install the schema, commit a
composition, amend it, read the history oldest-first, resolve a point-in-time
read, query the archetype index, watch a stale predecessor be refused, print a
tamper-evidence checkpoint, and then go **around** this crate with a raw
`UPDATE` on the connection and watch the database's own trigger refuse it — and
check the row is intact afterwards, because a `FOR EACH ROW` trigger on zero
rows never fires (`C0.12`).

It is the only runnable tutorial for the persistence layer, and CI runs it on
every push. A tutorial in a README is read as a demonstration that the thing
works, which makes it a claim (`W0.3`).

## The API

| Operation | Does |
| --- | --- |
| `SqliteStore::in_memory()` / `::open(path)` | open a store; `from_connection` adopts an existing `rusqlite::Connection` |
| `install()` | create the schema, idempotently |
| `create_ehr` / `get_ehr` | one health record |
| `create_contribution` | one change set |
| `commit_composition` | one version, with every commit rule enforced |
| `get_version` / `latest_version` | read by identifier, or the head of a container |
| `version_at_time` | the version current at an instant |
| `all_versions` | a container's history, **oldest first** (`H5.12`) |
| `find_compositions_by_archetype` | the query the index exists for |

## What it guarantees

**The same commit rules as the library.** A version belonging to another
container, a duplicate, a missing predecessor, or a *stale* one are each refused
and each distinguishable. The stale case is the one that matters: two clients
both read version 2 and both write version 3, and openEHR's answer is a branch
rather than a silent overwrite — so the store refuses and lets the caller
decide.

**Append-only, in the database.** `UPDATE` and `DELETE` on the version table are
refused by triggers, not by application convention. A guarantee that lives only
in Rust ends the first time somebody opens the file with the `sqlite3` CLI, and
a test goes around the store to prove it.

**Validation before writing.** An invalid composition is refused before the
transaction opens, so a refused commit leaves no container behind. A store that
accepted one would make every later reader's `validate()` fail on data it cannot
fix.

**Foreign keys actually on.** SQLite disables them per connection by default;
this store enables them explicitly. Not doing so would accept a version pointing
at a container that does not exist, silently, while the schema said otherwise.

## SQLite specifics

| Decision | Why |
| --- | --- |
| `TEXT` for identifiers, with no lengths | SQLite applies type affinity rather than enforcement, so a declared length would be documentation nothing checks. |
| `TEXT` for JSON | There is no JSON type; the JSON1 extension operates on `TEXT`, so this is both the honest declaration and the working one. |
| `INTEGER` — Unix seconds — for derived instants | SQLite has no date type. Storing the derived instant as text would make it sort identically to the authoritative column, collapsing the distinction the two columns exist to keep. |
| `bundled` SQLite | The DDL and the JSON1 functions are version-dependent, so the engine is pinned rather than whatever the host ships. |

## Every instant is stored twice

openEHR times are ISO 8601 **strings** with deliberate partial precision:
`2024-05` is a date known to the month and is not `2024-05-01`. A native
timestamp column silently completes it — fabricating a clinical fact — and
normalises the lexical form.

So each time occupies `…_text` (authoritative, exact) and `…_utc` (derived,
nullable, for ordering). The derived column is `NULL` whenever the instant is
not established, and `version_at_time` skips those rows rather than guessing at
a time zone — exactly as the library does.

## What is not here

| Not here | Why |
| --- | --- |
| AQL execution | Needs archetype path resolution; `openehr` parses and statically checks AQL and returns no rows (`S1.6`). |
| Archetype or template validation | Validation is Reference-Model-level only (`V9.9`, `lib:S1.4`). |
| A tamper-evidence chain | `openehr` has the primitives; this store does not use them and the schema has no digest columns (`M3.16`). Append-only is a **weaker** guarantee and must not be described as tamper evidence (`PR12.11`). |
| Read auditing | Only writes are recorded (`PR12.5`). |
| Schema migration | No migration mechanism and no applied-version metadata (`O10.14`). |
| Concurrency guarantees | The commit rules and the unique index are in place, but **nothing tests concurrent access** (`db:D-02`). |

## Fuzzing

The dialect's identifier quoting is fuzzed by
[`openehr-sqlite-fuzz`](../openehr-sqlite-fuzz), run in CI on every push.

## Specification

- [`spec/databases/`](../spec/databases/index.md) — storage model, commit rules,
  conformance ladder
- [`spec/databases/conformance-matrix.md`](../spec/databases/conformance-matrix.md)
  — what is verified today, including the two store requirements that are **not**
- [`spec/audit.md`](../spec/audit.md) — known gaps

## Testing

```sh
cargo test
```

Runs the shared suite against an in-memory database, plus this crate's own
tests for the trigger, the validation gate, lexical fidelity, and DDL
idempotence — and `tests/dialects.rs`, which compares all six engines' DDL and
fails if any two agree.

## Licence

Any of these, at your option — MIT, Apache-2.0, BSD-3-Clause, GPL-2.0-only, or
GPL-3.0-only. See [`LICENSE.md`](LICENSE.md).
