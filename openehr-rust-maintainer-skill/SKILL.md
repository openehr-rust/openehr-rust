---
name: openehr-rust-maintainer-skill
description: Technical implementation guide for maintainers working on this repository — layout, build commands, the five rules, and the traps that have already bitten someone. Use when working on any of the eighteen crates, their specs, or their documentation in this repository, or when asked how this repository works, how to build/test/publish a crate here, or what to check before claiming something works. For openEHR concepts and terminology rather than this repository's own engineering conventions, use openehr-skill instead.
---

# openehr-rust-maintainer-skill — how to work in this repository

This is the condensed, portable form of this repository's own operating
guide. Inside the repository, [`CLAUDE.md`](../CLAUDE.md) and
[`AGENTS.md`](../AGENTS.md) are loaded automatically and are the fuller,
authoritative version — read them, and [`agents/index.md`](../agents/index.md)
for topic guides, before this file if they are available. This file exists so
the same orientation travels to a session, tool, or reviewer that does not
have those loaded.

**Not normative.** The specifications decide what must be true: `spec/index.md`,
`spec/databases/index.md`, `openehr/spec/index.md`.

## What this is

Eighteen crates, **each its own Cargo workspace** — there is no root
workspace, so `cargo` runs from inside a crate directory, never from the repo
root. Eight are published to crates.io at 0.9.0 (`openehr`, `openehr-store`,
and the six dialect crates: `openehr-sqlite`, `openehr-postgresql`,
`openehr-mysql`, `openehr-mariadb`, `openehr-mssql`, `openehr-oracle`); the
other ten (`openehr-loco`, `openehr-assets`, and eight fuzz harnesses) are
`publish = false`. `openehr` is the Reference Model, paths, AQL, and RM
validation. `openehr-store` is everything about persistence that is not a SQL
spelling. `openehr-sqlite` is the only crate at **Verified** conformance
(schema, `Store`, and a passing conformance suite); every other engine crate
is at **Schema** or **Dialect** and must not be described as more.

## Before you touch anything: five rules

1. **Never claim more than is verified.** "The same code path works
   elsewhere" is not evidence. Run the test; read the CI run
   (`gh run list`); don't infer.
2. **A gap not written down reads as a pass.** Found something wrong and
   can't fix it now? File it in the relevant audit register
   (`spec/audit.md`, `openehr/spec/audit.md`, `spec/databases/audit.md`)
   rather than leaving it implicit.
3. **A guard is only as wide as its input list.** A check that enumerates
   cases by hand silently stops covering a case nobody added to the list.
   Derive from the tree where you can.
4. **Never create a crate by copying a sibling.** Two engine crates in this
   repo's history have drifted apart after being started that way.
5. **Specification first.** Behavior lives in `spec/`, `openehr/spec/`, and
   `spec/databases/` before it is implemented. Requirement ids are permanent
   and never renumbered, even when withdrawn.

## Commands

```sh
cd <crate> && cargo test
cd <crate> && RUSTFLAGS="-D warnings" cargo clippy --all-targets   # CI's flag; without it a lint can pass locally and fail in CI

# regenerate the committed assets, or fail if what is committed is stale
(cd openehr-assets && cargo run -- write)   # or: -- check

# the documentation's counts, versions, levels, and shared blocks
python3 scripts/check-docs.py               # or: --fix

# verify a dialect against a real engine (needs podman or docker)
sh openehr-store/scripts/verify-schema.sh postgresql|mysql|mariadb
```

Lints are `deny` (`missing_docs`, `missing_errors_doc`, `missing_panics_doc`),
`unsafe_code` is `forbid`, clippy runs at `pedantic`, and the tree is kept at
zero warnings.

## Traps that have already bitten someone here

- **`spec/databases/` was rewritten 2026-08-01.** Withdrawn requirements keep
  their numbers, marked, at the foot of each section — a citation still
  resolves. Do not renumber.
- **Two spec trees allocate the same ids.** `lib:S1.4` and `db:S1.4` are
  different requirements. Qualify citations with `lib:` or `db:`.
- **A published version is immutable.** Read
  [`../agents/publishing.md`](../agents/publishing.md) before any publish or
  version bump — it is the only file that tracks the current version.
- **`ColTy` is deliberately not `#[non_exhaustive]`.** A new column type
  *should* break every dialect at compile time. Never add a `_` arm to
  silence it.
- **A constructor validates; `Deserialize` does not.** Anything arriving as
  JSON must be run through `validate()` — a derived `Deserialize` writes
  fields straight in, invariants unchecked. No accessor may rely on a
  constructor's guarantee, because a deserialized value never went through
  one.
- **A path that resolves to nothing is not an error** — it is an empty
  match, deliberately, so a wrong attribute name is silently indistinguishable
  from "no data here" unless you test the attribute exists.
- **A duplicated passage is either marked or forbidden.** If the same prose
  needs to live in two documents, mark the copy
  (`<!-- shared: name (copy) -->`) so `scripts/check-docs.py` checks it byte
  for byte — don't let two copies quietly drift.
- **Read the last CI run before believing a claim about CI** (`gh run list`).
  A red job on the default branch is as visible as a signal gets, and it has
  been missed here before.
- **The mutation job re-checks only the lines a push carried**
  (`event.before..HEAD`). A survivor in earlier code is never seen again by
  CI; before pushing, run `cargo mutants --in-diff` over the exact range
  and `--re '<function>'` over any function you touched. A `TIMEOUT` is a
  hang, and counts as a survivor.
- **An unstated `occurrences` is `None`, never a default** (`lib:A-71`,
  `K15.32`). Every `C_OBJECT` constructor takes an `Option`; the effective
  value comes from `CObject::effective_occurrences(owner)`, and a parser must
  not fill it in.
- **The archetype corpus is never vendored** — `openEHR/adl-archetypes`
  carries no licence. `openehr/tests/adl_corpus.rs` reads it from
  `OPENEHR_ADL_CORPUS`; results, with the corpus commit and date, go in
  `openehr/spec/corpus.md`.

## Where to go deeper

- [`../CLAUDE.md`](../CLAUDE.md) — the short version, plus this repository's
  own trip-ups in full.
- [`../AGENTS.md`](../AGENTS.md) — the full operational guide.
- [`../agents/index.md`](../agents/index.md) — topic guides: adding an engine,
  conformance levels, publishing, openEHR concepts for a FHIR-background
  reader, and auditing.
- [`../spec/index.md`](../spec/index.md), [`../openehr/spec/index.md`](../openehr/spec/index.md),
  [`../spec/databases/index.md`](../spec/databases/index.md) — the
  specifications themselves, which decide what must be true.
- [`../spec/audit.md`](../spec/audit.md), [`../openehr/spec/audit.md`](../openehr/spec/audit.md),
  [`../spec/databases/audit.md`](../spec/databases/audit.md) — what is known
  to be wrong right now.
