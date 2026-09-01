# Publishing to crates.io

Not normative; `W0.20`–`W0.21` in [`spec/index.md`](../spec/index.md) are.

## Read this first

**A published version is immutable.** It cannot be edited, and yanking does not
change its metadata — a yanked version keeps its `repository`, its description,
and every conformance claim in its documentation.

This has already cost something here. `openehr` 0.1.0 was published on
2026-07-31 carrying `repository = ".../fhir-databases"`, pointing at an unrelated
project. That field is permanent for 0.1.0. The remedy was to publish 0.1.1 and
let it become the version people see, which is what happened — and 0.1.0 is
still there, still pointing at the wrong project, and always will be.

So: **a conformance claim becomes permanent the moment you publish it.** Do not
publish a crate with an open finding against its claims (`W0.21`).

## State today

**Published 2026-08-29.** All eight publishable crates are live at **0.8.0**
on crates.io, in the order below, and local matches published. Verified
against the registry API rather than read off `cargo publish`'s output.

**`openehr-loco` became publishable on 2026-09-01.**
`spec/databases/16-repository-and-release.md`'s `W16.1` was amended to
require it, `publish = false` was removed from its manifest, and
`cargo publish --dry-run` was run for real — it packages, downloads its three
already-published path dependencies (`openehr`, `openehr-store`,
`openehr-sqlite`) at their real crates.io versions, builds against them, and
would upload. **It has not actually been published.** That is deliberate:
publishing is the manual step below, from the maintainer's own machine, and
a specification change is not that step. Its version is already `0.8.0`,
matching the other eight for consistency, so its first release will not be
`0.1.0` — there is no earlier `0.x` history for it to follow.

**0.7.2 went out ahead of this file's process** (2026-08-26): the versions
were bumped and the eight crates published before the inter-crate pins, this
file's staged state, `CITATION.cff`, or the changelog moved, and before the
description shape was final. What 0.7.2 carries — immutably — is the notice
verbatim in every `description` but not the canonical three-part shape: the
closing "This project is an independent work." sentence is absent from all
eight, and `openehr-mysql`'s runs "DDL" straight into "openEHR®" with no
full stop. 0.7.3 is the remedy, exactly as 0.1.1 was for 0.1.0's wrong
`repository`.

**0.8.0 is the MSRV release.** `RV6` forbids raising the floor as a patch, so
this went out as a minor bump, 0.7.4 to 0.8.0, over a change that touched no
public API — `rust-version` in eighteen manifests, `spec/rust-msrv-n-minus-2/
index.md` replacing the N−3 document, and the `msrv` CI job's own derivation.
CI ran green on `29fd23f`, the commit that cut it, before publishing —
`cargo +1.96 test --all-features` had already been run for real across all
ten buildable crates in that same change, not only declared.

**The gate earned its keep again on 0.7.0, and harder.** CI on `4f6e418` — the
commit that cut it — went **red**: the `mutants` job reported **43 of 147
mutants surviving**, every one of them in the new `openehr::am` module. Nearly
all were accessors no test asserted, so `CAttribute::rm_attribute_name` could
have returned `""` and the suite stayed green. Four were logic, and one of those
would have refused **every archetype whose children exactly fill their
container** — two mandatory elements under a `0..2` container, which is what a
blood pressure looks like. Three tests later, `10ef34d` ran 32 jobs green and
0.7.0 went out from there. A published version is immutable; that defect would
have been.

**The gate earned its keep on this one.** CI on the commit before the bump
**failed**: the `mutants` job caught `DvQuantity::accuracy_real -> None`
surviving — an accessor added with the `Real` migration that nothing called.
Every other check was green, including `real.rs` mutating clean, because
`real.rs` was the file chosen by hand and `quantity.rs` was where the migration
landed. Without the gate that would have gone to crates.io, where it is
immutable.

Two releases went out on this date, 0.4.0 and 0.5.0, each after CI ran green on
the commit that carried it — `adcfaae` and `adc0e4b`. That gate is not a
formality: the run before 0.4.0's was **red**, and reading it is what produced
`lib:A-37`.

