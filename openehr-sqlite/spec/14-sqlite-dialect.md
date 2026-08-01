# 14. SQLite dialect annex

**Status: proposed** (`X15.9`). This crate is at **Verified**, and that level
rests on `T11.2`/`conformance::run` in CI, not on this annex.

Normative only where it explicitly amends a core requirement by number
(`C0.12`, `X15.7`). Requirement prefix for departures: `M14`.

This is the only crate here with a `Store`, so it is the only annex where the
store-level requirements are in force rather than not applicable.

## 1. Engine floor (`S1.4`)

**SQLite 3.35+, compiled in.**

The engine is **pinned, not discovered**: the `bundled` feature compiles a known
SQLite rather than linking whatever the host ships. The generated DDL and the
JSON1 functions are version-dependent, and a store whose semantics depend on the
operating system's patch level is not portable (`S1.13`).

That also means the CI job cannot silently skip for want of an engine, which is
what makes **Verified** reachable here and nowhere else (`C0.8`).

## 2. `ColTy` binding (`M3.6`)

| `ColTy` | SQL | Why |
| --- | --- | --- |
| `Id(n)`, `Text(n)`, `LongText` | `TEXT` | SQLite applies type *affinity* rather than enforcement, so a declared length would be documentation nothing checks. |
| `Json` | `TEXT` | There is no JSON type; JSON1 operates on `TEXT`, so this is both the honest declaration and the working one. |
| `Instant` | `TEXT` | Authoritative lexical form. |
| `InstantUtc` | `INTEGER` | Unix seconds. SQLite has no date type, and storing the derived instant as text would make it sort identically to the authoritative column — collapsing the distinction the two columns exist to keep. |
| `Int` | `INTEGER` | |
| `Bool` | `INTEGER` | No boolean; 0 and 1 is the documented convention. |

`InstantUtc` and `Int` share `INTEGER`. That is permitted: `M3.31` constrains
only `Instant` against `InstantUtc`.

## 3. The two instant columns (`M3.31`)

`Instant` → `TEXT`, `InstantUtc` → `INTEGER`. **Distinct**, and the choice is
load-bearing rather than incidental — see the table above.

## 4. Idempotence (`G2.13`)

| Statement | Declaration | Engine fact |
| --- | --- | --- |
| `CREATE TABLE` | `IfNotExists` | supported |
| `CREATE INDEX` | `IfNotExists` | supported |
| `CREATE TRIGGER` | `IF NOT EXISTS` inline | supported |

## 5. Append-only enforcement (`M3.17`, `M3.37`)

`CREATE TRIGGER IF NOT EXISTS ... BEFORE UPDATE` / `BEFORE DELETE`, each body
`SELECT RAISE(ABORT, '<table> is append-only (openEHR V8.10)')`.

**No window exists**: `IF NOT EXISTS` means nothing is dropped. `M3.37`
satisfied.

Tested by going **around** the store, with the connection directly, because a
guarantee that only the store enforces is one the `sqlite3` CLI walks around.

## 6. Snapshot isolation (`R4.5`) and write serialization (`H5.4`)

Reads that return more than one row run inside a transaction. Commits are one
transaction covering the version row, the index row, and any container created
by it (`R4.4`).

**Neither is verified.** Nothing in this repository runs two threads against a
store, so both are recorded `?` in the conformance matrix and open as
`db:D-02`. SQLite's own locking makes the guarantee plausible; plausible is not
the bar (`C0.20`).

## 7. Foreign keys

SQLite disables foreign keys **per connection** by default. This store enables
them explicitly on open. Not doing so would accept a version pointing at a
container that does not exist — silently, while the schema said otherwise.

## 8. Placeholder syntax

`Placeholder::Question` — `?`. Driver: `rusqlite` 0.32, `bundled`.

## 9. Identifier quoting

`"identifier"`, embedded `"` doubled. SQLite also accepts `[…]` and backticks
for compatibility; this dialect emits only the SQL-standard form. Fuzzed by
[`openehr-sqlite-fuzz`](../../openehr-sqlite-fuzz).

## 10. Difference from the nearest neighbouring dialect (`X15.18`)

Nearest neighbour is **PostgreSQL**, sharing `"…"` quoting and both
`IfNotExists` declarations. Every type binding differs — `TEXT` against
`jsonb`/`timestamptz`/`boolean` — and the append-only mechanism differs
(`RAISE(ABORT)` in a trigger body against a `plpgsql` function).

This crate also hosts `tests/dialects.rs`, the cross-dialect comparison, because
it is the only crate that can see all six dialects.

## 11. Unmet core requirements

- **M14.5 amends `M3.29`.** The core requires `Id` and `Text` to carry a maximum
  length; this dialect discards the bound and emits `TEXT`.

  What survives: the bound is honoured by the engines that need it for indexing.
  What changes: SQLite will store an over-long identifier. Given type affinity,
  a declared length would not have been enforced anyway — the departure makes
  the schema honest rather than weakening it.

- **`M3.16`** (tamper-evidence chain), **`M3.18`** (erasure), **`PR12.5`** (read
  auditing), and **`O10.14`** (migration) are unimplemented here as everywhere,
  and are not amended by this annex.
