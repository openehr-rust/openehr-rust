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

**Published 2026-08-21 at 0.4.0. Local is 0.5.0 and NOT yet published.**

All eight publishable crates are live at **0.4.0** on crates.io, in the order
below. 0.5.0 is staged — manifests, `Cargo.lock`, and
[`CHANGELOG.md`](../CHANGELOG.md) all say 0.5.0 — and waits on the same gate
0.4.0 waited on: **CI green on the commit being published.**

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
| `openehr` | 0.1.0, 0.1.1, 0.2.0, 0.3.0, **0.4.0** | **0.5.0** (unpublished) |
| `openehr-store` | 0.1.1, 0.2.0, 0.3.0, **0.4.0** | **0.5.0** (unpublished) |
| `openehr-sqlite` | 0.1.1, 0.2.0, 0.3.0, **0.4.0** | **0.5.0** (unpublished) |
| `openehr-postgresql` | 0.1.1, 0.2.0, 0.3.0, **0.4.0** | **0.5.0** (unpublished) |
| `openehr-mysql` | 0.1.1, 0.2.0, 0.3.0, **0.4.0** | **0.5.0** (unpublished) |
| `openehr-mariadb` | 0.1.1, 0.2.0, 0.3.0, **0.4.0** | **0.5.0** (unpublished) |
| `openehr-mssql` | 0.1.1, 0.2.0, 0.3.0, **0.4.0** | **0.5.0** (unpublished) |
| `openehr-oracle` | 0.1.1, 0.2.0, 0.3.0, **0.4.0** | **0.5.0** (unpublished) |

`openehr-loco`, `openehr-assets`, and the eight fuzz crates are `publish = false`
and are not on crates.io. `openehr-loco`'s own version moves in lockstep with
the published crates for consistency (0.3.0 locally) even though it is never
itself published.

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
```

`openehr-sqlite` is last because its **dev-dependencies** name the other five
engine crates with versions, for the cross-dialect comparison. Dev-dependencies
are stripped from the published package's resolution, but `cargo publish`
verifies the package builds — including its tests — so the five must be
resolvable. Publish them first and the ordering problem disappears; there is no
cycle.

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
- **The licence expression is the full five** (`W0.22`), identical in all eight
  manifests, and `LICENSE.md` is the crate's only licence file (`W0.23`):

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

After the first publish of a crate, add the other owners:

```sh
cargo owner --add <user-or-team>
```

## After

- Check <https://docs.rs/<crate>> built. A docs.rs failure is usually a doctest
  or an intra-doc link that resolves locally and not in isolation.
- Tag the release: `git tag openehr-v0.3.0 && git push --tags`.
- If you find a bad claim after publishing, **do not yank silently**. Record it
  in [`spec/audit.md`](../spec/audit.md), fix it, and publish a new version. A
  yank removes the version from resolution and leaves the claim readable.
