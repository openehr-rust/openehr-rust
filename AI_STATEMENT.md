# AI statement

| | |
| --- | --- |
| Version | 1.0.0 |
| Effective date | 2026-08-26 |
| Status | Active |
| Owner | Joel Parker Henderson, sole maintainer ([`MAINTAINERS.md`](MAINTAINERS.md)) |
| Canonical location | `AI_STATEMENT.md` at the repository root |
| Review | at every release, and on any trigger in §9 |

**What this is.** A disclosure of how artificial-intelligence tools are used to
build this software, written for the person doing supplier due diligence on a
library that will hold clinical records. It is a self-declaration by the
maintainer. It is not a certification, it has not been audited, and the only
reason to believe it is that the repository contains machine-checked artefacts
that would contradict it if it were false (§5).

**Not normative** (`W0.2`): the specifications in [`spec/`](spec/index.md)
decide what must be true of the code. This document describes practice.

## 1. Scope, and the thing that is not in scope

This covers the use of AI tools to produce everything in this repository: crate
code, tests, fuzz targets, the SQL dialects, the specification tree, the
operational guides, and this file.

**This software ships no AI.** No model is trained, embedded, downloaded, or
called. Nothing in any of the eighteen crates performs inference, and
`openehr-loco` — the HTTP service — talks to a database and nothing else. AI is
used to *write* this software, in the sense that a compiler and a linter are
used to build it.

## 2. Which rules actually apply

Borrowed authority is worse than none, so, plainly:

- **The EU AI Act places no obligation on this project.** It binds providers and
  deployers of AI systems. This project is not one, because it contains none.
  This disclosure is voluntary.
- **This is not a medical device and makes no clinical claim.** These crates
  store, validate, and retrieve records. A downstream product that gives that a
  medical purpose brings *itself* into scope, and that classification is the
  integrator's to make — this file exists partly so they can answer their own
  supplier questions.
- **No standard is claimed as conformity.** The words *certified*, *audited*,
  *validated*, and *compliant* apply to nothing here, and appear in this
  document only in this sentence to say so.

## 3. Vocabulary

Borrowed from the W3C AI Content Disclosure vocabulary rather than invented:
**none** (entirely human-authored), **ai-assisted** (human-authored, AI edited
or completed), **ai-generated** (AI-produced under human prompting and review),
**autonomous** (AI-produced without meaningful human oversight). An **agentic
tool** plans and executes multi-step work — reading files, editing them, running
the tests — under a human's direction.

## 4. Accountability, and where AI is used

One named human is the author of, and accountable for, every change in this
repository, whatever produced the bytes. **A tool is never named as an author,
co-author, or signer**, because responsibility that cannot be borne cannot be
assigned. There is no AI sign-off of anything.

The tooling is agentic coding assistance — currently Claude Code, by Anthropic —
run in sessions the maintainer directs, reads, and merges. The repository is
explicit about this to the point of carrying instructions for it:
[`CLAUDE.md`](CLAUDE.md), [`AGENTS.md`](AGENTS.md), and
[`agents/`](agents/index.md) are addressed to those sessions, and their contents
are a fair description of how the work is actually done.

| Activity | Level | Notes |
| --- | --- | --- |
| Crate code, SQL dialects, tests, fuzz targets | ai-generated | written against the openEHR specifications and this tree's own requirement identifiers; every change passes §5 before it lands |
| Specifications in `spec/` and `openehr/spec/` | ai-generated | drafted in session, adopted by the maintainer. `W0.19` requires the requirement to exist before the commit, which makes the *order* of the work reviewable |
| Documentation, including this file | ai-generated | held to the same claim discipline as the code |
| What a specification silence means; what ships in a release; whether a conformance level is earned | none | the maintainer's, and recorded in the tree — the conformance matrix, the audit register, `agents/publishing.md` |
| Merging, publishing, tagging | none | manual, by the one account that can (`MAINTAINERS.md`) |

**No row says autonomous**, and no percentage appears anywhere in this document,
because no defensible way to measure one exists.

## 5. The controls, and what each actually proves

AI-produced work gets no shortcut around process. The gates are committed
scripts, not intentions, and they are the reason this disclosure is checkable:

- **Continuous integration runs ten jobs** on every change: `test`, `msrv`,
  `examples`, `bench`, `schema`, `fuzz`, `assets`, `layering`, `claims`, and
  `mutants`. Their names and purposes are tabulated in [`AGENTS.md`](AGENTS.md)
  and in [`spec/audit.md`](spec/audit.md), and `scripts/check-docs.py` fails if
  the tables and the workflow disagree.
- **Mutation testing is the control that catches plausible-but-untested code**,
  which is the characteristic failure of generated work. It has already earned
  its keep: the run before 0.6.0 went red because `DvQuantity::accuracy_real`
  survived mutation — an accessor that existed, compiled, read correctly, and was
  called by no test. Everything else was green. That release waited.
- **Fuzzing** — eight harness crates, twenty-one targets — with the properties
  living in `openehr_store::conformance` rather than duplicated per crate. A
  fuzz job on this repository's default branch once stayed red for seventeen
  days holding a real defect (`lib:A-37`), which is recorded rather than
  smoothed over.
- **Executed schema verification.** `openehr-store/scripts/verify-schema.sh`
  runs a dialect's DDL against a real database server and round-trips canonical
  JSON bytes through it. This is what separates the **Schema** level from the
  **Dialect** level, and it is why `db:D-08` — MySQL's `JSON` type rewriting a
  stored magnitude of `1.10` as `1.1` — is a measurement rather than a worry.
- **Documentation is machine-checked** (`scripts/check-docs.py`): crate counts,
  the published version, the conformance level of every crate against its single
  owner, duplicated passages byte for byte, and the audit register against its
  own summary. A generated document that overstates something is the failure
  mode this script exists for.
