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
- **D3.7a** *(added 2026-09-05)* `D3.7`'s own "report *not checked*, never a
  false violation" principle is not yet met for a non-English record: this
  crate's rubric table carries the **English** rubric only, and the check does
  not know the enclosing record's own declared `language` — so a valid
  Spanish rubric (`"evento"` for openEHR's own code `433`, whose English
  rubric is `"event"`) is reported as a *violation*, not as *unchecked*,
  which is exactly the false claim `D3.7` says not to make, just in the
  refusing direction rather than the accepting one. Found running the JSON
  corpus (`EHRbase`'s own `my_spanish_template_v0_COMPOSITION_EXAMPLE.json`,
  declared `language: es`), recorded as `A-81`, open: closing it needs
  either threading the record's language through the validation walk and
  reporting unchecked for anything but English, or sourcing and citing real
  multi-language rubric data — a decision, not a same-day fix.

## Dates, times, and durations

- **D3.9** Partial precision MUST be preserved. `2024`, `2024-05`, and
  `2024-05-17` are three different dates and MUST NOT be completed to one
  another. A completed date is a fabricated clinical fact that no downstream
  reader can distinguish from a real one.
- **D3.10** The lexical form MUST be preserved exactly, including the choice
  between `Z` and `+00:00`, and including the number of digits in a fractional
  second.
- **D3.10b** *(added 2026-09-05)* A fractional second's decimal sign MUST be
  accepted as either `.` or `,` on read — `openEHR/adl-antlr`'s own
  `fragment SECOND_DEC_SEP : '.' | ',' ;` (`base_lexer.g4`), not merely
  general ISO 8601 permissiveness this crate could choose to ignore. `D3.10`
  is unaffected either way: the stored lexical form is the whole input text,
  separator included, never reconstructed from the parsed digits, so
  accepting `,` changes nothing about what a round trip preserves.
- **D3.11** Date, time, and date-time MUST be validated on construction:
  component ranges, month lengths, and the full Gregorian leap rule including
  the century exceptions.
- **D3.12** A leap second (`:60`) MUST be accepted. A record that captured one
  must not be rejected on read-back.
- **D3.13** `DV_DURATION` MUST accept openEHR's leading `-`, which plain
  ISO 8601 does not, and MUST refuse designators that are out of order or
  repeated.
- **D3.13a** The extended ISO 8601 format is required; the basic format
  (`20240517`) MUST be refused, and accepting it would make `2024` ambiguous.
  Recorded as a limitation in [`audit.md`](audit.md). *Corrected 2026-09-05
  (`A-02`)*: this requirement previously also claimed the basic format "does
  not appear in openEHR canonical JSON" — false, and known false rather than
  merely unverified: `EHRbase`'s own reference test corpus writes
  `DV_DATE.value` as `"20190114"` in three real fixtures
  (`openehr/spec/corpus.md`). The refusal itself is unchanged and the
  ambiguity reasoning stands; only the false supporting claim is withdrawn
  (`C0.5`: the id is not renumbered for a text correction).

## Comparison

- **D3.14** Comparison MUST be partial. Where an ordering is not established,
  the answer MUST be "not comparable", never a default. Specifically:
  - two dates or times whose known components agree but whose precisions differ,
  - a local time and one carrying a UTC offset,
  - two durations whose approximations agree and whose calendar components
    differ,
  - values of different `DATA_VALUE` classes.
- **D3.14a** *(added 2026-08-21)* `INTERVAL<T>::contains` MUST treat "not
  comparable" as **not contained**, at either bound and in either direction.

  `D3.14` says what comparison answers. This says what the one caller that
  matters does with that answer, and it was load-bearing and undocumented until
  `A-39` — implemented correctly, relied on by `DvOrdered::is_abnormal`, and
  written down nowhere.

  **Fail closed.** An undecidable comparison must never admit a value into a
  range: a reference range that silently included values it cannot order would
  report them as normal. The operators `>=` and `<=` do this by accident —
  `None` makes them false — which is why the rewrite in `D3.18c` had to say it
  on purpose rather than inherit it.

  The consequence runs one step further, and is why `A-39` is a **Medium**
  rather than a note about test coverage. `is_abnormal` asks
  `normal_range.contains(&value)`. If a comparison that should have decided
  returns "not comparable" — because a match arm was deleted, or a variant was
  added without one — the value is reported as **inside no range**, and
  therefore as **not abnormal**. A wrong answer that reads as reassurance.

- **D3.15** `DV_QUANTITY` values MUST be comparable only when their `units`
  strings are equal (`S1.9`).
