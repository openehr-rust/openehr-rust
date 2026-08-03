# 3. Data types

Requirement prefix: `D3`.

The `DATA_VALUE` hierarchy is every leaf value a clinical record can hold. Two
themes run through this section: **precision is preserved**, and **comparison
refuses more often than it answers**.

## Where the quantity invariants come from

The rendered RM 1.1.0 Data Types page does not include the class-definition
tables — the specification sources `include::` them from a UML export that is
generated at build time and is not in the repository. The invariants in the
Quantity part of this section were therefore read from the **Release 1.0.2
publication** (*Data Types Information Model*, Rev 2.1.1, 20 Nov 2008,
§6.2.1–6.2.12), and the RM amendment record was then checked for every change
since:

| Release | Change affecting these classes |
| --- | --- |
| 1.0.3 | `SPECRM-32` — `DV_PROPORTION` gains the zero-denominator invariant |
| 1.1.0 | `SPECRM-19` — `DV_SCALE` added; `DV_ORDINAL.limits` removed |
| 1.1.0 | `SPECRM-65` — `units_system`, `units_display_name` added to `DV_QUANTITY` |
| 1.1.0 | `SPECRM-94`, `SPECRM-96` — arithmetic, comparison, negative durations |

None of them altered `precision`, `accuracy`, `magnitude_status`, or the
`DV_ORDERED` range invariants, so the 1.0.2 text governs them. `DV_SCALE`, new
in 1.1.0, is taken to inherit `DV_ORDERED`'s invariants unchanged — it is a
`DV_ORDERED` descendant and the amendment describes it as `DV_ORDINAL` with a
`Real` value. That last inference is the only one not read from a source.

## Coverage

- **D3.1** The crate MUST model every concrete `DATA_VALUE` descendant in
  openEHR RM 1.1.0: `DV_BOOLEAN`, `DV_STATE`, `DV_IDENTIFIER`, `DV_TEXT`,
  `DV_CODED_TEXT`, `DV_PARAGRAPH`, `DV_ORDINAL`, `DV_SCALE`, `DV_QUANTITY`,
  `DV_COUNT`, `DV_PROPORTION`, `DV_DATE`, `DV_TIME`, `DV_DATE_TIME`,
  `DV_DURATION`, `DV_MULTIMEDIA`, `DV_PARSABLE`, `DV_URI`, `DV_EHR_URI`,
  `DV_INTERVAL`, `DV_PERIODIC_TIME_SPECIFICATION`, and
  `DV_GENERAL_TIME_SPECIFICATION`.
- **D3.2** `DV_PARAGRAPH` MUST be readable and MUST NOT be recommended. openEHR
  deprecates it in favour of markdown-formatted `DV_TEXT`, and instances written
  before the deprecation must still round-trip.

## Text and terminology

- **D3.3** `DV_TEXT.value` MUST be non-empty. A field that is present and empty
  is indistinguishable from one that is absent, and openEHR already has a way to
  say absent.
- **D3.4** `DV_TEXT.formatting` MUST be non-empty when present, and a value of
  `plain_no_newlines` MUST be refused when `value` contains a newline.
- **D3.5** `CODE_PHRASE` MUST require a non-empty `code_string` and a
  well-formed `TERMINOLOGY_ID`, and MUST NOT check the code against any
  terminology other than openEHR's own (`S1.10`).
- **D3.6** `TERM_MAPPING.match` MUST be one of `>`, `=`, `<`, `?`, modelled as a
  closed set rather than a character.
- **D3.7** `DV_CODED_TEXT.value` is required by openEHR to be the rubric of
  `defining_code`. The crate MUST check this **only** for codes in the openEHR
  support terminology, and MUST report *not checked* as an outcome distinct from
  *valid*. Reporting an unchecked external code as valid would be a claim about
  a terminology the crate cannot see.
- **D3.8** Where a code appears in more than one openEHR terminology group with
  different rubrics, agreement with **any** group MUST be accepted: a bare
  `DV_CODED_TEXT` does not know which attribute it sits on.

## Dates, times, and durations

