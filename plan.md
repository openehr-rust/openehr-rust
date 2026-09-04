# Plan — `openehr` Rust monorepo

Goal: a production-grade, spec-driven Rust monorepo for openEHR — the
`openehr/` model crate (Reference Model, paths, AQL parsing, RM validation,
audit-chain security), six `openehr-<engine>/` database ports,
`openehr-store/`, `openehr-loco/`, and the fuzz crates; eight crates
published at 0.9.0 (`openehr-loco` and the fuzz crates stay unpublished) —
professionalized for its real audience: healthcare
professionals and the engineers who serve them, worldwide, in settings where
a wrong claim has clinical cost.

This project is not affiliated with, endorsed by, or certified by openEHR
International; its conformance ladder is its own, and no procurement document
should read it otherwise (GOVERNANCE.md §Independence).

Method: **specification-driven development, with audited claims.** Behavior
lives in `spec/`, `openehr/spec/`, and `spec/databases/` before it is
implemented; three findings registers (`W-xx` repo-wide, `A-xx` library,
`D-xx` databases) hold what was found wrong, and the CI `claims` job checks
that the registers and the conformance matrices count themselves correctly.
Engineering status is read from `openehr/spec/conformance-matrix.md` and
`spec/databases/conformance-matrix.md` — never from a checklist. Execution
items live in [`tasks.md`](tasks.md), where a `[x]` means verified, not
intended.

## Where the repository stands (verified 2026-09-04)

Eight crates at 0.9.0 on crates.io (2026-09-02) — up from 0.6.0 on
2026-08-26, through 0.7.0–0.7.4 (Archetype Model in scope, trademark
notices), 0.8.0 (MSRV N−3 to N−2, 1.95 to 1.96), and 0.9.0 (`A-54`, `A-55`,
`A-57`). Three breaking changes are unreleased as of this date (`A-63`,
`A-69`, `A-71`), so the next release is **0.10.0**, not 0.9.1
(`agents/publishing.md` tracks it). An eleven-job CI workflow — test, msrv,
examples, bench, schema, fuzz, assets, layering, trademarks, claims,
mutants — green on the pushed head (run 33785547358, 33 of 33 jobs); the
library matrix machine-checked for exact-once coverage (345 ids, 271
verified, 16 `spec`) and its tally re-derived by hand five times, each
dated. The root document set — SECURITY.md, GOVERNANCE.md,
CONTRIBUTING.md, MAINTAINERS.md, CITATION.cff, `CODEOWNERS`,
`help/outreach/index.md` and more — is committed and visible on GitHub;
"largely uncommitted" was true on 2026-08-26 and stopped being true the same
week. Commits and tags are SSH-signed and verified (`git log --show-
signature`, and both GitHub's and GitLab's API confirm the commit as
`verified`); Trusted Publishing to crates.io is specified
(`spec/trusted-publishing/index.md`) but not adopted — the stated condition
is production-readiness across every forge this repository actually pushes
to (GitHub, GitLab, Codeberg) and every destination it publishes to
(crates.io), not met yet, so publishing stays a maintainer-run
`cargo publish` (`agents/publishing.md`).

The headline capability gap is honest and registered: the Archetype Model
entered scope on 2026-08-26 with 28 of 32 requirements having no code
(**A-40**, open). Ten closed the same week (2026-08-30): `openehr::am::validate`
— a Reference Model instance checked against an `Archetype` already in
memory, a verdict kept separate from Reference-Model validation, no partial
pass — and `openehr::am::repository`, resolving a `C_ARCHETYPE_ROOT` filler
through a repository the caller supplies (`openehr` performs no I/O). Two
more closed on 2026-09-03 (`K15.6`–`K15.7`, the refusal discipline, once
`openehr::am::cadl` existed to be held to it), and `K15.32` was added and
satisfied the same day. That leaves **16 of 33**: no whole-archetype ADL
parser (the `definition` section and the header are read; they do not
compose), no ADL 1.4 body, no flattening, no template expansion, no
operational template, no evaluation of a slot's own assertions. What
changed the picture is evidence nobody here wrote: the external corpus
(`openehr/spec/corpus.md`, `openEHR/adl-archetypes` at `093c77ea`), run
five times across 2026-09-03 and 2026-09-04, took the `definition` reader
from 178 to **969 of 1,379** ADL 2 files parsed, through seven findings it
produced (`A-70`–`A-76`), and left the remaining candidates and one
decision written down. That remainder is engineering work tracked by the
matrix and by `corpus.md`, not by this file.

