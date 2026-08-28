# Conformance levels

Non-normative in itself; the levels it defines are normative for every engine
crate's documentation.

A crate MUST NOT claim a level it has not earned, and its README MUST state its
level in the first screenful. This exists because the sibling FHIR monorepo in
this repository shipped six READMEs claiming a working store, a CLI, and 7,399
losslessly round-tripped resources — in ports where two had no store at all and
none had ever had a CLI (**F-01**).

## The levels

| Level | Means | Evidence required |
| --- | --- | --- |
| **Dialect** | Emits DDL for the shared schema. | The golden DDL tests, and [`conformance::check_dialect`]. |
| **Schema** | The engine itself has executed that DDL. | A transcript against that engine's own server: the script applied cleanly, applied *again* cleanly, and the append-only tables were observed refusing `UPDATE` and `DELETE` **with a row present**. |
| **Store** | Implements `Store` against a real database. | [`conformance::run`] passing against that engine. |
| **Verified** | Store level, run in CI against the engine's own server on every commit. | A CI job that provisions the engine and fails — not skips — without it. |

Schema is a level because the step from Dialect to Schema found three defects
that no golden test could have found; see `A-13`, `A-14`, and `A-15` in the
register.

A level is a claim about the present, not about an afternoon in the past. The
`schema` job in `.github/workflows/openehr.yml` runs
`scripts/verify-schema.sh` for PostgreSQL and MySQL on every change under
`openehr*/`, so both crates' Schema claims are continuously checked rather than
attested once. The job provisions its own container instead of using GitHub's
`services:`, so the command in CI is byte-for-byte the command a contributor
runs locally — two ways of doing the same check drift, and the one that drifts
is always the one nobody runs.

The same job **must fail, never skip**, when a database is absent. The sibling
monorepo carries **F-06** for two ports whose database jobs invoked a test
target that did not exist, so they could not have passed and did not say so.

The "with a row present" clause is not pedantry. The first enforcement run
looked like a pass and proved nothing: the `DELETE` matched zero rows, and a
`FOR EACH ROW` trigger on zero rows never fires. A test whose subject is absent
reports the silence as success.

## Where each crate stands today

**Assessed 2026-08-01.** Anything not in this table is not claimed.

| Crate | Level | What is verified | What is not |
| --- | --- | --- | --- |
| `openehr-sqlite` | **Store** | The full suite against a real in-process database: every commit rule, every read, the archetype index, the append-only triggers, DDL idempotence. | Nothing runs in CI yet, so the level is Store and not Verified. |
| `openehr-postgresql` | **Schema** | DDL executed against **PostgreSQL 18**: 5 tables, 7 indexes, idempotent across repeated runs, foreign keys enforced, and both append-only tables refused `UPDATE` and `DELETE` with the row surviving unmodified. | No driver and no `Store`. |
| `openehr-mysql` | **Schema** | The same, against **MySQL 8.4**. | as above |
| `openehr-mssql` | **Dialect** | DDL emission, type distinctness, quoting, identifier coverage, and that the declared catalogue guard is actually emitted. | No server has parsed it. SQL Server 2022 segfaults under qemu on arm64, so it could not be run on the machine available; this is a gap in evidence, not a judgement that it works. |
| `openehr-oracle` | **Dialect** | as above, plus Oracle's identifier-length limit. | as above; the Oracle images additionally require registry authentication. |

## What "Dialect" deliberately does not mean

It does not mean the DDL runs, and the two crates still at this level should be
read accordingly. The golden tests assert what the emitter produces, not that a
parser accepts it — those are different claims, and the sibling monorepo has two
findings (**F-25**, **F-26**) for a migration path that could never have
executed in a port with no store to notice.

The distance between the two claims is now measured rather than asserted. Both
crates that moved to Schema were **wrong** at Dialect level, and passed every
golden test while being wrong.

## Why the shared logic being verified once is worth something anyway

The commit rules, the projection onto rows, the ordering semantics, and the
conformance suite live in `openehr-store` and are shared by all five. Running
them against SQLite exercises that logic for every engine. What stays unverified
per engine is the DDL against a real parser and the driver glue — which is
narrow, but is exactly where an untested engine crate will fail.