- **D3.9** Partial precision MUST be preserved. `2024`, `2024-05`, and
  `2024-05-17` are three different dates and MUST NOT be completed to one
  another. A completed date is a fabricated clinical fact that no downstream
  reader can distinguish from a real one.
- **D3.10** The lexical form MUST be preserved exactly, including the choice
  between `Z` and `+00:00`, and including the number of digits in a fractional
  second.
- **D3.11** Date, time, and date-time MUST be validated on construction:
  component ranges, month lengths, and the full Gregorian leap rule including
  the century exceptions.
- **D3.12** A leap second (`:60`) MUST be accepted. A record that captured one
  must not be rejected on read-back.
- **D3.13** `DV_DURATION` MUST accept openEHR's leading `-`, which plain
  ISO 8601 does not, and MUST refuse designators that are out of order or
  repeated.
- **D3.13a** The extended ISO 8601 format is required; the basic format
  (`20240517`) MUST be refused. It does not appear in openEHR canonical JSON,
  and accepting it would make `2024` ambiguous. Recorded as a limitation in
  [`audit.md`](audit.md).

## Comparison

- **D3.14** Comparison MUST be partial. Where an ordering is not established,
  the answer MUST be "not comparable", never a default. Specifically:
  - two dates or times whose known components agree but whose precisions differ,
  - a local time and one carrying a UTC offset,
  - two durations whose approximations agree and whose calendar components
    differ,
  - values of different `DATA_VALUE` classes.
- **D3.15** `DV_QUANTITY` values MUST be comparable only when their `units`
  strings are equal (`S1.9`).
- **D3.16** `DV_ORDINAL` and `DV_SCALE` values MUST be comparable only when
  their symbols come from the same terminology. Ordinal `2` on a pain scale and
  ordinal `2` on a sedation scale are unrelated.
- **D3.17** `DV_PROPORTION` values MUST be comparable only when their kinds
  match.
- **D3.18** Times carrying UTC offsets MUST be normalised to UTC before
  comparison.
- **D3.18a** *Declared inconsistency.* `Eq` on `Time`, `Date` and `DateTime` is
  **lexical** — two values are equal when they were written the same way —
  while `PartialOrd` compares instants under `D3.18`. So `11:00:00Z` and
  `12:00:00+01:00` order `Equal` and are **not** `==`.

  This contradicts the standard library's requirement that the two agree, and it
  is declared rather than resolved because both halves are load-bearing and
  neither moves without loss. Lexical equality is record identity: the text is
  the stored value (`db:M3.28`), `.5` and `.50` are different strings a record
  must round-trip, and `Hash` must agree with `Eq`. Instant ordering is what a
  query needs.

  **What a caller must not do:** treat the two as interchangeable. `dedup` after
  `sort` keeps both spellings of one instant, because `dedup` uses `==`. Recorded
  as `A-32` and pinned by
  `iso8601::lexical_equality_and_instant_ordering_are_different_questions`.

## Quantities and proportions

- **D3.19** `DV_QUANTITY` MUST require a finite magnitude and a non-empty
  `units`. A quantity with no units is a number, and openEHR has `DV_COUNT`.
- **D3.19a** The non-empty requirement in `D3.19` is a **declared narrowing** of
  openEHR's `Units_valid: units /= void`, which forbids only a missing value and
  not an empty one. What survives: every `DV_QUANTITY` this crate accepts has a
  units string that names a measured property, which is what
  `is_strictly_comparable_to` depends on (`D3.15`). What it costs: an instance
  carrying `"units": ""` is refused rather than read.
- **D3.20** `precision` MUST satisfy openEHR's `Precision_valid: precision >= -1`.
  `0` means an integral quantity; **`-1` means no limit** — any number of
  decimal places — and is a *stated* precision, distinct from the attribute
  being absent. Requiring `precision >= 0` would reject conformant data.
- **D3.20a** `is_integral` MUST be exactly `precision = 0`, as openEHR defines
  it, and MUST answer "unknown" when no precision is recorded. A magnitude that
  happens to be whole is not a quantity *declared* integral.
