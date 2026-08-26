# Plan — `openehr` Rust monorepo

Goal: a production-grade, spec-driven Rust monorepo for openEHR — the
`openehr/` model crate (Reference Model, paths, AQL parsing, RM validation,
audit-chain security), six `openehr-<engine>/` database ports,
`openehr-store/`, `openehr-loco/`, and the fuzz crates; eight crates
published at 0.6.0 (`openehr-loco` and the fuzz crates stay unpublished) —
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

## Where the repository stands (verified 2026-08-26)

Eight crates at 0.6.0 on crates.io; a ~700-line CI workflow with test/msrv/
examples/bench/schema/fuzz/assets/layering/claims jobs; the library matrix
machine-derived (344 ids, 258 verified); SECURITY.md the most complete in the
five-repo family, including a "Known gaps in this project's own security
posture" section checked against the live repository settings on 2026-08-26.
The root document set is nearly complete — and, like the pass in the sibling
repositories, **largely uncommitted**: SECURITY.md, GOVERNANCE.md,
CONTRIBUTING.md, MAINTAINERS.md, CITATION.cff, `.github/CODEOWNERS`,
`help/outreach/index.md` and more are untracked, so none of it is visible on
GitHub yet.

The headline capability gap is honest and registered: the Archetype Model
entered scope on 2026-08-26 with 28 of 32 requirements having no code
(**A-40**, open) — no ADL parser, no template expansion, no archetype
validation. That is engineering work tracked by the matrix, not by this file.

## Workstreams — professionalization (2026-08 onward)

Six workstreams, shared with the sibling repositories (`hl7-rust`,
`er7-rust`, `fhir-rust`, `snomed-rust`) so the family converges on one
posture. Open items for each are in `tasks.md`.

1. **Governance.** GOVERNANCE.md, MAINTAINERS.md, RFC.md exist and are
   candid (sole maintainer, bus factor one, machines do not decide). The
   self-declared hole: no `CODE_OF_CONDUCT.md` — named as a known gap in
   CONTRIBUTING.md:140 and in the outreach readiness checklist itself.

2. **Compliance — licensing and trademarks.** The weakest of the five
   repositories here, and the highest-exposure gap: the org, repository, and
   crate name all use the openEHR mark with **no `TRADEMARKS.md`, no ™/®
   anywhere, and no disclaimer in README.md, LICENSE.md, CITATION.cff, or
   crate metadata** — the only non-affiliation statements are buried in
   GOVERNANCE.md §Independence and `openehr/README.md`. The `LICENSES/`
   directory for the five-license SPDX expression was added 2026-08-26.

3. **Security and supply chain.** The in-crate security work is real
   (HMAC-capable audit chain, constant-time verify, redaction with
   distinctive-marker tests). The settings-level half of the repository
   posture closed 2026-08-26: private vulnerability reporting, Dependabot
   alerts, automated security fixes, and secret scanning are enabled, each
   verified with a `GET`, and `.github/dependabot.yml` registers every
   workspace with routine version-update PRs capped off. Still open:
   commits and tags unsigned, no SBOM, no release attestation.

4. **Privacy and patient data.** PHI rules are specified
   (`openehr/spec/11-security.md`, the compliance mappings, `redact.rs`) but
   scattered; there is no root `PHI.md` a clinician or CISO can read. The
   compliance mapping's honesty — HIPAA §164.312(b) and (c)(1) marked
   Partial at the database layer — must carry into that page, not be
   smoothed over by it.

5. **Outreach.** `help/outreach/index.md` (~440 lines) is a full campaign
   plan with the right governing rule (never say safe/compliant/certified/
   clinically) and a readiness checklist that correctly blocks on the
   conduct file. A root `index.md` routes evaluators to the project
   documents as of 2026-08-26; there is still no public site (`plan.md`
   §Open decisions).

6. **Audit and harmonization.** Three registers, counts machine-checked, and
   `scripts/check-docs.py` as the doc gate — the strongest audit machinery
   in the family alongside `fhir-rust`. Harmonization items: re-sync
   `spec/special-files-for-public-repos/index.md` with the canonical
   `fhir-rust` version (the local copy lists neither the conduct file, nor
   PHI, nor LICENSES/, nor FUNDING). CODEOWNERS moved to the repository
   root 2026-08-26, matching all four siblings.

## Open decisions (awaiting a call, not code)

- **Funding surface.** CONTRIBUTING.md states there is no funding vehicle;
  `.github/FUNDING.yml` is therefore a decision to create one, not a missing
  file. Decide, then make the files agree.
- **Site.** Whether to stand up a `openehr-rust.github.io` landing surface
  (the `hl7-rust`/`er7-rust` pattern) before or after the outreach
  prerequisites close. Outreach §6.5 presumes one exists.
- **`spec/databases/conformance-matrix.md`** remains hand-assessed apart from
  one CI check; deciding whether to machine-derive it like the library
  matrix is a real cost/benefit call.

## Non-goals (for now)

- No claim of openEHR conformance beyond the project's own ladder, ever.
- No new engines until the professionalization workstreams close; the
  Archetype Model work (A-40) proceeds on its own track under the matrix.

## Risks & watch items

- The unqualified use of the openEHR mark in the org and crate names is the
  one item on this list with third-party legal exposure; it should be
  resolved (notice now, contact with openEHR International if needed) before
  any outreach.
- The uncommitted document batch has zero external effect until it lands and
  can rot against the moving tree.
- A-40's 28 unimplemented requirements are correctly labeled `spec` in the
  matrix today; the risk is a future page summarizing them as capability.
  The matrix, not prose, is the status document.

## Trademarks

openEHR® is the registered trademark of the openEHR Foundation. Use of the
trademark does not constitute endorsement of this product by openEHR
International or openEHR Foundation.
