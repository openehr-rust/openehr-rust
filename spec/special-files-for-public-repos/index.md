# Special files for public repos

Special files that use top-level markdown:

- README.md
- LICENSE.md with SPDX license information
- CITATION.cff with ORCID citation for Joel Parker Henderson (joel@joelparkerhenderson.com) (see ~/git/assertables/assertiables/CITATION.md for template)
- NEWS.md with news, update information, press contacts, etc.
- COMPARISONS.md comparisons to relevant projects, context, etc.
- BENCHMARKS.md with any benchmarks, speed tests, optimizaiton profiles, etc.
- INSTALL.md how to install and use any of the software
- CONTRIBUTING.md how a person can contribute their time, or update code, or donate money
- CODEOWNERS with joel@joelparkerhenderson.com
- MAINTAINERS.md with Joel Prker Henderson (joel@joelparkerhenderson.com) as sole maintainer (use this as template: https://github.com/rubentalstra/FerroEHR/blob/develop/MAINTAINERS.md)
- CHANGELOG.md with change log history summries
- AI_STATEMENT.md (use this as template: https://github.com/rubentalstra/FerroEHR/blob/develop/AI_STATEMENT.md)
- GOVERNANCE.md how decisions are made, what binds them, how to disagree, how to become a maintainer
- SECURITY.md how to report a vulnerability, what is in scope, response windows, known open issues
- CODE_OF_CONDUCT.md Contributor Covenant 2.1, plus this project's claim-accuracy clause
- PHI.md what the software does and does not do with patient data, in plain language
- RFC.md the open questions this project wants answered, and what feedback helps
- LICENSES/ the full text of every licence the SPDX expression offers (REUSE convention)
- .github/FUNDING.yml the donation routes CONTRIBUTING.md points at

## Status in this repository

Re-synced with the canonical `fhir-rust` copy on 2026-08-26 — the local list
had predated the family's and omitted CODE_OF_CONDUCT.md, PHI.md, RFC.md's
current description, LICENSES/, and FUNDING.yml. Three notes, honest to this
repository rather than inherited:

- **Everything on the list exists here as of 2026-08-26 except
  `.github/FUNDING.yml`**, whose absence is a decision rather than a gap:
  CONTRIBUTING.md states plainly that no funding vehicle exists, so adding
  the file means creating one — an open decision in `plan.md` §Open
  decisions and `tasks.md` §Outreach, asked openly in RFC.md. When it is
  decided, the files must agree, whichever way it goes.
- **CODEOWNERS lives at the repository root**, matching all four siblings
  (moved 2026-08-26; the file's own comment carries the rationale).
- **The openEHR® trademark rules are met by all of these files.**
  `spec/professionalization/` rule 5 requires every root document using the
  mark in prose to carry the notice verbatim; `scripts/check-trademarks.py`
  verifies it and runs in the CI `trademarks` job on every push.

Beyond the list, this repository also keeps a root `index.md` routing
evaluators to the project documents, as every sibling does.

## Trademarks

openEHR® is the registered trademark of the openEHR Foundation. Use of the
trademark does not constitute endorsement of this product by openEHR
International or openEHR Foundation.
