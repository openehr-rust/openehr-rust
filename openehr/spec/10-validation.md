# 10. Validation

Requirement prefix: `L10`.

## Two gates

- **L10.1** Constructors MUST enforce the invariants of the class they build, so
  that data the program **builds** cannot be invalid.
- **L10.1a** Validation MUST enforce them on data the program **receives**.
  `serde` writes fields directly and never calls a constructor, so a document
  read from a wire has passed no gate but this one.
- **L10.1b** Documentation MUST state that a service which deserializes and
  stores without validating has no invariant checking at all, whatever its
  constructors do.

## What validation means, and what it does not

- **L10.2** *(amended 2026-08-26 — `S1.4` was withdrawn and the Archetype Model
  brought into scope by `S1.21`)* Validation has **two levels**, and a verdict
  MUST say which one produced it: Reference-Model validation (this section), and
  archetype validation against an operational template (§15, `K15.19`). The two
  MUST NOT be merged into one verdict — "not a valid `COMPOSITION`" and "a valid
  `COMPOSITION` that does not conform to this template" are repaired
  differently.

  **While §15 is unimplemented**, the crate offers Reference-Model validation
  only, and wherever it is offered the documentation MUST state that a passing
  composition can still violate its archetype. That sentence stays until the
  matrix says archetype validation exists — not until the specification says it
  should (`K15.30`, `C0.11`).
- **L10.3** Validation MUST collect every violation, not the first. A caller
  fixing a rejected composition wants the whole list; one violation per round
  trip turns a five-minute fix into five deployments.
- **L10.4** Every violation MUST name the path to the offending node, the RM
  class, and the invariant **using openEHR's own invariant name**, so a reader
  can find it in the class definition without a translation step.
- **L10.5** No violation may include node content (`X11.7`). Paths, class names,
  and invariant names only.
- **L10.5a** A violation MUST be attributed to the class whose invariant it
  actually breaks. An empty `LOCATABLE.name` breaks `DV_TEXT.Valid_value`, not
  `LOCATABLE.Name_valid` — openEHR's `Name_valid` is only `name /= Void` — and
  the wrong attribution sends a reader to the wrong class definition.

- **L10.9** A check the crate performs that openEHR does **not** state for that
  class is an **addition**, not a rename, and MUST be declared in the register
  below. `L10.4` cannot govern it: there is no openEHR name to use.

  The distinction is not pedantry. A rename is a defect — the same rule reported
  under a name the specification does not contain, so a reader cannot find it. An
  addition is a *strengthening*: a rule openEHR does not require, which this
  crate enforces anyway. Confusing the two makes both invisible, and it is what
  made the first sweep for `A-20` over-report by treating every unmatched name as
  wrong.

- **L10.10** An addition MUST NOT use a name openEHR uses for that class, even
  for a different rule, and SHOULD avoid a name openEHR uses anywhere, so that a
  future openEHR release cannot silently collide with it.

- **L10.11** *(added 2026-08-02)* An openEHR invariant the crate does **not**
  enforce MUST be declared in the register below, with the reason. A rule that
  is absent and undeclared is indistinguishable from one nobody noticed
  (`C0.14`).

  This is `L10.9`'s mirror. That one registers a rule openEHR does not state and
  the crate enforces anyway; this one registers a rule openEHR states and the
  crate does not. Both exist because the interesting cases are the ones where
  the crate and the specification differ, and a difference nobody wrote down
  reads as an oversight.

  The register MUST agree with the **Not enforced** group of
  [`assets/invariant-coverage.md`](../../assets/invariant-coverage.md), which is
  generated. `openehr-assets` fails the build when the two disagree, so neither
  can drift: a rule quietly abandoned appears in the report and not the
  register, and a rule quietly enforced appears in the register and not the
  report.

### Unenforced openEHR checks

Declared under `L10.11`. Ten of RM 1.1.0's 155 invariants, and none of them is
merely undone.

| Class | Invariant | Why not |
| --- | --- | --- |
| `COMPOSITION` | `Language_valid` | ISO 639 is not carried (`S1.18`) |
| `COMPOSITION` | `Territory_valid` | ISO 3166 is not carried (`S1.18`) |
| `DV_ENCAPSULATED` | `Charset_valid` | IANA character sets are not carried |
| `DV_ENCAPSULATED` | `Language_valid` | ISO 639 is not carried |
| `DV_MULTIMEDIA` | `Media_type_valid` | IANA media types are not carried |
| `DV_TEXT` | `Encoding_valid` | IANA character sets are not carried |
| `DV_TEXT` | `Language_valid` | ISO 639 is not carried |
| `ENTRY` | `Encoding_valid` | IANA character sets are not carried |
| `ENTRY` | `Language_valid` | ISO 639 is not carried |
| `EHR_ACCESS` | `Scheme_valid` | an `EHR_ACCESS` may record no policy (`S1.20`) |

Nine are one decision wearing five hats: the crate does not carry external code
sets, so it cannot check a code against one. `S1.18` declares that for
`COMPOSITION`, where openEHR names the code sets itself and `S1.10` therefore
does not cover it; the other seven are the same departure on other classes.

