# AGENTS.md

Operational guidance for anyone — human or agent — working in this repository.

**This file is not normative.** It describes how to work; the specifications
decide what must be true. Where this file and a specification disagree, the
specification governs and this file has a defect (`W0.2`). The specifications are:

- [`spec/index.md`](spec/index.md) — the repository: crate map, identifier
  namespaces, the conformance ladder, publishing.
- [`spec/databases/`](spec/databases/index.md) — storing openEHR in SQL.
- [`openehr/spec/`](openehr/spec/index.md) — the Reference Model library.

Detailed topic guides live in [`AGENTS/`](AGENTS/index.md).

## What this repository is

Eight crates implementing openEHR in Rust: one Reference Model library, one
engine-agnostic persistence library, and six SQL engine crates.

| Crate | Role | Level |
| --- | --- | --- |
| `openehr` | RM types, validation, paths, AQL, security | library |
| `openehr-store` | schema, projection, commit rules, conformance suite | library |
| `openehr-sqlite` | SQLite dialect **and a store** | **Store** |
| `openehr-postgresql` | PostgreSQL 18 dialect | **Schema** |
| `openehr-mysql` | MySQL 8.4 dialect | **Schema** |
| `openehr-mariadb` | MariaDB 11.4 dialect | **Schema** |
| `openehr-mssql` | SQL Server dialect | **Dialect** |
| `openehr-oracle` | Oracle dialect | **Dialect** |

Each crate is **its own Cargo workspace**. There is no root workspace. Run cargo
from inside a crate directory.

## The rules that matter most here

These are the ones this repository has broken, not a generic list.

1. **Never claim more than is verified** (`W0.3`). "The same code path works
   elsewhere" is not evidence. Every finding in
   [`spec/audit.md`](spec/audit.md) is a version of this.
2. **A gap that is not written down reads as a pass** (`W0.4`). If you find
   something wrong and cannot fix it now, add it to the audit register. Do not
   leave it for the next reader to rediscover.
3. **A guard is only as wide as its input list.** The cross-dialect comparison
   that exists to catch copied dialects compared five of six, and the sixth was a
   copy. When you add a guard, add a check that the guard covers everything.
4. **Do not copy a crate to make a new one.** That is how `openehr-mariadb`
   became `openehr-mysql` under another name. Start from the trait and implement
   the four things a dialect owns.
5. **Specification first** (`W0.19`). Discovering a requirement while
   implementing is normal; write it down before the commit lands.

## Build, test, verify

```sh
# One crate
cd openehr && cargo test
cd openehr && cargo clippy --all-targets

# Every crate
for d in openehr openehr-store openehr-sqlite openehr-postgresql \
         openehr-mysql openehr-mariadb openehr-mssql openehr-oracle; do
  (cd "$d" && cargo test --quiet && cargo clippy --all-targets --quiet) || echo "FAIL $d"
done
```

Lints are `deny`, not `warn`: `missing_docs`, `missing_errors_doc`,
`missing_panics_doc`; `unsafe_code` is `forbid`. Clippy runs at `pedantic`. The
tree is currently at **zero warnings** — keep it there.

### Verifying a dialect against a real engine

This is what separates conformance level **Dialect** from **Schema**, and it has
found a defect in every crate it has been run against — three of three.

```sh
sh openehr-store/scripts/verify-schema.sh postgresql   # PostgreSQL 18
sh openehr-store/scripts/verify-schema.sh mysql        # MySQL 8.4
sh openehr-store/scripts/verify-schema.sh mariadb      # MariaDB 11.4
```

Requires `podman` (or `docker` via `$CONTAINER`). It provisions the engine, runs
the generated DDL, runs it **again** to prove idempotence, seeds a row, and then
checks the append-only tables refuse `UPDATE` and `DELETE` **with that row
present**. The row matters: a `FOR EACH ROW` trigger on an empty table never
fires, so a check on zero rows reports a refusal it never performed.

SQL Server and Oracle have no branch: SQL Server 2022 segfaults under qemu on
arm64, and the Oracle images need registry authentication. Both crates stay at
**Dialect** until someone runs them somewhere they work.

### CI

`.github/workflows/ci.yml` runs on every push and pull request:

