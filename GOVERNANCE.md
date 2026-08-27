# Governance

**Who decides, on what basis, and where the decision is written down.**

The short version: one person decides, and **the specification decides what must
be true**. Those are not in tension — the first is about authority, the second
about evidence, and the second is what a reader can check years later without
asking anyone.

## Decision rights

One maintainer, listed in [`MAINTAINERS.md`](MAINTAINERS.md), which also states
the bus factor, the publishing identities, and what happens if that person is
unavailable. Nothing in this file softens any of it.

There is no board, no steering committee, no voting, and no legal entity. There
is also no pretence otherwise: a governance document describing bodies that do
not exist is the same defect as a README describing tests that do not run.

## The specification governs, not memory

This repository is developed specification-first, and that is a governance rule
before it is an engineering one:

- **A requirement is written before the commit lands** (`W0.19`). Discovering a
  requirement while implementing is normal; shipping the behaviour without
  writing the requirement is how a decision becomes untraceable.
- **A README, a rustdoc comment, a tutorial, `AGENTS.md`, or a commit message is
  descriptive** (`W0.2`). Where such a file and a specification disagree, the
  specification governs and the other file has a defect — to be fixed, not
  reconciled by amending the specification to match.
- **Where this project and openEHR disagree without a declared departure,
  openEHR governs and this project has a defect** (`C0.3`).
- **Requirement identifiers are permanent** (`C0.5`). Never renumbered, never
  reused. A citation written into code five years ago still resolves.

## Where each kind of decision is made and recorded

| Decision | Made by | Recorded in |
| --- | --- | --- |
| What the software must do | a numbered requirement | [`spec/`](spec/index.md), [`openehr/spec/`](openehr/spec/index.md), [`spec/databases/`](spec/databases/index.md) |
| Reversing an earlier decision | withdrawal in place, reason preserved (`C0.19`) | the requirement's own text, marked |
| Amending a requirement | edit in place, identifier kept (`C0.15`), and an amendment made to match existing code must say so (`C0.16`) | the requirement, dated |
| A crate's conformance level | evidence, then the matrix — which owns it (`W0.40`) | [`spec/databases/conformance-matrix.md`](spec/databases/conformance-matrix.md), [`openehr/spec/conformance-matrix.md`](openehr/spec/conformance-matrix.md) |
| A known gap or defect | a numbered finding with evidence (`W0.4`, `C0.9`) | the audit registers |
| A release, and its version number | the publishing procedure and its gate (`W0.20`, `W0.21`) | [`agents/publishing.md`](agents/publishing.md), [`CHANGELOG.md`](CHANGELOG.md) |
| Whether a change is breaking | the reasoning, written out | `CHANGELOG.md` — 0.6.0 is the worked example of why a representation change was not a patch |

**A decision that exists only in an issue comment, a chat message, or one
person's head is not a decision this project has made.** If it matters, it is in
the tree; if it is in the tree, it is checkable.

## How disagreement is settled

By evidence, in this order:

1. **Run it.** Every finding in these registers was found by running something,
   none by reading. A server transcript, a failing test, or a reproduction
   outranks any opinion here, including the maintainer's.
2. **Cite it.** An openEHR specification section, or a requirement id.
3. **If the specification is wrong**, that is the most valuable kind of report,
   and the specification is amended — in place, keeping its identifier, saying
   what changed and why. `S1.4` is the worked example: an exclusion that stood
   from the first release, withdrawn in 2026 with its reasoning kept, because
   the reasoning is what the replacement had to answer.
4. **A claim beyond what was verified is a defect no matter who made it**
   (`W0.3`). The maintainer's own claims have been the ones caught most often;
   the registers say which.

If a disagreement survives all of that, the maintainer decides and writes down
why. That is what a bus factor of one means, and the remedy is the same one
[`MAINTAINERS.md`](MAINTAINERS.md) names: the licence is permissive, the history
is public, the reasoning is in the tree, and a fork is a legitimate continuation.

## Becoming a maintainer

The route and the three edits that implement it are in
[`MAINTAINERS.md`](MAINTAINERS.md); what a contribution is held to is in
[`CONTRIBUTING.md`](CONTRIBUTING.md). Neither is restated here, deliberately —
`W0.1` puts each statement in exactly one place, and a governance file that
paraphrases the roster is how the two come to disagree.

## Machines do not decide

This project is developed with agentic AI assistance, disclosed in
[`AI_STATEMENT.md`](AI_STATEMENT.md). For governance the relevant rules are
that no tool merges, publishes, signs, adjudicates a specification question, or
decides whether a conformance level has been earned, and that a tool is never
named as an author or signer. One named human is accountable for every change,
whatever produced the bytes.

## Independence

**This project is not affiliated with, endorsed by, or certified by openEHR
International**, and no crate here has been through openEHR's conformance
programme. The conformance ladder used in this repository — Dialect, Schema,
Store, Verified — is **this project's own** and means what
[`spec/index.md`](spec/index.md) says it means. Do not read it as an openEHR
certification, and do not let a procurement document read it that way either.

openEHR's specifications are the authority for what openEHR is; this repository
is one implementation's reading of them, with its disagreements written down as
declared departures.

openEHR granted this project permission to use their trademarks (2026-08-27,
owner-reported; [`TRADEMARKS.md`](TRADEMARKS.md) §Permission is the record).
Permission to use a mark is not affiliation: everything in the paragraph above
remains true and stays.

## Changing this file

By pull request, like everything else. This file is **descriptive** (`W0.2`) —
the normative rules it points at live in the specification tree, and changing
one of those means changing the requirement, not this page.

## Trademarks

openEHR® is the registered trademark of the openEHR Foundation and is used with
the permission of openEHR International. Use of the trademark does not
constitute endorsement of this product by openEHR International or openEHR
Foundation.
