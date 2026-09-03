# External corpus runs — `am::cadl::parse_definition`

**Not normative, and not a gate.** This file records what happened when
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

## Trademarks

openEHR® is the registered trademark of the openEHR Foundation and is used
with the permission of openEHR International. Use of the trademark does not
constitute endorsement of this product by openEHR International or openEHR
Foundation.
