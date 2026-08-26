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
5. **Trademark discipline.** The mark is **openEHR**, a registered trademark
   of openEHR International (the openEHR Foundation) — U.S. Reg. No.
   4,272,380, EUIPO Reg. 002994853, IP Australia Reg. 939279, verified at
   <https://openehr.org/logos/> on 2026-08-26. This project uses the mark in
   its own name and is not affiliated with its owner, which is exactly the
   arrangement in which a missing notice misleads a reader.

   The binding rule: **every root document and every top-level published
   crate's rustdoc that uses the mark in prose carries the following notice
   verbatim**, including its non-affiliation sentence:

   > openEHR® is a registered trademark of openEHR International (the
   > openEHR Foundation). This project is an independent implementation: it
   > is not affiliated with, endorsed by, or certified by openEHR
   > International.

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
Registration in `spec/index.md`'s prefix table is therefore not required, and
listing this directory there is deferred anyway: that file's working tree
carries an unrelated in-flight change (the `lib:S1.4` withdrawal note), and
landing it from here would commit someone else's half-done work. This
paragraph is the registration until then.

- **Rule 1: met.** `plan.md` and `tasks.md` are committed at the root
  (commit `f1f091d`), and every `[x]` added today names its evidence.
- **Rule 2: partly met.** The special files exist; the local copy of the
  canonical list predates the family's and its re-sync is an open `tasks.md`
  item under Audit and harmonization.
- **Rule 3: partly met.** SECURITY.md's known-gaps section is accurate as of
  2026-08-26; the gaps it names (repository settings, signing, SBOM) are open
  `tasks.md` items, not yet closed.
- **Rule 4: partly met.** The documentation gate (`scripts/check-docs.py`,
  the CI `claims` job) runs on every push. `scripts/check-trademarks.py`
  exists and runs green from the repository root, but is **not yet a CI
  job**: adding one requires edits to `ci.yml`, `AGENTS.md`, and
  `spec/audit.md` (the `claims` gate binds the trio), and all three carry
  unrelated in-flight changes as of 2026-08-26; a `tasks.md` item tracks the
  wiring until those land. Until then the check is exactly what rule 4 calls
  a laptop-only check, and this line is the admission.
- **Rule 5: met in the working tree, not yet fully in history.**
  `scripts/check-trademarks.py` passes: every in-scope file using the mark
  in prose carries the notice verbatim. Three of those files carry it
  **uncommitted only** — `README.md` and `openehr/src/lib.rs` (which also
  carry the in-flight Archetype Model hunks) and `CHANGELOG.md` (which also
  carries the in-flight unsafe-sweep entry) — so the green is a statement
  about the working tree, and their commits belong to whoever lands those
  changes. **Scope decision:** `openehr/spec/**`, `spec/databases/**`, and the other
  specification and agent-guide trees are deliberately out of the checker's
  scope — they use the mark in nearly every file, a notice per specification
  section would drown the text it annotates, and the root documents those
  trees are reached from carry it.
- **Rule 6: met.** `PHI.md` at the root (commit `ba8b199`), citing the
  security specification and both compliance mappings, claiming no
  compliance or certification.
- **Rule 7: met.** `CODE_OF_CONDUCT.md` at the root (commit `f1f091d`) with
  the claim-accuracy clause citing `W0.3`.
- **Rule 8: in progress.** This file is the family template adapted; the
  special-files re-sync and CODEOWNERS placement are the open harmonization
  items.
- **Rule 9: met as a gate.** No outreach has been executed;
  `help/outreach/index.md` §11 blocks on the conduct file (now closed) and
  the repository security settings (still open).
