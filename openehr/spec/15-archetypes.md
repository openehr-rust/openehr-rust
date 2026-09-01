# 15. Archetypes, templates, and knowledge artefacts

Requirement prefix: `K15`.

## Why this section exists

Until 2026-08-26 the crate's scope said the opposite of this section. `S1.4`
excluded the Archetype Model outright, and gave a reason worth keeping:

> An archetype is a constraint language with its own parser and its own
> conformance rules; implementing a partial one would let "valid" mean "the
> parts I understood were satisfied".

**That reason did not stop being true when the decision reversed.** `S1.4` is
withdrawn (`C0.19`), not deleted, and this section is written to answer the
argument it made rather than to ignore it. The answer is `K15.6`, `K15.9`,
`K15.17`, and `K15.24`: everywhere a construct is not implemented, an artefact
is incomplete, or a repository is unreachable, the result is an explicit refusal
and never a pass. A partial constraint engine is still prohibited. What changed
is that the refusal now lives inside an implementation instead of standing in
for one.

**Fourteen of these requirements are implemented; eighteen are not.**
`K15.1`–`K15.4` — the AOM2 object model — landed as `openehr::am` on
2026-08-26; `K15.18`–`K15.23` — validating a Reference Model instance against
an archetype, as a separate verdict from Reference-Model validation, never a
partial pass — landed as `openehr::am::validate` on 2026-08-30; and
`K15.24`–`K15.27` — a repository abstraction, and resolving a
`C_ARCHETYPE_ROOT` filler through one, `openehr` itself performing no I/O
(`K15.25`) — landed as `openehr::am::repository` and
`validate_with_repository` the same day. All three with tests named in the
[conformance matrix](conformance-matrix.md). What remains is still real: no
ADL parser, no flattening, no template expansion, no operational template.
`validate_with_repository` validates each resolved `Archetype` **as given**,
not as a flattened OPT2 would be — flattening (`K15.11`) and template
expansion (`K15.14`) do not exist to merge a specialisation's inherited
constraints in first, and there is still no parser to read one from ADL text.
A bare `ARCHETYPE_SLOT` stays unchecked even with a repository supplied: which
archetype filled it is recorded on the instance's `ARCHETYPED.archetype_id`,
an attribute `crate::path::Node` does not expose, so nothing here can name
what to resolve — a gap in `crate::path`, stated rather than worked around.
So the crate can now tell you whether a `COMPOSITION` conforms to an
archetype you built, already have, or can retrieve through a repository you
supply, and still cannot tell you whether it conforms to the *published*
archetype unless whatever produced or retrieved that `Archetype` already did
the flattening by hand. [`audit.md`](audit.md) **A-40** keeps the remaining
gap visible until the code closes it (`C0.9`), and `K15.30` is what stops the
documentation from moving
before the code does.

## Vocabulary

| Term | Means here |
| --- | --- |
| **AOM2** | the openEHR Archetype Object Model, release 2 — the object model an archetype parses into |
| **ADL 2** | Archetype Definition Language 2, the authored source form of an AOM2 archetype |
| **ADL 1.4** | the earlier syntax, in which most of the published clinical corpus is still written |
| **flat archetype** | a specialised archetype with its parents' constraints merged in |
| **template** | an artefact that specialises archetypes and fills their slots for a local purpose |
| **operational template (OPT)** | the fully flattened, self-contained artefact a runtime validates against |
| **CKM** | the openEHR Clinical Knowledge Manager, the repository the published corpus is governed in |

## 15.1 The object model

- **K15.1** The crate MUST implement AOM2 as Rust types — `ARCHETYPE`,
  `C_COMPLEX_OBJECT`, `C_ATTRIBUTE`, the `C_OBJECT` descendants,
  `C_PRIMITIVE_OBJECT` and its constraint kinds, `ARCHETYPE_SLOT`,
  `ARCHETYPE_TERMINOLOGY`, and the resource descriptors — with
  construction-time invariant checking, on the same terms `S1.1` sets for the
  Reference Model.
- **K15.2** The crate MUST name the AOM release it targets, in the way `S1.16`
  names RM 1.1.0, and MUST carry an artefact's declared version rather than
  enforcing it. An archetype authored against an older AM is readable, and what
  it declares is preserved so a caller can decide.
