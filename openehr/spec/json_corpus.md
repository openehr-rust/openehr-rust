# External corpus runs — `serde_json → Composition → Validate`

**Not normative, and not a gate.** The JSON half of `tasks.md`'s "run an
external corpus" item — [`corpus.md`](corpus.md) is the archetype half,
`am::cadl::parse_definition` against real ADL text; this file is real
canonical JSON compositions against `serde_json::from_str::<Composition>`
and [`Validate::validate`](../src/validation.rs). Both exist for the same
reason: a conformance claim graded only by its own tests is the weakest
kind (`tasks.md` P1, from the FerroEHR thread's post #15), and every
finding in [`audit.md`](audit.md) was found by running something. The
runner is [`openehr/tests/json_corpus.rs`](../tests/json_corpus.rs),
`#[ignore]`d because the corpus is not in this tree (§Licence).

Read the counts the way the runner prints them. **Parsed clean** means
`serde_json` built a `Composition` and `validate()` found nothing wrong.
**Parsed with violations** means it deserialized but `validate()` did not
pass it — a violation is not necessarily a defect in the fixture; several
below are this crate's own defects surfacing as a false violation, and
`corpus.md`'s own §Read the counts warns of the same for a refusal. **Did
not parse** means `serde_json::from_str` itself returned an error; the
categories are that error's own message with line/column numbers and
quoted tokens stripped, the same reasoning `corpus.md`'s `category`
applies to a `CadlError`.

## How to run it

```sh
# The corpus is read where it is. Never copy it into this tree (§Licence).
git clone --depth 1 https://github.com/ehrbase/openEHR_SDK /some/where
git -C /some/where rev-parse HEAD                 # record this with the run
cd openehr
OPENEHR_JSON_CORPUS=/some/where/test-data/src/main/resources/composition/canonical_json \
OPENEHR_JSON_CORPUS_REPORT=/some/where/report.tsv \
  cargo test --test json_corpus -- --ignored --nocapture
```

The TSV has one row per file: the relative path, then either `valid`,
`invalid\t<n> violations\t<comma-separated invariant names>`, or
`parse-error\t<category>`.

## Scope

Only files whose top-level `_type` is `COMPOSITION` are meaningful here —
the directory's own name (`composition/canonical_json`). `EHRbase`'s SDK
carries other canonical forms (`EHR_STATUS`, `CONTRIBUTION`, …) that
`tests/canonical_json.rs` already exercises against fixtures built
in-process; running those through this file's own runner would need a
second `serde_json::Value`-level dispatch by `_type` this run does not do,
and is not this run's subject. The corpus itself also carries files that
are not canonical JSON at all despite living in this directory — see
§Findings, "out of scope, not a defect" below — and the runner does not
try to filter them out before reading; it reports what happens, which is
the same discipline `adl_corpus.rs` applies to a `no-definition-section`
verdict.

## Licence

`ehrbase/openEHR_SDK` is Apache License 2.0 (`LICENSE.md`), verified by
reading the file rather than trusting GitHub's own licence detector, which
reports `NOASSERTION` for this repository because the file is named
`LICENSE.md` rather than the bare `LICENSE` its heuristic expects. Unlike
`openEHR/adl-archetypes` (`corpus.md`'s own subject, which carries no
licence at all), vendoring this corpus into this tree would be permitted.
It is still read from a checkout the runner is pointed at, never vendored:
consistency with `corpus.md`'s own practice, a decision made once rather
than per corpus, not a requirement this licence itself imposes.

## Run 1 — 2026-09-05

- Corpus: `ehrbase/openEHR_SDK` at `e57511c6aca27ed501d31d663762c37c3491e74e`,
  directory `test-data/src/main/resources/composition/canonical_json`.
- Crate: three passes against the tree at the commit that adds this file —
  before any fix, after `A-78`, and after `A-79` — recorded as one run
  since all three passes happened the same day chasing what the first pass
  found.
- Files: 57 (58 minus one `.bak`, correctly excluded by extension).

### Totals

| Pass | Parsed clean | Parsed with violations | Did not parse |
| --- | ---: | ---: | ---: |
| Before any fix | 8 | 7 | 42 |
| After `A-78` (comma decimal sign) | 8 | 13 | 36 |
| After `A-79` (`TEMPLATE_ID` whitespace) | **14** | **15** | **28** |

The middle pass raising "parsed with violations" rather than "parsed
clean" is expected, not a regression: files that used to fail to
*deserialize* now deserialize and meet `validate()` for the first time,
and several of them turn out to carry a real, separate violation
(`Is_archetype_root`, mostly) that a parse failure had been hiding.

### What did not parse, examined file by file

Every one of the 28 remaining files was read, not merely categorised.
Twenty-four are out of scope or deliberately invalid; three are `A-02`'s
own open finding; one is `HIER_OBJECT_ID`'s own already-tested grammar
correctly refusing two fixture files' inconsistency.

| Files | What | Disposition |
| ---: | --- | --- |
| 15 | `uid.value` is the literal placeholder text `__THIS_SHOULD_BE_MODIFIED_BY_THE_TEST_::ehrbase.org::1` | **Out of scope, not a defect.** A template-substitution marker the SDK's own Java test harness rewrites before use, not a real canonical value. Refusing a literal placeholder is correct. |
| 5 | `rawdb_*.json` — keys shaped `/$CLASS$`, `/$PATH$`, AQL-flattened attribute paths | **Out of scope, not a defect.** `EHRbase`'s own internal flat-query-result format, not canonical JSON at all, despite living in this directory. |
| 1 | `full_composition.json` uses `@class` as its type tag, not `_type` | **Out of scope, not a defect.** A different (Jackson-native) serialization convention, not openEHR's canonical `_type` tagging. |
| 1 | `composition_with_dvinterval_composite.json`: `content` is a JSON object, not an array | **Not a defect.** `COMPOSITION.content` is `List<CONTENT_ITEM>` unambiguously; a map there is malformed regardless of implementation. |
| 1 | `invalid.json`: a garbage date value, a literal key named `BAD-----------`, an empty `uid` | **Deliberately invalid**, the file's own name says so — the same shape as the ADL corpus's own `validity` directory. |
| 2 | `cardinality_of_section__full.json`, `nested.en.v1.json`: `uid` is tagged `HIER_OBJECT_ID` but its value has two `::` separators (`uuid::ehrdb::1`), the shape of an `OBJECT_VERSION_ID` | **The fixture's own inconsistency, not a defect.** `HIER_OBJECT_ID` admits at most one `::` extension (already tested, `hier_object_id_rejects_a_double_colon_extension`); a value shaped like a different type under this type's tag is wrong regardless of implementation. |
| 3 | `all_types_no_multimedia.json` and two siblings: `DV_DATE.value` is `"20190114"`, ISO 8601's basic format | **`A-02`, open.** `D3.13a` refuses this deliberately, on two grounds; one of them ("does not appear in openEHR canonical JSON") is false, per exactly these three files. See `audit.md`. |

### Findings this run produced

- **`A-78`** (fixed): a comma decimal sign in a fractional second, refused
  though `openEHR/adl-antlr`'s own grammar names it. 21 files.
- **`A-79`** (fixed): `TEMPLATE_ID` refused whitespace the specification
  never asked it to. 8 files.
- **`A-02`** (classified, open): `D3.13a`'s own supporting claim was false;
  the refusal and its other ground stand pending an actual design
  decision. 3 files.

### What `validate()` found, in files that do parse

`Is_archetype_root` fired on 11 of the 15 files that parse with
violations — the single largest invariant by file count, worth a closer
look before treating it as settled; not yet examined for whether it is
these fixtures' own construction (several are clearly hand-built for a
different Java-side unit test's purposes, not authored against a
published archetype) or a rule this crate checks more strictly than
`EHRbase`'s own writer does. `Periodic_validity`, `Archetype_id_rm_entity_
matches`, `Time_after_origin`, and `Value_is_rubric` each fired on one or
two files. None examined yet.

### Candidates — recorded, not yet findings

1. **`Is_archetype_root`, 11 files.** Needs reading each fixture's own
   `archetype_details` before concluding either way.
2. **The other four invariants**, one or two files each. Not yet
   examined.
3. **`A-02`'s own design question**: is `20240517` genuinely ambiguous
   with a bare year `2024` the way `D3.13a` states, given that a basic
   date always carries eight digits and a year-only value never does? A
   requirement change, not a quick parser fix, and not this file's call.

## Trademarks

openEHR® is the registered trademark of the openEHR Foundation and is used
with the permission of openEHR International. Use of the trademark does not
constitute endorsement of this product by openEHR International or openEHR
Foundation.