> **The gate that held 0.4.0, kept as the record of why.** CI ran green on
> **`adcfaae`**: 28 jobs succeeded and one — `mutants` — was skipped because it
> is pull-request only. 0.4.0 went out after that run and not before.
>
> The gate said: do not publish until the `msrv` job, the `bench` jobs, the two
> new fuzz matrix rows, the tenth crate in the `test` matrix, and the rewritten
> `layering` and `claims` guards had all actually run. They have now, all of
> them, together. `W0.11` is the rule that a committed workflow is not a working
> one, and a published version is immutable — 0.4.0's documentation describes
> those checks, so their first green run had to precede it.
>
> The run before it, `decd78b`, was **red**: `fuzz / openehr` had been failing
> since 2026-08-04 (`lib:A-37`) and `test / openehr-loco` failed a lint that
> only fires under `RUSTFLAGS="-D warnings"`. Both are fixed in `adcfaae`.

| Crate | crates.io | Local |
| --- | --- | --- |
| `openehr` | 0.1.0, 0.1.1, 0.2.0, 0.3.0, 0.4.0, 0.5.0, 0.6.0, 0.7.0, 0.7.1, 0.7.2, 0.7.3, 0.7.4, **0.8.0** | 0.8.0 |
| `openehr-store` | 0.1.1, 0.2.0, 0.3.0, 0.4.0, 0.5.0, 0.6.0, 0.7.0, 0.7.1, 0.7.2, 0.7.3, 0.7.4, **0.8.0** | 0.8.0 |
| `openehr-sqlite` | 0.1.1, 0.2.0, 0.3.0, 0.4.0, 0.5.0, 0.6.0, 0.7.0, 0.7.1, 0.7.2, 0.7.3, 0.7.4, **0.8.0** | 0.8.0 |
| `openehr-postgresql` | 0.1.1, 0.2.0, 0.3.0, 0.4.0, 0.5.0, 0.6.0, 0.7.0, 0.7.1, 0.7.2, 0.7.3, 0.7.4, **0.8.0** | 0.8.0 |
| `openehr-mysql` | 0.1.1, 0.2.0, 0.3.0, 0.4.0, 0.5.0, 0.6.0, 0.7.0, 0.7.1, 0.7.2, 0.7.3, 0.7.4, **0.8.0** | 0.8.0 |
| `openehr-mariadb` | 0.1.1, 0.2.0, 0.3.0, 0.4.0, 0.5.0, 0.6.0, 0.7.0, 0.7.1, 0.7.2, 0.7.3, 0.7.4, **0.8.0** | 0.8.0 |
| `openehr-mssql` | 0.1.1, 0.2.0, 0.3.0, 0.4.0, 0.5.0, 0.6.0, 0.7.0, 0.7.1, 0.7.2, 0.7.3, 0.7.4, **0.8.0** | 0.8.0 |
| `openehr-oracle` | 0.1.1, 0.2.0, 0.3.0, 0.4.0, 0.5.0, 0.6.0, 0.7.0, 0.7.1, 0.7.2, 0.7.3, 0.7.4, **0.8.0** | 0.8.0 |

`openehr-assets` and the eight fuzz crates are `publish = false` and are not
on crates.io. `openehr-loco` is no longer in that set as of 2026-09-01 (above)
— it is not on crates.io either, but for a different reason: its manifest now
permits publishing, and nobody has run `cargo publish` for it yet.

This table was wrong until 2026-08-02, and again briefly during the 0.3.0
bump — it said seven of the eight were "not published" and that the next
version was 0.1.1, long after 0.2.0 had gone out, in the file that exists to
stop a bad publish. If this row disagrees with `git log` or with crates.io
itself, trust those, not this file.

## What 0.3.0 changed, for anyone still on 0.2.0

Published and local agree again as of 2026-08-04. This section is now a
historical record of what the 0.3.0 release changed, kept for anyone
upgrading from a published 0.2.0 — catalogued in full in
[`CHANGELOG.md`](../CHANGELOG.md), summarised here for the decision it drove:

