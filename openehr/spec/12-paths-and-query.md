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
- **Q12.9b** *(amended 2026-08-21 — was a limitation, now a rule)* A numeric
  literal MAY carry a leading `-`, and the crate MUST parse one. `-1` and `-2.5`
  are values.

  **This was a stated limitation until 2026-08-21**, and the reason it stayed
  one is the reason the amendment is worth reading: `-` is also the character
  that separates the parts of an archetype id, so a sign could not simply be
  added to the number scanner. `A-27` recorded the decision as unmade.

  The decision, now made: **the sign is resolved by the parser, at a position
  where an operand is expected — never by the number scanner.** The lexer gains
  `-` as an ordinary symbol and nothing else changes there. An archetype id
  cannot be affected, because `openEHR-EHR-COMPOSITION.encounter.v1` begins with
  a letter and is therefore scanned as a **word**, which absorbs its own
  hyphens and never reaches the symbol branch. A `-` only ever stands alone
  where a word did not claim it.

  What follows from resolving it in the parser rather than the lexer:

  - `WHERE o/value/magnitude > -2.5` is a comparison against minus two point
    five — a base excess, a temperature difference, a scale scored below zero.
  - `MATCHES {-1, 0, 1}` is a three-element set, because `MATCHES` parses its
    values as operands like everything else.
  - `> -openEHR-EHR-COMPOSITION.encounter.v1` is an **error**, because a sign
    must be followed by a number. It is refused rather than read as anything.

- **Q12.9e** *(added 2026-08-21)* A rendered numeric literal MUST lex back as
  the **same kind** of literal. A real that renders without a decimal point is a
  real that comes back an integer.

  `format!("{v}")` writes `0` for `0.0` and `-0` for `-0.0`, and this lexer
  reads a run of digits with no `.` as an integer — so `Number(0.0)` round
  tripped to `Integer(0)`, a type change, which `Q12.15`'s tree equality
  forbids. It had been true since the renderer was written and was invisible
  until `Q12.9b` allowed a sign, because `-0.0` is the first value where the
  *text* also differs: `-0` reparses as `Integer(0)` and renders `0`.

  Display and not `Debug`: `Debug` for `f64` may use exponent form for extreme
  values, and `e` is not in this number scanner, so `1e300` would lex as `1`
  followed by a word. Display writes the full decimal expansion and never does
  that.

- **Q12.9d** *(added 2026-08-21)* `LIMIT` and `OFFSET` MUST refuse a signed
  literal, and MUST say so.

  They take a count, not a value, and `Q12.9b` makes `-5` lexically
  well-formed where it previously was not — so what used to be refused
  incidentally, as `unexpected character`, now has to be refused deliberately.
  A `LIMIT` that clamped `-5` to `0` would return an empty result set that looks
  like an answer (`db:P6.15`).

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
- **Q12.15** *(amended 2026-08-21)* A parsed query MUST render to text that
  re-parses to an **equal** query — the same tree, not merely one that parses.

  "Equivalent" was the original word and it was too weak to be checkable. The
  `FROM` renderer emitted `(a CONTAINS b OR c)` for `Or(Contains(a, b), c)`, and
  that text re-parses cleanly into `Contains(a, Or(b, c))` — *a containing
  either b or c*, where the caller wrote *either (a containing b) or c*. Both
  are valid queries over different records, so "it re-parses" was satisfied and
  the caller's query had still been rewritten. See [`audit.md`](audit.md)
  **A-37**.

- **Q12.15a** *(added 2026-08-21)* The renderer MUST emit whatever parentheses
  the grammar needs to reproduce the tree it was given.

  `FROM` puts `CONTAINS`, `AND` and `OR` at one precedence level and lets
  `CONTAINS` take the whole remainder as its right operand, so an operand that
  is not a bare class is parenthesised. A bare class is not, so the ordinary
  query still renders as `EHR e CONTAINS COMPOSITION c` rather than
  `(EHR e) CONTAINS (COMPOSITION c)`.

- **Q12.15b** *(added 2026-08-21)* Lexing a string literal MUST preserve it
  exactly, and rendering one MUST escape whatever the lexer treats as special.

  The lexer copied the literal one **byte** at a time into a `String`, widening
  each UTF-8 byte to its own `char`: `'Müller'` lexed to `'MÃ¼ller'`, and a
  `WHERE` against it matched nobody. The query parsed, checked clean, and was
  about a different string — there was nothing for a caller to notice. Scanning
  by byte is still correct, because the only bytes the lexer examines are ASCII
  delimiters and an ASCII byte never occurs inside a multi-byte sequence; what
  was wrong was copying by byte.

  Rendering escapes `'` and `\` and nothing else, because the lexer's rule is
  "a backslash introduces the next character literally" rather than a C-style
  table — so a rendered `\n` would mean the letter `n`.
