# openehr-mysql

openEHR® persistence for **MySQL 8.4** — the schema dialect.

> openEHR® is the registered trademark of the openEHR Foundation and is used
> with the permission of openEHR International. Use of the trademark does not
> constitute endorsement of this product by openEHR International or openEHR
> Foundation.

## Conformance level: Schema

This crate emits DDL for the shared openEHR schema, and a MySQL 8.4 server has
executed it: five tables, seven indexes, idempotent on re-application, and both
append-only tables refusing `UPDATE` and `DELETE` with a row present and intact
afterwards.

```sh
../openehr-store/scripts/verify-schema.sh mysql
```

That run is why two of this crate's decisions look unlike the others. MySQL
declares its indexes **inside** `CREATE TABLE`, because it accepts
`CREATE TABLE IF NOT EXISTS` and rejects `CREATE INDEX IF NOT EXISTS` (**A-13**
— before the fix, `install()` created every table and then failed at the first
index). And its triggers are dropped before being created, because MySQL 8 has
neither `CREATE TRIGGER IF NOT EXISTS` nor `CREATE OR REPLACE TRIGGER`.

**It does not contain a store.** There is no driver dependency, no connection
handling, and no implementation of `Store`. Schema level means the database
accepts the schema, not that this crate can talk to it.

See [`openehr-store/spec/conformance.md`](../openehr-store/spec/conformance.md)
for what each level means and why they are stated this bluntly.

```rust
use openehr_mysql::MysqlDialect;
use openehr_store::ddl_script;

println!("{}", ddl_script(&MysqlDialect));
```

## Install

```toml
[dependencies]
openehr-mysql = "0.8"
openehr-store = "0.8"
```

Requires Rust 1.96+ (edition 2024).

## What this crate owns

Four things: type spellings, identifier quoting, placeholder style, and how the
engine enforces append-only. Everything else — which tables exist, which
columns, which indexes, the projection from openEHR objects onto rows, the
commit rules, the conformance suite — lives in
[`openehr-store`](../openehr-store) and is shared by all six engines.

That boundary is deliberate. The sibling FHIR monorepo in this repository gave
each of six ports a full copy of the DDL generator, and one of the copies spent
the fork's whole life emitting another engine's types (**F-08**). A dialect that
owns only spellings cannot do that, and
`openehr-sqlite/tests/dialects.rs` compares all six to make sure.

## MySQL 8.4 specifics

| Decision | Why |
| --- | --- |
| `VARCHAR(n)` with a mandatory length | InnoDB cannot index an unbounded column, and every identifier column in this schema is a key or part of one. |
| `JSON`, the native type | MySQL validates it on write, which catches a malformed document at the boundary rather than at read time. |
| `DATETIME(6)` for derived instants | Microseconds is MySQL's maximum. openEHR permits finer fractional seconds in the lexical form — which is why that form is stored separately and authoritatively. |
| `TINYINT(1)` for booleans | MySQL has no boolean type; this is the conventional spelling and what every driver maps to `bool`. |
| No append-only trigger | MySQL triggers cannot raise a clean error before 8.0's `SIGNAL`, and this crate has not been run against a server to confirm the form. Left undone and stated, rather than emitted untested. |

## Every instant is stored twice, and that is the point

openEHR times are ISO 8601 **strings** with deliberate partial precision:
`2024-05` is a date known to the month, and it is not `2024-05-01`. A native
timestamp column silently completes it — fabricating a clinical fact — and
normalises the lexical form, breaking round-trip fidelity.

So each time occupies `…_text` (authoritative, exact) and `…_utc` (derived,
nullable, for ordering). The derived column is `NULL` whenever the instant is
not established, which is the same answer the library gives, so SQL and Rust
cannot disagree about one record.

## Testing

```sh
cargo test
```

The tests are golden: they assert the SQL this crate emits, including
assertions that it is *not* another engine's SQL.

## What is not here

| Not here | Why |
| --- | --- |
| A `Store` | This crate is a dialect. Level **Schema** means the schema is emitted and the engine has executed it, not that this crate can talk to a database. |
| A driver dependency | A dependency implies a capability, and readers reasonably infer one (`W16.4`). |
| Archetype or template validation | Not implemented anywhere in this project (`lib:S1.4`). |
| AQL execution | Parsed and statically checked by `openehr`, never executed (`S1.6`). |

## Fuzzing

Identifier quoting is fuzzed by
[`openehr-mysql-fuzz`](../openehr-mysql-fuzz), because an identifier that
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
