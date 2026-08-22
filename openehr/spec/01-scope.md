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

- **S1.18** *Departure from `COMPOSITION.Territory_valid` and
  `COMPOSITION.Language_valid`.* The crate MUST NOT validate `territory` against
  ISO 3166-1 or `language` against ISO 639-1. It checks `CODE_PHRASE`
  well-formedness only (`D3.5`), so `ISO_639-1::zz` is accepted although `zz` is
  not a language.

  **What openEHR requires.** RM 1.1.0 states both as invariants on `COMPOSITION`:
  `code_set(Code_set_id_countries).has_code(territory)` and
  `code_set(Code_set_id_languages).has_code(language)`. These are code sets
  openEHR names, so `S1.10` — which excludes *external* terminologies like
  SNOMED CT — does not cover them. This is a genuine departure and is declared
  here because an undeclared one is a defect rather than a decision (`C0.12`,
  `C0.14`).

  **What survives.** Structural validity: the code is well-formed and names the
  right terminology id. What is lost is membership — a caller may store a
  syntactically valid code that denotes no country or language, and nothing here
  will say so.

  **Why declared rather than implemented.** Both code sets are closed, small, and
  *mutable*: ISO 3166-1 gains and retires codes, and a table compiled into a
  library is wrong from the day a country changes. Validating against a stale
  copy would reject conformant data, which `D3.5`'s own reasoning calls the worse
  failure. A deployment that needs the check should do it where the tables can be
  updated.

  **That last sentence is a requirement on this crate, not a shrug.** It holds
  only while a caller can reach every code the crate declines to check, and
  `A-34` is the finding where two such codes round-tripped perfectly and could
  not be read at all. Every one of them is now reachable and
  `guarantees::a_caller_can_read_every_code_the_crate_declines_to_check` asserts
  it — so an accessor cannot be deleted, leaving a departure that is silently
  worse than the one declared here.

  `A-19` is **classified** rather than open: the decision is made, and what was
  genuinely unresolved was whether the advice above could be followed.

- **S1.19** The crate MUST NOT model a demographic **repository** — an object
  store that can be asked, of a relationship, for its reverse. It models
  demographic *values*: a `PARTY` holds its relationships, and nothing holds the
  parties.

  **What this makes unenforceable.** Four openEHR invariants constrain the
  demographic graph rather than any one object:
  `PARTY.Relationships_validity` and `PARTY_RELATIONSHIP.Source_valid` require a
  relationship's source to be the party holding it;
  `PARTY.Reverse_relationships_validity` and `PARTY_RELATIONSHIP.Target_valid`
  require the repository to answer for the other end. None can be checked from a
  value in hand, and this crate never has more than that.

  Declared because the alternative is that four unenforced invariants look like
  four oversights (`C0.14`). A deployment holding a demographic repository is
  the layer that can check them, in the same way a deployment is the layer that
  authenticates (`S1.14`).

- **S1.20** *Departure from `EHR_ACCESS.Scheme_valid`.* The crate MUST allow an
  `EHR_ACCESS` with **no** access-control settings, and therefore does not
  enforce openEHR's requirement that its derived `scheme` be non-empty.

  **What openEHR requires.** `scheme` is derived from the concrete `settings`
  instance and `Scheme_valid` states `not scheme.is_empty`, so every
  `EHR_ACCESS` must carry a policy.

  **Why this departs.** "No access policy has been set" and "the policy is
  deny-all" are different facts about a record, and a type that cannot express
  the first forces every caller to assert the second. `EhrAccess::new` records
  no policy deliberately, and a reader can tell the two apart. The cost is that
  a record can exist with no policy, which a deployment MUST NOT read as
  permission — `X11.24` already requires the fail-closed default, and this
  departure is why that requirement matters.
