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

- **L10.2** Validation is **Reference-Model-level only**. Wherever it is
  offered, the documentation MUST state that a passing composition can still
  violate its archetype, because archetypes are out of scope (`S1.4`).
- **L10.3** Validation MUST collect every violation, not the first. A caller
  fixing a rejected composition wants the whole list; one violation per round
  trip turns a five-minute fix into five deployments.
- **L10.4** Every violation MUST name the path to the offending node, the RM
  class, and the invariant **using openEHR's own invariant name**, so a reader
  can find it in the class definition without a translation step.
- **L10.5** No violation may include node content (`X11.7`). Paths, class names,
  and invariant names only.
- **L10.5a** A violation MUST be attributed to the class whose invariant it
  actually breaks. An empty `LOCATABLE.name` breaks `DV_TEXT.Value_valid`, not
  `LOCATABLE.Name_valid` — openEHR's `Name_valid` is only `name /= Void` — and
  the wrong attribution sends a reader to the wrong class definition.

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
  (`E6.21`).
- **L10.7** Violation order MUST be stable across runs — depth-first, document
  order — so that a report diffed between two runs shows real change.
- **L10.8** Where a check cannot be performed, the result MUST be reported as
  *not checked* rather than as a pass (`D3.7`).
