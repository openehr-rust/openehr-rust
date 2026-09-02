# Install

## Requirements

- **Requires Rust 1.96+ (edition 2024).** The minimum supported version is a
  formula, not a number — N−2, where N is current stable — and it moves on the
  Rust release schedule rather than on this project's
  ([`spec/rust-msrv-n-minus-2/index.md`](spec/rust-msrv-n-minus-2/index.md)).
- **Nothing else** for the library and the SQLite store. `openehr-sqlite`
  bundles its own SQLite through `rusqlite`'s `bundled` feature rather than
  linking whatever the host ships, because the emitted DDL and the JSON1
  functions the store uses are version-dependent.
- **A database server** only if you want a dialect other than SQLite, and
  **podman or docker** only if you want to re-run the schema verification
  yourself.

## Add it to a project

```sh
cargo add openehr
cargo add openehr-store openehr-sqlite   # if you want persistence
```

or, in `Cargo.toml`:

```toml
[dependencies]
openehr = "0.9"

# and, if you want persistence:
openehr-store = "0.9"
openehr-sqlite = "0.9"
```

The eight published crates are versioned in lockstep and released together, so
take the same minor version of each. **There are no Cargo features to choose**;
nothing here is behind a flag.

| Add this | To get |
| --- | --- |
| `openehr` | Reference Model types, validation, openEHR paths, AQL parsing, change-control security |
| `openehr-store` | the storage model, projection onto rows, commit rules, the `Store` and `Dialect` traits, and the conformance suite |
| `openehr-sqlite` | a complete embedded store, and the SQLite dialect |
| `openehr-postgresql`, `openehr-mysql`, `openehr-mariadb`, `openehr-mssql`, `openehr-oracle` | that engine's DDL. **A dialect is not a store** — it emits schema; you write the driver code |

Which of those has been verified how far is one table in one place:
[`spec/databases/conformance-matrix.md`](spec/databases/conformance-matrix.md).
Read it before choosing an engine; the levels are the point.

## Use it: an embedded store in ten lines

```rust
use openehr_sqlite::SqliteStore;
use openehr_store::Store;

let mut store = SqliteStore::in_memory()?;   // or `SqliteStore::open(path)?`
store.install()?;                            // idempotent: five tables, seven indexes
```

Then commit a contribution, read a version back, query it, and verify the stored
digests. The runnable version of that whole loop is one file:

```sh
(cd openehr-sqlite && cargo run --example 01_store_a_record)
```

For the library itself, five numbered tutorials run in order and each prints
what it did:

```sh
cd openehr
cargo run --example 01_build_composition     # build and validate a composition
cargo run --example 02_validate_incoming     # validate what arrived as JSON
cargo run --example 03_paths_and_aql         # address it, query it
cargo run --example 04_versioning_and_audit  # contributions, versions, audit trail
cargo run --example 05_access_and_redaction  # disclosure and redaction
```

**Validate anything that arrives as JSON.** A constructor checks the Reference
Model's invariants; a derived `Deserialize` does not, and writes fields straight
in (`lib:A-23`). `validate()` is what reports the bad ones.

## Get the DDL for another engine

Each engine crate prints its schema, so you can read it without running anything
against a database:

```sh
cargo run -q --manifest-path openehr-postgresql/Cargo.toml --example ddl
cargo run -q --manifest-path openehr-oracle/Cargo.toml --example ddl
```

From code, `openehr_store::ddl_script(&openehr_postgresql::PostgresqlDialect)`
returns the same text.

## Build from source

**There is no root workspace.** Each of the eighteen crates is its own Cargo
workspace, so run cargo from inside a crate directory:

```sh
git clone https://github.com/openehr-rust/openehr-rust.git
cd openehr-rust/openehr
cargo test
```

To check the whole tree the way CI does — note `RUSTFLAGS`, without which a lint
that only fires under `-D warnings` passes locally and fails in CI:

```sh
for d in openehr openehr-store openehr-sqlite openehr-postgresql \
         openehr-mysql openehr-mariadb openehr-mssql openehr-oracle \
         openehr-loco openehr-assets; do
  (cd "$d" && cargo test --quiet \
     && RUSTFLAGS="-D warnings" cargo clippy --all-targets --quiet) \
    || echo "FAIL $d"
done

python3 scripts/check-docs.py     # the documentation's counts, versions, levels
```

`Cargo.lock` is committed in every crate, unusually for libraries, so a build
from a given commit resolves the dependency versions that commit was tested
with. Leave it in place.

## Verify a dialect against a real engine yourself

This is the step that separates a claim from a check, and you can run it:

```sh
sh openehr-store/scripts/verify-schema.sh postgresql   # or mysql, mariadb
```

It starts the engine in a container, executes the DDL twice, confirms the
append-only tables refuse `UPDATE` and `DELETE` with a row present, and
round-trips canonical JSON bytes through the server unchanged.

## Documentation

- API documentation: [docs.rs/openehr](https://docs.rs/openehr),
  [docs.rs/openehr-store](https://docs.rs/openehr-store),
  [docs.rs/openehr-sqlite](https://docs.rs/openehr-sqlite)
- What must be true, and why: [`spec/index.md`](spec/index.md)
- Known defects, with evidence: [`spec/audit.md`](spec/audit.md)
- How to work in this repository: [`AGENTS.md`](AGENTS.md)

## Licence

Take **any one** of MIT, Apache-2.0, BSD-3-Clause, GPL-2.0-only, or
GPL-3.0-only — whichever suits your project. You do not need to satisfy all
five. Full text and SPDX identifiers: [`LICENSE.md`](LICENSE.md).

## Trademarks

openEHR® is the registered trademark of the openEHR Foundation and is used with
the permission of openEHR International. Use of the trademark does not
constitute endorsement of this product by openEHR International or openEHR
Foundation.