- **D3.21** `accuracy` MUST be finite and MUST satisfy both openEHR rules:
  `Accuracy_validity` — a percentage accuracy lies in 0–100 — and
  `Accuracy_is_percent_validity` — an accuracy of **0** must not be flagged as a
  percentage, because 0 means 100% accurate and "0%" reads as the opposite.
- **D3.21a** `-1.0` MUST be honoured as openEHR's `unknown_accuracy_value`: it
  records that accuracy was not measured, and a reader treating it as an error
  of minus one is wrong by the width of the scale. `accuracy_unknown` MUST
  answer true for it **and** for an absent accuracy, because both mean the same
  thing to a reader and only one of them is obvious.
- **D3.22** `magnitude_status` MUST be one of `=`, `<`, `>`, `<=`, `>=`, `~`,
  modelled as a closed set — openEHR's `valid_magnitude_status`. A result below
  an assay's detection limit is a real result and recording it as the limit
  loses the only part that matters.
- **D3.23** `DV_PROPORTION` MUST refuse a zero denominator, MUST require
  denominator 1 for `pk_unitary` and 100 for `pk_percent`, and MUST require
  integral parts for `pk_fraction` and `pk_integer_fraction`. These comparisons
  are exact: a tolerance would accept a denominator of 99.999 and then render it
  as a percentage.
- **D3.23a** `DV_PROPORTION.precision` MUST satisfy
  `Precision_validity: precision = 0 implies is_integral`. Note the direction:
  declaring precision `0` **asserts** the parts are whole numbers. It does not
  forbid an integral *kind* from carrying a precision, which is the reading an
  implementation usually arrives at and which rejects valid data.
- **D3.24** `DV_ORDERED` reference ranges MUST be modelled, and "outside the
  normal range" MUST be distinguishable from "no normal range recorded".
- **D3.24a** `normal_status` MUST come from the openEHR `normal statuses` code
  set (`Normal_status_validity`). A renderer prints this letter verbatim beside
  a result, so an invented one reaches a clinician.
- **D3.24b** Where both `normal_range` and `normal_status` are present, they
  MUST agree: `N` if and only if the value lies inside the range
  (`Normal_range_and_status_consistency`). They disagree when a result is copied
  from one system and its flag from another — which is the case where a
  clinician sees a normal flag beside an abnormal number.

## Encapsulated data

- **D3.25** `DV_MULTIMEDIA` MUST NOT render its bytes in any `Display` or
  `Debug` output. `Debug` MUST print media type and length instead. This is the
  one place the crate's usual "`Debug` may show everything" rule is overridden,
  because "everything" is a megabyte of base64 that ends up in a ticket.
- **D3.26** `DV_MULTIMEDIA.integrity_check` MUST be verifiable, and the
  verification MUST distinguish *passed*, *failed*, *not recorded*, *no inline
  data*, and *unsupported algorithm*. A field saying "SHA-256: …" that nothing
  checks is worse than no field, because it reads as an assurance.
- **D3.27** The crate MUST read an integrity check declared as `SHA-1` and MUST
  NOT emit one. openEHR lists SHA-1; a clinical record may outlive it, and it
  has already been outlived.
- **D3.28** `DV_PARSABLE` MUST require both `value` and `formalism` to be
  non-empty. An empty formalism leaves content nothing can decide how to read.
- **D3.29** Base64 decoding MUST tolerate embedded whitespace, which
  XML-derived payloads carry and which conveys nothing.

## URIs

- **D3.30** `DV_URI` MUST require a well-formed scheme and MUST refuse spaces
  and control characters.
- **D3.31** `DV_EHR_URI` MUST require the scheme `ehr`. `LINK.target` is typed
  `DV_EHR_URI` precisely so that a link cannot point out of the record without
  saying so.
- **D3.32** Validity of a `DV_EHR_URI` is **structural only**. The crate cannot
  resolve it, and documentation MUST NOT let "valid" be read as "resolvable".

## Identifiers as values

- **D3.33** `DV_IDENTIFIER.id` MUST be non-empty.
- **D3.34** `DV_IDENTIFIER`'s `Display` MUST render the type and issuer and MUST
  NOT render the identifier (`X11.6`).
