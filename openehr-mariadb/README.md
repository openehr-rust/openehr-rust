# openehr-mariadb

openEHR persistence for **MariaDB 11.4** — the schema dialect.

## Conformance level: Schema

This crate emits DDL for the shared openEHR schema, and a MariaDB 11.4 server has
executed it: five tables, seven indexes, idempotent on re-application, foreign
keys enforced, and both append-only tables refusing `UPDATE` and `DELETE` with a
row present and intact afterwards.

```sh
../openehr-store/scripts/verify-schema.sh mariadb
```

CI runs the same script on every push. The level is Schema rather than Verified
because this crate has no `Store`, not because the check is unverified. See
[`spec/audit.md`](../spec/audit.md) **W-02**.

**It does not contain a store.** There is no driver dependency, no connection
handling, and no implementation of `Store`. Schema level means the database
accepts the schema, not that this crate can talk to it.

See [`spec/index.md`](../spec/index.md) for what each level means and why they
are stated this bluntly.

```rust
use openehr_mariadb::MariadbDialect;
use openehr_store::ddl_script;

println!("{}", ddl_script(&MariadbDialect));
```

## This crate was `openehr-mysql` wearing a different name

Worth stating plainly, because the corrected version below is only meaningful
against it. Until 2026-08-01 this crate:

- emitted **byte-identical DDL** to `openehr-mysql` (both hashed to
  `40f32f64e5015f8640830a67aecb9c72`);
- exported a public struct named **`MysqlDialect`**;
- claimed Schema level against **"MariaDB 8.4"** — a release that has never
  existed, MariaDB having gone 10.x then 11.x;
- told readers to reproduce it with a script that answered
  `FAIL: unknown engine 'mariadb'`;
- and described MariaDB as *rejecting* `CREATE INDEX IF NOT EXISTS`, which is
  true of MySQL and false of MariaDB.

It survived because `openehr-sqlite/tests/dialects.rs` — the test whose entire
purpose is to fail when two dialects emit the same schema — compared five
dialects, and this was the sixth. A comparison that omits a dialect cannot find
it identical to another.

All of it is fixed, and the record is kept in [`spec/audit.md`](../spec/audit.md)
**W-01** rather than quietly deleted.

## Install

```toml
[dependencies]
openehr-mariadb = "0.1"
openehr-store = "0.1"
```

Requires Rust 1.90+ (edition 2024).

## What this crate owns

Four things: type spellings, identifier quoting, placeholder style, and how the
engine enforces append-only. Everything else — which tables exist, which columns,
which indexes, the projection from openEHR objects onto rows, the commit rules,
the conformance suite — lives in [`openehr-store`](../openehr-store) and is
shared by all six engines.

## MariaDB is not MySQL, and here is where that is decided

Two differences, both real engine facts rather than cosmetic edits, and both
covered by a test that fails if this crate drifts back into being a copy:

| MariaDB 11.4 | MySQL 8.4 | Consequence here |
| --- | --- | --- |
| `CREATE INDEX IF NOT EXISTS` (since 10.0.5) | not supported | indexes are their own statements, not folded into `CREATE TABLE` |
| `CREATE OR REPLACE TRIGGER` (since 10.1.4) | not supported | no drop-then-create window in which the table is unprotected |

The second is the one that matters. MySQL must `DROP TRIGGER` before recreating
it, which leaves an interval — short, but real — in which an append-only table
would accept an `UPDATE`. MariaDB replaces the trigger in one statement, so the
guarantee never lapses.

## MariaDB 11.4 specifics

| Decision | Why |
| --- | --- |
| `VARCHAR(n)` with a mandatory length | InnoDB cannot index an unbounded column, and every identifier column in this schema is a key or part of one. |
| `JSON` | In MariaDB this is an alias for `LONGTEXT`, not a distinct binary type as in MySQL. The spelling is kept because it documents intent and because `json_valid()` and the `JSON_*` functions work against it; nothing here relies on binary storage, since the canonical bytes are regenerated from the parsed object rather than read back from the column. |
| `DATETIME(6)` for derived instants | Microseconds is MariaDB's maximum. openEHR permits finer fractional seconds in the lexical form — which is why that form is stored separately and authoritatively. |
| `TINYINT(1)` for booleans | MariaDB has no boolean type; this is the conventional spelling and what every driver maps to `bool`. |
| `SIGNAL SQLSTATE '45000'` in append-only triggers | The documented way for a trigger to refuse, and what a driver surfaces as an error. Without it the guarantee would hold in application code only — and a guarantee a SQL console can walk around is not one. |

## Every instant is stored twice, and that is the point

openEHR times are ISO 8601 **strings** with deliberate partial precision:
`2024-05` is a date known to the month, and it is not `2024-05-01`. A native
timestamp column silently completes it — fabricating a clinical fact — and
normalises the lexical form, breaking round-trip fidelity.

So each time occupies `…_text` (authoritative, exact) and `…_utc` (derived,
nullable, for ordering). The derived column is `NULL` whenever the instant is not
established, which is the same answer the library gives, so SQL and Rust cannot
disagree about one record.

## Testing

```sh
cargo test
```

The tests are golden: they assert the SQL this crate emits, including assertions
that it is *not* another engine's SQL, and that the two differences from MySQL
above are actually present.

## What is not here

| Not here | Why |
| --- | --- |
| A `Store` | This crate is a dialect. Level **Schema** means the schema is emitted and the engine has executed it, not that this crate can talk to a database. |
| A driver dependency | A dependency implies a capability, and readers reasonably infer one (`W16.4`). |
| Archetype or template validation | Not implemented anywhere in this project (`lib:S1.4`). |
| AQL execution | Parsed and statically checked by `openehr`, never executed (`S1.6`). |

## Fuzzing

Identifier quoting is fuzzed by
[`openehr-mariadb-fuzz`](../openehr-mariadb-fuzz), because an identifier that
escapes its own delimiter is SQL injection and archetype ids reach a `WHERE`
clause from caller input. Run in CI on every push.

## Specification

This crate implements the shared persistence specification; it defines nothing
of its own beyond its dialect.

- [`spec/databases/`](../spec/databases/index.md) — the storage model, the
  dialect boundary, the conformance ladder
- [`spec/databases/conformance-matrix.md`](../spec/databases/conformance-matrix.md)
  — what is verified for **this** engine today
- [`spec/audit.md`](../spec/audit.md) — known gaps

## Licence


Any of these, at your option — MIT, Apache-2.0, BSD-3-Clause, GPL-2.0-only, or
GPL-3.0-only. See [`LICENSE.md`](LICENSE.md).
