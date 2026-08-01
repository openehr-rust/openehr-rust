# 9. Validation

**Rewritten 2026-08-01.** The previous text described validation at a service
interface — a strict mode, `OperationOutcome` responses, HTTP 422 — for a service
that does not exist (`C0.6`). See [`spec/audit.md`](../audit.md) **W-04**.

Withdrawn requirements keep their numbers (`C0.5`); new ones start at `V9.5`
(`C0.19`).

Requirement prefix: `V9`.

## Validate before writing

- **V9.1** *(amended)* A store MUST validate a composition against the Reference
  Model before writing it, and MUST refuse to write one that fails (`R4.13`).
- **V9.5** Validation MUST use the `openehr` crate's own `validate()`, not a
  reimplementation. Two validators eventually disagree, and the one that says
  "valid" is the one that gets believed.
- **V9.6** A refusal MUST be reported as a validation report — the path, the
  class, and the invariant — and MUST NOT be flattened to a string.

  A caller that receives "invalid composition" cannot fix anything. A caller that
  receives the failing path and the invariant name can.

- **V9.7** A validation report MUST NOT contain a submitted value (`M3.38`,
  `lib:X11.7`). It names *where* and *which rule*, never *what*.

## Two gates, not one

- **V9.8** A store MUST NOT rely on construction-time invariant checking as its
  only guard.

  This is the single most important thing in this section. The `openehr` crate
  enforces invariants in constructors, but serde writes fields directly and never
  calls a constructor. A composition that arrived as JSON has therefore been
  checked by *nothing* until `validate()` is called on it. A service that
  deserializes and stores without validating has no invariant checking at all —
  and it will look like it does, because the type system is satisfied.

## What validation does not mean

- **V9.9** Validation here is **Reference-Model-level only**. It MUST NOT be
  described, in documentation or in an error message, as validating a composition
  against its archetype or template.

  Neither this core nor the `openehr` crate implements archetypes (`lib:S1.4`).
  A partial archetype validator would let "valid" mean "the parts I understood
  were satisfied", which is worse than no claim: it invites a caller to skip the
  check that would have caught the problem.

- **V9.4** Terminology validation is **out of scope** (`S1.9`). External codes —
  SNOMED CT, LOINC, ICD-10 — are carried opaquely and checked only for
  `CODE_PHRASE` well-formedness. A store MUST NOT emit a `CHECK` constraint
  enumerating a value set's codes, because the set is a terminology-server
  concern and a constraint would freeze it at DDL time.

## Withdrawn

Withdrawn 2026-08-01. Numbers are retained and MUST NOT be reused (`C0.5`).

| Id | Was | Why withdrawn |
| --- | --- | --- |
| `V9.2` | **[service]** a strict mode deserializing through the typed model | there is no service, and there is no lenient mode to contrast with: everything goes through the typed model |
| `V9.3` | **[service]** validation failure returns 422 with an `OperationOutcome` | there is no service; a failure returns `StoreError::Invalid` carrying a validation report (`V9.6`) |

---

Part of the [openEHR persistence specification](index.md).