- **K15.3** Every AOM2 type MUST round-trip losslessly through the
  serialisations the crate accepts (§9 governs the JSON form). `S1.13` applies
  unchanged: not interpreting a construct is not a licence to lose it.
- **K15.4** An AOM2 instance MUST be constructible in memory without a parser.
  A caller that builds constraints programmatically — a test, a generator, a
  tool — MUST NOT be forced through ADL text, and the parser MUST NOT be the
  only way to reach a valid model.

## 15.2 Parsing

- **K15.5** The crate MUST parse **ADL 2** into the AOM2 model: header,
  specialisation, language, description, definition, rules, terminology, and
  annotations.
- **K15.6** **A construct the parser does not implement MUST be a refusal that
  names it** (`S1.12`), attributed to its position in the source. It MUST NOT be
  skipped, defaulted, or carried as an opaque blob that later reads as "no
  constraint". This requirement is the whole reason the exclusion in `S1.4`
  could be withdrawn; a parser that recovers by ignoring what it did not
  understand produces exactly the silent pass `S1.4` predicted.
- **K15.7** The parser MUST NOT resynchronise by skipping to the next section
  after an error. Partial parses are not returned.
- **K15.8** The crate MUST parse **ADL 1.4** archetypes and convert them to
  AOM2, because the published corpus is largely still written in it and a
  library that cannot read the corpus cannot validate against it. A converted
  archetype MUST record its provenance — source syntax, source text digest, and
  the conversion the crate performed — so that a 1.4-derived constraint is
  distinguishable from an authored ADL 2 one at every later step.

  Still not implemented, in this sense, as of the addition below.
  `am::adl14::parse_header` reads an ADL 1.4 archetype's `archetype` and
  `concept` lines only — an identifier and a term code, nothing that reaches
  `definition`, `terminology`, or any other section — and does not build an
  `Archetype`, which needs both. It is useful for identifying and cataloguing
  `.adl` source, and it is not a step toward this requirement: satisfying
  `K15.8` needs the full cADL and ODIN grammars this function does not parse,
  which `spec/audit.md` **A-40**'s residual already scopes at several weeks
  of work. Recorded here so the addition cannot later be misread as partial
  progress on `K15.8` itself.
- **K15.9** Where a 1.4 construct has no faithful AOM2 equivalent, conversion
  MUST fail naming the construct. An approximate conversion is prohibited: it
  produces an archetype that no author wrote and no reviewer approved.
- **K15.10** The crate MUST parse the assertion language used in `rules` and in
  slot fillers, and MUST define the subset it evaluates. An artefact whose
  assertions fall outside that subset MUST be refused **for validation
  purposes** (`K15.17`) rather than validated with those assertions ignored.
  Carrying them losslessly is still required (`K15.3`).

## 15.3 Specialisation and flattening

- **K15.11** The crate MUST implement AOM2 flattening: a specialised archetype
  combined with its ancestors into a flat archetype whose constraints are the
  ones a runtime applies.
- **K15.12** Flattening MUST refuse when an ancestor is unavailable. A flat
  archetype built from an incomplete lineage is a constraint set nobody
  authored, and it is indistinguishable from a complete one once built.
- **K15.13** The crate MUST check **specialisation conformance**: a specialised
  archetype narrows its parent and never widens it. A specialisation that widens
  MUST be reported as a defect in the artefact, not silently flattened.

## 15.4 Templates and operational templates

- **K15.14** The crate MUST expand a template: resolve each `ARCHETYPE_SLOT`
  against the artefact that fills it, apply the template's own overlays, and
  produce a flat, self-contained result.
- **K15.15** The crate MUST produce and consume an **operational template**
  (OPT2) — the flattened artefact with its terminology included — and OPT2 is
  the crate's normative internal form for validation input.
- **K15.16** The crate MUST ingest the legacy **OPT 1.4** operational template,
  converting to the internal form under `K15.8`'s provenance rule and `K15.9`'s
  refusal rule. Deployed openEHR systems emit it today; a validator that cannot
  read what the tooling produces validates nothing in practice.
- **K15.17** **A template MUST NOT weaken what it constrains.** Expansion MUST
  check that every overlay narrows the archetype it applies to, and MUST refuse
  a template that widens one. The narrowing direction is the property that makes
  an operational template safe to validate against at all.