## Workstreams — professionalization (2026-08 onward)

Six workstreams, shared with the sibling repositories (`hl7-rust`,
`er7-rust`, `fhir-rust`, `snomed-rust`) so the family converges on one
posture. Open items for each are in `tasks.md`.

1. **Governance.** GOVERNANCE.md, MAINTAINERS.md, RFC.md exist and are
   candid (sole maintainer, bus factor one, machines do not decide). The
   self-declared hole: no `CODE_OF_CONDUCT.md` — named as a known gap in
   CONTRIBUTING.md:140 and in the outreach readiness checklist itself.

2. **Compliance — licensing and trademarks.** Closed 2026-08-27, see
   §Risks & watch items below: openEHR granted permission to use their
   marks, `TRADEMARKS.md` records it, and the Foundation's own prescribed
   notice — verbatim, stating the permission — is on every crate
   description, every crate README, and every root document that uses the
   mark, checked in CI by `scripts/check-trademarks.py`. The `LICENSES/`
   directory for the five-license SPDX expression was added 2026-08-26.

3. **Security and supply chain.** The in-crate security work is real
   (HMAC-capable audit chain, constant-time verify, redaction with
   distinctive-marker tests). The settings-level half of the repository
   posture closed 2026-08-26: private vulnerability reporting, Dependabot
   alerts, automated security fixes, and secret scanning are enabled, each
   verified with a `GET`, and `.github/dependabot.yml` registers every
   workspace — including the GitHub Pages site's own `npm` ecosystem,
   added after its lockfile drift broke the "Dependabot Updates" workflow —
   with routine version-update PRs capped off. Commits and tags are
   SSH-signed and verified on both GitHub and GitLab, closed 2026-08-28.
   Still open: no SBOM, no release attestation.

4. **Privacy and patient data.** PHI rules are specified
   (`openehr/spec/11-security.md`, the compliance mappings, `redact.rs`) but
   scattered; there is no root `PHI.md` a clinician or CISO can read. The
   compliance mapping's honesty — HIPAA §164.312(b) and (c)(1) marked
   Partial at the database layer — must carry into that page, not be
   smoothed over by it.

5. **Outreach.** `help/outreach/index.md` (~440 lines) is a full campaign
   plan with the right governing rule (never say safe/compliant/certified/
   clinically). A root `index.md` routes evaluators to the project
   documents as of 2026-08-26, and `openehr-rust.github.io` has been live
   since 2026-08-28. No repository topics on GitHub remains the one open
   readiness gap the checklist tracks; the outreach sequence itself
   (`help/outreach/index.md` §11) has not been executed.

6. **Audit and harmonization.** Three registers, counts machine-checked, and
   `scripts/check-docs.py` as the doc gate — the strongest audit machinery
   in the family alongside `fhir-rust`. `spec/special-files-for-public-repos/
   index.md` was re-synced with the canonical `fhir-rust` version on
   2026-08-26; every file on that list now exists here, `.github/
   FUNDING.yml` included since 2026-08-28. CODEOWNERS moved to the
   repository root 2026-08-26, matching all four siblings.

## Open decisions (awaiting a call, not code)

- ~~**Funding surface.** CONTRIBUTING.md states there is no funding vehicle;
  `.github/FUNDING.yml` is therefore a decision to create one, not a missing
  file.~~ — answered 2026-08-28, per `spec/free-open-source-funding/index.md`:
  GitHub Sponsors, under the maintainer's personal account, no legal entity.
  Verified live before acting on it rather than assumed —
  `sponsorsListing.slug` present and `isPublic: true` via the GitHub API,
  `github.com/sponsors/joelparkerhenderson` returns 200 — then
  `.github/FUNDING.yml` added and `CONTRIBUTING.md`/`NEWS.md` brought into
  line. **Open Collective was asked for and is not done**: the spec's own
  acceptance check was "verify GitHub Sponsors is set up correctly," and
  running the equivalent check for Open Collective found `joelparkerhenderson`
  is only an auto-generated `INDIVIDUAL` profile (everyone who uses the site
  gets one), not a `COLLECTIVE`, and `opencollective.com/openehr-rust` 404s.
  Creating a real one means choosing a fiscal host through Open Collective's
  own interactive application flow — a decision, and one this session cannot
  make or complete on the owner's behalf.