- **D3.16** `DV_ORDINAL` and `DV_SCALE` values MUST be comparable only when
  their symbols come from the same terminology. Ordinal `2` on a pain scale and
  ordinal `2` on a sedation scale are unrelated.
- **D3.17** `DV_PROPORTION` values MUST be comparable only when their kinds
  match.
- **D3.18** Times carrying UTC offsets MUST be normalised to UTC before
  comparison.
- **D3.18a** *Resolved by design, not by compromise.* `Eq` on `Time`, `Date`,
  `DateTime` and `Duration` is **lexical** — two values are equal when they
  were written the same way — while chronological (or, for `Duration`, length)
  order compares what the value denotes under `D3.18`. So `11:00:00Z` and
  `12:00:00+01:00` order equal semantically and are **not** `==`.

  These types therefore do **not** implement `PartialOrd`/`Ord`. Rust requires
  `a == b` wherever `partial_cmp` reports `Some(Equal)`, and no fixed
  implementation of both traits together can satisfy that here — lexical
  equality is record identity (`db:M3.28`: the text is the stored value, `.5`
  and `.50` are different strings a record must round-trip, and `Hash` must
  agree with `Eq`), while semantic ordering is what a query needs, and neither
  moves without losing the other. Semantic comparison is instead a plain method,
  `semantic_cmp(&self, other: &Self) -> Option<Ordering>`, so `<`, `sort()`,
  and `dedup()` simply do not exist for these types — there is no operator
  that could silently mean the wrong one.

  Formerly recorded as the open finding `A-32`; closed once the trait impl
  was removed rather than left to disagree with `Eq`. Pinned by
  `iso8601::eq_is_lexical_and_semantic_cmp_is_not_the_same_question`.

- **D3.18b** *(added 2026-08-21)* `D3.18a` binds **every `DV_ORDERED`
  descendant and `DATA_VALUE`**, not only the base ISO 8601 types. None of them
  implements `PartialOrd`; semantic comparison is the named method
  `DvOrdered::semantic_cmp`, and `DataValue::semantic_cmp` for the enum.

  `D3.18a` was written about lexical form, because that is where the
  disagreement was first seen. **That is not the mechanism, only one instance of
  it.** Every `DV_ORDERED` carries `OrderedAttrs` — normal range, normal status,
  other reference ranges — and every one of them derives `PartialEq` over all
  its fields while comparing only its magnitude. So:

  | These two values | `==` | `partial_cmp` |
  | --- | --- | --- |
  | `DV_DATE_TIME` `11:00:00Z` and `12:00:00+01:00` | false | `Some(Equal)` |
  | `DV_QUANTITY` `5 mg` with `precision` 1 and with 2 | false | `Some(Equal)` |
  | `DV_QUANTITY` `5 mg` with and without `units_display_name` | false | `Some(Equal)` |
  | `DV_COUNT` `5` with and without a normal range | false | `Some(Equal)` |
  | `DV_PROPORTION` `1/4` with `precision` 1 and with 2 | false | `Some(Equal)` |

  Rust requires `a == b` if and only if `partial_cmp` reports `Some(Equal)`, so
  each row is a broken trait contract: `a != b` while `a <= b` and `a >= b` are
  both true. Nothing inside this crate depended on it — every comparison here
  goes through the ordering consistently — which is exactly why it survived. A
  caller's `binary_search`, `dedup_by`, `max_by`, or `sort_by` is where it
  surfaces, in someone else's code.

  Neither trait can move. Field equality is **record identity**: a
  `DV_QUANTITY` that records its precision is not the same stored value as one
  that does not, `db:M3.28` requires the text a record round-trips to be exactly
  what arrived, and a content digest is taken over those bytes (`db:M3.43`).
  Magnitude ordering is what a query and a reference range need. Making `==`
  semantic would make a canonicaliser that rewrote `1.10` as `1.1` pass its own
  round-trip test, which is `db:D-08` reintroduced.

  Formerly the open finding `A-35`, which named five types; the survey that
  closed it found **ten**.

- **D3.18c** *(added 2026-08-21)* `INTERVAL<T>` MUST NOT be bounded on
  `PartialOrd`. It is bounded on `SemanticOrd` — a trait this crate defines,
  whose single method is the same partial comparison — and implemented for the
  primitives an interval is used over and for every type in `D3.18b`.

  A blanket `impl<T: PartialOrd> SemanticOrd for T` is deliberately **not**
  written: it would collide with the explicit impls under Rust's coherence
  rules, and more to the point it would let a type that has the `D3.18b` defect
  reach `INTERVAL<T>` again without anyone deciding that it should.

