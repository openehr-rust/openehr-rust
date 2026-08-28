# The `openehr` crate specification

This directory is **normative** for the `openehr` crate. It says what must be
true; the code says how. When the two disagree, reconcile them — do not let
them drift.

Requirements use RFC 2119 keywords (MUST, SHOULD, MAY) as defined in
[§0 Conformance](00-conformance.md), and every one carries a stable identifier
that is cited from the code, the tests, and this documentation. That trace —
openEHR specification → requirement → code → test → matrix — is what makes a
claim about this crate checkable years later by someone who was not here.

## Relationship to the openEHR specifications

openEHR is specified at [specifications.openehr.org](https://specifications.openehr.org/).
**Those documents are the authority; this directory is not.** What lives here
is everything that a Rust implementation has to decide and openEHR does not
say:

| openEHR says | This spec says |
| --- | --- |
| `DV_DATE.value` is an ISO 8601 date string | which forms are accepted, and that partial precision is preserved (`D3.9`) |
| `DV_ORDERED` declares `is_strictly_comparable_to` | that non-comparable values yield `None`, not `false` (`D3.14`) |
| `_type` is required where the declared type is abstract | which classes emit it and which infer it on input (`J9.4`) |
| access control is scheme-agnostic | that an unevaluatable scheme denies and round-trips (`X11.3`) |
| — | what is out of scope, and how the code says so (`S1.9`) |

Where openEHR is ambiguous or its own documents disagree, the resolution is
recorded with the reason, not silently chosen. Where this crate departs from
openEHR, the departure is written down as a requirement that names what it
departs from; an undeclared departure is a defect, not a decision.

## Contents

### Framework

- **0.** [Conformance](00-conformance.md) — `C0.x`. Normative language,
  requirement-id grammar, what a conformance claim means, how to amend.
- **1.** [Scope](01-scope.md) — `S1.x`. What is implemented, what is
  deliberately excluded, and how an exclusion behaves at runtime.

### The Reference Model

- **2.** [Identifiers](02-identifiers.md) — `I2.x`. `UID`, `OBJECT_ID` and
  descendants, `OBJECT_REF`, and the lexical grammars.
- **3.** [Data types](03-data-types.md) — `D3.x`. `DATA_VALUE` and every
  descendant, including comparison and ISO 8601 handling.
- **4.** [Data structures](04-data-structures.md) — `R4.x`. `ITEM_STRUCTURE`,
  `CLUSTER`, `ELEMENT`, null flavours, `HISTORY` and events.
- **5.** [Common and archetyping](05-common.md) — `M5.x`. `LOCATABLE`,
  `ARCHETYPED`, `LINK`, `FEEDER_AUDIT`, parties, participation.
- **6.** [EHR and composition](06-ehr.md) — `E6.x`. `EHR`, `EHR_STATUS`,
  `COMPOSITION`, sections, the five entry classes, folders.
- **7.** [Demographics](07-demographics.md) — `G7.x`. `PARTY`, `ACTOR`,
  `ROLE`, identities, contacts, capabilities.
- **8.** [Versioning and change control](08-change-control.md) — `V8.x`.
  `VERSIONED_OBJECT`, `VERSION`, `CONTRIBUTION`, `AUDIT_DETAILS`,
  `ATTESTATION`.

### Cross-cutting

- **9.** [Serialization](09-serialization.md) — `J9.x`. openEHR canonical JSON:
  what is emitted, what is accepted, and what round-trips.
- **10.** [Validation](10-validation.md) — `L10.x`. Which invariants are
  checked, where, and what a passing validation does **not** mean.
- **11.** [Security](11-security.md) — `X11.x`. The trust boundary, access
  control, the audit chain, redaction, and PHI in output.
- **12.** [Paths and query](12-paths-and-query.md) — `Q12.x`. openEHR path
  syntax and navigation; AQL lexing, parsing, and static checks.

### Assurance

- **13.** [Conformance testing](13-conformance-testing.md) — `T13.x`. What must
  be tested, how a test proves it can fail, and what a skipped check must say.
- **14.** [Compliance mapping](14-compliance-mapping.md) — a table, not
  requirements. Which requirements a regulator's control maps onto.

### Status, not requirements

- [Conformance matrix](conformance-matrix.md) — which requirements are
  satisfied and verified **today**. Non-normative: it records reality, not
  intent.
- [Audit](audit.md) — the findings register: every known gap between what this
  spec requires, what the documentation claims, and what the code does.

## The rules that cost nothing to keep and cannot be repaired once broken

1. **Requirement ids are permanent** (`C0.5`). Never renumber, never reuse.
2. **A gap that is not written down reads as a pass** (`C0.9`). Unverified goes
   in [`audit.md`](audit.md) and shows as `?` in the matrix, not `•`.
3. **Do not claim above what is verified** (`C0.11`). "The code is shared with
   a path that works" is `?`, not `•`.
4. **Behaviour is decided here first.** Discovering a requirement while
   implementing is normal; the fix is to write it here before the commit lands.
