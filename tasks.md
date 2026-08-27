# Tasks

Repository-level professionalization checklist; rationale and workstreams
live in [`plan.md`](plan.md). A `[x]` here means the work is **verified
done**, not intended — check items off in the same change that completes
them, with the evidence named.

**This file is not engineering status.** Capability is read from
`openehr/spec/conformance-matrix.md` and
`spec/databases/conformance-matrix.md`; open engineering findings live in the
three audit registers (`spec/audit.md`, `openehr/spec/audit.md`,
`spec/databases/audit.md`). Nothing here speaks for the matrices.

## Done (verified 2026-08-26, the state this file starts from)

- [x] Eight crates published at 0.6.0 (`openehr-loco` and the fuzz crates
      are `publish = false`); CI runs test/msrv/examples/
      bench/schema/fuzz/assets/layering/claims, with the library conformance
      matrix machine-derived (344 ids) and audit counts self-checked.
- [x] Root document set exists (mostly uncommitted): README, LICENSE.md,
      CITATION.cff, NEWS, COMPARISONS, BENCHMARKS, INSTALL, CONTRIBUTING,
      MAINTAINERS, CHANGELOG, AI_STATEMENT, GOVERNANCE, SECURITY, RFC,
      `.github/CODEOWNERS`.
- [x] SECURITY.md includes a "Known gaps in this project's own security
      posture" section verified against live repository settings on
      2026-08-26 (private vulnerability reporting, Dependabot, secret
      scanning: all disabled; commits/tags unsigned; no SBOM).
- [x] Outreach plan exists (`help/outreach/index.md`) with the governing
      claims rule and a readiness checklist that blocks on the conduct file.
- [x] Archetype Model scope change recorded honestly: `S1.4` withdrawn under
      `C0.19`, new `openehr/spec/15-archetypes.md`, **A-40** filed open (28
      of 32 requirements with no code), matrix re-derived.

## Next up

Grouped by `plan.md` workstream. Order within a group is priority order.

### Compliance — licensing and trademarks (highest exposure)

- [x] **Add `TRADEMARKS.md` and a visible non-affiliation notice.** The org,
      repository, and crate name all use the openEHR mark with no notice
      anywhere a reader looks first: README.md, LICENSE.md, CITATION.cff,
      and crate metadata carry nothing; the only statements are
      GOVERNANCE.md §Independence and `openehr/README.md:252`. Model:
      `er7-rust/TRADEMARKS.md` (mark-by-mark table, what is and is not
      claimed), adapted to openEHR International's trademark usage terms —
      read those terms first and record what they require.
      *Progress 2026-08-26: the notice half is done —
      `spec/professionalization/` rule 5 defines the verbatim notice
      (registration verified at openehr.org/logos/: U.S. 4,272,380, EUIPO
      002994853, IP Australia 939279), it is on the root documents and the
      published crates' rustdoc, and the two deferred files (`README.md`,
      `openehr/src/lib.rs`) landed with the Archetype Model change
      (`fe01c63`–`30273ef`). `TRADEMARKS.md` itself is still to write.*
      *Done 2026-08-27: `TRADEMARKS.md` at the root, on the `er7-rust`
      model — mark-by-mark table with the verified registrations, the
      verbatim notice, what is and is not claimed — unblocked by openEHR
      granting permission to use their trademarks (owner-reported,
      2026-08-27; correspondence held by the maintainer). Later the same
      day the owner directed the notice to reference the permission, and
      every notice site moved to the Foundation's prescribed attribution
      from openehr.org/logos/ — see `TRADEMARKS.md` §Permission.*
- [x] **Wire `scripts/check-trademarks.py` into CI** once the in-flight
      changes land. *Done 2026-08-26: the in-flight changes landed
      (`fe01c63`–`30273ef`), and CI now has a `trademarks` job running the
      same script a contributor runs locally, with the rows the `claims`
      gate requires in `AGENTS.md` and `spec/audit.md`; the
      `spec/professionalization/` Status entry that admitted the check was
      laptop-only is updated. Checker and `scripts/check-docs.py` both green
      locally before push.*
- [x] Add `LICENSES/` with the full text of all five licenses in the SPDX
      expression (REUSE convention; `fhir-rust/LICENSES/` is the model).
      *Done 2026-08-26: the five texts copied from the family model into
      `LICENSES/`, and root `LICENSE.md` references them file by file; the
      eighteen per-crate `LICENSE.md` copies keep stating the grant, with
      the full texts living once at the root.*

### In flight — land it

- [x] **Commit the untracked professionalization batch** (SECURITY.md,
      GOVERNANCE.md, CONTRIBUTING.md, MAINTAINERS.md, CITATION.cff,
      `.github/CODEOWNERS`, `help/`, `spec/serial-comma/`, AI_STATEMENT.md,
      BENCHMARKS.md, COMPARISONS.md, INSTALL.md, NEWS.md, RFC.md) together
      with the Archetype Model diff already in flight. *Done 2026-08-26 by
      the owner: landed across `3fafca2`–`30273ef` (conduct file, PHI.md,
      RFC.md, professionalization spec, trademark notices, CITATION/
      MAINTAINERS/CODEOWNERS, Archetype Model, 0.7.0), pushed, and the tree
      is clean — `git status` shows nothing uncommitted.*

