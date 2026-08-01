# Claiming a conformance level

Not normative. `C0.8`–`C0.13` in
[`spec/databases/00-conformance.md`](../spec/databases/00-conformance.md) are.

## The ladder

| Level | Means | Evidence |
| --- | --- | --- |
| **Dialect** | Emits DDL for the shared schema. | Golden tests + `conformance::check_dialect`. |
| **Schema** | The engine has executed that DDL. | A transcript: applied cleanly, applied *again* cleanly, append-only tables refused `UPDATE` and `DELETE` **with a row present**. |
| **Store** | Implements `Store` against a real database. | `conformance::run` passing against that engine. |
| **Verified** | Store, run in CI on every commit. | A CI job that provisions the engine and **fails, not skips**, without it. |

## Where each crate stands

| Crate | Level | Why not higher |
| --- | --- | --- |
| `openehr-sqlite` | **Verified** | — |
| `openehr-postgresql` | **Schema** | No driver, no `Store`. |
| `openehr-mysql` | **Schema** | as above |
| `openehr-mariadb` | **Schema** | as above |
| `openehr-mssql` | **Dialect** | No server has parsed it — SQL Server 2022 segfaults under qemu on arm64. |
| `openehr-oracle` | **Dialect** | No server has parsed it — the images need registry authentication. |

**`openehr-sqlite` is at Verified** as of the green run on 2026-08-01. A
committed workflow is not a working one — the level followed the run, not the
commit. Any text implying
continuous verification is a defect; `openehr-store/spec/conformance.md` carried
exactly such a claim, naming a workflow file that never existed
([`spec/audit.md`](../spec/audit.md) **W-02**).

## How to raise a level

### Dialect → Schema

```sh
sh openehr-store/scripts/verify-schema.sh <engine>
```

Add a branch if your engine has none. Then update, in the same commit: the
crate's README, its crate-level rustdoc, the table in
`openehr-store/spec/conformance.md`, and `spec/index.md`.

**Expect to fail the first time.** Three of three crates that have made this
step were wrong at Dialect level and passed every golden test while being wrong:

- MySQL rejects `CREATE INDEX IF NOT EXISTS`; the script created every table and
  died at the first index (`A-13`).
- Three dialects enforced append-only nowhere, inheriting an empty default, while
  the shared documentation described append-only as a property of the design
  (`A-15`).
- MariaDB was emitting MySQL's script in its entirety (**W-01**).

A golden test compares an emitter against its author's belief. Only an engine
compares it against the engine.

### Schema → Store

Implement `openehr_store::Store` and run `conformance::run` against a real
connection. The suite lives in `openehr-store` and is called from the engine
crate's own tests — written once, so it cannot agree with itself four times and
drift once.

### Store → Verified

Add CI that provisions the engine and runs the suite on every commit. It must
**fail, never skip**, when the database is absent: a skip is indistinguishable
from a pass in a CI summary, and the sibling monorepo carries a finding for two
ports whose database jobs invoked a test target that did not exist.

## Rules that are easy to break

- **State the level in the first screenful** of both the README and the crate
  docs (`C0.9`).
- **Never claim a level whose evidence came from another engine** (`C0.10`).
- **Never describe a capability above the crate's level** (`C0.11`). A README
  copied from a Store-level crate asserts, in the new crate's name, results never
  obtained for it.
- **"With a row present" is load-bearing** (`C0.12`). A `FOR EACH ROW` trigger on
  an empty table never fires, so a `DELETE` matching zero rows reports a refusal
  it never performed. The first enforcement run here looked like a pass and
  proved nothing.
- **A local run is not continuous verification** (`C0.13`). Say which it is.

## Checking a claim

Do not read it. Run it.

```sh
# Does the DDL actually differ from its neighbour's?
cargo run --manifest-path openehr-mysql/Cargo.toml   --example ddl | md5
cargo run --manifest-path openehr-mariadb/Cargo.toml --example ddl | md5

# Does the cited reproducer exist and work?
sh openehr-store/scripts/verify-schema.sh <engine>

# Does the CI a document claims exist?
ls .github/workflows
```

Every finding in [`spec/audit.md`](../spec/audit.md) was found this way. None was
found by reading.
