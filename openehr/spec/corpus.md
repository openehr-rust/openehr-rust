# External corpus runs — `am::cadl::parse_definition`

**Not normative, and not a gate.** This is the archetype half of
`tasks.md`'s "run an external corpus" item; [`json_corpus.md`](json_corpus.md)
is the other half, canonical JSON compositions against
`serde_json → Composition → Validate`. This file records what happened when
archetypes nobody in this repository wrote were run through the
`definition` reader: how many parsed, how many were refused and under which
stated reason, and which refusals turned out to be this crate's defects
rather than the corpus's. It exists because a conformance claim graded only
by its own tests is the weakest kind (`tasks.md` P1, from the FerroEHR
thread's post #15), and because every finding in
[`audit.md`](audit.md) was found by running something. The runner is
[`openehr/tests/adl_corpus.rs`](../tests/adl_corpus.rs), `#[ignore]`d,
because the corpus is not in this tree (§Licence).

Read the counts the way the runner prints them. **Parsed** means the
`definition` section produced a `CComplexObject`; it does not mean the
archetype is valid, and it says nothing about the header, terminology, or
rules sections, which the runner does not read. **Refused** means
`parse_definition` returned a `CadlError` with a stated reason; the
categories below are those reasons with per-file tokens stripped. A refusal
is a pass for `K15.6` (refuse by name) only when the name is *right* — the
run's first finding (`A-70`) was a refusal under the wrong name.

## How to run it

```sh
# The corpus is read where it is. Never copy it into this tree (§Licence).
git clone --depth 1 https://github.com/openEHR/adl-archetypes /some/where
git -C /some/where rev-parse HEAD                 # record this with the run
cd openehr
OPENEHR_ADL_CORPUS=/some/where \
OPENEHR_ADL_CORPUS_REPORT=/some/where/report.tsv \
  cargo test --test adl_corpus -- --ignored --nocapture
```

The TSV has one row per file: extension, path, `utf8`/`not-utf8`, verdict,
and for a refusal the category, byte offset, and raw reason.

## Licence

`openEHR/adl-archetypes` carries no licence file as of the commit below.
Its archetypes are therefore read from a checkout the runner is pointed
at, never vendored, never committed as fixtures, and never quoted here
beyond a file name and the single line that reproduces a finding. That is
also why the regression job `tasks.md` asks for — "fails when a
previously-parsing file stops parsing" — does not exist yet: it needs a
corpus this tree may carry.

## Run 1 — 2026-09-03

- Corpus: `openEHR/adl-archetypes` at `093c77ea003742b9540e3dd377d615e2b26f2996`.
- Crate: the tree at the commit that adds this file (`A-70` included).
- Files: 1,379 `.adls` (ADL 2), 593 `.adl` (ADL 1.4).

### Totals

| Extension | Files | Parsed | Refused | No `definition` | Not UTF-8 |
| --- | ---: | ---: | ---: | ---: | ---: |
| `.adls` | 1,379 | 206 | 1,172 | 1 | 2 |
| `.adl` | 593 | 26 | 567 | 0 | 2 |

Before `A-70` the same run parsed 178 `.adls` files and reported 4 with no
`definition` section. The one that remains is
`ADL2-reference/validity/basics/openEHR-TEST_PKG-ENTRY.FAIL_definition_missing.v1.0.0.adls`,
which is meant to have none. The other three were Latin-1 files
(a `©` in the description) that `read_to_string` had failed on — a runner
defect, fixed by decoding lossily and counting the encoding separately.
ADL 2 is UTF-8 by specification, so that column is a corpus fact worth
keeping, not a parser verdict.

The `.adl` column is expected to be low: ADL 1.4's cADL differs from ADL 2's
in ways this parser refuses by name (`at`-coded nodes are accepted, `matches
{*}` and id-less objects are not), and the reader here is an ADL 2 reader
that also happens to read the ADL 1.4 subset it shares (`K15.5`).

### Refusals by stated reason, `.adls`

| Files | Stated reason (tokens stripped) | What it is |
| ---: | --- | --- |
| 909 | occurrences omitted; this parser does not implement AOM's `effective_occurrences()` inference for a non-root node | The parser's own stated limitation, now with a number on it: **two thirds of every refusal**. See §Decisions. |
| 116 | expected `[`, found `matches` | An object with no node id — `ELEMENT matches {…}`. `cadl2.g4` `c_complex_object: rm_type_id '[' ( ROOT_ID_CODE \| ID_CODE ) ']' …` requires the id, so the refusal is correct; 89 of the files are `Development/CIMI`, 18 `Development/Intermountain` (ADL 1.5-era), and 3 are the reference suite's own `VCOID` invalid twins. |
| 43 | `CIMI` is not a valid id-, at-, or ac-code | `use_archetype ITEM_GROUP [CIMI-CORE-CLUSTER.…]` — the ADL 1.5 form with no id code (`ADL2-reference/upgrade/upgrade_from_15`). Correctly refused; the name could say "expected an id code before the archetype reference". |
| 35 | `SIBLING_ORDER` (`after`/`before`) is not implemented by this parser | Stated limitation, refused by name. |
| 29 | expected an attribute name, found `*` | `matches {*}` at attribute level — ADL 1.4, not in `cadl2.g4`. Correctly refused. |
| 15 | generic RM type parameters are not implemented by this parser | Stated limitation, refused by name. |
| 14 | `d` is not a valid `ISO8601_DATE` | **Candidate finding**, see §Candidates: `matches {yyyy-mm-dd}` is `DATE_CONSTRAINT_PATTERN`, valid ADL 2 (`cadl2_primitives.g4` `c_date`), refused under the wrong name. All 24 across both extensions are `Reference/ISO_13606`. |
| 13 | expected an RM type name, found `*` | `PARTICIPATION [at0001] … matches {*}` — ADL 1.4 (`Development/CIMI/reference`). Correctly refused. |
| 9 | a closed `ARCHETYPE_SLOT` is not implemented by this parser | Stated limitation (`A-62`), refused by name. |
| 9 | an unwrapped interval's primitive kind (`C_INTEGER` vs `C_REAL`) cannot be told apart without a wrapping `rm_type_id` | Stated limitation (`A-67`), refused by name. |
| 4 | `…` is not a valid at- or ac-code | Not yet examined. |
| 4 | expected `[`, found `}` | Not yet examined. |
| 2 | a single-valued attribute's child occurrences upper bound exceeds 1 | An AOM2 validity rule applied; whether the files are invalid twins or a parser misreading is **not yet examined**. |
| 1 each | five further categories (`unexpected content after the definition's root object`, `expected an RM type name, found end of input`/`(`/`}`, `expected matches, found occurrences`, a cardinality bound refusal) | Not yet examined; several are in `ADL2-reference/validity`, the reference suite's own invalid files, where a refusal is the right answer. |

### Refusals by stated reason, `.adl`

| Files | Stated reason | What it is |
| ---: | --- | --- |
| 288 | occurrences omitted (as above) | As above. |
| 212 | expected `[`, found `matches` (and 3 variants) | ADL 1.4 objects without node ids, `CODED_TEXT matches {*}`. Correctly refused for an ADL 2 reader. |
| 25 | expected an attribute name, found `*` | ADL 1.4 `matches {*}`. |
| 14 | expected an RM type name, found `*` | ADL 1.4 `matches {*}`. |
| 13 | generic RM type parameters | Stated limitation. |
| 10 | `d` is not a valid `ISO8601_DATE` | The same ISO 13606 date-pattern files, ADL 1.4 copies. |
| 3 | unwrapped interval primitive kind | Stated limitation. |
| 2 | `…` is not a valid at- or ac-code | Not yet examined. |

### Findings this run produced

- **`A-70`** (fixed the same day): a differential-form attribute
  (`/data/events cardinality matches {2..8; ordered}`,
  `openEHR-EHR-OBSERVATION.redefine_cardinality.v1.0.0.adls`) mis-parsed
  into three attributes and was refused as `VOKU (got "")`. The category
  disappeared entirely after the fix — the number that says the diagnosis
  was complete — and parsed `.adls` went from 178 to 206.

### Candidates — recorded, not yet findings

Each needs a test written and the grammar re-read before it gets an `A-`
number (`W0.19`, specification first; `W0.3`, nothing claimed unverified).

1. **`DATE_CONSTRAINT_PATTERN` refused under the wrong name.** `date
   matches {yyyy-mm-dd}` is `c_date: ( DATE_CONSTRAINT_PATTERN | date_value
   | … )` in `cadl2_primitives.g4`, and the same for time and date-time
   patterns. `am::cadl` has no arm for a pattern, reads `yyyy-mm-dd` as an
   ISO 8601 literal, and refuses the `d`. Twenty-four files, all ISO 13606.
   The fix is a `CPrimitive` pattern form for `C_DATE`/`C_TIME`/
   `C_DATE_TIME` (`A-63` modelled the value forms only) — a real construct,
   so the model changes first, then the parser.
2. **`primitive_kind` matches the type name case-insensitively.** Read
   while examining the same files: an RM class spelled `DATE` (ISO 13606
   has one) would be taken for the `Date` primitive. Not yet reproduced by a
   test; the corpus files above are refused earlier, on the pattern, so the
   run does not show it.
3. **Refusal names that are correct but unhelpful.** `CIMI is not a valid
   id-, at-, or ac-code` for an id-less `use_archetype`, and `expected an
   RM type name, found *` for ADL 1.4's `matches {*}`. Both are right to
   refuse; both could say what the writer actually did. `K15.6` asks for a
   name, and these are names; whether they are good enough is a judgement
   for the next run to make with the ADL 1.4 reader in view.

### Decisions the run asks for

- **`effective_occurrences`.** 1,197 of 1,739 refusals across both
  extensions are the parser declining to infer omitted `occurrences` on a
  non-root node. AOM2 states the rule (`C_OBJECT.effective_occurrences`:
  lower bound `0`; upper bound the owning attribute's cardinality upper
  bound if it is a container, else the RM's own multiplicity for the
  attribute, i.e. `1`). The parser already knows which of the two cases it
  is in — a `cardinality` clause is syntactic. Implementing the rule is a
  `K15.x` requirement in [`15-archetypes.md`](15-archetypes.md) first; what
  it changes is that `CObject::occurrences()` would return an *inferred*
  interval a reader cannot tell from a stated one unless the type records
  which it was. That distinction is the whole decision, and it is the
  matrix's to make, not this file's.

## Run 2 — 2026-09-03, after `A-71`

- Corpus: unchanged, `093c77ea003742b9540e3dd377d615e2b26f2996`.
- Crate: run 1's tree plus `A-71` (`K15.32`): an omitted `occurrences` is
  carried unstated and inferred from its owning attribute, not refused.

### Totals

| Extension | Files | Parsed | Refused | No `definition` | Not UTF-8 |
| --- | ---: | ---: | ---: | ---: | ---: |
| `.adls` | 1,379 | **774** (run 1: 206) | 604 | 1 | 2 |
| `.adl` | 593 | 29 (run 1: 26) | 564 | 0 | 2 |

The `.adls` count went from 15% to 56% on one change, which is what run
1's "two thirds of every refusal" predicted. The `.adl` count barely
moved: ADL 1.4 files are refused earlier, on syntax an ADL 2 reader
correctly does not accept (`matches {*}`, id-less objects, `0|[local::…]`
ordinals).

### Refusals by stated reason, `.adls`

| Files | Stated reason (tokens stripped) | What it is |
| ---: | --- | --- |
| 179 | an unwrapped interval's primitive kind (`C_INTEGER` vs `C_REAL`) cannot be told apart without a wrapping `rm_type_id` | **Candidate finding**, §Candidates 4: the grammar *does* tell them apart. 80 are `Reference/CKM_2013_12_09`, 40 `Reference/Nehta_2014_04_25` — the clinical corpus. |
| 130 | expected `[`, found `matches` | Id-less objects (`ELEMENT matches {…}`), as in run 1; correctly refused. |
| 66 | generic RM type parameters are not implemented by this parser | Stated limitation, refused by name. Up from 15 because 51 files that used to fail earlier on `occurrences` now reach it. |
| 61 | `CIMI` is not a valid id-, at-, or ac-code | ADL 1.5-form `use_archetype` (60 `Development/CIMI`); correctly refused, name could be better (run 1, candidate 3). |
| 54 | `SIBLING_ORDER` is not implemented by this parser | Stated limitation. |
| 22 | expected `[`, found `}` | `duration_attr1 matches {PT0S}` — an **unwrapped duration literal** taken for an RM type name. §Candidates 5. |
| 14 | `d` is not a valid `ISO8601_DATE` | `DATE_CONSTRAINT_PATTERN`, run 1's candidate 1. |
| 14 | expected `[`, found `-` | `value matches {yyyy-??-??T??:??:??}` — `DATE_TIME_CONSTRAINT_PATTERN`, the same candidate 1, unwrapped this time. |
| 13 | a closed `ARCHETYPE_SLOT` is not implemented by this parser | Stated limitation (`A-62`). |
| 11 | a single-valued attribute's child occurrences upper bound exceeds 1 | **`A-71`'s residual made visible**: `items matches {` with no `cardinality` clause is built single-valued, so a child stating `0..*` is refused. 4 are CKM 2013, 2 the reference suite's own `VACSO` invalid twins (where the refusal is right). |
| 9 | expected an attribute name, found `*` | ADL 1.4 `matches {*}`; correctly refused. |
| 8 | expected `[`, found `/` | Not yet examined. |
| 6 | `…` is not a valid at- or ac-code | Not yet examined. |
| 3 each and fewer | `unexpected content after the definition's root object` (3), `expected an RM type name, found (` (3), `expected }, found \|` (2), one each of four more | Not yet examined; several are the reference suite's own invalid files. |

### Refusals by stated reason, `.adl`

| Files | Stated reason | What it is |
| ---: | --- | --- |
| 442 | expected `[`, found `matches` | ADL 1.4 id-less objects. Correctly refused. |
| 35 | generic RM type parameters | Stated limitation. |
| 28 | expected an attribute name, found `*` | ADL 1.4 `matches {*}`. |
| 25 | expected `}`, found `\|` | ADL 1.4 `C_DV_ORDINAL` (`0\|[local::at0003]`). Correctly refused by an ADL 2 reader; `K15.8` is where this belongs. |
| 13 | expected an RM type name, found `*` | ADL 1.4 `matches {*}`. |
| 10 | `d` is not a valid `ISO8601_DATE` | Date patterns, candidate 1. |
| 5 | unwrapped interval primitive kind | Candidate 4. |
| 3 | expected `[`, found `occurrences` | Not yet examined. |
| 2 | `…` is not a valid at- or ac-code | Not yet examined. |

### Findings this run produced

- **`A-71`** (fixed, breaking): the model itself could not carry an
  unstated `occurrences`; `K15.32` written first, then
  `Option<MultiplicityInterval>` on every `C_OBJECT` type,
  `effective_occurrences`, and the parser carrying rather than refusing.
  The test fixture that had asserted a named refusal of the real
  `openEHR-EHR-CLUSTER.device.v1.0.0` definition since `A-62` now asserts
  the parse.

### Candidates — added by this run

4. **An unwrapped interval's kind is decided by the grammar, not
   unknowable.** `A-67` refused `integer_attr3 matches {|0..100|; 10}` on
   the ground that `C_INTEGER` and `C_REAL` "cannot be told apart without
   a wrapping `rm_type_id`". `odin_values.g4` says otherwise:
   `integer_interval_value` is built from `INTEGER` tokens (`DIGIT+`) and
   `real_interval_value` from `REAL` tokens (`DIGIT+ '.' DIGIT+`,
   `base_lexer.g4`), so `|0..100|` *is* a `C_INTEGER` and `|0.0..100.0|`
   *is* a `C_REAL` — the same lexical distinction this parser already
   applies to an unwrapped single value (`Token::Integer` vs
   `Token::Real`). 184 files across both extensions, the largest refusal
   left, and 120 of them the clinical corpus. The fix is to read the first
   bound's token kind and dispatch; the refusal should survive only for a
   bound mixing the two (`|0..100.0|`), which the grammar also refuses.
5. **Unwrapped temporal literals.** `duration_attr1 matches {PT0S}` and
   the date-time pattern above reach `c_objects` as a `Word`, are taken
   for an RM type name, and are refused as "expected `[`". `c_inline_primitive_object` admits every primitive kind, and `ISO8601_DURATION`,
   `ISO8601_DATE`, `ISO8601_DATE_TIME`, and the `*_CONSTRAINT_PATTERN`
   tokens each have a lexical shape (`base_lexer.g4`) this parser's
   `read_iso8601` already recognises inside a typed primitive. 36 files,
   with candidate 1's patterns in the same place.

### Decisions the run asks for

- **Reference Model multiplicity.** `A-71`'s residual and the 11-file row
  above are the same fact: without a table of which RM attributes are
  containers, `items matches {` with no `cardinality` clause is
  single-valued here and `0..*` in AOM2. The table is small (the RM's
  `List`/`Set`-typed attributes) and this crate already has the RM in
  `rm::`; deriving it there, once, is the `lib:A-33` shape — one rule, one
  home — rather than a second table in `am::cadl`. A requirement first.

## Run 3 — 2026-09-03, after `A-72`, `A-73`, `A-74`

- Corpus: unchanged, `093c77ea003742b9540e3dd377d615e2b26f2996`.
- Crate: run 2's tree plus `A-72` (an unwrapped interval's kind decided by
  its first bound's token), `A-73` (`allow_archetype … closed` parsed), and
  `A-74` (the relop interval spelling `|>=0.0|`), the last found by the
  intermediate run between the first two and this one.

### Totals

| Extension | Files | Parsed | Refused | No `definition` | Not UTF-8 |
| --- | ---: | ---: | ---: | ---: | ---: |
| `.adls` | 1,379 | **916** (run 2: 774; run 1: 206) | 462 | 1 | 2 |
| `.adl` | 593 | 32 (run 2: 29) | 561 | 0 | 2 |

By directory, the clinical corpora now lead: `Reference/CKM_2013_12_09`
237 of 322 parsed, `Reference/Nehta_2014_04_25` 132 of 164. The reference
suite (`ADL2-reference`) is 219, a third of it the `validity` directory,
where a refusal is the expected answer for the invalid twins.

### Refusals by stated reason, `.adls`

| Files | Stated reason (tokens stripped) | What it is |
| ---: | --- | --- |
| 130 | expected `[`, found `matches` | Id-less objects; correctly refused (runs 1–2). |
| 72 | generic RM type parameters are not implemented by this parser | Stated limitation, refused by name. Now the largest *limitation* left; the count rises each run as more files reach it. |
| 61 | `CIMI` is not a valid id-, at-, or ac-code | ADL 1.5-form `use_archetype`; correctly refused (run 1, candidate 3). |
| 54 | `SIBLING_ORDER` is not implemented by this parser | Stated limitation. |
| 32 | expected `[`, found `}` | Unwrapped temporal literals (`{PT0S}`) taken for RM type names — candidate 5, up from 22 as more files reach it. |
| 28 | a single-valued attribute's child occurrences upper bound exceeds 1 | **`A-71`'s residual**, 19 of them CKM/NEHTA: `items matches {` with no `cardinality` clause under a `CLUSTER`, built single-valued. The Reference Model multiplicity decision (`plan.md`) is now the largest lever on the clinical corpus. |
| 21 | expected `[`, found `-` | `DATE_TIME_CONSTRAINT_PATTERN` unwrapped — candidate 1/5. |
| 14 | `d` is not a valid `ISO8601_DATE` | `DATE_CONSTRAINT_PATTERN` — candidate 1. |
| 13 | expected `[`, found `/` | Not yet examined. |
| 9 | expected an attribute name, found `*` | ADL 1.4 `matches {*}`; correctly refused. |
| 6 | `…` is not a valid at- or ac-code | Not yet examined. |
| 3 | an unwrapped temporal interval … is not implemented by this parser | `A-72`'s own named refusal; the three files are the reference suite's temporal-interval features. |
| 3 each and fewer | `unexpected content after the definition's root object` (3), `expected an RM type name, found (` (3), `expected a primitive value, found -` (2), `expected }, found \|` (2), one each of four more | Not yet examined. |

The `.adl` column is unchanged in shape: 471 id-less objects and `matches
{*}`, the rest stated limitations.

### Findings this run produced

- **`A-72`** (fixed): `A-67`'s "cannot be told apart" was wrong;
  `odin_values.g4` decides by token. 184 files.
- **`A-73`** (fixed): the closed slot stayed refused after `A-71` removed
  the reason. 13 files.
- **`A-74`** (fixed): the relop spelling failed on its `=`, not by name.
  106 files, surfaced only once `A-72` let intervals reach the reader — a
  refusal can hide another, which is why runs are recorded one at a time.

### Candidates — status

1. `DATE_CONSTRAINT_PATTERN` (and the date-time form): **open**, 35 files.
2. `primitive_kind` case-insensitivity: **open**, still not reproduced.
3. Refusal names that are correct but unhelpful: **open**.
4. Unwrapped interval kind: **closed, `A-72`**.
5. Unwrapped temporal literals: **open**, 32 files; shares candidate 1's
   fix (a `CPrimitive` pattern form and unwrapped temporal dispatch by
   lexical shape).
6. **New — the `+/-` interval spelling** is refused by name and no corpus
   file uses it; recorded so its absence from the numbers is not read as
   support.

### Decisions the run asks for

- **Reference Model multiplicity** (run 2, restated): 28 files now, 19 of
  them clinical, and the count will keep rising as other refusals fall.
  The decision is in `plan.md`; a requirement first.

## Run 4 — 2026-09-04, after `A-75`

- Corpus: unchanged, `093c77ea003742b9540e3dd377d615e2b26f2996`.
- Crate: run 3's tree plus `A-75`: temporal `*_CONSTRAINT_PATTERN`s read
  for all four kinds, wrapped or unwrapped, and the four temporal kinds
  reachable unwrapped (`c_inline_primitive_object`'s own full grammar,
  not the five-kind subset this parser used to admit).

### Totals

| Extension | Files | Parsed | Refused | No `definition` | Not UTF-8 |
| --- | ---: | ---: | ---: | ---: | ---: |
| `.adls` | 1,379 | **959** (run 3: 916) | 419 | 1 | 2 |
| `.adl` | 593 | 32 (run 3: 32) | 561 | 0 | 2 |

The `d is not a valid ISO8601_DATE` category (14 files, all
`Reference/ISO_13606`) and its duration sibling (1 file) both dropped —
but not to zero, which is what led straight to `A-76`: one file still
failed the same way, for a different reason `A-75` does not touch.

### Refusals by stated reason, `.adls`, that changed

| Files | Stated reason | Change |
| ---: | --- | --- |
| 5 (was 3) | an unwrapped temporal interval … is not implemented | Up: `A-75`'s unwrapped dispatch reaches more files that then meet the one remaining named refusal `A-72` already gives temporal interval bounds. |
| 2 (new) | expected a primitive value, found `-` | Both `Reference/CKM_2013_12_09` and `Reference/Nehta_2014_04_25` copies of `openEHR-EHR-CLUSTER.symptom.v1`: `[{-3}, {[at49]}]`, a *negative* unwrapped integer tuple item. Not yet examined — likely `expect_signed_integer`'s own reach inside a tuple row versus the plain `c_objects` dispatch path this parser takes first. |

## Run 5 — 2026-09-04, after `A-76`

- Corpus: unchanged, `093c77ea003742b9540e3dd377d615e2b26f2996`.
- Crate: run 4's tree plus `A-76`: `primitive_kind` matches exactly, not
  case-insensitively.

### Totals

| Extension | Files | Parsed | Refused | No `definition` | Not UTF-8 |
| --- | ---: | ---: | ---: | ---: | ---: |
| `.adls` | 1,379 | **967** (run 4: 959) | 411 | 1 | 2 |
| `.adl` | 593 | 33 (run 4: 32) | 560 | 0 | 2 |

The `ISO8601_DATE`/`ISO8601_DURATION` categories are gone entirely. One
category rose in their place: `children require more occurrences than the
cardinality permits` went from 1 to 8 files (7 in `.adls`, 7 in `.adl`,
all `Reference/ISO_13606`) — not a regression, but the files parsing
*further* than before and meeting a real `VACMCU` check the earlier
misparse never let them reach. Not yet examined for whether the archetype
or this parser's cardinality inference is at fault.

### Findings runs 4 and 5 produced

- **`A-75`** (fixed): temporal patterns and unwrapped temporal literals,
  corpus run 1's candidates 1 and 5, closed together.
- **`A-76`** (fixed): `primitive_kind`'s case-insensitive match, corpus run
  1's candidate 2 — recorded as "not yet reproduced by a test" there, and
  reproduced for real chasing `A-75`'s own residual refusal.

### Candidates — status after run 5

1. `DATE_CONSTRAINT_PATTERN`: **closed, `A-75`**.
2. `primitive_kind` case-insensitivity: **closed, `A-76`**.
3. Refusal names that are correct but unhelpful: **open**.
4. Unwrapped interval kind: **closed, `A-72`**.
5. Unwrapped temporal literals: **closed, `A-75`**.
6. The `+/-` interval spelling: **open**, refused by name, still unused
   in the corpus.
7. **New — a negative unwrapped integer in a `C_ATTRIBUTE_TUPLE` row**
   (`{-3}`), 2 files, `expected a primitive value, found`-``. Not yet
   examined.
8. **New — a `VACMCU` cardinality violation** surfaced by `A-76`, 8 files,
   all ISO 13606. Not yet examined; may be the corpus's own defect, not
   this parser's.

### Decisions the run asks for

- **Reference Model multiplicity** (runs 2–3, restated): still open,
  `plan.md`.

## Run 6 — 2026-09-04, after `A-77`

- Corpus: unchanged, `093c77ea003742b9540e3dd377d615e2b26f2996`.
- Crate: run 5's tree plus `A-77`: a negative unwrapped number
  (`{-3}`) is dispatched, wrapped or unwrapped.

### Totals

| Extension | Files | Parsed | Refused | No `definition` | Not UTF-8 |
| --- | ---: | ---: | ---: | ---: | ---: |
| `.adls` | 1,379 | **969** (run 5: 967) | 409 | 1 | 2 |
| `.adl` | 593 | 33 (run 5: 33) | 560 | 0 | 2 |

The `expected a primitive value, found` `-`` category is gone — both files
that produced it (candidate 7) now parse.

### Candidates — status after run 6

1. `DATE_CONSTRAINT_PATTERN`: **closed, `A-75`**.
2. `primitive_kind` case-insensitivity: **closed, `A-76`**.
3. Refusal names that are correct but unhelpful: **open**.
4. Unwrapped interval kind: **closed, `A-72`**.
5. Unwrapped temporal literals: **closed, `A-75`**.
6. The `+/-` interval spelling: **open**, still unused in the corpus.
7. A negative unwrapped integer in a tuple row: **closed, `A-77`**.
8. A `VACMCU` cardinality violation surfaced by `A-76`, 8 files, all ISO
   13606: **examined and closed, no code change** — the archetype's own
   inconsistency, not this parser's. Seven of the eight are
   `Reference/ISO_13606/Spanish_MOH` `COMPOSITION`s that share one
   boilerplate `SECTION`: `members cardinality matches {0..1; unordered;
   unique} matches { allow_archetype ENTRY[…] occurrences matches {1} …
   allow_archetype ENTRY[…] occurrences matches {1} … }` — two mandatory
   children under an attribute whose cardinality permits at most one,
   contradictory on its own terms and correctly refused
   (`CAttribute::container`'s own `VACMCU` check, `A-71`'s residual
   documentation). Present verbatim in every file examined, so it reads as
   a systematic artefact of whatever produced these from EN13606, not a
   one-off. The eighth, `ADL2-reference/validity/structure/
   openEHR-EHR-OBSERVATION.WACMCL_container_items_out_of_bounds.v1.0.0.adls`,
   is the reference suite's own deliberately invalid fixture — its name
   says so, and refusing it is the correct answer, the same as every other
   `validity` directory file this corpus contains.

### Decisions the run asks for

- **Reference Model multiplicity** (runs 2–3, restated): still open,
  `plan.md`.

## Trademarks

openEHR® is the registered trademark of the openEHR Foundation and is used
with the permission of openEHR International. Use of the trademark does not
constitute endorsement of this product by openEHR International or openEHR
Foundation.
