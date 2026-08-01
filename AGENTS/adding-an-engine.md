# Adding an engine crate

Not normative. The requirements are in
[`spec/databases/`](../spec/databases/index.md); this is how to satisfy them.

## Do not copy a sibling crate

This is the first instruction because it is the one that has already gone wrong.

`openehr-mariadb` was created by copying `openehr-mysql` and substituting the
name. The result emitted byte-identical DDL, exported a struct still called
`MysqlDialect`, described MariaDB as rejecting a statement MariaDB has accepted
since 10.0.5, and claimed verification against "MariaDB 8.4" — a release that has
never existed. It passed every test it had.

Start from the `Dialect` trait and decide each method for your engine.

## What a dialect owns

Exactly four things:

| Method | Owns |
| --- | --- |
| `col_sql` | how the engine spells each logical column type |
| `quote` | how it quotes an identifier |
| `placeholder` | how it writes a bind placeholder |
| `append_only_sql` | how it enforces append-only |

Plus two idempotence declarations — `table_idempotence` and
`index_idempotence` — and optionally `guard` and `terminator`.

Everything else comes from `openehr_store::schema`: which tables exist, which
columns, which indexes, what order they are emitted in. A dialect **cannot**
emit another engine's schema, because it does not own the schema.

## Steps

### 1. Create the crate

Its own workspace, matching the others:

```toml
[workspace]

[package]
name = "openehr-<engine>"
version = "0.1.1"
edition = "2024"
rust-version = "1.90"
license = "MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR GPL-3.0-only"
description = "openEHR persistence for <Engine>: schema dialect and DDL"
keywords = ["openehr", "ehr", "<engine>", "healthcare", "sql"]
categories = ["database"]
repository = "https://github.com/openehr-rust/openehr-rust"
readme = "README.md"

[dependencies]
openehr = { path = "../openehr", version = "0.1.1" }
openehr-store = { path = "../openehr-store", version = "0.1.1" }
serde_json = { version = "1", features = ["preserve_order"] }

[lints.rust]
missing_docs = "deny"
unsafe_code = "forbid"

[lints.clippy]
pedantic = { level = "warn", priority = -1 }
missing_errors_doc = "deny"
missing_panics_doc = "deny"
```

### 2. Implement `Dialect`

Name the struct after the engine — `PostgresqlDialect`, `MariadbDialect`. Two
crates exporting the same type name is a symptom.

Decide each `ColTy` from the engine's own documentation, not from a neighbour's
implementation. The ones that actually differ across engines:

- `Bool` — PostgreSQL `boolean`, MySQL/MariaDB `TINYINT(1)`, SQL Server `bit`,
  SQLite and Oracle numerics.
- `Json` — `jsonb`, native `JSON`, an alias for `LONGTEXT`, or plain text with a
  check constraint. Say which, and why it does not matter for round-tripping.
- `Instant` vs `InstantUtc` — these MUST NOT map to the same type. The first is
  the authoritative lexical form and is text; the second is derived, nullable,
  and native. A test enforces this.
- `Id(n)` / `Text(n)` — whether the length is required. It is where the engine
  cannot index an unbounded column.

### 3. Enforce append-only

`openehr_version` and `openehr_contribution` are append-only. The empty default
for `append_only_sql` exists only so a half-finished dialect compiles; it is not
a resting state, and `conformance::check_dialect` fails a dialect that leaves an
append-only table unenforced.

Three dialects silently inherited that default for as long as they existed while
the shared documentation described append-only as a property of the design.

Prefer a form that does not lapse. MySQL must `DROP TRIGGER` before recreating,
leaving an interval where the table would accept an `UPDATE`; MariaDB's
`CREATE OR REPLACE TRIGGER` does not. If your engine has the safer form, use it,
and say in the crate docs that this is why it differs from its neighbour.

### 4. Wire it into the cross-dialect comparison

**Both parts, or the guard does not cover you.** In `openehr-sqlite`:

- `Cargo.toml` — add to `[dev-dependencies]`.
- `tests/dialects.rs` — add to `all()` **and** bump `ENGINE_CRATES`.

The count exists because `all()` is hand-maintained, and a hand-maintained list
is what let a copied dialect go uncompared.

### 5. Write the golden tests

In `tests/ddl.rs`: self-consistency via `conformance::check_dialect`, the types
that are yours and not another engine's, identifier quoting, and full table and
index coverage.

Add a test for whatever distinguishes your engine from its nearest neighbour. If
you cannot name such a difference, you may not need a separate crate — or you
have not found it yet, which is worse.

### 6. Verify against a real server

Add a branch to `openehr-store/scripts/verify-schema.sh` and run it:

```sh
sh openehr-store/scripts/verify-schema.sh <engine>
```

It provisions the engine, applies the DDL, applies it **again**, seeds a row, and
checks that the append-only tables refuse `UPDATE` and `DELETE` with that row
present and intact afterwards.

Expect this to fail the first time. It has found a defect in every crate it has
been run against: MySQL rejecting `CREATE INDEX IF NOT EXISTS`, three dialects
enforcing append-only nowhere, and MariaDB emitting MySQL's script entirely.
Golden tests compare an emitter against its author's belief; only an engine
compares it against the engine.

Until this passes, the crate is at **Dialect** and its README and crate docs must
say so (`C0.9`).

### 7. Write the dialect annex

`X15.6` requires `spec/14-<engine>-dialect.md` in the crate, addressing nine
subjects by name: engine floor, `ColTy` bindings, the two instant columns,
idempotence, append-only enforcement, placeholder syntax, identifier quoting, the
difference from the nearest neighbouring dialect, and every unmet core
requirement as a numbered `M14.x` departure.

All six existing crates have one — read `openehr-mariadb`'s first, which explains
what happens when a dialect is a copy. Mark yours **proposed** (`X15.9`) until a
live run backs it.

## Checklist

- [ ] Own workspace; correct `repository`; version matches its siblings.
- [ ] Struct named after the engine.
- [ ] Every `ColTy` decided from the engine's documentation.
- [ ] `Instant` and `InstantUtc` map to different types.
- [ ] Append-only enforced on both append-only tables.
- [ ] Added to `dialects.rs` `all()` **and** `ENGINE_CRATES`, and to dev-deps.
- [ ] Golden tests, including what distinguishes this engine.
- [ ] `verify-schema.sh` branch, run, and passing — or the level says **Dialect**.
- [ ] README states the level in the first screenful.
- [ ] `spec/14-<engine>-dialect.md` written, marked **proposed** (`X15.6`).
- [ ] `openehr-<engine>-fuzz` crate with `quote` and `col_sql` targets, seeded,
      wired into CI, and `publish = false`.
- [ ] `LICENSE.md` present; licence expression matches the other seven (`W0.22`).
- [ ] `cargo clippy --all-targets` clean at pedantic.
