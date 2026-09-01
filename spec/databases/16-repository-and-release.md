# 16. Repository and release

**Rewritten 2026-08-01.** The previous text described six sibling *ports*, each a
directory holding `-map`, `-gen`, and `-store` crates with its own git remote and
changelog, and required CI to diff the copies for drift. The layout here is one
shared library plus six thin engine crates, and there are no copies to diff. See
[`spec/audit.md`](../audit.md) **W-04**.

Withdrawn requirements keep their numbers (`C0.5`); new ones start at `W16.16`
(`C0.19`).

Requirement prefix: `W16`.

## Layout

- **W16.1** *(amended 2026-08-20, publish status changed 2026-09-01)*
  The repository holds **eighteen** crates: `openehr` (the Reference Model),
  `openehr-store` (engine-agnostic persistence), six `openehr-<engine>`
  dialect crates, `openehr-loco` (the HTTP service), `openehr-assets` (the
  generator for the committed assets), and **eight** fuzz harnesses — one per
  dialect, plus `openehr-fuzz` over the Reference Model parsers and
  `openehr-store-fuzz` over projection and the integrity check. Shared
  normative text lives at the root, in `spec/`; each engine crate carries only
  its own dialect annex (`X15.6`).

  The **eight** published crates are `openehr`, `openehr-store`, and the six
  dialects. `openehr-loco` MUST NOT declare `publish = false` as of
  2026-09-01 — it is a published crate by this requirement, whether or not it
  has yet had a first release; `agents/publishing.md` tracks whether one has
  actually happened, and this specification does not run `cargo publish` on
  its own authority. The other nine MUST declare `publish = false`
  (`W0.25`): `openehr-assets` and the eight fuzz harnesses.

  This requirement said **fourteen** until 2026-08-20, having been amended once
  already when the layout changed and then not again when `openehr-loco`,
  `openehr-assets`, and `openehr-fuzz` were added. A count is the cheapest
  possible claim to check and the easiest to leave behind; `spec/index.md`,
  `README.md`, and `AGENTS.md` all said seventeen while this said fourteen, and
  a specification disagreeing with a specification is worse than either
  disagreeing with a README (`W0.1`). See [`../audit.md`](../audit.md)
  **W-11**.
- **W16.2** *(amended)* An engine crate MUST be named `openehr-<engine>`, and the
  `<engine>` component MUST name the engine it actually targets. There are no
  `-map`, `-gen`, or per-engine `-store` crates.
- **W16.16** Each crate MUST be its own Cargo workspace.

  This is unusual and deliberate: `openehr` shares no code with the persistence
  crates, and folding it into their workspace would imply a code-sharing
  relationship that does not exist. The cost is that a build command must be run
  per crate, and CI must therefore build each separately — a single
  `--workspace` invocation would silently cover one of the eight.

- **W16.3** Every crate's `description` MUST name the engine the crate actually
  targets, and MUST NOT claim a capability the crate does not have. A crate with
  no `Store` MUST NOT describe itself as providing persistence beyond a schema.
- **W16.4** A crate MUST declare only the drivers it uses. A dialect-only crate
  MUST NOT depend on a database driver, because a dependency implies a capability
  and readers reasonably infer one.

## One copy of everything

- **W16.5** Normative text lives in `spec/` **once**. An engine crate's `spec/`
  directory holds only that crate's dialect annex (`X15.6`), and MUST NOT restate
  core requirements (`X15.8`).
- **W16.6** *(amended)* CI MUST verify that no two dialects emit the same DDL and
  that the comparison covers every engine crate (`X15.15`, `X15.16`).

  The original required CI to diff six copies of a shared tree for drift. There
  are no copies: the shared code is one crate the six depend on, so drift is
  impossible by construction. What remains checkable — and what actually failed —
  is whether two *dialects* have become the same thing.

- **W16.7** *(withdrawn in part, amended)* A change to shared behaviour is one
  edit in `openehr-store`. It MUST NOT be reproduced in the engine crates.

