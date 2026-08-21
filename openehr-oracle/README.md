# openehr-oracle

openEHR persistence for **Oracle Database 23ai** — the schema dialect.

## Conformance level: Dialect

This crate emits DDL for the shared openEHR schema. **It does not contain a
store.** There is no driver dependency, no connection handling, no
implementation of `Store`, and no statement in this crate has ever been
submitted to a Oracle Database 23ai server.

See [`openehr-store/spec/conformance.md`](../openehr-store/spec/conformance.md)
for what each level means and why they are stated this bluntly.

```rust
use openehr_oracle::OracleDialect;
use openehr_store::ddl_script;

println!("{}", ddl_script(&OracleDialect));
```

## Install

```toml
[dependencies]
openehr-oracle = "0.2"
openehr-store = "0.2"
```

Requires Rust 1.95+ (edition 2024).

## What this crate owns

Four things: type spellings, identifier quoting, placeholder style, and how the
engine enforces append-only. Everything else — which tables exist, which
columns, which indexes, the projection from openEHR objects onto rows, the
commit rules, the conformance suite — lives in
[`openehr-store`](../openehr-store) and is shared by all six engines.

That boundary is deliberate, and this crate is the reason it exists. The
sibling FHIR monorepo in this repository gave each of six ports a full copy of
the DDL generator, and **its Oracle port spent the fork's whole life emitting
MySQL types** (**F-08**) — a file copied with three lines changed, which nothing
ever compared. A dialect that owns only spellings cannot do that;
`openehr-sqlite/tests/dialects.rs` compares all six; and this crate's own
tests assert that its DDL contains no `TINYINT`.

## Oracle Database 23ai specifics

| Decision | Why |
| --- | --- |
| `VARCHAR2(n CHAR)`, with `CHAR` semantics | `VARCHAR2(255)` means 255 *bytes* on Oracle, so a name in a non-Latin script would be rejected at about a third of its length. |
| `CLOB` for JSON and unbounded text | `VARCHAR2` maxes out at 4000 bytes; anything unbounded must be a LOB. |
| `NUMBER(19)` for integers, `NUMBER(1)` for booleans | Oracle SQL has neither an integer nor a boolean type, whatever PL/SQL offers. `NUMBER(19)` is the range of the `i64` the row types use. |
| Quoted, case-sensitive identifiers | Unquoted Oracle identifiers fold to upper case, which would make this schema's lower-case names disagree with every other engine's. |
| No `IF NOT EXISTS` | Oracle has none before 23ai, and this script is meant to run on 19c too. |
| An identifier-length test | Oracle is the only one of the five with a limit short enough to hit, and an over-long name fails at `CREATE TABLE` and nowhere else — so a schema change made against another engine would break here alone. |

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
[`openehr-oracle-fuzz`](../openehr-oracle-fuzz), because an identifier that
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