- **Lints as gates**: `unsafe_code` is `forbid` — in all eighteen crates'
  manifests *and* as `#![forbid(unsafe_code)]` at every crate root and fuzz
  target — `missing_docs`,
  `missing_errors_doc`, and `missing_panics_doc` are `deny`, clippy runs at
  `pedantic`, and CI sets `RUSTFLAGS="-D warnings"`. Rustdoc examples are
  compiled and run, and adding `no_run` or `ignore` to make one pass is
  prohibited by the contributor guide, because it converts a checked claim into
  an unchecked one.
- **Invariant accounting**: `openehr-assets` fails the build if a Reference
  Model invariant is neither cited at a call site nor dispositioned with a
  reason (`lib:A-24`, `lib:A-25`). Coverage of the standard is a build product,
  not a claim.
- **An audit register that is not optional** ([`spec/audit.md`](spec/audit.md)).
  `W0.4` — a gap not written down reads as a pass. Every finding in it was found
  by running something, none by reading, and several of them are defects that
  generated code introduced and a gate caught.

**What none of this proves**: that the code is correct. It proves the behaviours
the tests, fuzzers, and servers actually exercised, which is a boundary, and the
conformance matrix publishes where the boundary is.

## 6. Licensing and provenance

The project offers a five-way licence choice ([`LICENSE.md`](LICENSE.md)). The
position on generated output follows the reasoning published by the Apache
Software Foundation and LLVM rather than a convenient shortcut: a tool's output
does not launder anyone's copyright, the provenance of generated text is not
fully knowable, and prompting alone is not treated as authorship. In practice,
generated code is held to the same originality expectation as human code under
the same review, and identifiable third-party material found in the tree is
removed or licensed properly exactly as it would be if a human had pasted it.
The tools are used under terms that do not restrict the output's use under these
licences.

## 7. Data

**No patient data, no personal health information, and no customer data exists
anywhere in this repository** — not in fixtures, not in examples, not in the
committed assets, and therefore not in any prompt. Test data is constructed in
code or derived from the openEHR specifications' own examples, which are
modelling artefacts rather than records about people. This is a structural
property of a public tree that a reader can check, not a promise about a
vendor's behaviour. How the tool vendor handles session data is governed by that
vendor's terms; this document makes no claim on their behalf, because such
claims go stale silently.

## 8. Contributors, and prohibited uses

Contributors may use AI tools. A contribution containing **ai-generated** content
should say so in the pull request description — which tool, and what it did —
rather than in commit trailers. The contributor remains fully responsible for
the submission: understood, explainable on request, tested, and honest.

In this project, AI **must not**: merge anything; publish anything; sign
anything; decide whether a conformance level has been earned; weaken a test, a
lint, or a gate to make something pass; or add `no_run`, `ignore`, or a `_ =>`
arm to silence a check that is doing its job. The last is written into
[`CLAUDE.md`](CLAUDE.md) because it is the temptation that actually arises.

## 9. Limitations, and the residual risk

A disclosure without this section is marketing.

- **Review depth is one person's.** The machine gates stand in for the review
  capacity a larger team would have. "The maintainer understands and can explain
  every merged change" is the honest claim; "every line was independently
  re-derived" is not.
- **The tree is young.** Public history begins 2026-08-01. Time is a control
  this project does not yet have.
- **One crate is Verified.** `openehr-sqlite` is checked against a real engine in
  CI; the other engines state a lower level for a reason. Do not read the
  breadth of the dialect list as breadth of evidence.
- **Retroactivity.** Commits predating this file carry no per-change disclosure
  marker, and none is claimed. This describes the practice, not an audit trail.
- **Provenance uncertainty survives.** Whether a generated fragment echoes
  training material is not fully knowable with current tooling. §6 states the
  handling, not a guarantee.
- **The legal ground is unsettled**, and the positions here may have to change.
- **This is self-declared.** The counterweight is §5: those artefacts can
  disagree with this document, and if they ever do, this document is what is
  wrong.

**Revision triggers**: the tooling changes materially; a vendor's terms change in
a way §6 or §7 depends on; a binding rule appears; or a claim here stops being
true. The change lands as an ordinary commit, and Annex A gains a row.

## 10. Reporting

A suspected provenance, licensing, or quality problem — including a claim in
this file that does not survive checking — is a report this project wants. Open
an issue and cite this file. For anything security-sensitive, email
joel@joelparkerhenderson.com rather than opening a public issue.

## Annex A. Change log

| Version | Date | Change |
| --- | --- | --- |
| 1.0.0 | 2026-08-26 | First issue. |

## Annex B. Machine-readable summary

Levels per §3. The prose above is authoritative wherever the two could disagree.

```yaml
ai-statement:
  version: 1.0.0
  last-updated: 2026-08-26
  vocabulary: w3c-ai-content-disclosure
  disclosure-default: ai-generated
  tools:
    - name: Claude Code
      provider: Anthropic
  processes:
    design: ai-generated
    implementation: ai-generated
    testing: ai-generated
    documentation: ai-generated
    review: none
    adjudication: none
    release-decisions: none
  ships-ai-system: false
  autonomous-use: none
```

---

Structure adapted from [FerroEHR's `AI_STATEMENT.md`](https://github.com/rubentalstra/FerroEHR/blob/develop/AI_STATEMENT.md)
(MIT), which set the bar for this kind of disclosure in the openEHR community.
The facts, the controls, and the limitations are this project's own.

## Trademarks

openEHR® is the registered trademark of the openEHR Foundation. Use of the
trademark does not constitute endorsement of this product by openEHR
International or openEHR Foundation.
