# Claiming a conformance level

Not normative. `C0.8`–`C0.13` in
[`spec/databases/00-conformance.md`](../spec/databases/00-conformance.md) are.

## The ladder

The table below is a **marked copy**: `db:C0.8` owns it, and
`scripts/check-docs.py` fails if this drifts from it (`W0.38`). Edit it there.

<!-- shared: conformance-ladder (copy) -->
| Level | Means | Evidence required |
| --- | --- | --- |
| **Dialect** | Emits DDL for the shared schema. | The golden DDL tests, and `conformance::check_dialect`. |
| **Schema** | The engine itself has executed that DDL. | A transcript against that engine's own server: the script applied cleanly, applied *again* cleanly, and the append-only tables were observed refusing `UPDATE` and `DELETE` **with a row present**. |
| **Store** | Implements `Store` against a real database. | `conformance::run` passing against that engine. |
| **Verified** | Store level, run in CI against the engine's own server on every commit. | A CI job that provisions the engine and fails — not skips — without it. |
<!-- /shared: conformance-ladder -->

## Where each crate stands

| Crate | Level | Why not higher |
| --- | --- | --- |
| `openehr-sqlite` | **Verified** | — |
| `openehr-postgresql` | **Schema** | No driver, no `Store`. |
| `openehr-mysql` | **Schema** | as above |
| `openehr-mariadb` | **Schema** | as above |
| `openehr-mssql` | **Dialect** | No server has parsed it — SQL Server 2022 segfaults under qemu on arm64. |
| `openehr-oracle` | **Dialect** | No server has parsed it — the images need registry authentication (`M14.7`). |

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

Add a branch if your engine has none. Then, **in the same commit**, change the
level in its one owner —
[`spec/databases/conformance-matrix.md`](../spec/databases/conformance-matrix.md)
(`W0.40`) — and in every document that restates it: the crate's README, its
crate-level rustdoc, `openehr-store/spec/conformance.md`, `spec/index.md`,
`README.md`, `CLAUDE.md`, and `AGENTS.md`.

You do not have to find them all by hand. `python3 scripts/check-docs.py` names
every restatement that disagrees with the owner, and the `claims` job runs it, so
a half-finished promotion fails CI rather than shipping as a crate that claims
two different levels in two places.

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
