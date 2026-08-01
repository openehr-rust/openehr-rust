# 14. SQL Server dialect annex

**Status: proposed** (`X15.9`).

**Conformance level: Dialect.** No SQL Server has ever parsed this DDL. Read
everything below as *what this dialect emits*, not as *what an engine accepted* —
the distance between those has been measured on this project at three defects in
three crates (`A-13`, `A-15`, **W-01**).

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
| statement terminator | `;` | `\n/` — every statement is a PL/SQL block |
| quoting | `[…]`, `]]` | `"…"`, `""` |

## 9. Unmet core requirements

- **M14.6 amends `T11.2`.** The core requires the DDL to be executed against a
  real server before **Schema** is claimed. It has not been, so this crate claims
  **Dialect** and this is a statement of the gap rather than a licence to skip it.

  Cause: SQL Server 2022 segfaults under qemu on arm64, the only architecture
  available. That is an evidence gap, **not** a judgement that the DDL works.

- **`db:D-01`** is closed by this file existing; ratifying it (`X15.9`) requires
  a live run.

No `Store` exists in this crate, so store-level requirements are **not
applicable** rather than unmet.
