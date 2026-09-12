# openehr-postgresql

openEHR® persistence for **PostgreSQL 18** — the schema dialect and a real
`Store`.

> openEHR® is the registered trademark of the openEHR Foundation and is used
> with the permission of openEHR International. Use of the trademark does not
> constitute endorsement of this product by openEHR International or openEHR
> Foundation.

## Conformance level: Verified

`PostgresqlStore` implements `openehr_store::Store` and passes the shared
conformance suite — `conformance::run`, `run_ehr_status`,
`run_is_modifiable_gate` — against a real PostgreSQL 18 server, plus every
`openehr-sqlite`-only test this crate had an equivalent for: concurrency,
the tamper-evident chain, the checkpoint, schema-version refusal. Re-checked
in CI on every push (the `schema` job's own `postgresql` matrix leg), green
run
[34710118312](https://github.com/openehr-rust/openehr-rust/actions/runs/34710118312),
2026-09-12.

```sh
sh scripts/verify-store.sh
```

Provisions a disposable `postgres:18-alpine` container with its port
published to the host, runs the full suite against it, and tears it down —
reproducible from a fresh checkout, the same discipline
`../openehr-store/scripts/verify-schema.sh` already holds this crate's DDL
to.

See
[`spec/databases/conformance-matrix.md`](../spec/databases/conformance-matrix.md),
the one file that owns this claim.

See [`openehr-store/spec/conformance.md`](../openehr-store/spec/conformance.md)
for what each level means and why they are stated this bluntly.

```rust
use openehr_postgresql::PostgresqlDialect;
use openehr_store::ddl_script;

println!("{}", ddl_script(&PostgresqlDialect));
```

## Install

```toml
[dependencies]
openehr-postgresql = "0.9"
openehr-store = "0.9"
```

Requires Rust 1.96+ (edition 2024).

## What this crate owns

Two things: the dialect — type spellings, identifier quoting, placeholder
style, and how the engine enforces append-only — and `PostgresqlStore`, the
driver glue that runs the shared logic against a real connection. Everything
in between — which tables exist, which columns, which indexes, the
projection from openEHR objects onto rows, the commit rules, the conformance
suite — lives in [`openehr-store`](../openehr-store) and is shared by all six
engines; this crate does not reimplement any of it.

That boundary is deliberate. The sibling FHIR monorepo in this repository gave
each of six ports a full copy of the DDL generator, and one of the copies spent
the fork's whole life emitting another engine's types (**F-08**). A dialect that
owns only spellings cannot do that, and
`openehr-sqlite/tests/dialects.rs` compares all six to make sure.

## PostgreSQL-specific choices

| Decision | Why |
| --- | --- |
| `text`, not `varchar(n)` | PostgreSQL stores both identically; the length would only add a check that rejects a long-but-legal `ARCHETYPE_ID`. |
| `text`, **not** `jsonb`, for canonical JSON | `jsonb` reorders keys and rewrites numbers — measured on PostgreSQL 18, not assumed. The chain's content digest is SHA-256 over the exact bytes committed (`M3.16`), so those bytes must be reproducible from storage; `jsonb` produces an equivalent document, not the same one (`M3.43`, `D-08`). |
| `timestamptz` for derived instants | The authoritative instant is stored as `text` alongside it — see below. `PostgresqlStore` converts through `time::OffsetDateTime` on both sides, since `postgres-types` has no `ToSql`/`FromSql` between a raw integer and `timestamptz`. |
| An append-only trigger | The guarantee lives in the database, not in application code, where it would end the first time somebody opened `psql`. |
| `postgres`, blocking, `NoTls` | A synchronous driver matches `Store`'s own synchronous trait — no async runtime for a caller to bring. `NoTls`: this crate connects to a server the deployment already trusts on its own network; TLS to a database is a separate decision this crate does not make on a caller's behalf. |

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
cargo test              # golden DDL tests: no server needed
sh scripts/verify-store.sh   # the Store, against a real, disposable server
```

The golden tests assert the SQL this crate's dialect emits, including that it
is *not* another engine's SQL. The `Store` tests are `#[ignore]`d — they need
`OPENEHR_POSTGRESQL_URL` pointing at a real server, which `verify-store.sh`
provisions, uses, and tears down.

## What is not here

| Not here | Why |
| --- | --- |
| Archetype or template validation | Not implemented anywhere in this project (`lib:S1.4`). |
| AQL execution | Parsed and statically checked by `openehr`, never executed (`S1.6`). |
| A keyed-chain or two-system-race test | Gaps in `openehr_store::conformance`'s own shared fixtures, not in this crate — `db:D-14`. |

## Fuzzing

Identifier quoting is fuzzed by
[`openehr-postgresql-fuzz`](../openehr-postgresql-fuzz), because an identifier that
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
