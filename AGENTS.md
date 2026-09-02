# AGENTS.md

Operational guidance for anyone — human or agent — working in this repository.

**This file is not normative.** It describes how to work; the specifications
decide what must be true. Where this file and a specification disagree, the
specification governs and this file has a defect (`W0.2`). The specifications are:

- [`spec/index.md`](spec/index.md) — the repository: crate map, identifier
  namespaces, the conformance ladder, publishing.
- [`spec/databases/`](spec/databases/index.md) — storing openEHR in SQL.
- [`openehr/spec/`](openehr/spec/index.md) — the Reference Model library.

Detailed topic guides live in [`agents/`](agents/index.md). The same
orientation, condensed, ships as a Claude Code Skill at
[`openehr-rust-maintainer-skill/SKILL.md`](openehr-rust-maintainer-skill/SKILL.md),
for a session or tool that has not loaded this file.
[`openehr-skill/SKILL.md`](openehr-skill/SKILL.md) is a different skill, for
openEHR concepts rather than this repository's own conventions.

## What this repository is

Eighteen crates implementing openEHR in Rust: one Reference Model library, one
engine-agnostic persistence library, six SQL engine crates, an HTTP service, an
asset generator, and eight fuzz harnesses. The first eight are published to
crates.io at 0.9.0; the other ten are `publish = false`.

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
| `openehr-loco` | HTTP service on Axum and Loco; verifies PASETO `v4.public` | outside the ladder (`W0.32`) |
| `openehr-assets` | regenerates `assets/`, and fails the build on a stale one | not published |
| `openehr-fuzz`, `openehr-<engine>-fuzz` × 6 | fuzz harnesses | not published |

`openehr-loco` states **evidence** rather than a level: every rung on the ladder
is defined by DDL, a `Store`, or a database server, and a service crate is none
of those (`W0.32`).

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
# `RUSTFLAGS="-D warnings"` is what CI sets, and the difference is not
# theoretical: a lint that fires only under it passed locally and failed in CI.
cd openehr && RUSTFLAGS="-D warnings" cargo clippy --all-targets

# Every buildable crate. The list is ten, not eight: `openehr-loco` and
# `openehr-assets` build and test like the rest, and `openehr-assets` spent its
# whole life outside every matrix with nine tests nobody ran (`spec/audit.md`
# W-12).
for d in openehr openehr-store openehr-sqlite openehr-postgresql \
         openehr-mysql openehr-mariadb openehr-mssql openehr-oracle \
         openehr-loco openehr-assets; do
  (cd "$d" && cargo test --quiet \
     && RUSTFLAGS="-D warnings" cargo clippy --all-targets --quiet) || echo "FAIL $d"
done

# The committed assets are what the code renders
(cd openehr-assets && cargo run -- check)

# The documentation's counts, versions, conformance levels, and shared blocks
python3 scripts/check-docs.py          # --fix rewrites copies from their owner

# Benchmarks. `--test` is the one-iteration form CI runs; bare `cargo bench`
# takes measurements and asserts nothing (`W0.34`, `W0.35`).
(cd openehr && cargo bench -- --test)
(cd openehr-store && cargo bench -- --test)
```

The MSRV is **N−2** — two Rust releases behind stable, currently 1.96 (raised
from N−3 on 2026-08-29) — and the `msrv` job re-derives that number rather than
trusting a constant, so it goes red within six weeks of every Rust release. Fix
the number; do not pin the toolchain. See
[`spec/rust-msrv-n-minus-2/index.md`](spec/rust-msrv-n-minus-2/index.md).

Lints are `deny`, not `warn`: `missing_docs`, `missing_errors_doc`,
`missing_panics_doc`; `unsafe_code` is `forbid`. Clippy runs at `pedantic`. The
tree is currently at **zero warnings** — keep it there.

`unsafe_code` is forbidden **twice over, in all eighteen crates**:
`[lints.rust]` in every manifest, and `#![forbid(unsafe_code)]` at every crate
root and every fuzz target (32 files). Both were completed on 2026-08-26, and
neither is redundant:

- The **manifest** covers files nobody has written yet. A fuzz target added
  tomorrow is forbidden `unsafe` whether or not its author types the attribute —
  an attribute-only guard is a guard only as wide as its file list, which is the
  shape that has already bitten this repository twice.
- The **attribute** survives a manifest edit and is visible in the file it
  protects, which a `Cargo.toml` line is not.

Until that date the eight fuzz crates had **no `[lints]` table at all**, so
`unsafe_code` was forbidden in none of the 21 fuzz targets while this file said
the tree forbids it. No `unsafe` was present — the claim was true of the code
and false of the configuration.

### Verifying a dialect against a real engine

This is what separates conformance level **Dialect** from **Schema**, and it has
found a defect in every crate it has been run against — three of three.

```sh
sh openehr-store/scripts/verify-schema.sh postgresql   # PostgreSQL 18
sh openehr-store/scripts/verify-schema.sh mysql        # MySQL 8.4
sh openehr-store/scripts/verify-schema.sh mariadb      # MariaDB 11.4
```

