# 12. Paths and query

Requirement prefix: `Q12`.

## openEHR paths

- **Q12.1** The crate MUST parse openEHR paths of the form
  `/attribute[predicate]/attribute…`, with predicates in these forms:
  `[at0004]`, `[openEHR-EHR-OBSERVATION.x.v2]`, `['Systolic']`,
  `[at0004, 'Systolic']`, `[archetype_node_id='at0004']`,
  `[name/value='Systolic']`, `[2]`, and conjunctions with `and` or `,`.
- **Q12.2** A parse failure MUST report the offset at which parsing stopped.
- **Q12.3** Quoted digits MUST be a name, not an index: `['3']` is an element
  named `3`.
- **Q12.4** Index predicates are **zero-based**. openEHR's own examples are
  inconsistent and implementations differ; this crate documents zero-based and
  means it, because every other index in it is zero-based and one crate with two
  conventions produces off-by-one errors that read as correct code.
- **Q12.5** `path_exists`, `path_unique`, `items_at_path`, and `item_at_path`
  MUST all be available, and `item_at_path` MUST **fail** on an ambiguous path
  rather than returning the first match. Taking the first of three repeated
  `ELEMENT`s silently returns one of three diagnoses.
- **Q12.6** A path naming an attribute the node's class does not have MUST
  resolve to no match rather than to a parse error: the path is well formed and
  this instance does not have it.
- **Q12.7** Navigation MUST cover the composition tree from `COMPOSITION` down
  to primitive attributes of a `DATA_VALUE`, including `value/magnitude`,
  `value/units`, `value/value`, and `defining_code`.
- **Q12.7a** Navigation MUST reach the `DV_ORDERED` attributes —
  `normal_range`, `other_reference_ranges`, `normal_status` — and MUST continue
  through a `DV_INTERVAL` to `lower`, `upper`, the two `*_unbounded` flags and
  the two `*_included` flags, and through a `REFERENCE_RANGE` to `meaning` and
  `range`. A reference range is queried in practice — "results outside their own
  normal range" is a population query a clinician actually asks — and a path
  that stops at the value cannot express it.
- **Q12.7b** The `*_unbounded` flags MUST be navigable even though they are
  derived rather than stored (`base::interval`). Whether the answer is looked up
  or computed is not something a path should have to know.
- **Q12.8** A parsed path MUST print in the long predicate form and MUST
  re-parse to an equal path.

## AQL

- **Q12.9** The crate MUST parse `SELECT` (with `DISTINCT` and `TOP`), aliases,
  function calls, `FROM` with `CONTAINS` / `NOT CONTAINS` / `AND` / `OR` and
  parentheses, archetype and standard predicates, `WHERE` with the comparison
  operators and `AND` / `OR` / `NOT` / `EXISTS` / `MATCHES` / `LIKE`, `$`
  parameters, `ORDER BY` with `ASC` / `DESC`, and `LIMIT` / `OFFSET`.
- **Q12.9a** Constructs the crate does not model — `SELECT *`, the `VERSION`
  class extension, terminology-function subqueries — MUST be **refused with an
  error naming this requirement**, never parsed-and-ignored. A
  partially-understood query that looks fully understood is the failure mode.
- **Q12.9b** *Limitation.* A numeric literal MUST NOT be required to carry a
  sign, and the crate does not parse one: `-1` and `-2.5` are refused at the
  lexer. This is a departure from `Q12.9`'s "comparison operators" in practice,
  because a condition such as `WHERE o/value/magnitude > -2.5` is ordinary
  clinical AQL — a base excess, a temperature difference, a scale scored below
  zero — and cannot be written here at all.

  It is recorded rather than fixed because the fix has a decision in it: `-` is
  also the character that separates the parts of an archetype id, so a sign
  cannot simply be added to the number scanner without deciding what
  `[openEHR-EHR-COMPOSITION.encounter.v1]` means when it follows an operator.
  See `A-27`.

- **Q12.9c** *Limitation.* A bracketed predicate MUST be either the archetype
  shorthand (`c[openEHR-EHR-COMPOSITION.encounter.v1]`) or a full condition.
  There is no node-id shorthand: `c[at0001]` is refused rather than read as
  `archetype_node_id = 'at0001'`. Recorded as `A-30`, and pinned by a test —
  refusal is also what keeps a bare word from being reported as an archetype id
  to the authorisation check of `Q12.13`.

- **Q12.10** The crate MUST NOT execute AQL (`S1.5`), and no API may return
  anything shaped like a result set.
- **Q12.11** Keywords MUST be case-insensitive.
- **Q12.12** A parse failure MUST report a byte offset.
- **Q12.13** The archetype ids, aliases, and parameters a query touches MUST be
  enumerable **before** it runs. That is what an authorisation check needs.
- **Q12.14** A static check MUST report a path rooted at an alias that `FROM`
  does not bind. This is the error AQL's syntax makes easiest to write — rename
  a class alias and miss one `SELECT` column — and hardest to see, because such
  a query is syntactically perfect and returns nothing.
- **Q12.15** A parsed query MUST render to text that re-parses to an equivalent
  query.