## Quantities and proportions

- **D3.18d** *(added 2026-08-22)* A **real number** in the Reference Model MUST
  preserve its lexical form, as `D3.10` requires of ISO 8601. `1.50` is not
  `1.5`.

  **Every digit is preserved**, including trailing zeros and significant digits
  beyond what an `f64` can hold or distinguish. The one departure, measured
  rather than assumed: **exponent notation is normalised** to a lowercase `e`
  with an explicit sign, so `1e5` and `1E5` are both stored as `1e+5`. No digit
  is lost and the value is unchanged. A clinical magnitude is not written in
  scientific notation, but "not in practice" is not a guarantee, so the limit is
  written down and
  `real::tests::every_digit_survives_and_only_the_exponent_form_is_normalised`
  asserts it.

  The two are the same decision for the same reason. A `DV_DATE` of `2024-05` is
  a date known to the month, and a `DV_QUANTITY` of `1.50 mg` is a quantity
  measured to two decimal places — in both cases the number of digits *is* the
  clinical content, and a layer that normalises has destroyed it before storage
  sees it. `db:D-08` is that failure realised: MySQL rewrote a magnitude of
  `1.10` as `1.1`, and `db:M3.43` moved canonical JSON onto a byte-preserving
  column because of it.

  Until 2026-08-22 the crate lost the distinction one layer earlier than MySQL
  did. `DV_QUANTITY.magnitude` was an `f64`, so `1.50` became the same value as
  `1.5` at **parse** time and no storage rule could recover it.
  `security::canonical`'s own test recorded that as the limit of the guarantee.

- **D3.18e** *(added 2026-08-22)* A real MUST carry both forms: the text as
  written, and the `f64` it denotes. The text is **authoritative**; the `f64` is
  **derived** and is what comparison uses.

  This is `db:M3.31`'s two-column instant rule, in one value instead of two
  columns, and it produces the same split `D3.18a` and `D3.18b` describe:
  equality is lexical, because `1.50` and `1.5` are different records; ordering
  is numeric, because a reference range asks which is larger. The two disagree
  on values that denote the same number, so a real MUST NOT implement
  `PartialOrd` — `SemanticOrd` and `DvOrdered::semantic_cmp` carry the ordering
  (`D3.18c`).

- **D3.18f** *(added 2026-08-22)* Reading a real MUST be total. A value that
  never passed a constructor — `Deserialize` writes fields directly (`L10.1a`)
  — still has to be readable, so an accessor MUST NOT panic on one.
  `D3.30a` says the same about `DV_URI`, for the same reason and after the same
  defect (`A-36`).

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

- **D3.30** *(amended 2026-08-20)* `DV_URI` MUST require a well-formed scheme
  and MUST refuse spaces and control characters, **at both gates** (`L10.1`,
  `L10.1a`) — in the constructor, and in validation.
- **D3.31** *(amended 2026-08-20)* `DV_EHR_URI` MUST require the scheme `ehr`,
  **at both gates**. `LINK.target` is typed `DV_EHR_URI` precisely so that a
  link cannot point out of the record without saying so.

  Both requirements said only "MUST require", and the crate satisfied that
  reading in the constructor alone. `DV_EHR_URI` is `#[serde(transparent)]` over
  `DV_URI`, whose `Deserialize` is derived, so a link target arriving as JSON
  reached neither check: `{"value":"https://example.org/x"}` deserialized into a
  `DV_EHR_URI` whose scheme was `https`, and `{"value":"nocolon"}` deserialized
  into a `DV_URI` that **panicked** when its scheme was read. See
  [`audit.md`](audit.md) **A-36**. "At both gates" is now written into the
  requirement, because "MUST require" was read as "the constructor requires"
  once and would be again.

- **D3.30a** *(added 2026-08-20)* Reading any part of a `DV_URI` — its scheme,
  or the remainder — MUST be **total**. A value that never passed a constructor
  is a value this crate is obliged to be able to look at, and the accessor's
  answer for one with no scheme is the empty string, which compares unequal to
  every real scheme and so fails closed. An accessor that panics converts a
  malformed document into a denial of service against the process reading it.
- **D3.32** Validity of a `DV_EHR_URI` is **structural only**. The crate cannot
  resolve it, and documentation MUST NOT let "valid" be read as "resolvable".

## Identifiers as values

- **D3.33** `DV_IDENTIFIER.id` MUST be non-empty.
- **D3.34** `DV_IDENTIFIER`'s `Display` MUST render the type and issuer and MUST
  NOT render the identifier (`X11.6`).
