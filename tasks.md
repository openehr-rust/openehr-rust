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

- [ ] **Add `TRADEMARKS.md` and a visible non-affiliation notice.** The org,
      repository, and crate name all use the openEHR mark with no notice
      anywhere a reader looks first: README.md, LICENSE.md, CITATION.cff,
      and crate metadata carry nothing; the only statements are
      GOVERNANCE.md §Independence and `openehr/README.md:252`. Model:
      `er7-rust/TRADEMARKS.md` (mark-by-mark table, what is and is not
      claimed), adapted to openEHR International's trademark usage terms —
      read those terms first and record what they require.
- [ ] Add `LICENSES/` with the full text of all five licenses in the SPDX
      expression (REUSE convention; `fhir-rust/LICENSES/` is the model).

### In flight — land it

- [ ] **Commit the untracked professionalization batch** (SECURITY.md,
      GOVERNANCE.md, CONTRIBUTING.md, MAINTAINERS.md, CITATION.cff,
      `.github/CODEOWNERS`, `help/`, `spec/serial-comma/`, AI_STATEMENT.md,
      BENCHMARKS.md, COMPARISONS.md, INSTALL.md, NEWS.md, RFC.md) together
      with the Archetype Model diff already in flight — run the CI `claims`
      checks and `scripts/check-docs.py` locally first. Until this lands,
      none of it is visible on GitHub. Ask before pushing.

### Security and supply chain

- [ ] **Flip the three disabled repository settings** — private
      vulnerability reporting, Dependabot, secret scanning — and update
      SECURITY.md's known-gaps section in the same change (it currently,
      correctly, says they are off as of 2026-08-26).
- [ ] Sign commits and tags going forward; record the posture change in
      MAINTAINERS.md and SECURITY.md.
- [ ] Add release tags/attestation for 0.6.0 and future releases; consider
      crates.io Trusted Publishing.
- [ ] Add `.github/ISSUE_TEMPLATE/` and a stated issue-response expectation.

### Governance

- [x] **Add `CODE_OF_CONDUCT.md`** (Contributor Covenant 2.1 plus the
      claim-accuracy clause from `fhir-rust`), closing the gap
      CONTRIBUTING.md:140 and the outreach readiness checklist both name;
      update both in the same change. *Done 2026-08-26: file at root with
      the claim-accuracy clause citing `W0.3` and the single-maintainer
      reporting limitation; CONTRIBUTING.md §Conduct and the outreach
      readiness table now point at it instead of naming the gap.*
- [ ] Decide CODEOWNERS placement (`.github/CODEOWNERS` vs root — siblings
      use root) and align the family convention.

### Privacy and patient data

- [ ] **Add root `PHI.md`** for a clinician/CISO reader, consolidating
      `openehr/spec/11-security.md` §PHI-in-output, the redaction behavior,
      and the compliance mappings — carrying the mappings' honesty forward
      (HIPAA §164.312(b)/(c)(1) Partial at the database layer; "components,
      not certified systems"). Model: `fhir-rust/PHI.md`'s Q&A table.

### Outreach

- [ ] Add a root `index.md` routing evaluators to the project documents
      (every sibling has one).
- [ ] Decide the site question (`plan.md` §Open decisions); if yes, the
      `hl7-rust`/`er7-rust` in-repo SvelteKit pattern is the model.
- [ ] Decide the funding question: CONTRIBUTING.md says no funding vehicle
      exists; either create one and add `.github/FUNDING.yml`, or keep the
      statement and record the decision.
- [ ] Execute `help/outreach/index.md`'s sequence only after the trademark
      notice, conduct file, and repository security settings above are done
      — its own checklist already blocks on them.

### Audit and harmonization

- [ ] Re-sync `spec/special-files-for-public-repos/index.md` with the
      canonical `fhir-rust` version (the local 19-line copy omits
      CODE_OF_CONDUCT.md, PHI.md, TRADEMARKS.md, LICENSES/, FUNDING.yml,
      and index.md).
- [ ] Decide whether `spec/databases/conformance-matrix.md` gets
      machine-derived like the library matrix (`plan.md` §Open decisions).