| Change | Why it breaks |
| --- | --- |
| `SCHEMA_VERSION` exists, and is `4` | Did not exist at 0.2.0. A database written by published 0.2.0 records **no** version, and `install()` refuses a populated database with none rather than guessing (`db:O10.16`). |
| `ColTy::Json` moved off `jsonb` and MySQL `JSON` | `db:M3.43`, from `db:D-08`. A PostgreSQL or MySQL database created by 0.2.0 has a column of the wrong type, and its stored bytes are already normalised — reordered keys, and on MySQL `1.10` rewritten as `1.1`. |
| Nine chain columns on `openehr_version` | `db:D-07`. Absent at 0.2.0. |
| `ColTy::Digest` added | `ColTy` is deliberately not `#[non_exhaustive]`, so this breaks any external `Dialect` implementation at compile time — by design. |
| `OriginalVersion::new` refuses what it accepted | `lib:A-23`. A first version naming a predecessor, or a successor naming none, now fails to construct. |
| `Date`/`Time`/`DateTime`/`Duration` lost `PartialOrd`/`Ord` | `lib:A-32`. `Eq` on these was lexical while `PartialOrd` was semantic, contradicting the standard library's requirement that the two agree. `<`, `.partial_cmp()`, `.min()`/`.max()`, `sort()` on these four types no longer compile; call the new `.semantic_cmp()` instead. Did not touch `DvDate`/`DvTime`/`DvDateTime`/`DvDuration` at 0.3.0, whose own `PartialOrd` was unchanged **then**. That is history: `lib:A-35` removed `PartialOrd` from those four and from every other `DV_ORDERED` after 0.3.0 — see `CHANGELOG.md` under *Unreleased*. |

That is why the release was **0.3.0**, not 0.2.1: cargo treats `0.2.x` as
compatible with `0.2.0`, and shipping any of the above as a patch would have
broken a dependent on `cargo update`.

**There is no migration for the schema change and there will not be one before
1.0** (`db:O10.14`). A deployment on published 0.2.0 exports, recreates, and
reloads. Say that in the release notes rather than leaving it to be discovered.

## Tagging

- A release is tagged **`v<version>`**, annotated, on the `Record that <version>
  is published` commit — the one that records the release, not the one that cut
  it. The distinction matters: what a tag should point at is the tree that was
  verified against crates.io, and that is the later commit.

`v0.2.0` established both the name and the placement. `openehr-v0.3.0` then used
a per-crate name that was never continued — these eight crates are versioned in
lockstep and released together, so one tag per release is right and a per-crate
scheme would need eight. `v0.3.0` was added later as an alias at the same
commit; `openehr-v0.3.0` is left in place, because a published tag somebody may
have fetched is not worth deleting to tidy a name.

**0.4.0 and 0.5.0 went out untagged** and were tagged the following day, after
the question "are you tracking tasks" prompted a look at what had been missed.
Tag as part of publishing, not after it — a tag added later points at the right
commit only because the history was still legible.

```sh
git tag -a "v$V" "$(git log --format=%H -1 --grep="^Record that $V is published")" \
  -m "openEHR crates $V"
git push --tags
```

## Order

Dependencies must exist on crates.io before the crates that depend on them. Path
dependencies here all carry a `version`, so cargo will look them up.

```
1. openehr
2. openehr-store          (depends on openehr)
3. openehr-postgresql  ┐
   openehr-mysql       │  (depend on openehr + openehr-store;
   openehr-mariadb     │   independent of each other)
   openehr-mssql       │
   openehr-oracle      ┘
4. openehr-sqlite         (dev-depends on all five above)
5. openehr-loco           (depends on openehr + openehr-store + openehr-sqlite)
```

`openehr-sqlite` is fourth because its **dev-dependencies** name the other five
engine crates with versions, for the cross-dialect comparison. Dev-dependencies
are stripped from the published package's resolution, but `cargo publish`
verifies the package builds — including its tests — so the five must be
resolvable. Publish them first and the ordering problem disappears; there is no
cycle.

`openehr-loco` is fifth and last, added 2026-09-01: it is a normal path
dependency on the three ahead of it, no dev-dependency wrinkle of its own.
It is not one of the eight this file otherwise calls "the published crates"
in most places below — that phrase predates it and still means those eight —
but it is no longer `publish = false` either; treat it as its own,
not-yet-exercised, one-crate release.

**This ordering is not advisory — cargo enforces it.** Publish out of order and
a dependent fails with:

```
error: failed to prepare local package for uploading
Caused by:
  failed to select a version for the requirement `openehr = "^0.3.0"`
  candidate versions found which didn't match: 0.2.0
```

That was the actual state through most of 2026-08-01: only `openehr` packaged
successfully, and the other seven waited on it. Each crate becomes packageable
as soon as its dependencies are published — which is what happened on
2026-08-04, in this file's documented order. Do not "fix" the error by
loosening a version requirement to `0.2` — that would let a crate resolve
against an older *published* version rather than the local path, so the
workspace would test something other than what it ships.

Allow a minute between publishes for the index to update.

## Before publishing anything