## Documentation

- **W16.8** Documentation MUST NOT be text-substituted from another crate.

  This is the most-violated requirement in the repository and the one that caused
  the most damage. `openehr-mariadb`'s entire crate — code, tests, README, and a
  conformance claim — was `openehr-mysql` with the engine name replaced. The
  substitution produced statements that were false about MariaDB and true about
  MySQL, including a version number, "MariaDB 8.4", that has never existed
  (**W-01**).

  A text substitution asserts, in the new crate's name, results obtained for a
  different thing. Write the documentation from the crate.

- **W16.9** A code example in documentation MUST be runnable against the code as
  shipped, and MUST be compiled and run by CI (`T11.21`).
- **W16.10** Measured numbers MUST name what measured them and when. A number
  with no provenance cannot be rechecked, and will be repeated long after it
  stopped being true.
- **W16.12** *(amended)* A changelog, where one exists, MUST describe changes to
  the crate it sits in. No crate here has one yet; adding one is not required,
  but an inherited or substituted changelog is forbidden by `W16.8`.

## Versioning and publishing

- **W16.11** *(amended)* Crates MAY version independently. They currently share
  `0.2.0`, which is a fact about their history rather than a rule.
- **W16.20** A release that changes behaviour a caller can observe MUST bump the
  version cargo treats as **incompatible**, not the one it treats as a patch.
  Below `1.0` that means `0.1.x` → `0.2.0`, never `0.1.1` → `0.1.2`.

  Cargo resolves `openehr-store = "0.1"` to any `0.1.x`, so a behaviour change
  shipped as a patch reaches every existing caller on a routine `cargo update`,
  unasked. That is the same failure this repository spends its audit register on,
  moved from documentation into version numbers.

  0.2.0 was published for exactly this reason: `commit_composition` began
  refusing versions carrying an attestation, an audit description, merge inputs,
  or a signature (`D-07`), and invariant names in error text changed (`lib:A-20`).
  Both are improvements; neither should arrive without being asked for.
- **W16.17** A crate's declared dependency version on a sibling MUST match that
  sibling's actual version.

  A mismatch resolves to the **published** crate rather than the local path, so
  the workspace silently builds and tests something other than what it ships.
  Loosening a requirement to make a build pass is the specific way this goes
  wrong.

- **W16.14** A crate MUST NOT be published to a registry above its conformance
  level, and its published documentation MUST state that level (`C0.9`, `C0.11`).

  Publication makes a claim permanent. A registry version is immutable: it cannot
  be edited, and yanking does not change its metadata. Publishing
  `openehr-mariadb` while it claimed Schema on evidence that did not exist would
  have put that claim on crates.io for good.

- **W16.18** A crate MUST NOT be published while a finding against its
  conformance claims is open (`W0.21`).
- **W16.13** *(amended)* Before publishing, a crate MUST pass its tests and
  lints, package cleanly, and ship every file its documentation links to. A
  rustdoc link into a `spec/` file that is not in the package is a dangling
  reference for every reader on docs.rs.
- **W16.15** *(amended)* The `repository` field MUST name the repository the
  crate is actually developed in.

  All eight named an unrelated project until 2026-08-01, and `openehr` 0.1.0 was
  published carrying it. That version is immutable and will point readers at the
  wrong repository permanently; the remedy was to publish a corrected version and
  let it become the one people see (**W-03**).

- **W16.19** Every crate MUST declare the same licence expression, and MUST ship
  a licence file naming all of it (`W0.22`, `W0.23`).

## Withdrawn

Withdrawn 2026-08-01. Numbers are retained and MUST NOT be reused (`C0.5`).

| Id | Was | Why withdrawn |
| --- | --- | --- |
| `W16.7` *(part)* | apply a shared-code change to every port in the same commit | there is one copy; the amended text keeps the prohibition on reproducing it |

---

Part of the [openEHR persistence specification](index.md).