Requires `podman` (or `docker` via `$CONTAINER`), which is also what CI uses. It
provisions the engine, runs
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
| `test` | clippy, tests, and docs for each of the ten buildable crates separately — one `--workspace` invocation would silently miss one, since each crate is its own workspace. `openehr-assets` was absent from this list and its nine tests had never run (`spec/audit.md` **W-12**) |
| `msrv` | derives N−2 from the stable toolchain it just installed, checks every manifest and document declares exactly that, then **builds and tests on it**. See [`spec/rust-msrv-n-minus-2/index.md`](spec/rust-msrv-n-minus-2/index.md); this job is expected to go red within six weeks of every Rust release, and that is the point |
| `examples` | the five runnable tutorials in `openehr`, plus the persistence tutorial in `openehr-sqlite` |
| `bench` | `cargo bench -- --test`: every criterion benchmark runs once. Nothing is gated on wall-clock (`W0.35`) — a threshold on a shared runner fails for unrelated reasons and gets silenced |
| `schema` | `verify-schema.sh` against real PostgreSQL, MySQL, and MariaDB containers |
| `assets` | `openehr-assets` regenerates the committed DDL/schema files and fails the build if a committed one is stale |
| `fuzz` | a short regression run of every fuzz target — a crash, panic, or abort fails the build; this is a gate, not a campaign |
| `layering` | `openehr` and `openehr-store` depend inward only, including dev-dependencies. The crate list is **derived** from the tree, not written here: it used to name nine of seventeen and could not see a cycle through the eight it skipped (**W-13**) |
| `claims` | that mssql and oracle still claim only Dialect, that the library matrix covers every requirement exactly once, that the conformance matrix does not contradict itself, that the audit summary counts itself correctly, and that **all eighteen** crates declare the same five licences (**W-14**) |
| `trademarks` | `scripts/check-trademarks.py`: every root document and every published crate's rustdoc that uses the openEHR mark in prose carries the notice of professionalization rule 5 verbatim, non-affiliation sentence included |
| `mutants` | `cargo-mutants --in-diff` over the lines a **push or a pull request** changed, per crate touched. It was pull-request-only until 2026-08-21, so nine commits made straight to `main` bypassed it entirely while it reported `skipped` (**W-18**) — see [`agents/auditing.md`](agents/auditing.md) |

Two rules it follows, both from the specification rather than habit: the schema
jobs **fail rather than skip** without a container runtime (`C0.13`), and they
invoke the same script you run locally rather than a parallel implementation in
YAML — two ways of doing one check drift, and the one that drifts is always the
one nobody runs.