- ~~**Site.** Whether to stand up a `openehr-rust.github.io` landing surface
  (the `hl7-rust`/`er7-rust` pattern) before or after the outreach
  prerequisites close.~~ — answered 2026-08-28: yes, and it exists. The owner
  stood up `openehr-rust.github.io` directly (its own repository, the same
  in-repo SvelteKit/Lily-theme pattern the siblings use) — confirmed live
  (`curl` returns 200, a real rendered page with the project's own title and
  description, not a placeholder) rather than taken on trust. Linked from
  this repository's own front doors, `README.md` and `index.md`, which had
  no reference to it until now.
- ~~**`spec/databases/conformance-matrix.md`** remains hand-assessed apart from
  one CI check; deciding whether to machine-derive it like the library
  matrix is a real cost/benefit call.~~ — answered 2026-08-27, with evidence
  rather than a guess: **not exact-once, not yet.** The library matrix's shape
  (one linear walk, one row per requirement) makes "exactly once" a coherent
  question; this file is five topic tables in which a requirement can
  legitimately appear more than once (`PR12.5` correctly appears in both the
  service table and "not implemented in the store", for different true
  reasons), so porting the library's exact-once checker verbatim would flag
  correct rows as defects. `scripts/check-databases-matrix-coverage.py` checks
  the floor that shape does allow — mentioned **at least once**, not
  necessarily marked right — and running it found the real cost: **144 of 221
  requirements have never been assessed at all** (`db:D-11`), including `M3.19`
  ("canonical JSON... **is** the record") and every one of `V9`. The script is
  written and correct, deliberately **not wired into CI** until the 144 are
  assessed (see the script's own docstring for why a red-on-day-one gate is the
  wrong shape here), and the recommended path is in `db:D-11`: assess in
  batches by section, `M3` and `S1` first.

- **A regex engine for `K15.10`** — opened 2026-09-03, awaiting a call.
  `openehr/Cargo.toml` has never carried one, and every dependency there is
  justified in its own comment. `lib:A-66` parses an `ARCHETYPE_SLOT`'s
  `include`/`exclude` assertions and carries them; `am::validate` reports the
  filler *unchecked* because nothing evaluates the regex. Three honest
  options: add `regex` (safe Rust, well audited, several transitive crates
  — the first genuinely new architectural dependency since the crate's
  audit chain), hand-roll a documented subset (smaller, and a second thing
  to get wrong), or decide that assertions stay carried-not-evaluated and
  say so in the matrix. `tasks.md` P0 names it; nothing is blocked on it but
  the last slice of `K15.10`.
- **A Reference Model multiplicity table for `am`** — opened 2026-09-03,
  `A-71`'s residual. `am::cadl` decides single-valued or container by
  whether a `cardinality` clause was written, because the crate has no
  table of which RM attributes are containers; AOM2 infers from the RM. The
  corpus shows the cost (11 refused files, and an inferred `0..1` where AOM2
  infers `0..*`). The table is small and the RM is already in `rm::`; the
  decision is whether to derive it there once (`lib:A-33`'s shape) or carry
  a second table in `am`. A requirement first, then code.

## Non-goals (for now)

- No claim of openEHR conformance beyond the project's own ladder, ever.
- No new engines until the professionalization workstreams close; the
  Archetype Model work (A-40) proceeds on its own track under the matrix.

## Risks & watch items

- ~~The unqualified use of the openEHR mark in the org and crate names is
  the one item on this list with third-party legal exposure~~ — resolved
  2026-08-27: openEHR granted permission to use their trademarks
  (owner-reported; correspondence held by the maintainer;
  [`TRADEMARKS.md`](TRADEMARKS.md) §Permission is the record). Every
  notice now carries the Foundation's own prescribed attribution, which
  states the permission — adopted the same day at the owner's direction.
  Nothing remains open on this axis.
- The uncommitted document batch has zero external effect until it lands and
  can rot against the moving tree.
- A-40's 16 unimplemented requirements (28 on 2026-08-26) are correctly
  labeled `spec` in the matrix today; the risk is a future page summarizing
  them as capability — a `definition` reader that parses 969 of 1,379
  published files is still not archetype support (`K15.31`). The matrix,
  not prose, is the status document.
- The mutation job mutates `event.before..HEAD` on a push, so a survivor in
  an earlier push is never re-checked by CI; run 33775455452 found 14 in
  code CI had already passed once, and the whole-function local run found
  14 more. `agents/auditing.md` says how to run it by name.

## Trademarks

openEHR® is the registered trademark of the openEHR Foundation and is used with
the permission of openEHR International. Use of the trademark does not
constitute endorsement of this product by openEHR International or openEHR
Foundation.
