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

**Last verified against crates.io on 2026-08-02; local crates bumped to 0.3.0
on 2026-08-04, not yet published.** All eight publishable crates are live at
**0.2.0** on crates.io.

| Crate | crates.io | Local |
| --- | --- | --- |
| `openehr` | 0.1.0, 0.1.1, **0.2.0** | 0.3.0 |
| `openehr-store` | 0.1.1, **0.2.0** | 0.3.0 |
| `openehr-sqlite` | 0.1.1, **0.2.0** | 0.3.0 |
| `openehr-postgresql` | 0.1.1, **0.2.0** | 0.3.0 |
| `openehr-mysql` | 0.1.1, **0.2.0** | 0.3.0 |
| `openehr-mariadb` | 0.1.1, **0.2.0** | 0.3.0 |
| `openehr-mssql` | 0.1.1, **0.2.0** | 0.3.0 |
| `openehr-oracle` | 0.1.1, **0.2.0** | 0.3.0 |

`openehr-loco`, `openehr-assets`, and the seven fuzz crates are `publish = false`
and are not on crates.io. `openehr-loco`'s own version moves in lockstep with
the published crates for consistency (0.3.0 locally) even though it is never
itself published.

This table was wrong until 2026-08-02. It said seven of the eight were "not
published" and that the next version was 0.1.1, long after 0.2.0 had gone out —
a table an agent would have acted on, in the file that exists to stop a bad
publish. If this row disagrees with `git log` or with crates.io itself, trust
those, not this file.

## The tree has moved past what is published

**The local 0.3.0 and the published 0.2.0 are not the same software.** Several
of the commits separating them are **breaking**, catalogued in full in
[`CHANGELOG.md`](../CHANGELOG.md); summarised here for the publishing decision
they drive:

| Change | Why it breaks |
| --- | --- |
| `SCHEMA_VERSION` exists, and is `4` | Did not exist at 0.2.0. A database written by published 0.2.0 records **no** version, and `install()` refuses a populated database with none rather than guessing (`db:O10.16`). |
| `ColTy::Json` moved off `jsonb` and MySQL `JSON` | `db:M3.43`, from `db:D-08`. A PostgreSQL or MySQL database created by 0.2.0 has a column of the wrong type, and its stored bytes are already normalised — reordered keys, and on MySQL `1.10` rewritten as `1.1`. |
| Nine chain columns on `openehr_version` | `db:D-07`. Absent at 0.2.0. |
| `ColTy::Digest` added | `ColTy` is deliberately not `#[non_exhaustive]`, so this breaks any external `Dialect` implementation at compile time — by design. |
| `OriginalVersion::new` refuses what it accepted | `lib:A-23`. A first version naming a predecessor, or a successor naming none, now fails to construct. |
| `Date`/`Time`/`DateTime`/`Duration` lost `PartialOrd`/`Ord` | `lib:A-32`. `Eq` on these was lexical while `PartialOrd` was semantic, contradicting the standard library's requirement that the two agree. `<`, `.partial_cmp()`, `.min()`/`.max()`, `sort()` on these four types no longer compile; call the new `.semantic_cmp()` instead. Does not touch `DvDate`/`DvTime`/`DvDateTime`/`DvDuration`, whose own `PartialOrd` is unchanged. |

So the next release is **0.3.0**, not 0.2.1. Cargo treats `0.2.x` as compatible
with `0.2.0`, and shipping any of the above as a patch would break a dependent
on `cargo update`.

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

**This ordering is not advisory — cargo enforces it.** As of 2026-08-01, only
`openehr` packages successfully; the other seven fail with:

```
error: failed to prepare local package for uploading
Caused by:
  failed to select a version for the requirement `openehr = "^0.3.0"`
  candidate versions found which didn't match: 0.2.0
```

That is the expected state, not a defect: every crate depends on `openehr`
0.3.0, and crates.io still has only 0.2.0. Each crate becomes packageable as
soon as its dependencies are published. Do not "fix" it by loosening a version
requirement to `0.2` — that would let a crate resolve against the *published*
0.2.0 rather than the local path, so the workspace would test something other
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
- Tag the release: `git tag openehr-v0.3.0 && git push --tags`.
- If you find a bad claim after publishing, **do not yank silently**. Record it
  in [`spec/audit.md`](../spec/audit.md), fix it, and publish a new version. A
  yank removes the version from resolution and leaves the claim readable.