It first ran green on 2026-08-01, [run 30713623082](https://github.com/openehr-rust/openehr-rust/actions/runs/30713623082), across all nineteen jobs — which
is what closed **W-02**, rather than the commit that added the file. The `msrv`
and `bench` jobs and two more fuzz targets were added on 2026-08-20 and have
**not** yet run on `main`; every step of the `msrv` job was run locally,
including against deliberately broken inputs, which is evidence and is not the
same evidence (`W0.11`).
`openehr-sqlite` is now at **Verified**; no other crate is eligible, having no
`Store` ([`spec/audit.md`](spec/audit.md) **W-02**).

### Fuzzing

Eight fuzz crates, 21 targets. They need nightly and `cargo-fuzz`, and run from
the crate they fuzz, with `--fuzz-dir`:

```sh
cargo install cargo-fuzz
cd openehr-postgresql
cargo +nightly fuzz run --fuzz-dir ../openehr-postgresql-fuzz quote -- -max_total_time=60
```

| Crate | Targets | What is untrusted about the input |
| --- | --- | --- |
| `openehr-<engine>-fuzz` × 6 | `quote`, `col_sql` | An archetype id reaches a `WHERE` clause from caller input, so an identifier escaping its own quoting is SQL injection (`P6.12`). |
| `openehr-fuzz` | `iso8601`, `object_id`, `aql`, `path`, `canonical_json`, `uri`, `data_value` | These are the parsers that read documents from outside the process. |
| `openehr-store-fuzz` | `project`, `integrity` | Not parsers: a composition that passed no constructor becoming rows, and rows read back out of a database somebody may have edited. |

The **properties live in `openehr_store::conformance`** wherever more than one
crate drives them (`W0.38`, `W0.26`); a dialect fuzz target is a thin call. Six
copies of one assertion is the arrangement that produced W-01.

Seed corpora are committed and inputs the fuzzer discovers are not — and the
**corpora themselves are checked**, by `openehr/tests/fuzz_seeds.rs`, because a
seed that quietly stopped deserializing is an unseeded target wearing a corpus
and no fuzz run's output would say so (**W-15**).

CI runs every target on every push, because a committed fuzz target nobody
executes is a claim rather than a check (`T11.9`). One of them has a result:
`uri` was written against the code as it stood and found `lib:A-36` — a panic
reachable from any JSON document — from an empty corpus.

None of these crates is published (`publish = false`).

### Benchmarks

`openehr/benches/rm.rs` and `openehr-store/benches/store.rs`, on criterion.

```sh
cd openehr && cargo bench              # measure
cd openehr && cargo bench -- --test    # one iteration each; what CI runs
```

**A number from these is not a conformance claim** (`W0.34`) and **nothing is
gated on wall-clock** (`W0.35`). No requirement in this repository is stated in
seconds. What CI asserts is the only thing it honestly can — that the benchmarks
still compile and still run (`W0.36`), which is the part that rots.

## Adding an engine crate

Read [`agents/adding-an-engine.md`](agents/adding-an-engine.md) first. The short
version:

1. A dialect owns **four** things: type spellings, identifier quoting,
   placeholder style, append-only enforcement. Nothing else.
2. Implement `Dialect` from scratch. Do not copy a sibling crate.
3. Add it to `openehr-sqlite/tests/dialects.rs` — both the `all()` list **and**
   the `ENGINE_CRATES` count — and to that crate's dev-dependencies.
4. Add a branch to `verify-schema.sh` and run it. Until you have, the crate is at
   **Dialect** and its documentation must say so.
5. Write the dialect annex (`X15.6`) — `spec/14-<engine>-dialect.md`, covering
   the nine subjects it names. All six existing crates have one; do not be the
   exception.

## Documentation rules

- State the crate's conformance level in the **first screenful** of its README
  and its crate docs (`C0.9`). The level has **one owner**,
  [`spec/databases/conformance-matrix.md`](spec/databases/conformance-matrix.md)
  (`W0.40`); change it there first, and `scripts/check-docs.py` will name every
  restatement that has not caught up.
- Never describe a capability at a level above the crate's (`C0.11`).
- When you fix something that was wrong in public, say what was wrong. The
  `openehr-mariadb` README documents its own history because a corrected claim is
  only meaningful against the claim it corrects.
- Rustdoc examples are compiled and run. A `no_run` or `ignore` example is a
  claim nothing checks.
- **A count is a claim** (`W0.39`). How many crates, how many are published, how
  many fuzz targets, which version is live, which CI jobs exist — all of it is
  derived from the tree by `scripts/check-docs.py` and checked against the prose.
  Two findings here were nothing but stale counts (**W-10**, **W-11**).
- **Do not paste a passage into a second document unmarked** (`W0.38`). If it
  genuinely belongs in both, mark one `<!-- shared: NAME (owner) -->` and the
  rest `(copy)`, and they are compared byte for byte. The conformance ladder was
  copied four times unmarked, and two of the four had drifted (**W-16**).

## Publishing

All eight publishable crates are live on crates.io at **0.9.0**, published
2026-09-02, and **local matches published** — the per-crate table is in
[`agents/publishing.md`](agents/publishing.md), which is the one file to trust
on this and the one to update first.

Each of the last three bumped the minor rather than the patch, for one reason:
cargo treats `0.x.y` as compatible within `0.x`, so a patch reaches a dependent
on `cargo update` unasked. 0.4.0 removed `PartialOrd` from every `DV_ORDERED`
(`lib:A-35`) and moved the MSRV to N−3; 0.5.0 changed what a rendered AQL query
looks like (`lib:Q12.9e`); 0.6.0 changed what a canonical `DV_QUANTITY` looks
like, because its magnitude now keeps the digits it was written with
(`lib:D3.18d`). None of the three breaks a signature and all three change bytes
somebody may be comparing.

There is no schema migration and there will not be one before 1.0
(`db:O10.14`). A deployment on a published 0.2.0 schema exports, recreates, and
reloads; 0.3.0 → 0.4.0 changed no DDL, so that step is not repeated.
Read [`agents/publishing.md`](agents/publishing.md) before publishing again.

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
agents/                   topic guides
scripts/check-docs.py     counts, versions, levels, shared blocks
openehr/                  the Reference Model library
  spec/                   library specification (lib:) + audit + matrix
  src/{rm,base,security}/ the model, identifiers, change-control security
  examples/               five runnable tutorials (the Reference Model)
  benches/rm.rs           criterion; not a conformance claim (W0.34)
  tests/fuzz_seeds.rs     the committed seed corpora still parse (W0.30)
openehr-store/            engine-agnostic persistence
  src/{schema,dialect,record,store,conformance}.rs
  benches/store.rs        projection and chain verification
  spec/conformance.md     marked copy of the ladder; C0.8 owns it
  scripts/verify-schema.sh
openehr-<engine>/         one Dialect each; sqlite also has a Store
  spec/14-<engine>-dialect.md  that dialect's annex and its M14.x departures
openehr-sqlite/examples/01_store_a_record.rs
                          the persistence tutorial; the only crate with a Store
openehr-fuzz/             fuzz harness for the RM parsers; publish = false
openehr-store-fuzz/       projection and integrity; publish = false
openehr-<engine>-fuzz/    fuzz harness per dialect; publish = false
.github/workflows/ci.yml  test, examples, schema, fuzz, claims
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
