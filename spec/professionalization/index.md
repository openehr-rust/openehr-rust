# Professionalization

This specification defines what "professional" means for this repository and
binds the maintainer as much as any contributor. The audience is healthcare
professionals and the engineers who serve them, worldwide, in production use;
the standing constraint is that a wrong claim in this domain has clinical
cost. Rationale and current execution state live in [`plan.md`](../../plan.md)
and [`tasks.md`](../../tasks.md); this file holds the rules.

## Rules

1. **Plans are files, and a checked box is a verified fact.** `plan.md` and
   `tasks.md` exist at the repository root. A `[x]` means the work was done
   and verified, with the evidence named — never that it is intended,
   assumed, or inherited from a sibling repository.
2. **The special files exist and stay accurate.** The canonical list is
   [`spec/special-files-for-public-repos/`](../special-files-for-public-repos/index.md).
   Every countable claim in those files (crate counts, test counts, coverage
   lists, "X is enabled/disabled") is measured before it is written and
   re-verified when cited.
3. **Self-declared gaps are promises.** A gap named in SECURITY.md,
   MAINTAINERS.md, or AI_STATEMENT.md ("no CI", "unsigned commits") is either
   closed or consciously accepted in `tasks.md` — and the declaring document
   is updated in the same change that closes it.
4. **CI enforces what documents claim.** Every check a document says this
   repository runs (tests, clippy, fmt, MSRV, trademark rules, doc gates)
   runs in CI on every push. A laptop-only check is a claim, not a guarantee.
5. **Trademark discipline.** The mark is **openEHR**, the registered
   trademark of the openEHR Foundation — U.S. Reg. No. 4,272,380, EUIPO Reg.
   002994853, IP Australia Reg. 939279, verified at
   <https://openehr.org/logos/> on 2026-08-26. This project uses the mark in
   its own name and is not affiliated with its owner, which is exactly the
   arrangement in which a missing notice misleads a reader.

   The binding rule: **every root document and every top-level published
   crate's rustdoc that uses the mark in prose carries the following notice
   verbatim**, in the exact wording the owner specified on 2026-08-26:

   > openEHR® is the registered trademark of the openEHR Foundation. Use of
   > the trademark does not constitute endorsement of this product by
   > openEHR International or openEHR Foundation.

   The Foundation's own usage terms (same page, same date) additionally ask
   that third-party use of the mark carry the ® symbol and, beyond fair use,
   a Product Use Licence; this project holds no such licence and therefore
   does not use the Foundation's "used with permission" attribution text,
   which would be a false statement here. The exposure of the unlicensed
   name itself is an open decision tracked in `plan.md` §Risks and asked
   openly in `RFC.md` §9.

   The notice is enforced by `scripts/check-trademarks.py`, run the way the
   other documentation gates are (rule 4).
6. **Patient data is addressed in plain language.** `PHI.md` at the root
   states what the software does and does not do with patient data, for a
   reader who is a privacy officer, not a Rust programmer. It never claims
   compliance or certification.
7. **Conduct has a document and a path.** `CODE_OF_CONDUCT.md` at the root
   (Contributor Covenant 2.1 plus this family's claim-accuracy clause:
   overstating what the software does is a conduct matter, not only a bug).
8. **Harmonization runs through the family.** The sibling repositories
   (`hl7-rust`, `er7-rust`, `fhir-rust`, `snomed-rust`, `openehr-rust`)
   share these rules, the special-files list, and the six workstreams
   (governance; compliance — licensing and trademarks; security and supply
   chain; privacy and patient data; outreach; audit and harmonization).
   Conventions sync from the repository that owns the canonical copy rather
   than drifting independently.
9. **Outreach is gated.** No promotion while a rule above is unmet for the
   surface being promoted; `help/outreach/index.md` names the prerequisites.

## Status in this repository

Assessed 2026-08-26. This specification allocates no `W0.x` identifiers and
no prefix of its own; its rules are cited as "professionalization rule N".
Registration in `spec/index.md`'s prefix table is therefore not required.
(An earlier revision deferred even listing the directory there because that
file carried an in-flight change; the change landed 2026-08-26, and the
listing remains optional.) This paragraph is the registration.

- **Rule 1: met.** `plan.md` and `tasks.md` are committed at the root
  (commit `f1f091d`), and every `[x]` added today names its evidence.
- **Rule 2: met, with one recorded absence.** The special files exist and
  the canonical list was re-synced from the family 2026-08-26; the one
  listed file that does not exist, `.github/FUNDING.yml`, is a recorded
  open decision rather than a gap (the list's own status section says so).
- **Rule 3: partly met.** SECURITY.md's known-gaps section is accurate: the
  repository-settings gap it named (private vulnerability reporting,
  Dependabot, secret scanning) was closed 2026-08-26 and the section updated
  in the same change; signing and SBOM remain open `tasks.md` items.
- **Rule 4: met for the documentation gates.** `scripts/check-docs.py` runs
  in the CI `claims` job, and `scripts/check-trademarks.py` runs in the CI
  `trademarks` job — wired 2026-08-26, once the in-flight changes that had
  deferred it landed, with the rows the `claims` gate requires in `AGENTS.md`
  and `spec/audit.md`. An earlier revision of this entry admitted the
  trademark check was laptop-only; it no longer is.
- **Rule 5: met.** `scripts/check-trademarks.py` passes, in CI on every push
  (rule 4): every in-scope file using the mark in prose carries the notice
  verbatim, and the three files that carried it uncommitted-only as of
  2026-08-26 (`README.md`, `openehr/src/lib.rs`, `CHANGELOG.md`) landed with
  the Archetype Model change they were deferred behind. **Scope decision:** `openehr/spec/**`, `spec/databases/**`, and the other
  specification and agent-guide trees are deliberately out of the checker's
  scope — they use the mark in nearly every file, a notice per specification
  section would drown the text it annotates, and the root documents those
  trees are reached from carry it.
- **Rule 6: met.** `PHI.md` at the root (commit `ba8b199`), citing the
  security specification and both compliance mappings, claiming no
  compliance or certification.
- **Rule 7: met.** `CODE_OF_CONDUCT.md` at the root (commit `f1f091d`) with
  the claim-accuracy clause citing `W0.3`.
- **Rule 8: met for the named items.** This file is the family template
  adapted; CODEOWNERS moved to the repository root 2026-08-26 to match all
  four siblings, and the special-files list was re-synced from the
  canonical `fhir-rust` copy the same day.
- **Rule 9: met as a gate.** No outreach has been executed;
  `help/outreach/index.md` §11 blocks on the conduct file (closed
  2026-08-26) and the repository security settings (closed 2026-08-26).
