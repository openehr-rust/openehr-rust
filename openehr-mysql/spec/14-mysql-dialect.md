# 14. MySQL dialect annex

**Status: proposed** (`X15.9`).

Normative only where it explicitly amends a core requirement by number
(`C0.12`, `X15.7`). Requirement prefix for departures: `M14`.

## 1. Engine floor (`S1.4`)

**MySQL 8.4.**

Set by two dialect facts: `SIGNAL SQLSTATE` in a trigger body (8.0+), and the
`JSON` type (5.7+). 8.4 is declared because it is the version the DDL has been
executed against (`T11.2`, `C0.10`).

## 2. `ColTy` binding (`M3.6`)

| `ColTy` | SQL | Why |
| --- | --- | --- |
| `Id(n)`, `Text(n)` | `VARCHAR(n)` | The bound is load-bearing: InnoDB cannot index an unbounded column, and every `Id` here is a key or part of one. |
| `LongText` | `LONGTEXT` | |
| `Json` | `JSON` | A real binary type in MySQL, validated on write. |
| `Instant` | `VARCHAR(64)` | Bounded so it can be indexed alongside the derived column. The longest ISO 8601 form this project accepts is well under 64. |
| `InstantUtc` | `DATETIME(6)` | Microseconds is MySQL's maximum; openEHR permits finer fractional seconds in the lexical form, which is why that form is stored separately and authoritatively (`M3.25`). |
| `Int` | `BIGINT` | |
| `Bool` | `TINYINT(1)` | MySQL has no boolean; this is what every driver maps to `bool`. |

## 3. The two instant columns (`M3.31`)

`Instant` → `VARCHAR(64)`, `InstantUtc` → `DATETIME(6)`. **Distinct.**

## 4. Idempotence (`G2.13`)

| Statement | Declaration | Engine fact |
| --- | --- | --- |
| `CREATE TABLE` | `IfNotExists` | supported |
| `CREATE INDEX` | **`Inline`** | **MySQL rejects `CREATE INDEX IF NOT EXISTS`** |

This is the split `G2.13` exists for, and it was found the hard way: the first
live run created every table and then died at the first index (`A-13`). Indexes
are declared inside `CREATE TABLE`, where they inherit the table's own
`IF NOT EXISTS` — one statement, one object, one idempotence rule.

## 5. Append-only enforcement (`M3.17`, `M3.37`)

`DROP TRIGGER IF EXISTS` followed by `CREATE TRIGGER ... BEFORE UPDATE` and
`BEFORE DELETE`, each raising `SIGNAL SQLSTATE '45000'`. Two triggers per table:
MySQL does not accept `BEFORE UPDATE OR DELETE` in one trigger.

Verified live with a row present (`C0.12`).

**This leaves a window** — see `M14.3`.

## 6. Placeholder syntax

`Placeholder::Question` — `?`. No driver is depended on; this crate has no
`Store` (`W16.4`).

## 7. Identifier quoting

`` `identifier` ``, with an embedded backtick escaped by doubling to `` `` ``.
Not SQL-standard, which is why it needs its own target — fuzzed by
[`openehr-mysql-fuzz`](../../openehr-mysql-fuzz).

## 8. Difference from the nearest neighbouring dialect (`X15.18`)

Nearest neighbour is **MariaDB**, and the two agree on every `ColTy` binding and
on quoting. They are separated by two engine facts, both real:

| | MySQL 8.4 | MariaDB 11.4 |
| --- | --- | --- |
| `CREATE INDEX IF NOT EXISTS` | **rejected** | accepted since 10.0.5 |
| `CREATE OR REPLACE TRIGGER` | **not available** | since 10.1.4 |

That the type tables agree is legitimate. It is also why `openehr-mariadb` was
able to exist as a copy of this crate undetected for as long as it did
(**W-01**), and why `X15.18` now requires the difference to be *named and
tested* rather than assumed.

## 9. Unmet core requirements

- **M14.3 amends `M3.37`.** The core SHOULD-prefers a trigger form that does not
  drop first. MySQL has neither `CREATE OR REPLACE TRIGGER` nor
  `CREATE TRIGGER IF NOT EXISTS`, so this dialect drops and recreates.

  What survives: append-only is fully enforced in steady state. What changes:
  during `install()` there is an interval — short, but real — in which the table
  would accept an `UPDATE`. The window is confined to install and MUST NOT be
  reproduced at run time. A deployment that re-runs `install()` against a live
  database is doing so unprotected for that interval.

No `Store` exists in this crate, so store-level requirements are **not
applicable** rather than unmet.
