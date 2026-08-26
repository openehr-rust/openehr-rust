# 0. Conformance

Requirement prefix: `C0`.

This section defines the language the rest of the specification is written in,
and what a claim made in it means.

## Normative language

- **C0.1** The keywords MUST, MUST NOT, REQUIRED, SHALL, SHALL NOT, SHOULD,
  SHOULD NOT, RECOMMENDED, MAY, and OPTIONAL are to be interpreted as described
  in RFC 2119.
- **C0.2** A sentence without one of those keywords is explanatory. It gives the
  reason for a requirement; it is not itself one.
- **C0.3** Where this specification and the openEHR specifications conflict, and
  this specification does not explicitly declare a departure, the openEHR
  specification governs and this document has a defect. Record it in
  [`audit.md`](audit.md).

## Requirement identifiers

- **C0.4** Every normative statement MUST carry an identifier of the form
  `<prefix><section>.<ordinal>[<suffix>]`, where the prefix is fixed per section
  by [the index](index.md) — `D3.7`, `X11.7a`, `Q12.9`.
- **C0.5** Identifiers are **permanent**. They MUST NOT be renumbered and MUST
  NOT be reused for a different requirement.
  - Withdrawing a requirement keeps its number, marked withdrawn.
  - Amending a requirement keeps its number.
  - Splitting one uses letter suffixes (`D3.7` → `D3.7a`, `D3.7b`); the parent
    survives and says what it was split into.
  - Adding one takes the next unused ordinal in the section. Never insert
    mid-sequence by shifting.
- **C0.6** Section numbers have no gaps in this specification, and any that
  appear later MUST NOT be closed.

## What a conformance claim means

- **C0.7** A requirement is **satisfied** when the code implements it and a test
  demonstrates it.
- **C0.8** A requirement is **unverified** when the code appears to implement it
  and no test exercises it. Unverified is recorded as `?` in the
  [conformance matrix](conformance-matrix.md), never as satisfied.
- **C0.9** A gap that is not written down reads as a pass. Every known gap
  between this specification, the documentation, and the code MUST appear in
  [`audit.md`](audit.md) with evidence a reader can check.
- **C0.10** A test that cannot fail does not verify anything. Every test cited
  as evidence for a requirement MUST have been shown to fail when the behaviour
  it checks is removed or inverted (`T13.2`).
- **C0.11** Documentation MUST NOT claim more than is verified. In particular,
  "the same code path works elsewhere" is not evidence for a requirement here.

## Departures from openEHR

- **C0.12** Where this crate does not do what an openEHR specification says, it
  MUST be recorded as a numbered requirement that names what it departs from and
  states what holds instead.
- **C0.13** A departure MUST state which of the amended requirement's guarantees
  survive, so a reader can tell a narrowing from a contradiction.
- **C0.14** Implementing something different from openEHR and mentioning it only
  in a code comment is a defect, not a departure.

## Amending this specification

- **C0.15** An amendment edits the section in place, keeping the identifier
  (`C0.5`), and states the reason in the commit message.
- **C0.16** An amendment made to match what the code already does MUST say so.
  A considered generalization and a rubber stamp are indistinguishable
  afterwards.
- **C0.17** Every amendment MUST be checked against the
  [conformance matrix](conformance-matrix.md) — does it change a status? — and
  against [`audit.md`](audit.md) — does it close a finding, or open one?
- **C0.18** A requirement withdrawn because the behaviour was removed MUST keep
  its identifier and be marked *withdrawn*, with the release in which it went.
- **C0.19** *(added 2026-08-26)* An **exclusion that is reversed** — a `MUST NOT`
  that becomes something the crate does — MUST be withdrawn rather than deleted
  or rewritten: it keeps its identifier (`C0.5`), is marked *withdrawn* with the
  date, names the section that now specifies the behaviour, and **preserves the
  reason it gave**.

  The reason is the part worth keeping. An exclusion is an argument about a risk,
  and reversing the decision does not retire the risk — it transfers it to
  whoever now has to answer it. `S1.4` is the first case: it excluded the
  Archetype Model because a partial constraint engine lets "valid" mean "the
  parts I understood were satisfied", and §15 has to answer that sentence rather
  than inherit a blank page.

## Vocabulary

| Term | Means |
| --- | --- |
| **the crate** | the `openehr` Rust crate this specification governs |
| **the caller** | the program using the crate |
| **the deployment** | the system the caller runs in: its storage, its perimeter, its policies |
| **PHI** | protected health information — anything identifying or clinical about a person |
| **refuse** | return an error rather than a value; never a plausible default |
| **carry opaquely** | preserve on read and write without interpreting |