The tenth is `S1.20`.

**What is *not* in this register.** The four demographic-graph invariants of
`S1.19` are excluded rather than unenforced — the crate does not model the thing
they constrain — and appear in the report's *out of scope* group. The
distinction matters: an exclusion is answered by scope, an omission by this
table.

### Crate-added checks

Declared under `L10.9`. Each is a rule openEHR does not state for that class,
which this crate enforces anyway.

| Class | Name | Checks | Why it is added |
| --- | --- | --- | --- |
| `ARCHETYPED` | `Archetype_id_rm_entity_matches` | the archetype id's RM entity matches the class it annotates | an `OBSERVATION` carrying a `COMPOSITION` archetype id is a document nobody can query correctly |
| `COMPOSITION` | `Is_persistent_validity` | a persistent composition has no `context` | openEHR states this on `VERSIONED_COMPOSITION`; enforcing it where the data is caught it earlier |
| `CONTACT` | `Addresses_valid` | a contact has at least one address | a contact with no address is a record of nothing |
| `DV_AMOUNT` | `Accuracy_finite` | accuracy is a finite float | openEHR assumes real numbers; IEEE 754 has `NaN` and `±∞`, and a `NaN` accuracy compares false against everything |
| `DV_MULTIMEDIA` | `Integrity_check_matches` | the recorded digest matches the inline data | openEHR requires a check to name its algorithm (`Integrity_check_validity`) but never says the digest must be *right*. Reported under openEHR's name until `A-22`, which sent a reader to an invariant about something else |
| `DV_PARSABLE` | `Value_valid` | the value is non-empty | openEHR constrains `formalism` and `size` but not the value |
| `DV_PROPORTION` | `Parts_finite` | numerator and denominator are finite | as `DV_AMOUNT.Accuracy_finite` |
| `DV_URI` | `Uri_well_formed` | an RFC 3986 scheme, and no space or control character | openEHR's `DV_URI.Value_valid` is only `not value.is_empty`. This crate's constructor has always required more (`D3.30`), and until `A-36` nothing required it of a URI that arrived as JSON |
| `EVENT` | `Time_after_origin` | an event's time is at or after its history's origin | openEHR states the offset relation but not the ordering |
| `EVENT_CONTEXT` | `End_time_valid` | end time is at or after start time | a consultation that ended before it began |
| `INSTRUCTION` | `Narrative_valid` | the narrative is non-empty | the narrative is what a human reads when the structured form is not understood |
| `INTERVAL_EVENT` | `Width_non_negative` | width is not negative | an interval of negative duration |
| `ITEM_TABLE` | `Rows_regular` | every row has the same column count | openEHR requires rows of `ELEMENT`; a ragged table renders as a table and is not one |
| `ORIGINAL_VERSION` | `Data_valid` | data is present unless the lifecycle state is `deleted` | a version claiming `complete` and supplying nothing |
| `ORIGINAL_VERSION` | `Lifecycle_state_valid` | the state is in the openEHR group | openEHR states this on `VERSION`; reported on the concrete class the caller constructed |

Two of these — `Accuracy_finite` and `Parts_finite` — exist because openEHR is
written against mathematical reals and Rust is not. A specification that says
"accuracy is a real number" does not anticipate `NaN`, and a `NaN` that reaches a
clinical comparison is false against every bound including itself.

## The checks

- **L10.6** Validation MUST check, at minimum: `ELEMENT`'s value/null-flavour
  exclusivity (`R4.3`) and null-reason rule (`R4.4`); the null flavour's
  membership of the openEHR group (`R4.5`); `CLUSTER` non-emptiness (`R4.8`);
  `ITEM_TABLE` regularity (`R4.9`); `HISTORY` non-emptiness (`R4.11`) and event
  ordering against origin (`R4.13`); `INTERVAL_EVENT` width sign (`R4.15`);
  `LOCATABLE` node id and name (`M5.2`); `ARCHETYPED` class agreement (`M5.7`);
  `DV_CODED_TEXT` rubric agreement for openEHR codes (`D3.7`); `DV_QUANTITY`
  finiteness, units, and precision (`D3.19`, `D3.20`); `DV_PROPORTION`
  denominator and integrality (`D3.23`); `DV_MULTIMEDIA` content presence and
  integrity (`D3.26`); `PARTY_IDENTIFIED` basic validity (`M5.14`);
  `COMPOSITION.category` group membership (`E6.6`); `EVENT_CONTEXT.setting`
  group membership (`E6.11`); `ISM_TRANSITION.current_state` group membership
  (`E6.21`); and `DV_URI` well-formedness with `DV_EHR_URI` scheme (`D3.30`,
  `D3.31`), wherever a URI appears — as a `DATA_VALUE` **and** as a
  `LINK.target` on any `LOCATABLE` (`M5.9`), which is where one actually
  arrives from outside the process.
- **L10.7** Violation order MUST be stable across runs — depth-first, document
  order — so that a report diffed between two runs shows real change.
- **L10.8** Where a check cannot be performed, the result MUST be reported as
  *not checked* rather than as a pass (`D3.7`).
