# openehr-sqlite

openEHR persistence for **SQLite 3** — a complete embedded store.

## Conformance level: Store

Unlike the four dialect-only crates, this one contains a working `SqliteStore`
and runs the shared conformance suite against a **real database** in its own
tests: every commit rule, every read, the archetype index, the append-only
triggers, and DDL idempotence.

SQLite is the only one of the five engines that can be verified without
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

## Testing

```sh
cargo test
```

Runs the shared suite against an in-memory database, plus this crate's own
tests for the trigger, the validation gate, lexical fidelity, and DDL
idempotence — and `tests/dialects.rs`, which compares all five engines' DDL and
fails if any two agree.

## Licence

MIT OR Apache-2.0.
