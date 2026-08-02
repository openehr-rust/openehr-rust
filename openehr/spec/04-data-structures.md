# 4. Data structures

Requirement prefix: `R4` (openEHR's own name for the `CLUSTER`/`ELEMENT`
package is *Representation*).

## Where these invariants come from

Read from the **Release 1.0.2 publication** (*Data Structures Information
Model*, Rev 1.7.1, 5 Nov 2008, §5.2, §6.2), because the rendered RM page omits
its class-definition tables. This section uses openEHR's own invariant names —
`Items_non_empty`, `Null_flavour_indicated`, `Period_consistency` — so a reader
can find each one in the class definition without a translation step
(`L10.5a`).

## Coverage

- **R4.1** The crate MUST model `ITEM_SINGLE`, `ITEM_LIST`, `ITEM_TABLE`,
  `ITEM_TREE`, `CLUSTER`, `ELEMENT`, `HISTORY`, `POINT_EVENT`, and
  `INTERVAL_EVENT`.
- **R4.2** Every one of them MUST be a `LOCATABLE` (§5) and MUST therefore carry
  a runtime name and an archetype node id.

## `ELEMENT` and the four ways to say nothing

This is openEHR's most under-appreciated design decision, and the one a
relational or FHIR-shaped pipeline most often destroys.

- **R4.3** An `ELEMENT` MUST have either a `value` or a `null_flavour`, never
  both and never neither (`Null_flavour_indicated`; `Is_null_valid` ties
  `is_null` to the absence of `value`).
- **R4.4** `null_reason` MUST be permitted only on an element with no value.
- **R4.5** `null_flavour` MUST be one of the four openEHR null flavours —
  `271|no information|`, `253|unknown|`, `272|masked|`,
  `273|not applicable|` — and the constructor MUST take a code rather than a
  free `DV_CODED_TEXT`, so that a fifth flavour cannot be invented.
- **R4.6** The four MUST remain distinguishable through construction,
  serialization, and validation. "No allergy history recorded" is
  `271|no information|` and "no known allergies" is `273|not applicable|`;
  software that treats them alike will eventually give a penicillin-allergic
  patient penicillin.
- **R4.7** `272|masked|` MUST be separately identifiable, because it is the only
  flavour that says *a value exists*. Consent filtering produces masked elements
  (§11), and a reader must be able to tell that from an unanswered question.

## Containers

- **R4.8** `CLUSTER.items` MUST be non-empty (`Items_non_empty`). An empty cluster is a heading with
  nothing under it; openEHR's way to say "not filled in" is a null `ELEMENT`,
  which carries a reason.
- **R4.9** `ITEM_TABLE` MUST expose row and column counts and cell access, and
  MUST report whether the table is regular rather than assuming it. Cell indices
  are **zero-based**, departing from openEHR's one-based
  `element_at_cell_ij`: this is a Rust API in which every other index is
  zero-based, and one crate with two conventions produces off-by-one errors that
  read as correct code.
- **R4.10** Every item structure MUST offer one traversal reaching every
  `ELEMENT` in document order, across all four shapes. De-identification and
  value extraction both need it, and writing it four times is how they diverge.

## `HISTORY` and events

- **R4.11** `HISTORY` MUST have events, or a summary, or both (`Events_valid`). A history with
  neither records that observations were made and supplies none of them.
- **R4.12** `HISTORY.period` MUST be positive when present, and
  `is_periodic` MUST be exactly "a period was declared" (`Periodic_validity`) —
  never inferred from event times, which would report a series as periodic
  because two nurses happened to chart on the hour.
- **R4.12a** In a periodic history every event's offset from `origin` MUST be a
  whole multiple of `period` (`Period_consistency`). A series that declares a
  period its samples do not follow is not periodic, and software that resamples
  or graphs it on the strength of that declaration draws the wrong picture with
  nothing in the data looking wrong.
- **R4.12b** `EVENT.offset` MUST be available, computed as openEHR defines it —
  `time.diff(parent.origin)`. It is a derived value, so it is exposed on the
  history rather than the event: an `EVENT` here holds no back-pointer to its
  parent.
- **R4.12c** Where an offset or a period is not a whole number of seconds — a
  calendar period of months or years, or an event whose time is not comparable
  with the origin — `R4.12a` MUST report *not answerable* rather than a verdict,
  for the reason given in `R4.16`.
- **R4.13** An event's `time` MUST NOT precede the history's `origin`, and
  validation MUST report it when it does (`L10.6`).
- **R4.14** `INTERVAL_EVENT.time` is the **end** of the interval it summarises.
  Documentation MUST say so wherever the attribute is exposed, because a reader
  that treats it as an instant places an eight-hour urine output at the moment
  the bag was measured.
- **R4.15** `INTERVAL_EVENT.math_function` MUST come from the openEHR
  `event_math_function` group.
- **R4.15a** `INTERVAL_EVENT.width` MUST be non-negative. This is a **declared
  narrowing**: openEHR states no such invariant. What it buys is that
  `interval_start_time` (`R4.16`) cannot land after `time`, which would put an
  interval's start after its end. What it costs is that an instance carrying a
  negative width is refused rather than read.
- **R4.16** `interval_start_time` MUST be derivable where the arithmetic is
  exact, and MUST refuse where it is not: a width containing calendar months or
  years has no fixed length, because one month before 31 March is 28 February.
  Refusing is the correct answer (`S1.12`); approximating would place a clinical
  event on the wrong day, silently, in the direction nobody checks.
- **R4.17** `EVENT.state` MUST be modelled separately from `EVENT.data`. A blood
  pressure of 150/95 standing and the same value lying down are different
  findings.
