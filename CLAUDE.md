# CLAUDE.md

Guidance for Claude Code working in this repository.

Read [`AGENTS.md`](AGENTS.md) for the full operational guide and
[`AGENTS/`](AGENTS/index.md) for topic guides. This file is the short version
plus the things that specifically trip up automated work here.

**Not normative.** The specifications decide what must be true (`W0.2`):
[`spec/index.md`](spec/index.md), [`spec/databases/`](spec/databases/index.md),
[`openehr/spec/`](openehr/spec/index.md).

## Layout

Fifteen crates, **each its own Cargo workspace**. There is no root workspace —
run cargo from inside a crate directory. Eight are published at 0.1.1; the seven
fuzz harnesses (`openehr-fuzz` and six `openehr-<engine>-fuzz`) are
`publish = false`.

| Crate | Role | Level |
| --- | --- | --- |
| `openehr` | RM types, validation, paths, AQL, security | library |
| `openehr-store` | schema, projection, commit rules, conformance suite | library |
| `openehr-sqlite` | SQLite dialect **and a store** | **Verified** |
| `openehr-postgresql` | PostgreSQL 18 dialect | **Schema** |
| `openehr-mysql` | MySQL 8.4 dialect | **Schema** |
| `openehr-mariadb` | MariaDB 11.4 dialect | **Schema** |
| `openehr-mssql` | SQL Server dialect | **Dialect** |
| `openehr-oracle` | Oracle dialect | **Dialect** |
| `openehr-fuzz` | fuzz harness for the RM parsers | not published |
| `openehr-<engine>-fuzz` × 6 | dialect fuzz harnesses | not published |

## Commands

```sh
cd <crate> && cargo test
cd <crate> && cargo clippy --all-targets

# everything
for d in openehr openehr-store openehr-sqlite openehr-postgresql \
         openehr-mysql openehr-mariadb openehr-mssql openehr-oracle; do
  (cd "$d" && cargo test --quiet && cargo clippy --all-targets --quiet) \
    || echo "FAIL $d"
done

# verify a dialect against a real engine (needs podman or docker)
sh openehr-store/scripts/verify-schema.sh postgresql|mysql|mariadb
```

Lints are `deny` (`missing_docs`, `missing_errors_doc`, `missing_panics_doc`),
`unsafe_code` is `forbid`, clippy runs at `pedantic`. **The tree is at zero
warnings — keep it there.**

## The five rules

1. **Never claim more than is verified** (`W0.3`). "The same code path works
   elsewhere" is not evidence. Every finding in
   [`spec/audit.md`](spec/audit.md) is a version of this.
2. **A gap not written down reads as a pass** (`W0.4`). Found something wrong and
   can't fix it now? Add it to the audit register.
3. **A guard is only as wide as its input list.** The cross-dialect check
   compared five of six dialects, and the sixth was a copy of another.
4. **Never create a crate by copying a sibling.** That is exactly how
   `openehr-mariadb` became `openehr-mysql` under another name.
5. **Specification first** (`W0.19`). Write the requirement down before the
   commit lands.

## Things that will trip you up

- **`spec/databases/` was rewritten on 2026-08-01** from an imported FHIR
  specification. Withdrawn requirements keep their numbers in a table at the foot
  of each section — a citation to `M3.4` resolves to "withdrawn: shredding", not
  to nothing. New requirements start after the highest previously used ordinal,
  so §3 begins at `M3.19`. Do not renumber.
- **Two spec trees allocate the same identifiers.** `lib:S1.4` (no Archetype
  Model) and `db:S1.4` (declare an engine floor) are different requirements.
  Qualify citations with `lib:` or `db:` (`W0.5`).
- **`openehr` is already on crates.io at 0.1.0**, published with a wrong
  `repository` field that is now immutable. Local versions are 0.1.1. Read
  [`AGENTS/publishing.md`](AGENTS/publishing.md) before any publish.
- **CI is green and `openehr-sqlite` is at Verified.** Every other crate is at
  Schema or Dialect and must not be promoted without evidence. Do not write text
  implying more continuity than a job actually provides; a specification here
  once claimed a workflow file that never existed.
- **`ColTy` is deliberately not `#[non_exhaustive]`.** Adding a variant *should*
  break all six dialects at compile time. Do not add a `_` arm to silence it —
  that is how one engine silently acquires another's types.
- **`Cargo.lock` is committed** in every crate, unusually for libraries. Leave it.
- **Fuzz properties live in `openehr_store::conformance`,** not in the fuzz
  crates. A target is a thin call. Do not inline an assertion into six crates.
- **Each engine crate has a dialect annex** at `spec/14-<engine>-dialect.md`.
  A departure from a core requirement goes there as a numbered `M14.x`, not in a
  code comment — `C0.16` calls an undeclared departure a defect.
- **Rustdoc examples are compiled and run.** Do not add `no_run` or `ignore` to
  make one pass; that turns a checked claim into an unchecked one.
- **Times are two columns.** `…_text` is authoritative and exact; `…_utc` is
  derived and nullable. Never collapse them, and never make the derived column
  non-nullable — `2024-05` is a date known to the month, not `2024-05-01`.

## Before saying something works

Run it. Every finding in this repository's audit register was found by running
something; none by reading. In particular:

```sh
# two dialects hashing the same means one is a copy of the other
for e in postgresql sqlite mysql mariadb mssql oracle; do
  printf '%-12s ' "$e"
  cargo run -q --manifest-path "openehr-$e/Cargo.toml" --example ddl | md5
done
```

## Style

Match the surrounding code, which is unusually heavily commented and deliberately
so: comments explain *why* a decision was made, often citing a requirement id or
naming the defect the decision prevents. When you change such code, update the
reasoning rather than deleting it. A requirement whose reason is unrecorded is one
that will be removed by someone who does not know what it was protecting.
