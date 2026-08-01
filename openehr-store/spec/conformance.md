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

A level is a claim about the present, not about an afternoon in the past — and
until 2026-08-01 **every Schema claim here was exactly such an afternoon**, with
no continuous verification behind it.

This paragraph previously asserted that a `schema` job in
`.github/workflows/openehr.yml` ran the script on every change, so that the
claims were "continuously checked rather than attested once". No such workflow,
and no `.github/` directory, had ever existed. The requirement that continuity be
real is kept; the claim that it was has been withdrawn.

`.github/workflows/ci.yml` now exists and runs `scripts/verify-schema.sh` against
real PostgreSQL, MySQL, and MariaDB containers on every push and pull request. It
**fails rather than skips** when no container runtime is present, and it invokes
the same script a contributor runs rather than a parallel implementation in YAML
— two ways of doing one check drift, and the one that drifts is always the one
nobody runs. The sibling monorepo carries **F-06** for two ports whose database
jobs invoked a test target that did not exist, so they could not have passed and
did not say so.

**That workflow first ran green on 2026-08-01**, across all nineteen jobs, and
that run — not the commit that added the file — is what closed
[`spec/audit.md`](../../spec/audit.md) **W-02**. Treating a committed workflow as
a working one would have repeated the error this passage records; it took three
attempts to get the MySQL job passing, and the two failed ones were guesses.

`openehr-sqlite` is therefore at **Verified**. It is the only crate at Store
level and so the only one eligible.

The "with a row present" clause is not pedantry. The first enforcement run
looked like a pass and proved nothing: the `DELETE` matched zero rows, and a
`FOR EACH ROW` trigger on zero rows never fires. A test whose subject is absent
reports the silence as success.

## Where each crate stands today

**Assessed 2026-08-01.** Anything not in this table is not claimed.

| Crate | Level | What is verified | What is not |
| --- | --- | --- | --- |
| `openehr-sqlite` | **Verified** | The full suite against a real in-process database, run in CI on every push: every commit rule, every read, the archetype index, the append-only triggers, DDL idempotence. | Nothing exercises concurrent writers. |
| `openehr-postgresql` | **Schema** | DDL executed against **PostgreSQL 18**: 5 tables, 7 indexes, idempotent across repeated runs, foreign keys enforced, and both append-only tables refused `UPDATE` and `DELETE` with the row surviving unmodified. | No driver and no `Store`. |
| `openehr-mysql` | **Schema** | The same, against **MySQL 8.4**. | as above |
| `openehr-mariadb` | **Schema** | The same, against **MariaDB 11.4**. | as above. This crate was a name-substituted copy of `openehr-mysql` until 2026-08-01, claiming Schema against a "MariaDB 8.4" that does not exist; see [`spec/audit.md`](../../spec/audit.md) **W-01**. The current claim was earned by an actual run. |
| `openehr-mssql` | **Dialect** | DDL emission, type distinctness, quoting, identifier coverage, and that the declared catalogue guard is actually emitted. | No server has parsed it. SQL Server 2022 segfaults under qemu on arm64, so it could not be run on the machine available; this is a gap in evidence, not a judgement that it works. |
| `openehr-oracle` | **Dialect** | as above, plus Oracle's identifier-length limit. | as above; the Oracle images additionally require registry authentication. |

## What "Dialect" deliberately does not mean

It does not mean the DDL runs, and the two crates still at this level should be
read accordingly. The golden tests assert what the emitter produces, not that a
parser accepts it — those are different claims, and the sibling monorepo has two
findings (**F-25**, **F-26**) for a migration path that could never have
executed in a port with no store to notice.

The distance between the two claims is now measured rather than asserted. **All
three crates that moved to Schema were wrong at Dialect level**, and passed every
golden test while being wrong: PostgreSQL and MySQL surfaced `A-13`, `A-14`, and
`A-15`, and MariaDB was emitting another engine's script entirely (**W-01**).
Three of three is the number to carry into any judgement about the two crates
still at Dialect.

## Why the shared logic being verified once is worth something anyway

The commit rules, the projection onto rows, the ordering semantics, and the
conformance suite live in `openehr-store` and are shared by all six. Running them
against SQLite exercises that logic for every engine. What stays unverified per
engine is the DDL against a real parser and the driver glue — which is narrow,
but is exactly where an untested engine crate will fail.

A caution the MariaDB finding earns: shared logic is only shared where the
dialect actually differs from its neighbours. A dialect that is a copy of another
inherits that other engine's decisions along with the shared core, and the
cross-dialect comparison in `openehr-sqlite/tests/dialects.rs` is what makes the
difference visible. That comparison MUST list every engine crate; it listed five
of six for as long as the sixth was broken.