## 15.5 Validating data against an archetype

- **K15.18** The crate MUST validate a Reference Model instance against an
  operational template: node identity and archetype path, occurrences,
  cardinality, existence, primitive value constraints, internal terminology
  codes, and slot fills.
- **K15.19** **Reference-Model validation and archetype validation MUST be
  reported as separate verdicts.** "This is not a valid `COMPOSITION`" and "this
  is a valid `COMPOSITION` that does not conform to this template" are different
  facts about a document, and a caller repairs them differently. `L10.2` is
  amended to say so.
- **K15.20** **No partial pass.** If any constraint in the template uses a
  construct the crate does not implement, or names an artefact that could not be
  resolved, the affected node MUST be reported as *unchecked*, and the overall
  verdict MUST NOT be *conformant*. An unchecked node is not a passing node.
- **K15.21** A violation MUST name the archetype path, the archetype or template
  id, and the constraint that failed, and MUST NOT include node content
  (`X11.7`), on the same terms `L10.4` and `L10.5` set for Reference-Model
  violations.
- **K15.22** External terminology bindings remain unresolved: `S1.10` still
  governs, so a binding to SNOMED CT or LOINC MUST be reported as **unchecked**,
  never as satisfied. Internal `at`- and `ac`-codes are checked against the
  artefact's own terminology, which the crate has in hand.
- **K15.23** Validation MUST be deterministic and offline: given the same
  instance and the same operational template, the verdict MUST NOT depend on
  network state, wall-clock time, or retrieval order. A clinical verdict that
  cannot be reproduced cannot be audited.

## 15.6 Retrieval

- **K15.24** The crate MUST define a repository abstraction — resolve an
  artefact by identifier, resolve a slot filler, and answer *not found* — and
  every validation entry point MUST take its artefacts from one.
- **K15.25** **`openehr` MUST NOT perform network or filesystem I/O.**
  Retrieval implementations, CKM included, live in a separate crate that depends
  on `openehr` and carries the obligations every crate here carries — the licence
  expression, the `LICENSE.md`, and the README terms of `W0.22`–`W0.24`, declared
  in [`../../spec/index.md`](../../spec/index.md) before it lands.

  The library stays deterministic and offline (`K15.23`), and the `layering` CI
  job — which reads every manifest, dev-dependencies included, and fails when
  `openehr` depends outward — keeps it that way. A dependency implies a
  capability and readers reasonably infer one (`db:W16.4`); an `openehr` that
  pulled in an HTTP client would be claiming retrieval it does not do.
- **K15.26** A retrieved artefact MUST be verified to be the one requested —
  identifier and revision — and MUST be cached with its provenance: source,
  revision, retrieval time, and content digest. An artefact whose provenance
  cannot be established MUST NOT be used for validation unless the caller opts
  in explicitly, and the verdict MUST record that it did.
- **K15.27** **A retrieval failure MUST NOT degrade to a pass.** Unreachable
  repository, missing artefact, digest mismatch, and ambiguous revision are each
  a refusal naming what happened. Nothing here may fall back to "validate what
  we could reach".

## 15.7 What this section does not bring into scope

- **K15.28** Authoring remains out of scope. The crate reads, converts,
  flattens, and applies artefacts; it does not edit them, and it does not
  publish to CKM or any other repository. A modelling tool is a different
  program with a different audience.
- **K15.29** `S1.5` is unchanged: AQL is still parsed and statically checked and
  still not executed. Archetype support supplies *constraints*, and executing a
  query needs a repository of versioned *data*, which this crate still does not
  have.

## 15.8 Until it is implemented

- **K15.30** While a requirement in this section is unsatisfied, every entry
  point that would implement it MUST return an explicit `Unsupported` error
  naming this section (`S1.12`), and **no documentation may state or imply that
  the crate validates against archetypes** (`C0.11`, `W0.3`). The conformance
  matrix is the single place that says what is true today; a README that runs
  ahead of it is the failure this whole tree is arranged to prevent.
- **K15.31** The order in which this section is closed is not specified here,
  but a partial implementation MUST NOT be described as archetype support.
  Parsing without validation is a parser, and the crate MUST say so.