```sh
for d in openehr openehr-store openehr-postgresql openehr-mysql \
         openehr-mariadb openehr-mssql openehr-oracle openehr-sqlite; do
  (cd "$d" && cargo test --quiet && cargo clippy --all-targets --quiet) \
    || echo "FAIL $d"
done
```

Then, per crate:

```sh
cd <crate>
cargo package --list          # what actually ships
cargo publish --dry-run
```

`cargo package --list` is worth reading rather than skimming. Confirm the `spec/`
directory ships if the crate's docs cite it — `openehr`'s rustdoc cites
`spec/01-scope.md`, and a link into a file that is not in the package is a
dangling reference for everyone reading on docs.rs.

Verified 2026-08-04: `openehr` packages 67 files including all 18 `spec/*.md`
and its five examples, so those citations resolve. `openehr-store` cites
`spec/conformance.md`, which likewise ships.

## Checks specific to this repository

- **`repository` points at `openehr-rust/openehr-rust`.** All eight were wrong
  until 2026-08-01.
- **The conformance level in the README and crate docs is the earned one.** Not
  the neighbouring crate's. This is what `openehr-mariadb` got wrong, and
  publishing it then would have put a false claim on crates.io permanently.
- **No crate claims CI results it does not have.** CI is green as of
  2026-08-01; only `openehr-sqlite` may claim **Verified**.
- **Version numbers agree** between a crate's `version` and the `version` its
  siblings depend on. A mismatch resolves to a published crate rather than the
  local path, which is how a workspace silently tests something other than what
  it ships.
- **The licence expression is the full five** (`W0.22`), identical in all nine
  publishable manifests now that `openehr-loco` is one of them, and
  `LICENSE.md` is the crate's only licence file (`W0.23`):

  ```
  MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR GPL-3.0-only
  ```

  crates.io validates this as an SPDX expression and displays it on the crate
  page, so a subset published here understates the grant permanently for that
  version.

## Publishing

```sh
cd <crate>
cargo publish
```

Requires `cargo login` with a token from <https://crates.io/settings/tokens>.
No CI publish workflow exists — publishing is this manual step, from the
maintainer's own machine, on purpose (`spec/trusted-publishing/index.md`, and
see the section below).

After the first publish of a crate, add the other owners:

```sh
cargo owner --add <user-or-team>
```

### Trusted Publishing — not yet, and the condition that changes it

[Trusted Publishing](https://crates.io/docs/trusted-publishing) replaces a
long-lived API token with a short-lived one that crates.io issues per run,
after verifying — via OpenID Connect — that the workflow requesting it really
is this repository's CI, on this branch. It removes the token row from
`MAINTAINERS.md`'s publishing-identities table entirely: there is nothing
long-lived to leak, rotate, or reissue.

**Not adopted here, and not because it is a bad idea.** This repository's own
stated policy (`spec/trusted-publishing/index.md`) is to adopt Trusted
Publishing once it is production-ready **across every forge this repository
actually pushes to** — GitHub.com, GitLab.com, and Codeberg.org, all three
real remotes today (`MAINTAINERS.md`) — **and across every destination it
actually publishes to**, which for this repository is crates.io. Adopting it
for one forge while the repository is mirrored to three would make the
mirrors' provenance a second-class question nobody had answered, which is
exactly the kind of undeclared departure `db:C0.16` calls a defect elsewhere
in this tree.

When the condition is met, the change here is small and mechanical, not a
redesign: a `publish.yml` workflow using each registry's OIDC action, the
`cargo publish` step above moves into it, and `MAINTAINERS.md`'s token row is
deleted rather than amended. Revisit this section when it happens; until
then, `cargo publish` from a workstation, exactly as documented above, is the
whole publishing surface.

## After

- **Update [`CITATION.cff`](../CITATION.cff)**: `version` and `date-released`.
  Nothing checks it — `scripts/check-docs.py` reads Markdown, and a `.cff` is
  not Markdown — so it is the one restatement of the version with no guard
  behind it. `W-10` was five files stating a version and one of them being
  updated; this is the sixth file, and this line is the only thing standing
  between it and the same finding.
- Check <https://docs.rs/<crate>> built. A docs.rs failure is usually a doctest
  or an intra-doc link that resolves locally and not in isolation.
- Tag the release: `git tag openehr-v0.3.0 && git push --tags`.
- If you find a bad claim after publishing, **do not yank silently**. Record it
  in [`spec/audit.md`](../spec/audit.md), fix it, and publish a new version. A
  yank removes the version from resolution and leaves the claim readable.
