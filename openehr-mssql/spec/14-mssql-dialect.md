# 14. SQL Server dialect annex

**Status: proposed** (`X15.9`).

**Conformance level: Schema**, since 2026-09-06 (CI run
[34045294037](https://github.com/openehr-rust/openehr-rust/actions/runs/34045294037),
job `schema / mssql`). Restated here only for orientation — the one file that
owns a level is [`spec/databases/conformance-matrix.md`](../../spec/databases/conformance-matrix.md)
(`W0.40`); see `M14.6` below. The first real run found exactly the kind of gap
the distance between "emits" and "accepted" predicts: `D-12`, a `CREATE
TRIGGER` that shared a batch with every statement before it.

Normative only where it explicitly amends a core requirement by number
(`C0.12`, `X15.7`). Requirement prefix for departures: `M14`.

## 1. Engine floor (`S1.4`)

**SQL Server 2019+**, and *undeclared in evidence*. The constructs emitted —
`nvarchar(max)`, `datetimeoffset`, `sys.objects` catalogue guards — are long
established, but no server has confirmed the script, so the floor is a claim
about syntax rather than about behaviour.

## 2. `ColTy` binding (`M3.6`)

| `ColTy` | SQL | Why |
| --- | --- | --- |
| `Id(n)`, `Text(n)` | `nvarchar(n)` | **`nvarchar`, not `varchar`.** openEHR content is Unicode by construction — `DV_TEXT` carries an encoding attribute and clinical names are not ASCII — and a `varchar` column silently substitutes `?` for anything outside the collation's code page. Silent substitution in a clinical name is data loss that no downstream reader can detect. |
| `LongText`, `Json` | `nvarchar(max)` | No JSON type in the targeted versions. |
| `Instant` | `nvarchar(64)` | Authoritative lexical form. |
| `InstantUtc` | `datetimeoffset(7)` | **Not `datetime2`**: openEHR instants carry a UTC offset, and `datetime2` would drop it, making two records from different zones compare as the same moment. |
| `Int` | `bigint` | |
| `Bool` | `bit` | |

## 3. The two instant columns (`M3.31`)

`Instant` → `nvarchar(64)`, `InstantUtc` → `datetimeoffset(7)`. **Distinct.**

## 4. Idempotence (`G2.13`)

| Statement | Declaration | Engine fact |
| --- | --- | --- |
| `CREATE TABLE` | **`Guard`** | T-SQL has no `CREATE TABLE IF NOT EXISTS` |
| `CREATE INDEX` | **`Guard`** | nor `CREATE INDEX IF NOT EXISTS` |

The guard is a catalogue check wrapped around the statement, and the catalogue
view **differs by object kind**, so it cannot be one shared string.
`sys.objects ... type = 'U'` is used for tables rather than `sys.tables`, so
that a name collision with a *non-table* still fails loudly instead of the table
being created alongside it.

`G2.15` fails a dialect that declares `Guard` and does not actually wrap; this
one wraps, and a test asserts the guard is emitted.

## 5. Append-only enforcement (`M3.17`, `M3.37`)

A trigger per append-only table raising an error. Emitted, and **never executed
by an engine**.

`M3.36` is satisfied in the sense that the dialect does not inherit the empty
default — which is the check that exists, because three dialects once did
(`A-15`). Whether SQL Server accepts the trigger body is unknown.

## 6. Placeholder syntax

`Placeholder::AtP` — `@p1`, `@p2`. What `tiberius` expects. No driver is
depended on (`W16.4`).

## 7. Identifier quoting

`[identifier]`, with an embedded `]` escaped by doubling to `]]`.

**The only dialect here whose delimiters differ from each other.** A `[` inside
the brackets needs no escaping and is passed through — correct T-SQL, and a case
worth fuzzing precisely because it looks like an oversight. Fuzzed by
[`openehr-mssql-fuzz`](../../openehr-mssql-fuzz).

## 8. Difference from the nearest neighbouring dialect (`X15.18`)

Nearest neighbour is **Oracle**, the other `Guard` dialect. They differ in
approach as well as spelling:

| | SQL Server | Oracle |
| --- | --- | --- |
| guard strategy | query the catalogue, then create | attempt, and swallow ORA-00955 |
| guard varies by object kind | **yes** | no — one error code covers both |
| statement terminator | `\nGO` — every statement its own batch (`D-12`) | `\n/` — every statement is a PL/SQL block |
| quoting | `[…]`, `]]` | `"…"`, `""` |

## 9. Unmet core requirements

- **M14.6 amends `T11.2`. Met 2026-09-06.** The core requires the DDL to be
  executed against a real server before **Schema** is claimed.

  It has been, on CI's own x86_64 runners — no arm64 Linux SQL Server image
  exists, so this crate's DDL still cannot be verified on the machine this
  annex was originally written from, but that machine was never CI's own. The
  live run found a real defect, not merely an evidence gap: `append_only_sql`
  emitted `CREATE OR ALTER TRIGGER` as a bare statement sharing a batch with
  everything before it, which SQL Server refuses outright (`Msg 111`,
  `'CREATE TRIGGER' must be the first statement in a query batch`) — full
  account in `spec/databases/audit.md` **D-12**. Fixed by giving this dialect
  its own `terminator()`, `\nGO`, so every statement is its own batch; see
  §8's comparison table.

  Getting a clean run past that also needed one fix outside this crate
  entirely: `verify-schema.sh`'s own `sqlcmd` invocation was missing `-W`
  ("remove trailing spaces from a column"), so an `nvarchar(max)` value came
  back correctly followed by a large run of padding, comparing unequal to the
  literal it went in as — a display artifact in the verification harness, not
  in this dialect's DDL.

  `spec/databases/conformance-matrix.md` — the one file that owns a level
  (`W0.40`) — records the green `schema / mssql` run (34045294037); this
  annex does not itself grant the level (`X15.9`), and stays **proposed** for
  the same reason the three earlier Schema engines' annexes do.

- **`db:D-01`** is closed by this file existing; ratifying it (`X15.9`) requires
  a live run.

No `Store` exists in this crate, so store-level requirements are **not
applicable** rather than unmet.