| Job | Covers |
| --- | --- |
| `test` | clippy, tests, and docs for each of the eight crates separately — one `--workspace` invocation would silently miss one, since each crate is its own workspace |
| `examples` | the five runnable tutorials |
| `schema` | `verify-schema.sh` against real PostgreSQL, MySQL, and MariaDB containers |
| `claims` | that mssql and oracle still claim only Dialect, and that all eight crates declare the same five licences |

Two rules it follows, both from the specification rather than habit: the schema
jobs **fail rather than skip** without a container runtime (`C0.13`), and they
invoke the same script you run locally rather than a parallel implementation in
YAML — two ways of doing one check drift, and the one that drifts is always the
one nobody runs.

**It has not run yet.** It was added 2026-08-01 and cannot execute until pushed,
so no crate has been promoted to **Verified**, and none may be until a green run
on `main`. Treating a committed workflow as a working one is the same error the
finding it closes was about ([`spec/audit.md`](spec/audit.md) **W-02**).

## Adding an engine crate

Read [`AGENTS/adding-an-engine.md`](AGENTS/adding-an-engine.md) first. The short
version:

1. A dialect owns **four** things: type spellings, identifier quoting,
   placeholder style, append-only enforcement. Nothing else.
2. Implement `Dialect` from scratch. Do not copy a sibling crate.
3. Add it to `openehr-sqlite/tests/dialects.rs` — both the `all()` list **and**
   the `ENGINE_CRATES` count — and to that crate's dev-dependencies.
4. Add a branch to `verify-schema.sh` and run it. Until you have, the crate is at
   **Dialect** and its documentation must say so.
5. Write the dialect annex (`X15.6`). None of the six has one yet; do not make it
   seven.

## Documentation rules

- State the crate's conformance level in the **first screenful** of its README
  and its crate docs (`C0.9`).
- Never describe a capability at a level above the crate's (`C0.11`).
- When you fix something that was wrong in public, say what was wrong. The
  `openehr-mariadb` README documents its own history because a corrected claim is
  only meaningful against the claim it corrects.
- Rustdoc examples are compiled and run. A `no_run` or `ignore` example is a
  claim nothing checks.

## Publishing

The goal is all eight crates on crates.io. Read
[`AGENTS/publishing.md`](AGENTS/publishing.md) before doing it.

A published version is **immutable**. `openehr` 0.1.0 is already live carrying a
`repository` field pointing at an unrelated project; that cannot be fixed, only
superseded. Treat every conformance claim in a crate's docs as permanent the
moment you publish, and do not publish a crate with an open finding against its
claims (`W0.21`).

## Where things are

```
spec/                     repository specification + audit register
  index.md                crate map, id namespaces, ladder, publishing
  audit.md                repository-wide findings (W-xx)
  databases/              persistence specification (db:)
AGENTS.md                 this file
AGENTS/                   topic guides
openehr/                  the Reference Model library
  spec/                   library specification (lib:) + audit + matrix
  src/{rm,base,security}/ the model, identifiers, change-control security
  examples/               five runnable tutorials
openehr-store/            engine-agnostic persistence
  src/{schema,dialect,record,store,conformance}.rs
  scripts/verify-schema.sh
openehr-<engine>/         one Dialect each; sqlite also has a Store
```

## Things that will surprise you

- **`openehr` is a workspace of one.** It shares no code with the persistence
  crates and deliberately does not depend on them.
- **`Cargo.lock` is committed in every crate**, unusually for libraries. It is
  what makes "the tests passed" and "the audit ran against these versions" mean
  the same thing twice.
- **`ColTy` is deliberately not `#[non_exhaustive]`.** Adding a variant *should*
  break all six dialects at compile time, so each decides its own spelling. A `_`
  arm is how one engine silently acquires another's types.
- **Two `spec/` trees allocate the same identifiers.** `lib:S1.4` and `db:S1.4`
  are different requirements. Qualify citations (`W0.5`).
- **`spec/databases/` was rewritten on 2026-08-01** from an imported FHIR
  specification. Withdrawn requirements keep their numbers in a table at the foot
  of each section, so a citation to `M3.4` resolves to "withdrawn: shredding"
  rather than to nothing, and new requirements start after the highest previously
  used ordinal (§3 begins at `M3.19`). Do not renumber (`C0.5`).
