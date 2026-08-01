# 1. Scope

Requirement prefix: `S1`.

## What the crate is

- **S1.1** The crate MUST implement the openEHR **Reference Model** — the BASE
  identification and foundation types, and the RM Data Types, Data Structures,
  Common, EHR, and Demographic packages — as Rust types with construction-time
  invariant checking.
- **S1.2** The crate MUST implement openEHR **canonical JSON** (ITS-JSON)
  serialization and deserialization for every type it models (§9).
- **S1.3** The crate MUST implement the openEHR **support terminology** groups
  that the Reference Model refers to by code (§3, §4, §6, §8).

## What the crate is not

The exclusions below are decisions, not omissions. Each names why, because a
reader deciding whether to use this crate needs the reason more than the fact.

- **S1.4** The crate MUST NOT implement the **Archetype Model** — ADL, AOM2,
  templates, or archetype-constraint validation. An archetype is a constraint
  language with its own parser and its own conformance rules; implementing a
  partial one would let "valid" mean "the parts I understood were satisfied".
  Consequence: [validation](10-validation.md) is Reference-Model-level only,
  and `L10.2` requires that to be stated wherever validation is offered.
- **S1.5** The crate MUST NOT execute AQL. Executing means resolving archetype
  paths against a repository of versioned objects, and the crate has no
  repository. It parses and statically checks AQL instead (§12).
- **S1.6** The crate MUST NOT implement the **EHR Extract** model or the
  **Integration** package. Both exist to move data between systems, which is a
  service concern; the crate is a library.
- **S1.7** The crate MUST NOT implement an HTTP service, a persistence layer, or
  a command-line tool. Nothing in this repository builds any of them, and no
  documentation may suggest otherwise (`C0.11`).
- **S1.8** The crate MUST NOT interpret HL7 `GTS`, `PIVL`, or `EIVL`
  expressions inside `DV_TIME_SPECIFICATION`. It validates the wrapper and
  carries the expression opaquely. A partial timing engine produces a dosing
  schedule that is right most of the time.
- **S1.9** The crate MUST NOT convert units. `DV_QUANTITY` comparison is by
  exact units match (`D3.15`). A silent `mg`-to-`g` conversion is a
  thousand-fold dosing error that nothing downstream detects.
- **S1.10** The crate MUST NOT resolve external terminologies. `SNOMED-CT`,
  `LOINC`, and `ICD-10` codes are carried opaquely and checked only for
  `CODE_PHRASE` well-formedness (`D3.5`).
- **S1.11** The crate MUST NOT encrypt, and MUST NOT verify the `OpenPGP`
  signature in `ATTESTATION.proof`. Both require key management that belongs to
  the deployment (§11).

## How an exclusion behaves

- **S1.12** Where openEHR defines an operation that this crate does not
  implement, the operation MUST return an explicit `Unsupported` error naming
  what is missing and the specification section that records the exclusion. It
  MUST NOT return a plausible default, a zero value, or a silent success.
- **S1.13** A type whose openEHR definition includes an unimplemented operation
  MUST still round-trip that type's data losslessly (§9). Not interpreting a
  value is not a licence to lose it.

## Boundaries the caller must supply

- **S1.14** The crate MUST NOT authenticate. It records who acted; establishing
  who they are is the deployment's (`X11.1`).
- **S1.15** The crate MUST NOT bound recursion depth on deserialization, and its
  documentation MUST state that a caller reading untrusted documents has to.
  Unbounded nesting is a denial-of-service vector in every recursive reader, and
  a limit chosen by a library is either too low for a legitimate document or too
  high to be a limit.

## Versions of openEHR

- **S1.16** The crate targets openEHR **Reference Model 1.1.0** and the
  terminology published in `openEHR/specifications-TERM`. `ARCHETYPED.rm_version`
  is carried and not enforced: data authored against 1.0.2 is readable, and the
  version it declares is preserved so a caller can decide.
- **S1.17** Where the current terminology disagrees with the older
  `openEHR/terminology` repository, the current one governs, and the divergence
  MUST be recorded where the codes are defined (see `src/terminology.rs`).
