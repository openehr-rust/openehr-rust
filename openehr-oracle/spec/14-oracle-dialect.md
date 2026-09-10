# 14. Oracle Database dialect annex

**Status: proposed** (`X15.9`).

**Conformance level: Schema**, since 2026-09-06 (CI run
[34040865467](https://github.com/openehr-rust/openehr-rust/actions/runs/34040865467),
job `schema / oracle`). Restated here only for orientation — the one file that
owns a level is [`spec/databases/conformance-matrix.md`](../../spec/databases/conformance-matrix.md)
(`W0.40`); see `M14.7` below.

Normative only where it explicitly amends a core requirement by number
(`C0.12`, `X15.7`). Requirement prefix for departures: `M14`.

## 1. Engine floor (`S1.4`)

**Oracle 12.2+**, and this is the row `db:S1.13` flagged as open.

The floor is set by a hard dialect fact: identifiers were **30 bytes** before
12.2 and 128 after. Every generated name in this schema fits 128 and several
exceed 30 — `ix_composition_context_start` is 28, `openehr_composition_index` is
26, and the trigger names built from them exceed 30. So the schema is **not
installable below 12.2**, and a deployment attempting it would fail at object
creation rather than silently.

Until this annex existed the floor was unstated, which is what
[`spec/databases/01-scope.md`](../../spec/databases/01-scope.md) recorded as the
open row. Stating it here is the fix.

## 2. `ColTy` binding (`M3.6`)

| `ColTy` | SQL | Why |
| --- | --- | --- |
| `Id(n)`, `Text(n)` | `VARCHAR2(n CHAR)` | **`CHAR` semantics, not the default `BYTE`.** `VARCHAR2(255)` means 255 *bytes* on Oracle, so a name in a non-Latin script would be rejected at roughly a third of its length. |
| `LongText`, `Json` | `CLOB` | `VARCHAR2` maxes out at 4000 bytes, so anything unbounded must be a LOB. |
| `Instant` | `VARCHAR2(64 CHAR)` | Authoritative lexical form. |
| `InstantUtc` | `TIMESTAMP WITH TIME ZONE` | Derived, nullable. |
| `Int` | `NUMBER(19)` | Oracle has no integer type; 19 digits is the range of an `i64`, which is what the row types use. |
| `Bool` | `NUMBER(1)` | No boolean in SQL, whatever PL/SQL offers. |

## 3. The two instant columns (`M3.31`)

`Instant` → `VARCHAR2(64 CHAR)`, `InstantUtc` → `TIMESTAMP WITH TIME ZONE`.
**Distinct.**

## 4. Idempotence (`G2.13`)

| Statement | Declaration | Engine fact |
| --- | --- | --- |
| `CREATE TABLE` | **`Guard`** | no `IF NOT EXISTS` |
| `CREATE INDEX` | **`Guard`** | no `IF NOT EXISTS` |

The guard is **catch-and-inspect**, not query-then-create: checking
`user_tables` first is a race, and Oracle's own idiom is to attempt the DDL and
swallow the one error meaning "already there". `ORA-00955` covers tables and
indexes alike, so the guard does not vary by object kind — unlike SQL Server's.
**Every other `SQLCODE` is re-raised**, because a guard that swallows unrelated
failures turns a broken schema into a silent one.

## 5. Statement terminator

`\n/`. Every statement this dialect emits is a PL/SQL block, and SQL*Plus ends a
block with `/` on its own line — a bare `;` would submit only as far as the
block's first inner semicolon. This dialect is the only one that overrides
`terminator()`.

## 6. Append-only enforcement (`M3.17`, `M3.37`)

A trigger per append-only table raising an application error. Emitted, and never
executed by an engine.

## 7. Placeholder syntax

`Placeholder::Colon` — `:1`, `:2`. No driver is depended on (`W16.4`).

## 8. Identifier quoting

`"identifier"`, embedded `"` doubled.

Every identifier is quoted, and therefore **case-sensitive**: unquoted Oracle
identifiers fold to UPPER case, which would make this schema's lower-case names
disagree with every other engine's and break the name-for-name comparability
`X15.3` requires. Fuzzed by
[`openehr-oracle-fuzz`](../../openehr-oracle-fuzz).

## 9. Difference from the nearest neighbouring dialect (`X15.18`)

Nearest neighbour is **SQL Server**, the other `Guard` dialect — see the
comparison table in that crate's annex. The sharpest differences are the guard
strategy (catch-and-inspect against catalogue-query) and the terminator.

## 10. Unmet core requirements

- **M14.7 amends `T11.2`. Met as of 2026-09-06** — the departure this
  requirement number names no longer holds, and the id stays as the permanent
  record of it (`C0.5`). The DDL has been executed against a real, running
  Oracle server: `openehr-store/scripts/verify-schema.sh oracle` against
  `gvenzl/oracle-free:latest` (Oracle Database Free 26ai) — tables and indexes
  created, the script re-applied as a no-op, a seed row's canonical JSON
  round-tripped byte for byte, and both append-only triggers observed refusing
  `UPDATE`/`DELETE` with the row unchanged afterwards — both by hand on a local
  native-arm64 container and in CI on a real x86_64 runner, run
  [34040865467](https://github.com/openehr-rust/openehr-rust/actions/runs/34040865467),
  job `schema / oracle`.

  The original cause recorded here — "the Oracle container images require
  registry authentication, which the machine available did not have" — was
  true of `container-registry.oracle.com`'s own official images, but not of
  every Oracle image: `gvenzl/oracle-free` is an independently maintained,
  publicly pullable rebuild requiring no registry login, and is what this
  crate now verifies against. The evidence gap was real; the image landscape
  had a way around it this annex did not know about yet.

- **M14.8 amends `M3.42`.** `M3.42` requires digest comparison to be confirmed
  against the source value. Where the source is a `CLOB`, Oracle cannot compare
  it with `=` at all; a confirming comparison MUST use `DBMS_LOB.COMPARE`.

  This is presently theoretical **for this crate specifically** — this crate
  has no `Store` (below), so nothing here ever executes a digest comparison
  to make — not because no digest exists anywhere in the schema: `M3.39`–
  `M3.42`'s `ColTy::Digest` columns are real and populated by every commit
  `openehr-sqlite` makes, using the same shared schema this crate's DDL
  emits (corrected here 2026-09-10, found stale while assessing `db:X15`
  against `db:D-11`; the same premise was wrong in three other places,
  `db:D-13`). It is recorded now because the `CLOB` binding in §2 is what
  makes it necessary the day a Store exists for this engine, and a
  departure discovered later reads as an oversight.

No `Store` exists in this crate, so store-level requirements are **not
applicable** rather than unmet.
