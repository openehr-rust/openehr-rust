# openehr-mssql

openEHR® persistence for **Microsoft SQL Server 2022** — the schema dialect.

> openEHR® is the registered trademark of the openEHR Foundation and is used
> with the permission of openEHR International. Use of the trademark does not
> constitute endorsement of this product by openEHR International or openEHR
> Foundation.

## Conformance level: Dialect

This crate emits DDL for the shared openEHR schema. **It does not contain a
store.** There is no driver dependency, no connection handling, no
implementation of `Store`, and no statement in this crate has ever been
submitted to a Microsoft SQL Server 2022 server.

See [`openehr-store/spec/conformance.md`](../openehr-store/spec/conformance.md)
for what each level means and why they are stated this bluntly.

```rust
use openehr_mssql::MssqlDialect;
use openehr_store::ddl_script;

println!("{}", ddl_script(&MssqlDialect));
```

## Install

```toml
[dependencies]
openehr-mssql = "0.8"
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

## Microsoft SQL Server 2022 specifics

| Decision | Why |
| --- | --- |
| `nvarchar`, never `varchar` | openEHR content is Unicode by construction — `DV_TEXT` carries an encoding attribute and clinical names are not ASCII. A `varchar` column silently substitutes `?` for anything outside the collation's code page. |
| `nvarchar(max)` for JSON | SQL Server has no JSON column type; `JSON_VALUE` and friends operate on `nvarchar`. |
| `datetimeoffset(7)`, not `datetime2` | openEHR instants carry a UTC offset, and `datetime2` would drop it — making two records from different zones compare as the same moment. |
| No `IF NOT EXISTS` | SQL Server has no such clause for tables or indexes. Emitting it anyway would produce a script that fails on the one engine it targets, which is the shape of the sibling monorepo's **F-25** and **F-26**. |
| No append-only trigger | An `INSTEAD OF` trigger is the right mechanism, and its exact form has not been run against a server. Left undone and stated. |

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
| A `Store` | This crate is a dialect. Level **Dialect** means the schema is emitted, not that this crate can talk to a database. |
| A driver dependency | A dependency implies a capability, and readers reasonably infer one (`W16.4`). |
| Archetype or template validation | Not implemented anywhere in this project (`lib:S1.4`). |
| AQL execution | Parsed and statically checked by `openehr`, never executed (`S1.6`). |

## Fuzzing

Identifier quoting is fuzzed by
[`openehr-mssql-fuzz`](../openehr-mssql-fuzz), because an identifier that
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
