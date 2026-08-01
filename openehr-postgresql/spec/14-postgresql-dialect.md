# 14. PostgreSQL dialect annex

**Status: proposed** (`X15.9`). A proposed annex MUST NOT be cited as evidence
for a conformance level.

Normative only where it explicitly amends a core requirement by number
(`C0.12`, `X15.7`). Everything else here is description; the core
[`spec/databases/`](../../spec/databases/index.md) governs.

Requirement prefix for departures: `M14`.

## 1. Engine floor (`S1.4`)

**PostgreSQL 18.**

The floor is set by nothing exotic — every construct this dialect emits has been
available for many major versions. 18 is declared because it is what the DDL has
actually been executed against (`T11.2`), and `C0.10` forbids claiming a level on
evidence from a different engine version than the one named. A lower floor would
be plausible and unverified, which is the state this project treats as a defect
rather than a shortcut.

## 2. `ColTy` binding (`M3.6`)

| `ColTy` | SQL | Why |
| --- | --- | --- |
| `Id(n)`, `Text(n)` | `text` | PostgreSQL stores `text` and `varchar(n)` identically. The bound would add only a check that rejects a long-but-legal `ARCHETYPE_ID`, and rejecting conformant data is a worse failure than a few extra bytes. |
| `LongText` | `text` | as above |
| `Json` | `jsonb` | The canonical byte form is regenerated from the parsed object and never read back from the column, so preserving whitespace buys nothing; containment and path indexes buy a lot. |
| `Instant` | `text` | Authoritative lexical form. Always text (`M3.24`). |
| `InstantUtc` | `timestamptz` | Derived, nullable. |
| `Int` | `bigint` | |
| `Bool` | `boolean` | PostgreSQL is the only target with a real boolean. |

## 3. The two instant columns (`M3.31`)

`Instant` → `text`, `InstantUtc` → `timestamptz`. **Distinct**, and asserted by
the cross-dialect test for all six engines.

## 4. Idempotence (`G2.13`)

| Statement | Declaration | Engine fact |
| --- | --- | --- |
| `CREATE TABLE` | `IfNotExists` | supported |
| `CREATE INDEX` | `IfNotExists` | supported — unlike MySQL, which is why that dialect differs |

No `guard` is used, and none is declared. `G2.15` fails a dialect that declares
`Guard` and does not wrap.

## 5. Append-only enforcement (`M3.17`, `M3.37`)

A `plpgsql` function per append-only table, raising an exception, plus a
`BEFORE UPDATE OR DELETE ... FOR EACH ROW` trigger.

Both are emitted with `CREATE OR REPLACE`, so **no window exists** in which the
table is unprotected — `M3.37` satisfied. One trigger covers both `UPDATE` and
`DELETE`, which PostgreSQL permits and MySQL does not.

Verified against a live server with a row present (`C0.12`): the row survived
unmodified and both statements were refused.

## 6. Placeholder syntax

`Placeholder::Dollar` — `$1`, `$2`. What `postgres`/`tokio-postgres` expect.

No driver is depended on: this crate has no `Store` (`W16.4`).

## 7. Identifier quoting

`"identifier"`, with an embedded `"` escaped by doubling to `""`.

Every identifier is quoted, because PostgreSQL folds unquoted identifiers to
**lower** case while Oracle folds to **upper**; quoting everywhere is what keeps
one set of names legal and identical on all six engines (`X15.3`, `G2.18`).

Fuzzed by [`openehr-postgresql-fuzz`](../../openehr-postgresql-fuzz).

## 8. Difference from the nearest neighbouring dialect (`X15.18`)

Nearest neighbour is **SQLite**, which shares the `"…"` quoting and the
`IfNotExists` declarations. They differ in every type binding that matters:

| | PostgreSQL | SQLite |
| --- | --- | --- |
| `Json` | `jsonb` | `TEXT` |
| `InstantUtc` | `timestamptz` | `INTEGER` (epoch seconds) |
| `Bool` | `boolean` | `INTEGER` |
| append-only | `plpgsql` function + trigger | `RAISE(ABORT)` in a trigger body |

## 9. Unmet core requirements

- **M14.1 amends `M3.29`.** The core requires `Id` and `Text` to carry a maximum
  length. This dialect **discards the bound** and emits `text`.

  What survives: the bound remains meaningful in the shared schema and is
  honoured by the engines that need it. What changes: PostgreSQL will accept an
  identifier longer than the declared bound, so a value rejected by MySQL may be
  stored here. Since every identifier this schema writes is generated or
  model-constrained, the divergence is reachable only by a caller supplying an
  over-long `ARCHETYPE_ID`, which the `openehr` crate refuses first.

- **`T11.2`** is satisfied; **`db:D-01`** (this annex not being ratified) and
  **`db:D-02`** (no concurrency test) are open and not amended here.

No `Store` exists in this crate, so every `H5.x`, `R4.x`, and `P6.x` store-level
requirement is **not applicable** rather than unmet.