### Security and supply chain

- [x] **Flip the three disabled repository settings** — private
      vulnerability reporting, Dependabot, secret scanning — and update
      SECURITY.md's known-gaps section in the same change. *Done 2026-08-26:
      private vulnerability reporting, Dependabot alerts, automated security
      fixes, and secret scanning enabled via the GitHub API, each verified
      with a `GET` after the change (push protection remains off, recorded
      in SECURITY.md). Every document repeating the gap updated in the same
      change: SECURITY.md (three places), PHI.md, RFC.md, plan.md,
      `spec/professionalization/` Status, `help/outreach/index.md`. Plus
      `.github/dependabot.yml` registering all eighteen workspaces with
      version-update PRs capped at zero — security-only posture, the limit
      chosen because the sibling `fhir-rust` got 47 major-bump PRs in an
      hour on default limits.*
- [ ] Sign commits and tags going forward; record the posture change in
      MAINTAINERS.md and SECURITY.md.
- [ ] Add release tags/attestation for 0.6.0 and future releases; consider
      crates.io Trusted Publishing.
- [x] Add `.github/ISSUE_TEMPLATE/` and a stated issue-response expectation.
      *Done 2026-08-26: bug-report template (synthetic-data-only warning,
      per SECURITY.md, and a redirect for security defects), a wrong-claim
      template (the contribution this repository values most), and
      `config.yml` linking private vulnerability reporting (now enabled)
      and the response expectation; MAINTAINERS.md states the expectation
      itself — issues read within a week, a target one person can keep,
      not a contract.*

### Governance

- [x] **Add `CODE_OF_CONDUCT.md`** (Contributor Covenant 2.1 plus the
      claim-accuracy clause from `fhir-rust`), closing the gap
      CONTRIBUTING.md:140 and the outreach readiness checklist both name;
      update both in the same change. *Done 2026-08-26: file at root with
      the claim-accuracy clause citing `W0.3` and the single-maintainer
      reporting limitation; CONTRIBUTING.md §Conduct and the outreach
      readiness table now point at it instead of naming the gap.*
- [x] Decide CODEOWNERS placement (`.github/CODEOWNERS` vs root — siblings
      use root) and align the family convention. *Done 2026-08-26: moved to
      `/CODEOWNERS` — all four siblings keep it at the root, and
      professionalization rule 8 syncs conventions through the family; the
      file's own comment records the rationale, and MAINTAINERS.md's link
      follows it.*

### Privacy and patient data

- [x] **Add root `PHI.md`** for a clinician/CISO reader, consolidating
      `openehr/spec/11-security.md` §PHI-in-output, the redaction behavior,
      and the compliance mappings — carrying the mappings' honesty forward
      (HIPAA §164.312(b)/(c)(1) Partial at the database layer; "components,
      not certified systems"). Model: `fhir-rust/PHI.md`'s Q&A table.
      *Done 2026-08-26: PHI.md at root; every row cites a `lib:`/`db:`
      requirement or a mapping; reads-not-audited (`db:PR12.5`), no-erasure
      (`db:M3.18`), and both Partial HIPAA rows carried forward verbatim in
      spirit; `scripts/check-docs.py` green after the change.*

### Outreach

- [x] Add a root `index.md` routing evaluators to the project documents
      (every sibling has one). *Done 2026-08-26: `index.md` at the root,
      modeled on `fhir-rust`'s routing tables scaled to this repository —
      evaluating / building / contributing / auditing routes, the
      crate-level table (checked against the matrix by
      `scripts/check-docs.py`), a one-hour reading order, and the rule 5
      trademark notice; every relative link verified to resolve.*
- [ ] Decide the site question (`plan.md` §Open decisions); if yes, the
      `hl7-rust`/`er7-rust` in-repo SvelteKit pattern is the model.
- [ ] Decide the funding question: CONTRIBUTING.md says no funding vehicle
      exists; either create one and add `.github/FUNDING.yml`, or keep the
      statement and record the decision.
- [ ] Execute `help/outreach/index.md`'s sequence only after the trademark
      notice, conduct file, and repository security settings above are done
      — its own checklist already blocks on them.

### Audit and harmonization

- [x] Re-sync `spec/special-files-for-public-repos/index.md` with the
      canonical `fhir-rust` version (the local 19-line copy omits
      CODE_OF_CONDUCT.md, PHI.md, TRADEMARKS.md, LICENSES/, FUNDING.yml,
      and index.md). *Done 2026-08-26: list synced with the canon (which
      itself lists neither TRADEMARKS.md nor index.md; index.md is noted in
      the status section, TRADEMARKS.md stays with its own open item
      above), status section written to this repository's actual state —
      everything exists except FUNDING.yml, whose absence is the recorded
      open decision — and the HL7 notes adapted to the openEHR rule 5
      machinery.*
- [ ] Decide whether `spec/databases/conformance-matrix.md` gets
      machine-derived like the library matrix (`plan.md` §Open decisions).

## Trademarks

openEHR® is the registered trademark of the openEHR Foundation and is used with
the permission of openEHR International. Use of the trademark does not
constitute endorsement of this product by openEHR International or openEHR
Foundation.
