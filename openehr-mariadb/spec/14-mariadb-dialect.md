# 14. MariaDB dialect annex

**Status: proposed** (`X15.9`).

Normative only where it explicitly amends a core requirement by number
(`C0.12`, `X15.7`). Requirement prefix for departures: `M14`.

## 0. This crate was a copy, and this annex exists partly because of it

Until 2026-08-01 this crate was `openehr-mysql` with the engine name
substituted: byte-identical DDL, a struct still called `MysqlDialect`, and a
Schema claim naming a "MariaDB 8.4" that has never been released. See
[`spec/audit.md`](../../spec/audit.md) **W-01**.

An annex is how a dialect states what it actually is. Had one been required and
written, the claim would have had to name MariaDB facts, and no MariaDB fact
supports the text that was there.

## 1. Engine floor (`S1.4`)

**MariaDB 11.4.**

Set by the two features that separate this dialect from MySQL's:
`CREATE INDEX IF NOT EXISTS` (10.0.5+) and `CREATE OR REPLACE TRIGGER`
(10.1.4+). Both are far below 11.4; 11.4 is declared because it is the version
the DDL has been executed against (`T11.2`, `C0.10`).

## 2. `ColTy` binding (`M3.6`)

| `ColTy` | SQL | Why |
| --- | --- | --- |
| `Id(n)`, `Text(n)` | `VARCHAR(n)` | InnoDB cannot index an unbounded column. |
| `LongText` | `LONGTEXT` | |
| `Json` | `JSON` | **An alias for `LONGTEXT`**, not a distinct binary type as in MySQL. Kept for intent and because `json_valid()` and the `JSON_*` functions work against it; nothing here relies on binary storage, since canonical bytes are regenerated from the parsed object. |
| `Instant` | `VARCHAR(64)` | |
| `InstantUtc` | `DATETIME(6)` | Microseconds is the maximum. |
| `Int` | `BIGINT` | |
| `Bool` | `TINYINT(1)` | No boolean type. |

## 3. The two instant columns (`M3.31`)

`Instant` → `VARCHAR(64)`, `InstantUtc` → `DATETIME(6)`. **Distinct.**

## 4. Idempotence (`G2.13`)

| Statement | Declaration | Engine fact |
| --- | --- | --- |
| `CREATE TABLE` | `IfNotExists` | supported |
| `CREATE INDEX` | `IfNotExists` | **supported since 10.0.5** — unlike MySQL |

Indexes are their own statements here and read the way the schema declares them.
Using MySQL's `Inline` workaround would be an inherited decision rather than an
engine fact, which is exactly how this crate went wrong before.

## 5. Append-only enforcement (`M3.17`, `M3.37`)

`CREATE OR REPLACE TRIGGER ... BEFORE UPDATE` and `BEFORE DELETE`, each raising
`SIGNAL SQLSTATE '45000'`. Two triggers per table; MariaDB, like MySQL, does not
accept one trigger for both events.

**No window exists** — the trigger is replaced in one statement, so the
guarantee never lapses. `M3.37` satisfied, and this is the sharpest difference
from MySQL.

Verified against MariaDB 11.4 with a row present (`C0.12`).

## 6. Placeholder syntax

`Placeholder::Question` — `?`. No driver is depended on (`W16.4`).

## 7. Identifier quoting

`` `identifier` ``, embedded backtick doubled. Fuzzed by
[`openehr-mariadb-fuzz`](../../openehr-mariadb-fuzz).

## 8. Difference from the nearest neighbouring dialect (`X15.18`)

Nearest neighbour is **MySQL**. The type bindings agree; the two engine facts in
§4 and §5 are the difference, and both are asserted by a test in this crate that
fails if the crate drifts back into being a copy.

One more difference worth recording: the client binary is `mariadb`, not
`mysql`. MariaDB 11 renamed every binary and the compatibility symlinks are
deprecated, so `verify-schema.sh` uses `mariadb` — using `mysql` would work
today and break on the release that drops them.

## 9. Unmet core requirements

- **M14.4 amends nothing.** MariaDB satisfies `M3.37`, which MySQL departs from.
  Recorded as a non-departure deliberately: `X15.8` forbids restating unchanged
  core requirements, but the *absence* of MySQL's departure is a fact a reader
  comparing the two annexes needs.

No `Store` exists in this crate, so store-level requirements are **not
applicable** rather than unmet.
