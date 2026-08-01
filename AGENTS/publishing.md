# Publishing to crates.io

Not normative; `W0.20`–`W0.21` in [`spec/index.md`](../spec/index.md) are.

## Read this first

**A published version is immutable.** It cannot be edited, and yanking does not
change its metadata — a yanked version keeps its `repository`, its description,
and every conformance claim in its documentation.

This has already cost something here. `openehr` 0.1.0 was published on
2026-07-31 carrying `repository = ".../fhir-databases"`, pointing at an unrelated
project. That field is permanent for 0.1.0. The only remedy is to publish 0.1.1
and let it become the version people see.

So: **a conformance claim becomes permanent the moment you publish it.** Do not
publish a crate with an open finding against its claims (`W0.21`).

## State today

| Crate | crates.io | Local |
| --- | --- | --- |
| `openehr` | **0.1.0 published** (10 downloads) | 0.1.1 |
| `openehr-store` | not published | 0.1.1 |
| `openehr-sqlite` | not published | 0.1.1 |
| `openehr-postgresql` | not published | 0.1.1 |
| `openehr-mysql` | not published | 0.1.1 |
| `openehr-mariadb` | not published | 0.1.1 |
| `openehr-mssql` | not published | 0.1.1 |
| `openehr-oracle` | not published | 0.1.1 |

All seven unpublished names were available as of 2026-08-01.

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

**This ordering is not advisory — cargo enforces it.** As of 2026-08-01, only
`openehr` packages successfully; the other seven fail with:

```
error: failed to prepare local package for uploading
Caused by:
  failed to select a version for the requirement `openehr = "^0.1.1"`
  candidate versions found which didn't match: 0.1.0
```

That is the expected state, not a defect: every crate depends on `openehr`
0.1.1, and crates.io still has only 0.1.0. Each crate becomes packageable as
soon as its dependencies are published. Do not "fix" it by loosening a version
requirement to `0.1` — that would let a crate resolve against the *published*
0.1.0 rather than the local path, so the workspace would test something other
than what it ships.

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

Verified 2026-08-01: `openehr` ships 68 files including all 18 `spec/*.md` and
its five examples, so those citations resolve. `openehr-store` cites
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
- Tag the release: `git tag openehr-v0.1.1 && git push --tags`.
- If you find a bad claim after publishing, **do not yank silently**. Record it
  in [`spec/audit.md`](../spec/audit.md), fix it, and publish a new version. A
  yank removes the version from resolution and leaves the claim readable.
