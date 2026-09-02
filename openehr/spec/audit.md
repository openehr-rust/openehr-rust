# Audit findings

**Non-normative.** This is the register of known gaps between what
[`spec/`](index.md) requires, what the documentation claims, and what the code
does. Every finding carries evidence a reader can check.

A finding stays here until it is fixed or the specification is amended to match
reality. Deleting one because it is inconvenient, or because the text that
stated it was rewritten, is the failure mode this file exists to prevent
(`C0.9`).

**Audit date:** 2026-07-31; **remediation pass:** 2026-08-01. **Scope:** the whole crate at first release
(0.1.0). **Method:** every requirement in `spec/*.md` read against the code that
implements it and the test that exercises it; the specification sources
re-fetched from `specifications.openehr.org` and `openEHR/specifications-TERM`;
`cargo clippy --all-targets` and `cargo test` run clean.

**62 findings, 62 in the table below: 7 High, 33 Medium, 22 Low. 56 fixed or
classified, 6 open.** These counts are checked against the table by CI
(`claims` / *the audit summary counts itself correctly*) — if this paragraph
and the table disagree, the table is correct (`W0.3`: never claim more than is
verified), and the check should have failed. Every one of the 6 open findings
is open by a stated reason rather than by omission — **A-40** is the newest and
the largest, an entire specification section in force with no code behind it —
and the rest: **A-02**, **A-08** and
**A-19** are declared departures the crate does not intend to close; **A-05**, **A-10**,
**A-30** are recorded limitations or residuals with the reasoning
for leaving them written beside them — **A-38** is a defect in `serde_json`
that this repository could not repair — a conclusion that turned out to be
wrong, and is the reason that entry is worth reading; **A-27** was closed by making the
decision it recorded as unmade. **A-09**
(no property-based testing) is closed: `tests/properties.rs` covers the laws, and
`openehr-fuzz` now drives five targets over the parsers — ISO 8601, the
identifier grammars, AQL, paths, and canonical-JSON deserialization — run in CI
on every push rather than merely committed. The `canonical_json` target is
seeded with a real composition and reaches roughly 4,800 covered edges, against
650 for `iso8601`; without the seed it would have exercised the JSON lexer and
nothing else.

Deliberately **not** treated as findings: deep nesting on deserialization, which
`S1.15` states as a documented limitation rather than a defect, and for which
`serde_json`'s own recursion limit bounds the input. **A-10** was opened by the work that closed
A-06, which is the usual pattern: closing a finding is what surfaces the next
one.

**A-13, A-14, and A-15 were all found in a single afternoon by running the
generated DDL against PostgreSQL 18 and MySQL 8.4** — the two engines that could
be provisioned locally. Both crates passed every golden DDL test while emitting
a script the engine rejected. That is the measured cost of the distance between
*Dialect* and *Schema* in `openehr-store/spec/conformance.md`: two of two crates
tested were wrong, and the tests that existed could not have said so, because a
golden test compares an emitter against its author's belief rather than against
an engine.

**Four of the seven were closed by reading a primary source** — one per
Reference Model package — and **all four times the source contradicted what had
been implemented** from the rendered specification pages. The rendered pages
omit every class-definition table: the sources `include::` them from a UML
export generated at build time, and the model itself is a binary `.mdzip`. So
anything written from the prose alone is a guess.

Across the four: three rules backwards, sixteen invariants missing, five
reported under names openEHR does not use, two undeclared narrowings, and one
place where **openEHR contradicts itself** (`V8.7a`). Every one of the four also
caught at least one of this crate's own test fixtures, which had been wrong
since they were written.

That is the standing lesson of this register: for an invariant, go to the
published PDF and the amendment record. Never the rendered page.

**A-01 was the one worth chasing.** Two of the three rules it flagged were
genuinely wrong, both rejecting conformant data, and reading the primary source
found four further invariants missing entirely — including
`Normal_range_and_status_consistency`, which is the one that lets a clinician
see a normal flag beside an abnormal result. None of the nine is a false claim
in the documentation, which is the class this register most exists to catch.

## Severity

| | Meaning |
| --- | --- |
| **High** | Makes a false claim about clinical software, or defeats a control. |
| **Medium** | A real gap with a bounded blast radius, or one that will grow. |
| **Low** | Cosmetic, or already stated where a reader will meet it. |

## Summary

| Id | Severity | Finding | Status |
| --- | --- | --- | --- |
| A-01 | Medium | Three quantity rules came from secondary sources; two were wrong | **fixed** |
| A-02 | Low | ISO 8601 basic format refused | open, by decision |
| A-03 | Medium | Reading a composition cost more stack than a test thread has | **fixed**, residual documented |
| A-04 | Low | Reference ranges were not navigable by path | **fixed** |
| A-05 | Low | AQL re-rendering differs cosmetically from its input | open |
| A-06 | Medium | 54 requirements implemented with no test | **fixed** |
| A-07 | Low | `COMPOSITION` persistent/context invariant not implemented | **fixed** — the uncertainty was a misreading |
| A-08 | Low | The `property` and `extract_*` terminology groups are not carried | open, by decision |
| A-09 | Low | No property-based or fuzz testing; mutation verification is not systematic | **fixed** — property tests added (`A-17`); `openehr-fuzz` drives five targets over the parsers, run in CI |
| A-10 | Low | `X11.24` fail-closed has no provokable error path | open |
| A-18 | Medium | `ORIGINAL_VERSION` cannot carry a `signature`; openEHR puts it on `VERSION` | **fixed** — field, builder, accessor, round-trip test |
| A-19 | Medium | `COMPOSITION.Territory_valid` and `Language_valid` are neither enforced nor declared | **classified** — declared as `S1.18`, and in-crate enforcement is the wrong answer rather than missing work: ISO 3166-1 and ISO 639-1 change, and a table compiled into a library rejects conformant data from the day one does. What was genuinely open — whether a caller can do the check `S1.18` tells them to — is now pinned by a test |
| A-20 | Medium | `L10.4` requires openEHR's own invariant names; citations diverged and nothing checked | **fixed** — 15 renamed; the 13 crate *additions* declared under `L10.9`; both checked every build |
| A-21 | Medium | `EHR.Ehr_status_valid` and `Ehr_access_valid` unenforced; the shared fixture violated both | **fixed** — `Ehr::new` checks, fixture corrected, round-trip assertion strengthened |
| A-22 | Medium | `DV_MULTIMEDIA`: `Integrity_check_validity` reported for the wrong rule; three checkable invariants unenforced despite the crate shipping their code sets | **fixed** — four checks added, the addition renamed and declared |
| A-23 | High | A `VERSION`'s invariants were checked by `OriginalVersion::new` and by nothing else — deserialization bypassed them and no `Validate` impl existed, so the path an HTTP service takes was unchecked | **fixed** — `Validate for Version`, the store validates the envelope, and `Preceding_version_uid_validity` enforced for the first time |
| A-24 | Medium | The 75 unnamed RM invariants were undifferentiated, so a real gap was indistinguishable from a class deliberately not modelled | **classified** — 29 out of scope, 17 vacuous, 25 unenforced, 1 enforced-but-misnamed; the build now fails on an unclassified one. **Unenforced now 21**: the four `EHR` reference rules and the interval rename are fixed; two sub-findings open |
| A-25 | High | The invariant-coverage count matched invariant **names** without their class, matched names in comments, and saw only two of the ways a rule is reported | **fixed** — matches the cited `(class, name)` pair through a real scanner; **83 named became 69**, and 24 invariants nobody had examined were revealed |
| A-26 | Low | The conformance matrix boasted mechanical completeness — "291 ids, 291 covered, none missing" — and six requirements added afterwards had no row | **fixed** — 297 of 297, and CI re-derives the count on every push |
| A-27 | Medium | AQL could not express a **negative numeric literal** — `WHERE o/value/magnitude > -2.5` was refused at the lexer, and `Parser::integer`'s `v >= 0` guard was therefore unreachable | **fixed** — the sign is resolved by the parser at operand position, never by the number scanner, so an archetype id cannot be affected (`Q12.9b`, `Q12.9d`) |
| A-28 | High | The query surface — `aql.rs` and `path.rs` — had **115 surviving mutants of 435**, the largest untested area in the crate; fifty of them were navigation-table arms whose loss turns a resolvable path into an empty result | **fixed** for the navigation table and the AQL parser; the count is in the `A-09` table |
| A-29 | Medium | The four temporal data types carry `DV_ORDERED` attributes and implement `DvOrdered`, but `path.rs` reached them on five classes only — a normal range on a `DV_DATE` was unreachable by path, against `Q12.7a` | **fixed** — nine classes; found by a test written to kill a mutant |
| A-30 | Low | AQL has no node-id predicate shorthand: `c[at0001]` is refused, not read as `archetype_node_id = 'at0001'` | open, pinned by a test |
| A-31 | Medium | The invariant scanner paired **any** uppercase literal with a following identifier-shaped one, so eleven pairs that were never a citation — `ROLE._type`, `EHR_STATUS._type`, `ELEMENT.archetype_node_id` and eight more — stood in the committed divergence register | **fixed** — the two must be one call's arguments; 74 named is unchanged, so no real citation was lost |
| A-32 | Medium | `Eq` on the ISO 8601 types was lexical while `PartialOrd` compared instants, so `11:00:00Z` and `12:00:00+01:00` ordered `Equal` and were not `==` — contrary to the standard library's requirement that the two agree | **fixed** — `PartialOrd`/`Ord` removed from `Date`, `Time`, `DateTime`, `Duration`; semantic ordering is now the plain method `semantic_cmp`, so no trait contract applies |
| A-33 | Medium | The Gregorian leap rule was implemented **twice** — `base::iso8601` and `rm::data_structures` — byte-identical but for the fallback arm, and the second copy had never been run by any test | **fixed** — one implementation, `pub(crate)`; the interval-event arithmetic that used it is now tested against hand-computed dates |
| A-34 | Medium | `DV_ENCAPSULATED`'s `charset` and `language` were preserved across a round trip and **unreadable** — `EncapsulatedAttrs` is exported but no type returned one, so a caller holding a `DV_MULTIMEDIA` or `DV_PARSABLE` could not ask what it declared | **fixed** — an `encapsulated()` accessor on both; found because the two accessors had no reachable caller to test |
| A-36 | Medium | `DV_URI` and `DV_EHR_URI` enforced their invariants in the constructor only. A `DV_URI` deserialized from `{"value":"nocolon"}` **panicked** in `scheme()`, whose rustdoc said "# Panics — Never"; a `DV_EHR_URI` deserialized from `{"value":"https://example.org/x"}` reported scheme `https`, which is what the type exists to make impossible. `Validate for DataValue` reached both through a `_ => {}` arm, and `LINK.target` was validated nowhere | **fixed** — `scheme()`/`rest()` are total (`D3.30a`), validation checks both types and every `LOCATABLE`'s links, and `openehr-fuzz` has a `uri` target that reproduces the panic in seconds |
| A-37 | High | The `aql` fuzz target had been **failing in CI since 2026-08-04** and nobody had triaged it. Two defects: the lexer copied a string literal one UTF-8 **byte** at a time, so `'Müller'` lexed to `'MÃ¼ller'` and a `WHERE` against it matched nobody; and the `FROM` renderer omitted the parentheses its own grammar needs, so `Or(Contains(a,b), c)` rendered as text that re-parsed to `Contains(a, Or(b,c))` — a query over different records | **fixed** — slices not bytes, escaped rendering, precedence-correct parentheses (`Q12.15`, `Q12.15a`, `Q12.15b`) |
| A-38 | Medium | `serde_json`'s float parser was not the inverse of its own serializer, so a `DV_QUANTITY` magnitude **drifted** across repeated canonical round trips. Filed as open and upstream ([serde-rs/json#1336](https://github.com/serde-rs/json/issues/1336)); it was neither | **fixed** — `serde_json`'s `float_roundtrip` feature already existed and this repository had not enabled it (`spec/serde-json-float-roundtrip-arbitrary-precision/` `SJ1`) |
| A-39 | Medium | Two matches in `DataValue` whose arms could be deleted in silence — `semantic_cmp` (6 of 9) and `is_strictly_comparable_to` (all of them, plus the whole function replaceable with `false`) — and `trim_float`, whose guarded branch produced the same string as its `else` for **every finite `f64`**. Found by the retrospective mutation pass `W-18` required, not by anything failing | **fixed** — one table-driven test per arm with a row-count assertion, and the dead branch deleted rather than tested |
| A-35 | Medium | Ten types — every `DV_ORDERED` descendant and `DataValue` — implemented `PartialOrd` while deriving `PartialEq` over all their fields, so `a != b` while `a <= b` and `a >= b` were both true. Recorded as the lexical-vs-semantic shape of `A-32` and scoped to five types; the mechanism is `OrderedAttrs`, which every `DV_ORDERED` carries, and it reached five more | **fixed** — no `DV_ORDERED` implements `PartialOrd`; comparison is `DvOrdered::semantic_cmp`, and `INTERVAL<T>` is bounded on a new `SemanticOrd` (`D3.18b`, `D3.18c`) |
| A-11 | Medium | The Common Information Model was implemented from prose | **fixed** |
| A-12 | Medium | The Data Structures model was implemented from prose | **fixed** |
| A-13 | Medium | One `IF NOT EXISTS` flag covered two statements MySQL treats differently | **fixed**, verified on MySQL 8.4 |
| A-14 | Medium | SQL Server and Oracle documented an idempotence guard that was never emitted | **fixed**, not verified on either engine |
| A-15 | High | Append-only was enforced in the schema on two engines of five | **fixed**, verified on PostgreSQL 18 and MySQL 8.4 |
| A-16 | High | `Time`/`DateTime` panicked on a multi-byte character in the offset | **fixed**, regression pinned |
| A-17 | Medium | The first property tests passed vacuously | **fixed**, mutation-verified |
| A-40 | Medium | The Archetype Model is specified and mostly not implemented: §15 and `S1.21` are in force, 18 of 32 requirements with no code | open — object model, in-memory-archetype validation, and repository resolution of a filled slot built 2026-08-26/30; no parser, flattening, or template expansion |
| A-41 | Low | The conformance matrix's own totals went stale a second time — 291 claimed, 300 in one sentence, 311 in the rows | **fixed** — re-derived mechanically to 344 on 2026-08-26 |
| A-42 | Medium | Three invariants checked at construction and nowhere else: `AUDIT_DETAILS.System_id_valid`/`Change_type_valid` on a `VERSION`'s own `commit_audit`, `ISM_TRANSITION.Transition_valid`, `INTERVAL_EVENT.Math_function_validity` — `A-23`'s exact shape, recurring | **fixed** — a shared `check_audit_details` helper, and one group-membership check each beside the sibling check already there |
| A-43 | Low | `base::Interval<T>` had only the BASE foundation type's element-membership function (`has`, named `contains` here); `intersects` and interval-vs-interval `contains` did not exist | **fixed** — `contains_interval` and `intersects`, both checked exactly at shared open/closed boundaries rather than approximated |
| A-44 | Low | `C_ATTRIBUTE.container` checked children's occurrences lower-bound sum against the cardinality but not any child's own occurrences upper bound (`VACMCU`); `C_ATTRIBUTE.single` checked nothing about its children's occurrences at all (`VACSO`) | **fixed** — both added; `single`'s check required splitting out a shared `new_raw` constructor so `container`, built on `single`, would not inherit a rule that belongs only to single-valued attributes |
| A-45 | Medium | `C_DATE`/`C_TIME`/`C_DATE_TIME`/`C_DURATION` had no `CPrimitive` variant at all — every node they governed was `Unchecked`, which on most real archetypes (nearly all constrain at least one date or time field) meant `is_conformant()` was `false` far more often than the disclosure's wording suggested | **fixed** — `SemanticOrd` implemented for the four `base` temporal types (previously blocked on nothing implementing it, not a choice to skip it), then the four `CPrimitive` variants, each a list of ranges matching AOM2's own shape |
| A-46 | Low | `C_PRIMITIVE_OBJECT` could not carry a `node_id` at all, though `CObject::node_id`'s dispatcher already read the field — it just stayed `None` forever, since nothing could set it | **fixed** — `with_node_id`/`node_id()` added, plus a `PRIMITIVE_NODE_ID` constant for AOM2's own inline-form sentinel, which is a literal string rather than coded syntax |
| A-47 | Low | `Terminology_code`/`Terminology_term`, the BASE foundation types `AUTHORED_RESOURCE.original_language`, `RESOURCE_DESCRIPTION_ITEM.language`, and `TRANSLATION_DETAILS.language` are typed as, did not exist in this crate at all | **fixed** — both added to `openehr::base`; a standalone prerequisite, not a claim that any of the three classes that use them is now modelled |
| A-48 | Low | `C_PRIMITIVE_OBJECT.assumed_value` had no field at all — a default value could not be attached to a primitive constraint under any representation | **fixed** — `PrimitiveValue` and `with_assumed_value`/`assumed_value()` added; residual (`Inv_valid_assumed_value` unchecked) closed by **A-56** |
| A-49 | Medium | `parse_adl14_header`/`parse_adl2_header` used `ArchetypeId` for the header's own identifier — narrower than the grammar both cite in their own error messages, `ARCHETYPE_HRID`, which allows a namespace prefix and a prerelease version suffix neither reader accepted | **fixed**, residual documented — `ArchetypeHrid` added and both readers corrected to use it for the archetype's own identifier; the `specialize` line's identifier is unchanged and remains narrower than its own grammar allows |
| A-50 | Medium | `C_COMPLEX_OBJECT` had no `attribute_tuples` field — `C_ATTRIBUTE_TUPLE`/`C_PRIMITIVE_TUPLE` did not exist under any name, so a `{units, magnitude}` or `{value, symbol}` co-varying constraint (AOM2's replacement for ADL 1.4's `C_DV_QUANTITY`/`C_DV_ORDINAL`) could not be represented at all, not even as `CPrimitive::Unsupported` | **fixed** — `CAttributeTuple`/`CPrimitiveTuple` added, wired onto `CComplexObject` via a builder; the tree walk reports a node governed by one as `Unchecked` rather than silently passing it; residual (tuple constraints carried but never evaluated against instance data) closed by **A-58** |
| A-51 | Medium | `CPrimitive::TerminologyCode` had no `constraint_status` field, so an `extensible`/`preferred`/`example` (non-`Required`) terminology constraint could not be distinguished from a required one — `am::validate` reported a violation for conformant data whenever the actual code did not match the list or value set, which AOM2 states plainly is not a violation for a soft constraint | **fixed**; residual (`code_list` had no AOM2 counterpart) closed by **A-55** |
| A-52 | Low | `ARCHETYPE.rm_overlay` had no counterpart at all — visibility and aliasing statements for RM attributes outside the constrained structure could not be attached to an `Archetype`, silently vanishing on JSON read the same way `A-50`/`A-46` found elsewhere | **fixed** — `RmOverlay`/`RmAttributeVisibility`/`VisibilityType` added in a new `am::rm_overlay` module, attached via `Archetype::with_rm_overlay`; `Inv_alias_validity` checked at construction |
| A-53 | Medium | `C_COMPLEX_OBJECT_PROXY` had no counterpart under any name — an archetype using a proxy node to reference a constraint defined elsewhere in the same archetype, rather than repeating it, could not be represented at all, the same shape of gap `A-50` found for tuple constraints | **fixed**; residual (`use_target_occurrences()` unmodelled) closed by **A-54** |
| A-54 | Low | **BREAKING.** `CComplexObjectProxy` could not represent AOM2's `use_target_occurrences()` — `A-53`'s own residual — because `CObject::occurrences()` returned `&MultiplicityInterval`, a shape every other `C_OBJECT` variant already committed to as published API | **fixed** — `occurrences()` widened to `Option<&MultiplicityInterval>`; the four other variants are unaffected (always `Some`), and `CAttribute::single`/`container`'s own construction-time checks treat a deferred child per AOM2's own stated default (lower bound `0`, upper bound unchecked) rather than guessing |
| A-55 | Low | **BREAKING.** `CPrimitive::TerminologyCode::code_list: Vec<String>` — `A-51`'s own residual — had no counterpart in AOM2's actual single-valued `constraint: String` | **fixed** — `code_list` removed; multiple alternative codes are now expressed as sibling `C_OBJECT`s, matching every other node kind's own alternative-matching shape; `constraint`'s `at`-code/`ac`-code kind is now distinguished by AOM2's own `"ac"` leader convention rather than by which of two fields it was written into |
| A-56 | Low | `C_PRIMITIVE_OBJECT.assumed_value` conforming to its own `constraint` (`Inv_valid_assumed_value`) — `A-48`'s own residual — was never checked; a kind-mismatched or out-of-range assumed value was accepted silently all the way to a caller who never suspected one | **fixed** — checked in `Archetype::check`, not at `CPrimitiveObject::with_assumed_value` (which builds a node in isolation, before the terminology a `C_TERMINOLOGY_CODE` `ac`-code needs is in scope); `C_UNSUPPORTED` is excluded rather than guessed at |
| A-57 | High | `adl_lexer::Lexer::skip_parenthesised` put a side-effecting `self.next()` call inside `debug_assert!`, whose argument is not evaluated at all in a release build — every ADL 2/1.4 archetype header with a `meta_data` clause failed to parse in any release-profile build, silently, since the header readers were first added | **fixed** — the token consumed unconditionally into a binding, `debug_assert!` checking only the value; caught by `cargo bench --benches -- --test` (a release-profile run), invisible to `cargo test` (always debug profile) and to CI's own `test` job for the same reason |
| A-58 | Low | `walk_complex` visited a `C_ATTRIBUTE_TUPLE` and reported it `Unchecked` unconditionally — `A-50`'s own residual — never resolving the instance's actual values and comparing them against a row, so a co-varying `{units, magnitude}`/`{value, symbol}` constraint was never actually enforced no matter what the data said | **fixed** — `walk_attribute_tuple` resolves each co-varying attribute to its one instance value, evaluates every row's every column by delegating to `walk_primitive` itself, and combines the three-valued result (`Conforms`/`Violates`/`Unchecked`) across a row by AND and across the tuple by OR; a column that cannot be resolved to exactly one value, or a `tuples` list with no rows at all, stays `Unchecked` rather than being guessed at |
| A-59 | Low | `ArchetypeSlot` had no `is_closed` field — AOM2's `ARCHETYPE_SLOT.is_closed` and its `any_allowed()` function could not be represented at all, so an archetype that closes a slot to further filling could not say so under any representation | **fixed** — `is_closed`/`closed()`/`is_closed()`/`any_allowed()` added, defaulting `false` per AOM2's own stated default; residual (`crate::path::Node` did not expose `ARCHETYPED.archetype_id`, so nothing could be checked) closed by **A-60** |
| A-60 | Medium | `am::validate::walk_object`'s `CObject::Slot` arm reported every `ARCHETYPE_SLOT` `Unchecked` unconditionally, including a slot closed with `is_closed()` (`A-59`) that the instance filled anyway — a defect this crate could state (`A-59` gave it somewhere to put `is_closed`) but never actually catch, because `crate::path::Node` had no way to tell whether a position was filled by another archetype at all | **fixed** — `Node::archetype_details()` added, exposing `ARCHETYPED` at an archetype root; `walk_object`'s slot handling now reports a real violation when a closed slot was filled regardless, needs no further check when an open, unrestricted slot was filled or any slot was correctly left open, and stays `Unchecked` only for the one case this crate genuinely cannot resolve — a restricted open slot's filler, since `includes`/`excludes` assertions are not parsed (`K15.10`) |
| A-61 | Low | `TermDefinition` had no `other_items` field — AOM2's `ARCHETYPE_TERM.other_items`, a hash of extra keyed items "e.g. provenance", could not be represented at all, the same silent-loss shape `A-46`/`A-48`/`A-50`/`A-52`/`A-59` each found in a different class | **fixed** — `other_items`/`with_other_item()`/`other_items()` added, defaulting to an empty map and `#[serde(default)]` on the wire; carried, not interpreted, the same position this crate already takes on `ArchetypeTerminology`'s own external bindings — no fixed list of recognised keys exists to check against |
| A-62 | Medium | `am::cadl` (`A-40`'s own "smallest real slice") refused `use_archetype`, `use_node`, and `allow_archetype` outright, though `C_ARCHETYPE_ROOT`, `C_COMPLEX_OBJECT_PROXY`, and `ArchetypeSlot` all already existed as types (`A-50`, `A-53`) — the blanket refusal overstated what stood in the way: only `allow_archetype`'s own `matches { include ... }` form genuinely needs the `K15.10` assertion grammar this parser does not lex | **fixed** — `use_archetype`/`use_node` fully implemented (`archetype_ref` reconstructed by slicing the source between token boundaries, `ADL_PATH` read as raw text to the next whitespace — two new `Lexer` primitives, since neither lexes atomically as a `Word`); `allow_archetype` implemented for its unrestricted form only, with `closed` and `matches {...}` each refused by name for a distinct, real reason (the former's own grammar carries no occurrences to build `ArchetypeSlot` from; the latter genuinely needs `K15.10`) |

---

## A-01 — Three quantity rules came from secondary sources

**Severity:** Medium. **Status:** fixed — two of the three were wrong.
**Requirements:** `D3.20`, `D3.20a`, `D3.21`, `D3.21a`, `D3.22`, `D3.23a`,
`D3.24a`, `D3.24b`.

**What was found.** The rendered RM 1.1.0 Data Types page omits its
class-definition tables — the specification sources `include::` them from a UML
export generated at build time and absent from the repository, and the `.mdzip`
model is binary. Three rules had therefore been written from the class
*descriptions* and from implementation practice. The invariants were eventually
read from the **Release 1.0.2 publication** (Rev 2.1.1, 20 Nov 2008,
§6.2.1–6.2.12), and the amendment record checked for every change since (see
§3, *Where the quantity invariants come from*).

Two of the three were wrong, both in the direction that **rejects conformant
data**:

| Rule | Was | openEHR says |
| --- | --- | --- |
| `DV_QUANTITY.precision` | `>= 0` | `Precision_valid: precision >= -1` — `-1` is the stated "unlimited decimal places" |
| `DV_PROPORTION.precision` | an integral *kind* forbids a non-zero precision | `Precision_validity: precision = 0 implies is_integral` — the other direction entirely |
| `magnitude_status` | `{=, <, >, <=, >=, ~}` | the same six, confirmed verbatim |

Reading the tables also surfaced four rules that were simply **missing**:

- `DV_AMOUNT.Accuracy_is_percent_validity` — an accuracy of `0` must not be
  flagged as a percentage; `0` means 100% accurate and "0%" reads as the
  opposite.
- `DV_AMOUNT.unknown_accuracy_value` — `-1.0` records "accuracy not measured".
  Nothing in the crate honoured it, so `-1` would have been read as an error of
  minus one.
- `DV_ORDERED.Normal_status_validity` — the abnormal flag must come from the
  openEHR code set. A renderer prints it verbatim beside a result.
- `DV_ORDERED.Normal_range_and_status_consistency` — `N` if and only if the
  value is inside its normal range. This is the one that matters clinically: it
  is exactly the case where a result arrives from one system and its flag from
  another, and a clinician sees a normal flag beside a potassium of 9.9.

**Fix.** All seven rules are now implemented and each has a test that names the
failure it guards. `DvQuantity::UNLIMITED_PRECISION` and
`DvQuantity::UNKNOWN_ACCURACY` name the two sentinels rather than leaving them
as bare numbers, and the two `DV_ORDERED` invariants are checked by
[`crate::validation`] because the second is a relationship between attributes
rather than a property of one.

**Residual.** One inference is not read from a source: `DV_SCALE`, new in
1.1.0, is taken to inherit `DV_ORDERED`'s invariants unchanged. It is a
`DV_ORDERED` descendant and the amendment record describes it as `DV_ORDINAL`
with a `Real` value, so the inference is a short one — but it is an inference,
and it is recorded rather than assumed.

**What this finding is really evidence of.** The two wrong rules were both
plausible, both written by someone reading the prose carefully, and neither
would have failed a test written by the same person from the same reading. The
only thing that found them was going back to a primary source. That is the
argument for `C0.3`.

---

## A-02 — ISO 8601 basic format is refused

**Severity:** Low. **Requirement:** `D3.13a`, which records the decision.

`20240517` and `T091500` are valid ISO 8601 and are refused. They do not appear
in openEHR canonical JSON, and accepting four bare digits would make `2024`
ambiguous between a year and a basic-format fragment.

**Consequence:** an instance converted from a system that emits basic format
fails to parse, with a clear error, at the boundary.

**To close, if it should be closed:** accept basic format only where the length
is unambiguous, and never for a bare four-digit string. Recorded here rather
than left for a reader to discover from a rejection.

---

## A-03 — Reading a composition cost more stack than a test thread has

**Severity:** Medium. **Status:** fixed; residual documented.
**Requirements:** `J9.15`, `S1.15`.

**What was found.** Deserializing even a *minimal* `COMPOSITION` overflowed the
2 MiB stack Rust gives a spawned thread, in a debug build. `cargo test` runs
every test on a spawned thread, so this was not an edge case: a caller's own
test suite would have aborted with a stack overflow the first time it read a
composition. It surfaced when the README's own fragments were made into tests
(`tests/readme.rs`) — the first document small enough that nobody would have
suspected it.

**Cause.** `#[serde(flatten)]` and internally tagged enums both make serde
generate a `visit_map` holding an `Option<T>` local per field, and flattening
composes them: an `ENTRY` carries three flattened attribute groups, each
embedding types that reach `DATA_VALUE` — a 22-variant enum. In an unoptimized
build the locals are not coalesced, so the frames multiply down the tree.

**Fix.** The heavy *optional* fields on the hot path were boxed:
`LocatableAttrs::archetype_details` and `feeder_audit`, `Element::value`,
`EVENT::state`, `HISTORY::summary`, `COMPOSITION::context`,
`EHR_STATUS::other_details`, `EVENT_CONTEXT::other_context`,
`ENTRY::provider`, `CARE_ENTRY::protocol`, and `OBSERVATION::state`. Each trades
an allocation on a field that is absent from most nodes for a stack cost paid on
every node of every document.

**Measured**, 2026-07-31 on `rustc 1.96.1`, debug profile, by binary-searching an
explicit `stack_size` per type with each probe in its own `#[inline(never)]`
function — the last detail matters, because a debug frame is sized for the union
of all branches and an earlier measurement that dispatched inside one function
reported a uniform 1 MiB for everything, including
`serde_json::from_str::<u32>`:

| Type | Before | After |
| --- | --- | --- |
| `LocatableAttrs` | 128 KiB | 24 KiB |
| `ELEMENT` | 192 KiB | 48 KiB |
| `ITEM_STRUCTURE` | 384 KiB | 96 KiB |
| `ENTRY` (`EVALUATION`) | 1024 KiB | 192 KiB |
| `COMPOSITION` (minimal) | 2048 KiB | 256 KiB |
| the ~10 KB fixture in `tests/canonical_json.rs` | > 2048 KiB | 768 KiB |

`tests/canonical_json.rs::reading_a_composition_stays_within_a_small_stack`
guards the result at a 1 MiB ceiling, so a regression fails there rather than in
a user's CI.

**What remains open.** The cost is still a function of document depth and build
profile, and it is still unbounded (`S1.15`): a caller reading **untrusted**
documents must bound depth at the edge, because unbounded nesting is a
denial-of-service vector in every recursive-descent reader. Two earlier changes
made for the same reason are also still in place and were not sufficient on
their own: two intermediate `serde_json::Value` round trips were removed from
the `Text` and `PARTY_PROXY` readers, and `ContentItem`'s tagged-plus-untagged
encoding was replaced with one flat six-way dispatch.

---

## A-04 — Reference ranges were not navigable by path

**Severity:** Low. **Status:** fixed. **Requirements:** `Q12.7a`, `Q12.7b`.

`normal_range`, `other_reference_ranges`, and `normal_status` were modelled,
serialized, and round-tripped, and the path navigator would not walk into them:
a path into an interval bound needs `lower`/`upper` steps that were not defined.
AQL addressing a reference range therefore parsed and would not have resolved.

**Why it mattered more than it looked.** "Results outside their own normal
range" is a population query clinicians actually ask, and it is precisely the
one that needs `…/value/normal_range/upper/magnitude`.

**Fix.** `Node` gained `Interval`, `ReferenceRange`, and `PlainText` variants;
the three `DV_ORDERED` attributes are resolved once for all five ordered
classes rather than per class — five chances to omit one, taken away.

---

## A-05 — AQL re-rendering differs cosmetically from its input

**Severity:** Low. **Requirement:** `Q12.15` is satisfied; this is about the
text, not the meaning.

`AqlQuery`'s `Display` writes `[ehr_id/value = $ehrUid]` where the input had
`[ehr_id/value=$ehrUid]`, and parenthesises boolean groups it did not have to.
The rendered text re-parses to an equal query, which is what `Q12.15` requires,
so this is a normalisation and not a defect — recorded because a caller
round-tripping query text for storage will see it and should not have to guess
whether it matters.

**Evidence:** run `cargo run --example 03_paths_and_aql` and compare the
`normalised:` line with `QUERY`.

---

## A-06 — 54 requirements were implemented with no test

**Severity:** Medium. **Status:** fixed. **Requirement:** `T13.1`.

The matrix marked 54 of 269 requirements `?`: the code appeared to implement
them and nothing would have failed if the implementation were removed. Most of
them were **rejections** — a constructor refusing something openEHR forbids —
which is the worst kind of untested code, because the failure mode is silent.
The check stops working, nothing errors, and invalid clinical data is accepted
and stored.

**Fix.** `tests/invariants.rs` adds 29 tests covering 41 of them, each asserting
**both** directions — the invalid case refused and the valid one not — because a
constructor that refuses everything passes the first half. The largest is
`every_validation_check_fires_on_a_document_that_breaks_it`, which drives twelve
`L10.6` checks from JSON documents rather than from constructed values, since
validation exists precisely for data that never met a constructor (`L10.1a`).

A further 13 were **reclassified `type`** rather than tested: requirements the
compiler enforces, where a runtime test could not fail. Marking them `type`
rather than `•` keeps the verified count meaning what it says.

**Mutation verification.** Five checks were disabled one at a time and the tests
watched to fail: `DV_QUANTITY.Units_valid`, `DV_PROPORTION.Valid_denominator`,
`DV_PROPORTION.Precision_validity`, the `skip_serializing_if` that keeps `null`
out of the JSON, and the document-order walk in `validation`. All five were
detected — but **two initially reported "survived" and had never applied**,
because the substitution silently failed to match. A mutation that does not
mutate is a false negative, and the technique needs the mutation itself
verified. That is now noted in `T13.2`.

**What remains.** Three requirements are still `?`, and the reasons differ: a
constant-time comparison nobody has timed (`X11.12`), an error path that cannot
be provoked (`X11.24`, now **A-10**), and the fact that mutation verification is
five checks rather than a policy (`T13.2`, part of **A-09**).

---

## A-07 — `COMPOSITION` persistent/context invariant was not implemented

**Severity:** Low. **Status:** fixed — the uncertainty was mine, not the
specification's. **Requirements:** `E6.6a`, `E6.6b`, `E6.3a`, `E6.7a`,
`E6.12a`.

**What the finding claimed.** That RM 1.0.2 and 1.1.0 disagreed about whether a
persistent composition may carry an `EVENT_CONTEXT`, so neither rule could be
enforced.

**What was actually true.** They do not disagree. Reading the 1.0.2 class table
(*EHR Information Model*, Rev 5.1.1, §5.4.1) gives the formal invariant —
`Is_persistent_validity: is_persistent implies context = Void` — and the prose
two sections earlier says the same thing in words: *"Persistent Compositions do
not have an Event context."* The EHR amendment record shows nothing after 5.1.1
touching it. The 1.1.0 narrative sentence I had read as a relaxation
("optional for persistent composition updates") is about **when an event
context is used at all**, not about persistent compositions specifically. The
misreading was mine.

**What reading the tables also found.** Four more invariants missing, none of
them related to the original question:

| Invariant | Was |
| --- | --- |
| `COMPOSITION.Is_archetype_root` | not enforced |
| `EHR_STATUS.Is_archetype_root` | not enforced |
| `EVENT_CONTEXT.location_valid` | an empty location was accepted |
| `COMPOSITION.Language_valid`, `Territory_valid` | not enforced, and not declared |

The first two are now checked by validation; the third is refused by the
constructor and reported by validation; the fourth is a **declared**
non-enforcement (`E6.7a`) — the code sets are external and out of scope
(`S1.10`), which is a legitimate position but only once it is written down
(`C0.12`).

**Evidence the checks work.** Adding `Is_archetype_root` immediately failed five
of this crate's own fixtures, in `validation`, `redact`, `guarantees`,
`canonical_json`, and the crate-level doctest — every one of them a
`COMPOSITION` built without `archetype_details`. They were wrong and had been
wrong since they were written.

---

## A-08 — The `property` and `extract_*` terminology groups are not carried

**Severity:** Low.

`src/terminology.rs` carries sixteen of the twenty groups in
`openehr_terminology.xml`. Absent are `property` (about seventy physical
property codes, used by archetypes rather than by the Reference Model) and the
three `extract_*` groups, which belong to the EHR Extract model that is out of
scope (`S1.6`).

**Consequence:** a caller validating `DV_QUANTITY` against the openEHR property
vocabulary has to supply it. Nothing in the Reference Model refers to these
codes, so nothing in this crate reports them as unknown.

**To close, if wanted:** transcribe the `property` group. The `extract_*` groups
should stay out while `S1.6` stands — shipping their codes would imply support
for a model this crate cannot build.

---

## A-09 — No property-based or fuzz testing

**Severity:** Low.

Every test is example-based. The parsers most likely to reward fuzzing are the
ISO 8601 reader, the path parser, and the AQL lexer — all three take
attacker-influenced text and all three are hand-written.

**What exists instead:** the round-trip property is asserted over one broad
fixture (`T13.4`), and each parser has an explicit rejection test enumerating
near-misses.

**To close:** add `proptest` round-trip properties for the ISO 8601 types and
`cargo-fuzz` targets for the three parsers. Neither is in the dependency tree
today, and adding either is a supply-chain decision rather than a code change.

---


**Amended 2026-08-03 — mutation testing, measured.** `T13.2` has said since it
was written that mutation verification is "not systematic": four checks in
`tests/invariants.rs` and one in `validation` had been mutated by hand, and the
rest of the suite had not. This session added roughly fifteen more by hand, one
per change, which is evidence and still not systematic.

`cargo-mutants` was run over `security/audit_chain.rs` — the module carrying the
tamper-evidence chain, and the one where a weak test is worth the most.

| Run | Missed | Caught | Unviable |
| --- | --- | --- | --- |
| Before | **40** | 27 | 13 |
| After | **1** | 66 | 13 |

**What it found, which reading had not.** The largest cluster of survivors was
arithmetic inside the hex codecs — every `*`, `/`, `%` and `+` in
`hex_bytes`/`hex_vec` could be changed without a test noticing, because
**nothing in the crate had ever put a `Chain` through serde**. A digest or a tag
is what a chain *is*; a codec that drops a nibble drops the evidence. Four tests
closed it: a keyed chain through JSON, malformed hex refused, key ids selecting
keys, and a digest displaying as hex.

**What it found about coverage itself.** `Chain::from_stored` and
`Chain::resume_from` both survived being replaced with `Default::default()`,
because their only callers are in `openehr-store` and `cargo mutants` runs the
tests of the crate it mutates. A cross-crate caller is not coverage of this
crate — the `openehr-sqlite` tamper tests exercise both paths and could not have
caught a regression here.

**The one survivor.** `Debug for Mac` replaced with `Ok(())` prints nothing,
which is *safer* than what it does now, and pinning exact `Debug` output would
freeze formatting for no benefit. Recorded rather than chased.

**The blind spot, applied.** The cross-crate caveat above was then used as a
lead rather than a footnote, on the most safety-critical code written this
session: `openehr_store::integrity`, the detector `M3.16d` requires. Its tests
all live in `openehr-sqlite/tests/tamper.rs`.

| Run | Missed | Caught |
| --- | --- | --- |
| Before | **15 of 15 viable** | 0 |
| After | 0 | **15** |

Nothing in `openehr-store` caught anything. `is_breach` could return `true` for
every verdict, `is_intact` either constant, the content-digest comparison could
be inverted, and every match arm could be deleted, with that crate's suite
green. The engine tests would have caught each — in another crate's job, after
this one reported success.

A conformance suite shared by engines is the right home for *engine* behaviour.
This file is pure logic and needs no engine, and it now has seven unit tests
beside it.

**It also found dead code.** `verify_versions` began with an early return for an
empty slice, which made the `ChainStatus::Empty` arm below unreachable —
deleting that arm changed nothing, which is how the mutant survived. Removed:
an empty slice hashes nothing, builds an empty chain, and `verify` reports
`Empty`. One path instead of two saying the same thing.

**Four modules, and the pattern holds.**

| Module | Missed before | After | What the survivors were |
| --- | --- | --- | --- |
| `openehr/security/audit_chain.rs` | 40 of 67 | 1 | nothing had put a `Chain` through serde |
| `openehr-store/integrity.rs` | **15 of 15** | 0 | every test was in `openehr-sqlite` |
| `openehr-store/record.rs` | 4 of 6 | 0 | `M3.34` and half of `D-07` asserted on one side only |
| `openehr-store/dialect.rs` | 25 of 27 | 5 | the shared generator is only run by the six engine crates |
| `openehr-loco/auth.rs` | 0 of 6 | 0 | already covered; the module was written test-first with mutation checks |
| `openehr-loco/controllers/` | 6 of 36 | 0 | an endpoint with no test at all, and the status mapping |
| `openehr/validation.rs` | 25 of 115 | 9 | three whole `visit` bodies removable, two of them added the day before |
| `openehr-loco/access.rs` | 1 of 4 | 0 | a public accessor with no caller |
| `openehr-sqlite/store.rs` | 9 of 28 | 1 | `create_contribution` could write nothing |
| `openehr/security/redact.rs` | 8 of 33 | 0 | two of three redaction rule kinds had no test |
| `openehr/security/access.rs` | 6 of 20 | 1 | the `EHR_ACCESS` accessors |
| `openehr/security/canonical.rs` | 1 of 13 | 0 | nothing canonicalised an **array** |
| `openehr/aql.rs` + `openehr/path.rs` | **115 of 435** | 4, all equivalent | the query surface: fifty navigation-table arms, the whole path parser |
| `openehr/base/iso8601.rs` + `object_id.rs` | **95 of 510** | 2 | `days_from_civil`, `DURATION` ordering, the offset parser, the identifier grammars |
| `openehr/rm/common.rs` + `data_types/quantity.rs` | **72 of 386** | 1, equivalent | the change-control envelope's accessors, and the clinical markers |
| `openehr/rm/ehr.rs` + `rm/data_structures.rs` | **59 of 313** | **0** | `EHR_STATUS`'s two flags, and a duplicated calendar (`A-33`) |
| `openehr/rm/data_types/{text,encapsulated,basic}.rs` + `base/interval.rs` | **53 of 228** | **3, all equivalent** | `EncapsulatedAttrs` was unreachable (`A-34`); `Interval::contains`'s strict comparison |
| `openehr/rm/demographic.rs` + `terminology.rs` + `base/{object_ref,uid}.rs` | **49 of 148** | **0** | `ObjectRef::is_local`, `Role::was_held_on` — closes `openehr`'s measurable surface |
| five engine crates: `src/lib.rs` (`postgresql`, `mysql`, `mariadb`, `mssql`, `oracle`) | 20 of 59 | **0** | `Dialect::name` and `append_only_sql` were unchecked in all five |
| `openehr-sqlite/dialect.rs` | 2 of 10 | **0** | `Dialect::name`, same as the five above |
| `openehr-loco/app.rs` + `tasks.rs` + `views.rs` | 4 of 25 | 1, structural | `App::before_run` never installed anything under test — see below |

`record.rs` is the one worth reading twice. A test called
`the_attributes_that_used_to_be_dropped_are_persisted` existed, named for
`D-07`, and asserted two of the four attributes that finding restored — the two
that do not go through `encode_if_any`. Replacing that function with `Ok(None)`
dropped the other two and the test stayed green: the defect `D-07` is about,
reachable again with one edit, under a test named after it.

And `party_name` could return `Some("xyzzy")` for every party without failing
anything, while `M3.34` — an anonymous committer stored as `NULL`, so that a
privacy decision does not become a data-quality problem someone later cleans
up — was marked **•** in the conformance matrix.

`openehr-loco` is the counter-example that makes the pattern legible. `auth.rs`
missed **nothing** on the first run — it is the one module written test-first,
with a mutation check per guarantee. The controllers missed six, and the
sharpest was `contribution::routes` returning `Default::default()`: an endpoint
added earlier this session, wired into the router, documented in the README, and
**never called by a test**. Every other test passed with it removed.

The other two were the `hex` helper rendering every chain digest as an empty
string — a response claiming a digest of `""`, which a reader compares against a
witness and finds equal — and three arms of `status_for`, where a duplicate
commit would answer `500` instead of `409` and tell a caller to retry when it
should re-read.

`validation.rs` is where this session's invariant work lives, so it is the run
that matters most, and it found three `Validate` impls whose **entire body**
could be replaced with `()` while `openehr` stayed green.

- `Version<T>` — `A-23`'s fix. Its tests are in `openehr-loco` and
  `openehr-sqlite`, so the remedy for a High finding was removable without this
  crate noticing.
- `EhrAccess` and `Party` — added the day before, **with no test at all**. Not
  cross-crate coverage; none. Reading the diff had not shown it, including by
  the person who wrote it.

It also found that `Section`, `ContentItem` and `Event` could each stop
descending: every test put its entry directly in `content` and its element
directly in a tree, so the two nesting paths a real composition uses were never
walked. And `check_ordered` ran only for `DV_QUANTITY` — a `DV_COUNT` with a
normal status outside openEHR's code set is a result a renderer shows verbatim
beside a number, and nothing checked it.

Nine survive. Four are the `Range_is_simple` variant arms, which need a
reference range whose endpoint is itself an ordered value carrying reference
ranges; the rest are boolean operators and one boundary. Recorded rather than
chased.

The security modules produced two findings worth naming, and one methodological
correction.

**Redaction had one rule kind tested of three.** Every test used
`RedactionRule::node_id`, so the arms matching by **name** and by **archetype
root** could each be inverted with the suite green. Redaction is the
PHI-withholding mechanism (`X11.24`, `X11.25`); two thirds of its vocabulary
unexercised is not a coverage statistic but a rule nobody has watched work.

**Nothing canonicalised an array.** The separator's `i > 0` could become
`i < 0` — emitting no commas, producing `[1 2 3]` — and no test noticed. A
`COMPOSITION` is arrays most of the way down, and every digest in the system is
taken over these bytes (`db:M3.16`), so a broken separator breaks the chain and
the checkpoint together.

**The correction.** A survivor in `matches_element` looked like a live
PHI-disclosure bug, so it was reproduced by hand — and the hand-applied mutation
*did* fail the suite, which suggested `cargo mutants` was reporting falsely.
It was not. The mutation was on line 262, which is the `Name` arm; the
hand-edit had changed the `NodeId` arm on the line above. The tool was right and
the reproduction was wrong. Checking that before writing it up cost ten minutes
and would have cost a false finding in this register.

Redaction also turned out to depend on shape rather than `_type`: this crate
does not tag a bare `ELEMENT`, measured rather than assumed, so `is_element`'s
structural fallback is the path every `ITEM_SINGLE` takes and was untested.

**The query surface was the largest untested area in the crate** — 115
survivors of 435, more than every other module measured so far put together
(`A-28`). Three things came out of closing it, and only the first is a test.

*Fifty were arms of the navigation table.* `Node::children` answers an
attribute a class does not have with an empty vector, deliberately, so that a
wrong attribute is `NoMatch` rather than an error. The consequence is that
losing an arm is **silent**: the path stops resolving, an AQL query returns no
rows, and an empty result set reads as "there is no such record". Two
table-driven tests now state every navigable attribute of every data value and
every structural node, which is also the first place a reader can see what the
path language actually reaches.

*Writing those fixtures found a defect the mutant only pointed at.*
`ordered_attrs_of` listed five classes. The four temporal types implement
`DvOrdered` and carry `OrderedAttrs` like any other, so a normal range on a
`DV_DATE` was unreachable by path although the model held it — against
`Q12.7a`, whose stated purpose is the query "results outside their own normal
range" (`A-29`). The mutation report said one arm was untested; the fixture is
what read the list.

*Two survivors were proofs rather than gaps.* `Parser::integer`'s `v >= 0`
could become `true` because it is unreachable: a numeric token starts only at an
ASCII digit and `-` is not in the symbol table, so **AQL here cannot express a
negative literal at all** (`A-27`, declared as `Q12.9b`). And no bracketed
predicate may be a bare node id (`A-30`, `Q12.9c`) — which is what makes those
cases evidence, because widening either `&&` in `Parser::predicate` accepts them
*as archetype ids*, and `archetype_ids()` is what an authorisation check reads
before a query runs (`Q12.13`).

**Four survivors remain, and each was checked rather than left.** All four are
equivalent mutants — no test could distinguish them — and the reason is recorded
here because an unexplained survivor and an impossible one look identical in a
report:

| Survivor | Why no test can see it |
| --- | --- |
| `aql.rs:958` `v >= 0` → `true` | unreachable: the lexer never emits a negative integer (`A-27`) |
| `path.rs:166` `\|\|` → `&&` | both `""` and `"/"` reach the same empty-segment result by the ordinary path; the early return is a shortcut, not a decision |
| `path.rs:195` `+=` → `-=` | `i` is the index of `[`, and an empty attribute name is already refused, so `open >= 1` and the character before `[` is part of an attribute name — never a quote. The scan arrives at `open + 1` with the same state |
| `path.rs:195` `+=` → `*=` | `i * 1 == i`: the scan starts at `[` instead of past it, and `[` is not a quote either |

Each was confirmed by applying it and running a probe over the parser, not by
reading. That is the same standard the rest of this register holds, and the one
time it was skipped a survivor was nearly reported as a live PHI-disclosure bug
(below).

**The parsers of untrusted input came next**, and the shape of what was
missing repeated: the tests exercised each function on inputs where most of its
arithmetic cancels.

*`days_from_civil` had every operation free* — the era division, the `y - 399`
correction for negative years, the day-of-year term. It is reachable only
through `diff_seconds`, and every existing test differenced two dates in the
same era on the same side of the epoch, which is precisely where those terms
cancel. This is the conversion behind the derived UTC column that `db:P6.14`
requires time-ranged queries to use, so a wrong day is a query that returns the
wrong encounters and says nothing. The replacement table's values come from
Python's `datetime`; a table generated by running the function under test would
confirm only that it still does what it did.

*The same for `Duration::approx_seconds`*, where a flipped sign made `P2W`
shorter than `P1W`, and for `Time::millis_local`, whose fraction padding meant
`.5` could be read as smaller than `.499`.

*And for the offset parser*, which is where `A-16` lived: nineteen mutants, the
sign among them. A flipped sign on `-05:00` is a ten-hour error in a stored
clinical timestamp.

*Five of the nine survived the first round of tests too*, and the reason is the
one worth remembering: `days_from_civil` is private and its only caller
**differences** two of its results. A difference cancels every constant, so
`+ day - 1` could become `+ day + 1` and `- 719_468` could become `+ 719_468`
— shifting every date by the same amount — and no comparison could tell. The
fix was to call the function directly from the module's own test, which is
possible and was simply not done. Testing a function only through the caller
that cancels half of it is a coverage measurement that flatters itself.

The same round found that no test reached a **negative** `y`: `0001-01-01`
gives `y = 0`, which is still the non-negative branch, so the `y - 399`
correction that keeps the era division truncating the right way was never
exercised. `datetime` cannot represent year 0; those rows come from their
year-400 counterparts less one 400-year cycle of 146,097 days, which is exact
by construction and was checked against a representable pair.

And in `Time::millis_local`, comparing `09:00:00` with `09:01:00` cannot tell
`m * 60_000` from `m + 60_000` — addition is monotonic too, so the ordering is
unchanged. Only a pair that crosses a component boundary, like `00:02:00`
against `00:00:59`, tests the *scale* of a term rather than its direction.

*A third round found the `DURATION` ordering untested altogether.* Seven
mutants lived in `partial_cmp`, including the guard that **refuses** to order
`P1M` against `P30D`. That refusal is the point of the impl — a month is 28 to
31 days, so there is no order without a calendar anchor — and answering `Equal`
because the approximations agree would sort a medication interval into the
wrong place. The sign in `approx_seconds` was free too, which makes `-P1D` and
`P1D` the same length; openEHR permits negative durations (`SPECRM-96`).

*Two survivors were refused for the wrong reason rather than not refused.* A
non-numeric `VERSION_TREE_ID` component is caught again by `parse`, and a bare
`0` again by the zero check, so both guards could be inverted and the value
stayed rejected. Refusal alone could not tell them apart; only the reason
could, and the reason is what a caller is shown. Pinning it also surfaced that
`0` reports `trunk_version is 0` rather than the branch constructor's message —
correct, and previously unstated.

**Two survivors remain, and both were checked.**
`split_offset`'s `digits.len() == 2` can be `true` with no observable
difference: the `hh.len() != 2` check below it rejects everything the widened
arm would admit. Confirmed by applying the mutation and probing eight offset
spellings, not by reading — every result was identical. The other is a serde
`Visitor::expecting`, which only shapes the text of a deserialization error;
pinning that text would freeze a message for no benefit, the same judgement
already recorded for `Debug for Mac`.

**Writing those tests found `A-32`**, which was not a coverage gap at all: `Eq`
on these types is derived and lexical while `PartialOrd` normalised to UTC, so
`11:00:00Z` and `12:00:00+01:00` ordered `Equal` and were not `==`. That
contradicted the standard library's requirement that the two agree. First
recorded as `D3.18a` and left declared rather than fixed, on the reasoning that
both halves were load-bearing — the text *is* the stored value (`db:M3.28`),
`.5` and `.50` must round-trip, and `Hash` must agree with `Eq` — and a caller
who sorted and then `dedup`ed a collection of times got both spellings of one
instant.

**Later fixed properly, once asked to be:** `PartialOrd`/`Ord` do not have to
exist for a type at all, and removing them from `Date`, `Time`, `DateTime` and
`Duration` costs nothing that mattered — nobody needs `<` on a bare ISO 8601
value to compile, only the *comparison itself* to be available. Semantic
ordering is now the inherent method `semantic_cmp`, which returns the same
`Option<Ordering>` as before under a name that does not claim to be `Ord`. The
RM-level wrapper types (`DvDate`, `DvTime`, `DvDateTime`, `DvDuration`) keep
their own `PartialOrd` impls unchanged — they now delegate to `semantic_cmp`
internally rather than to the removed trait method — so `Interval<DvDate>`,
`ReferenceRange`, and everything built on `DvOrdered` kept compiling and kept
behaving identically. Those wrapper types have the same lexical-`Eq`-versus-
semantic-order shape one layer up and were not touched; that is a sibling gap,
not this one, and is left for whoever next has reason to look at it.

**The Reference Model classes failed a third way: nothing read them back.**
Almost every survivor in `rm/common.rs` was an *accessor returning a
constant*. The constructors here are thorough — `Basic_validity`,
`Data_valid`, `System_id_valid`, `Versions_valid` are all enforced on the way
in — but a getter could answer `None`, `""` or `"xyzzy"` on the way out and the
suite stayed green.

That matters more in this module than it would elsewhere, because these are the
fields the store projects into columns and the audit chain is taken over. A
lying accessor produces a record that reads back wrong **while its digest still
verifies**, because the digest is computed from the stored bytes and not from
what an accessor says about them (`db:M3.16d`).

The ones worth naming:

| Accessor | A constant answer means |
| --- | --- |
| `AuditDetails::is_deletion` | every version looks logically deleted |
| `OriginalVersion::data` | the same, from the other side: absent data *is* how a deletion is recorded |
| `Version::is_deleted` | as above, at the envelope |
| `VersionedObject::has_version_at_time` | a record existed before it did — the query `db:P6.11` requires |
| `PartyIdentified::identifiers`, `external_ref` | a party that satisfies `Basic_validity` reads as anonymous |
| `Contribution::versions` | a change set that changed nothing, contradicting `Versions_valid` |

**`quantity.rs` was worse in kind if not in number,** because its survivors
carry clinical meaning rather than structure:

- `MagnitudeStatus` — the `<` / `>` / `~` marker. `as_str` could return one
  wrong constant for all six variants and three `parse` arms could be deleted.
  Confusing `<` with `>` inverts what a result *means* while leaving the number
  correct: a below-detection-limit reading becomes a measured one.
- `ReferenceRange::contains` — a constant `true` reports every result as within
  its normal range. This is the machinery `A-01` already found rules missing
  from; the membership test underneath was equally unwatched.
- `DvScale::is_strictly_comparable_to` — `D3.16`. Answering `true` lets a
  pain-scale 2 order against a sedation-scale 2 as though they measured the
  same thing.
- `accuracy_is_percent` — a constant makes `±5` read as `±5%`; on a magnitude
  of 200 those differ by an order of magnitude.
- `ProportionKind::from_i32` — openEHR encodes the kind as a bare integer, so a
  deleted arm silently reinterprets `1/2` between a half, fifty percent, and
  one-in-two.

One survivor remains and it is the mutant writing itself: `PartySelf::anonymous`
replaced by `Default::default()` is the same code, because that is the whole
body of the function. The last real one was subtler — the tests read
`Version::attestations` through the enum but never `OriginalVersion::attestations`
on the concrete type, which is a *different accessor* that could still answer
nothing. Testing the wrapper is not testing what it wraps.

Note the denominator: 149 of 386 mutants here were **unviable**, meaning they
did not compile. That is generic and macro-generated code, and it makes the
usable sample 237 rather than 386 — worth knowing before reading 72 as a rate.

Three of the tests written for this were wrong before the code was: a version
with no data must be in the `DELETED` lifecycle state, `links` and
`feeder_audit` are defaults on the `Locatable` trait rather than on
`LocatableAttrs`, and `FEEDER_AUDIT.original_content` must be a
`DV_ENCAPSULATED`. All three are invariants the crate enforces correctly and the
test author did not know.

**`rm/ehr.rs` and `rm/data_structures.rs` repeated the accessor pattern, and
added two findings of their own.**

*`EhrStatus::is_queryable` could answer `true` for every record.* That is not a
descriptive field: `is_queryable = false` means the record must not appear in
population queries, because a patient or organisation excluded it. An accessor
that always says `true` **discloses a record that opted out**, and nothing
downstream can detect it. `is_modifiable` was equally free in both directions —
a constant `true` admits writes to a closed record, a constant `false` refuses
every write. All four combinations are now asserted, so neither flag can be a
constant nor be answering the other's field.

*The Gregorian leap rule existed twice* — `base::iso8601` and
`rm::data_structures`, byte-identical but for the fallback arm, and the second
copy had never been run by any test. That is `W-01` one level down: a calendar
rule fixed in one of two copies is a rule that disagrees with itself. Recorded
as `A-33` and consolidated to one `pub(crate)` implementation rather than
tested twice. The arithmetic that copy existed for —
`IntervalEvent::interval_start_time`, which computes when a measurement window
opened — had all fifteen of its mutants surviving, and is now tested against
hand-computed dates including `1900-03-01` (the century exception) and
`2000-03-01`.

Three more worth naming. `Item::type_name` could return one wrong constant for
both variants, and it is what goes into `_type` in canonical JSON — so a
`CLUSTER` would deserialize as an `ELEMENT`, under a digest that still
verifies. `ItemList::named_item`'s `==` could be `!=`, returning the first
element that is *not* the one asked for; the method matches on runtime name
rather than node id precisely because a list built from a repeating archetype
node shares one node id across every item, so the test gives three elements the
same `at0001`. And `History::is_period_consistent` must distinguish `None` from
`Some(false)`: a series *declared* periodic whose samples are off the period
will be resampled or graphed wrongly by anything that trusts the declaration.

**A structural note.** Several optional attributes have **no builder at all** —
`IntervalEvent::state`, `Folder::details`, `CareEntryAttrs::guideline_id`,
`FeederAuditDetails::version_id`. The crate can read records it cannot
construct. That is legitimate for round-tripping, but it means those paths are
reachable only through deserialization, and a test that only builds objects
will never touch them. Each is now covered through JSON.

**Two of the survivors were gaps in the first round of tests, not in the code.**
The seconds term of `subtract_seconds` survived because every event time in the
new table ended `:00` — added to zero, `+` and `-` are the same. And
`Folder::details` survived because only its absent case was asserted. Asserting
`None` is half a test.

**The last four `openehr` modules turned up one accessor that did not exist at
all.** `lib:A-34`: `DV_ENCAPSULATED`'s `charset` and `language` were preserved
across a round trip and **unreadable**. `EncapsulatedAttrs` is exported and both
its accessors exist, but neither `DvMultimedia` nor `DvParsable` returned one —
a caller holding either type had no way to ask what character set or language
it declared. This was not found by reading; it was found by trying to write a
test and discovering there was nowhere to call it from. Fixed with an
`encapsulated()` accessor on both types.

`Interval::contains`'s strict-exclusive-bound comparison (`value < hi`) had
survived becoming `value > hi`, because the only existing test checked the
value *equal to* the excluded bound — which both comparisons reject identically.
A value strictly inside the range is what tells them apart, and this is the
membership test `ReferenceRange::contains` delegates to (found closing
`rm/common.rs` two rounds earlier), so a flipped comparison here silently
inverts which results read as abnormal.

The rest repeated the now-familiar pattern: `DvIdentifier`'s `id_type`,
`issuer`, `assigner` (the fields that route a national identifier to the right
authority's namespace), `TermMapping`'s three predicates (`is_broader`,
`is_equivalent`, `is_narrower` — what an ICD-10 crosswalk claims about a
mapped SNOMED CT code), and several `Display` impls that could print nothing.

Three survivors remain in `base64::{encode,decode}`, where `|` could become
`^`: both operators agree because the implementation always assembles bits
into disjoint, non-overlapping ranges, which is what the algorithm is.

**The last four `openehr` modules closed out the crate's measurable surface,
and repeated the accessor pattern once more.** `terminology.rs` — the code-set
lookups everything else depends on — came through with **zero** survivors,
consistent with being exercised by every other module's tests.

The two worth naming: `ObjectRef::is_local` could be a constant or have its
comparison inverted. This is the flag an access-control decision reads first —
a reference into this system's own identifier space is one the system can
resolve and enforce policy on; a foreign one is not. And `Role::was_held_on`
could answer a constant — the predicate behind "was this person the on-call
registrar at the time?", where a wrong answer is a wrongly attributed
signature. It joins `Capability::was_valid_on`, already tested, as the same
question asked of a role rather than a credential.

Also closed: `Party::type_name` — one wrong constant for any of the five
variants would deserialize a `PERSON` as an `ORGANISATION` under a digest that
still verifies — and `Uuid`'s `Hash`, which could be replaced with a no-op.
`Hash` has to agree with the type's hand-written case-insensitive `PartialEq`,
or a `HashSet` keyed on an `OBJECT_ID` silently gains a duplicate entry per
case spelling of what is really one identity; the test proves the point with
an actual `HashSet`.

**Two of the survivors from the first pass were gaps in the tests, not the
code, and both repeat lessons from earlier in this register.** The first
`Uuid::hash` test used a `HashSet` and could not distinguish a real hash from a
no-op: `HashSet` correctness only requires equal keys to hash equal, so a
constant hash is pathological but not wrong, and lookup still works via `Eq`.
Fixed by hashing through `DefaultHasher` directly and comparing the digests.
And four `demographic.rs` accessors — `Capability::time_validity`,
`PartyRelationship::details`, `PartyAttrs::details`, `Role::time_validity` —
had only their absent case asserted, the same "asserting `None` is half a
test" mistake recorded during the `rm/ehr.rs` round.

**Mutation testing then moved outside `openehr` for the first time, into
`openehr-store` and the five schema-level engine crates.** `openehr-store`'s
`schema.rs`, `store.rs` and `error.rs` turned out to have almost no mutable
surface: `schema.rs` is the shared table layout declared as `const` data plus
six tests asserting invariants over it, `store.rs` is a bare trait with no
method bodies, and `error.rs` is thiserror-derived enums. `conformance.rs` —
the actual logic, the suite every engine runs — was deliberately not measured
*from this crate*, because nothing in `openehr-store`'s own test target calls
it; it is a library function the engine crates consume as a dev-dependency,
and mutating it here would report "untested" for code that `openehr-sqlite`'s
own run already exercises. Recorded rather than silently skipped.

**Every one of the five schema-level dialects — `postgresql`, `mysql`,
`mariadb`, `mssql`, `oracle` — had the same two blind spots**, and finding them
identical five times is itself the finding: `Dialect::name()` is used only
inside `conformance::check_dialect`'s panic messages, never compared against
anything, so it could return `""` in every dialect and nothing would notice.
And `append_only_sql` — the SQL enforcing `V8.10`, the rule the whole
change-control model rests on — is asserted **structurally** by the existing
golden `tests/ddl.rs` (does the DDL contain this table, this index, this
quoting) but never checked for containing an actual refusal. A generator that
emitted an empty trigger body would have passed every existing test in all
five crates.

Fixing it also exposed why the five differ: PostgreSQL and Oracle emit one
trigger covering both operations; MySQL and MariaDB emit two, because
`SIGNAL` cannot name more than one triggering event; SQL Server uses `INSTEAD
OF` rather than `AFTER`, so the refusal happens before anything is written.
Oracle's `terminator()` — the SQL*Plus block marker `
/`, not `;` — was
equally unchecked and is now pinned alongside it.

`openehr-mariadb`'s `tests/ddl.rs` is more thorough than its four siblings',
because it is the crate `W-01` was found in — the copy-of-MySQL defect — and
was hardened afterward. The other four never received the same treatment even
though the same risk applies to each of them individually (an Oracle crate
that silently started emitting MySQL types has no test here that would catch
it structurally beyond the type-spelling assertions already present). Adding
the same `name`/`append_only_sql` test to all five is a step toward that parity,
not the whole of it.

**`openehr-sqlite/dialect.rs` had the identical `name()` gap** the five
schema-only crates did, confirming the pattern is about the trait method's one
call site, not about any one crate's test discipline.

**`openehr-loco`'s `App::before_run` never actually ran in a test.**
`before_run` is the fail-closed startup path the module's own doc comment
singles out — a service that started without a working verifier would serve an
entire EHR to anyone who asked, with a green health check and no symptom
(`db:PR12.16`) — and `tests/http.rs` builds its router against a hand-populated
`AppContext`, bypassing `before_run` (and the whole `Hooks` trait) entirely. A
version that did nothing at all would have failed no test in this crate.

Testing it directly needs three environment variables (`PasetoVerifier::
from_env`, `AccessLog::from_env`, `OPENEHR_SQLITE_PATH`), and setting a
process environment variable has required `unsafe` since Rust's 2024 edition —
which this crate forbids outright (`unsafe_code = "forbid"`), with no local
override, by design. So the fix was a split, not a test-only workaround:
`before_run` now does three `from_env()` calls and one `?`-propagated error
each, then hands the results to a new private `install`, which does the part
with a consequence — putting the verifier, the access log, and the store into
`ctx.shared_store`. `install` takes its three arguments directly and is fully
tested.

**One survivor remains, and it is `before_run`'s own body**, not `install`'s:
the whole function could still be replaced with `Ok(())`, skipping all three
`from_env()` calls and the call to `install`. Closing it would mean either the
forbidden env-var mutation, or spawning the compiled binary as a long-running
server subprocess and managing its lifecycle from the test — disproportionate
for three lines whose only content is `?`-propagating three independently-
tested constructors into a function that is itself tested. Recorded rather
than forced.

`store.rs` produced the one that would have mattered most in production:
`create_contribution` could return `Ok(())` **without inserting anything** and
the entire suite passed, because nothing reads a contribution back and no later
operation depends on the row. A change set that silently vanished takes the
attribution of every version in it — `db:PR12.10` keeps a contribution's audit
distinct from its versions' precisely so one act can be traced across several
changes. It also found the untested half of `db:O10.16`: an **empty** database
predating the version table is treated as fresh, and only the populated half was
covered, so the comparison could have been `>=` and a fresh install refused.

`access.rs` produced the opposite kind of result — one survivor, and the right
answer was to **delete** rather than test. `AccessLog::path` was a public
accessor with no caller, carrying a doc comment describing a use that did not
exist. A log's location is the deployment's to know.

Two runs needed `--in-place`: `openehr-sqlite` dev-depends on its five sibling
engine crates so one test can compare all six dialects (`W-01`), and
`cargo mutants` copies a crate to a temporary directory where those relative
paths do not resolve. Worth knowing before concluding a crate cannot be
measured.

`dialect.rs` stops at 5. The remainder are branch conditions needing a third and
fourth test dialect for idempotence modes the schema does not currently use, and
the two written cover what it does.

**Residual.** Four modules of many, and not in CI: 80 mutants take two minutes
for one file, and a whole crate would be hours. `T13.2` stays **?** — what
changed is that "not systematic" is a measurement with numbers rather than an
impression, the method is written down, and its sharpest use has been aiming it
at code whose tests live in another crate.

## A-11 — the Common Information Model was implemented from prose

**Severity:** Medium. **Status:** fixed. **Requirements:** `M5.13a`, `M5.18a`,
`V8.7a`, `V8.7b`, `V8.17a`, `L10.5a`.

Opened and closed on 2026-08-01 by applying the lesson of **A-01** and **A-07**
to the one large package that had not been checked against a primary source.
Reading the *Common Information Model* (Rev 2.1.1, 20 Dec 2008) found five gaps
and one genuine contradiction in openEHR itself.

**The contradiction.** `REVISION_HISTORY`'s class table says three things about
the order of `items`:

| Where | Says |
| --- | --- |
| class *Purpose* | most-recent-**first** |
| `items` *Meaning* | most-recent-**last** |
| `most_recent_version` postcondition | `items.**last**.version_id.value` |

This crate had implemented most-recent-first, from the Purpose line and from the
rendered narrative. Two of the three sources say last, and one of those two is
executable, so the crate now follows the postcondition. A caller rendering an
audit trail from the other sentence gets it backwards — and gets it backwards
silently, because both orders look plausible.

**The four missing checks**, all of the same shape: an attribute openEHR binds
to a terminology group *when it happens to be coded* —
`PARTY_RELATED.relationship` (`Relationship_valid`),
`PARTICIPATION.function` and `.mode` (`Function_valid`, `Mode_valid`), and
`ATTESTATION.reason` (`Reason_valid`). None was checked. The first matters most:
`relationship` is the attribute that says whom an entry is about, so an
unrecognised code means a finding may be attributed to the wrong person.

The check is deliberately conditional. Applying it to a code from another
terminology would reject a SNOMED-coded participation function, which is the
commonest real case — so it fires only for openEHR's own terminology, and a
test asserts both halves.

**One misattribution.** An empty `LOCATABLE.name` was reported as
`LOCATABLE.Name_valid`; openEHR's `Name_valid` is only `name /= Void`, and an
empty name breaks `DV_TEXT.Valid_value` — reported as `Value_valid` until
`A-20` corrected the crate to openEHR's own spelling. The wrong invariant name
sends a reader to the wrong class definition, which is what `L10.4` exists to
prevent — so `L10.5a` now requires the attribution to be right.

---

## A-12 — the Data Structures model was implemented from prose

**Severity:** Medium. **Status:** fixed. **Requirements:** `R4.12a`–`R4.12c`,
`R4.15a`, and invariant-name corrections across `R4.3`, `R4.8`, `R4.11`,
`R4.12`, `R4.15`.

The last of the four Reference Model packages to be checked against a primary
source (*Data Structures Information Model*, Rev 1.7.1, 5 Nov 2008). Four for
four: the source again contradicted what had been implemented.

**A missing invariant with teeth.** `HISTORY.period_consistency` —
`is_periodic implies events.for_all (e | e.offset.to_seconds.mod(period.to_seconds) = 0)`
— was not implemented at all. A history that *declares* a period its samples do
not follow is not periodic, and software that resamples or graphs it on the
strength of that declaration draws the wrong picture with nothing in the data
looking wrong.

Implementing it required `EVENT.offset` (`time.diff(parent.origin)`), which the
crate also did not have, which in turn required exact date-time differencing —
now `DateTime::diff_seconds`, using Hinnant's civil-days algorithm and
inheriting the partial semantics of `D3.14`: an offset that is not established
yields *not answerable*, never a verdict.

**Evidence it works.** The check immediately failed this crate's own kitchen-sink
fixture, which declared `period = PT8H` over events fifteen minutes apart. The
fixture had been wrong since it was written.

**Five invariants were reported under names openEHR does not use** —
`Items_valid` for `CLUSTER.Items_non_empty`, `Value_null_flavour_valid` for
`ELEMENT.Null_flavour_indicated`, and three more. `L10.4` requires openEHR's own
invariant names precisely so a reader can find the rule in the class definition;
a name the specification does not contain fails that.

**One undeclared narrowing.** `INTERVAL_EVENT.width >= 0` is this crate's rule,
not openEHR's. Now declared (`R4.15a`) with what it buys and what it costs.

---

## A-10 — `X11.24` fail-closed has no provokable error path

**Severity:** Low. **Requirement:** `X11.24`.

Redaction returns `Result` and yields nothing on error, so a caller cannot
forward the unredacted original by mistake. There is no test, because there is
no way to make it fail: every `Composition` this crate can construct
serializes, and the only error variant is a round-trip failure.

This is recorded rather than papered over with a test that constructs an
impossible value through unsafe means. The requirement is satisfied by
construction — the function has no partial-output path — and "satisfied by
construction with no test" is `?`, not `•` (`C0.8`).

**Amended 2026-08-02 — the premise is now checked, and the reason it holds was
wrong.** This finding said every constructible `Composition` serializes. True,
and not for the reason implied. Measured:

```
serde_json::to_string(&f64::NAN)           = Ok("null")
serde_json::to_value(f64::NAN)             = Ok(Null)
security::to_canonical_string(&f64::NAN)   = Ok("null")
```

**`serde_json` does not refuse a non-finite float. It writes `null`.** So a
`NaN` magnitude reaching serialization would not fail the redactor — it would
silently become an absent value, in the canonical form the content digest of
`db:M3.16` is taken over. Serialization is not a barrier; it is a place where
the value disappears quietly.

What actually holds the line is the constructors. Every `f64` entry point in the
crate — `DV_QUANTITY.magnitude`, `DV_SCALE.value`, `DV_AMOUNT.accuracy`, and
both parts of `DV_PROPORTION` — refuses `NaN` and both infinities, and
`guarantees::no_document_this_crate_can_build_carries_a_non_finite_float`
asserts all five against all three values, plus the `null` behaviour that makes
them load-bearing.

That changes what this finding is about. It is not "a `Result` nobody can
provoke"; it is "five constructors standing between a document and silent data
loss, with nothing downstream to catch a miss". The test fails and names the
constructor if one is ever relaxed.

**To close:** unchanged. `X11.24` stays `?` — the redactor's error path is still
unprovokable, and making the redactor generic purely to inject a failing type
would be a test of the test harness. What has changed is that the *premise* is
no longer taken on trust.

---

## A-13 — one flag covered two statements that differ per engine

**Severity:** Medium. **Requirement:** `C0.8`. **Status: fixed.**

`Dialect::supports_if_not_exists()` was a single boolean governing both
`CREATE TABLE` and `CREATE INDEX`. MySQL accepts `CREATE TABLE IF NOT EXISTS`
and **rejects** `CREATE INDEX IF NOT EXISTS`, so `openehr-mysql` emitted a
script that created all five tables and then failed at the first index with
`ERROR 1064`. An operator running `install()` would be left with a schema that
had every table, no index, and no error until the first slow query.

Every golden DDL test passed throughout. They asserted the emitter's output
against an expectation written by the same author, from the same wrong belief.

**Fixed** by replacing the flag with `Idempotence` per object kind
(`IfNotExists` / `Guard` / `Inline`). MySQL declares `Inline` and now carries
its indexes inside `CREATE TABLE`, where they inherit the table's own
idempotence. Verified against MySQL 8.4: three consecutive runs, all clean, all
seven indexes present.

**Found by** executing the DDL against the engine it names, which is exactly the
step the Dialect level says it does not perform.

---

## A-14 — a guard that was documented but never emitted

**Severity:** Medium. **Requirement:** `C0.8`. **Status: fixed.**

The `Dialect` trait's own documentation read: "Oracle and SQL Server do not
[support `IF NOT EXISTS`], and both need a guard around the statement instead."
No guard existed. Both dialects emitted bare `CREATE TABLE` and `CREATE INDEX`,
so re-running `install()` on either engine fails outright.

This is the repository's signature failure mode — prose describing a mechanism
that was never built — and it was reintroduced here in the one file that
warns about it.

**Fixed** by giving `Dialect::guard` a real contract: SQL Server wraps in
`IF NOT EXISTS (SELECT 1 FROM sys.objects …) EXEC(…)`, Oracle in a PL/SQL block
that swallows ORA-00955 and re-raises every other `SQLCODE`.
`conformance::check_dialect` now fails any dialect that declares `Guard` and
inherits the no-op default, so the gap cannot silently reopen.

**Not verified against either engine.** SQL Server 2022 segfaults under qemu on
arm64 and the Oracle images require registry authentication. The fix is
reasoned and unit-tested, not observed. Both crates therefore stay at
**Dialect**.

---

## A-15 — append-only was enforced on two engines of five

**Severity:** High. **Requirement:** `V8.10`, `X11.9`. **Status: fixed.**

`Dialect::append_only_sql` returns empty by default, and `openehr-mysql`,
`openehr-mssql`, and `openehr-oracle` all inherited that default. Only
PostgreSQL and SQLite refused mutation in the schema.

The severity is not the missing SQL; it is that the guarantee was described in
the shared documentation as a property of the design. The method's own doc
comment says an engine that *can* enforce it in the schema **should**, "because
a guarantee enforced only in application code is a guarantee that ends the first
time somebody opens a SQL console" — and all three silently could and did not.
For a clinical record, an append-only claim that holds on 40% of the supported
engines is worse than no claim, because it is relied upon.

**Fixed** on all three: MySQL via `SIGNAL SQLSTATE '45000'` (with
`DROP TRIGGER IF EXISTS` first, because MySQL 8 has neither
`CREATE TRIGGER IF NOT EXISTS` nor `CREATE OR REPLACE TRIGGER` — found when
run 2 failed with `ERROR 1359`), SQL Server via an `INSTEAD OF` trigger that
throws, Oracle via `raise_application_error`.

`dialects.rs` now fails any dialect whose append-only tables lack enforcement
for both `UPDATE` and `DELETE`, so a sixth engine cannot be added without it.

Observed refusing both operations, with a row present, on PostgreSQL 18 and
MySQL 8.4. Reasoned only on SQL Server and Oracle.

---

## A-16 — a parser panicked on one multi-byte character

**Severity:** High. **Requirement:** `X11.7`, `T13.3`. **Status: fixed.**

`split_offset` in `base/iso8601.rs` split a suspected UTC offset with

```rust
None if digits.len() == 4 => digits.split_at(2),
```

`len()` counts **bytes**. A single four-byte character satisfies `len() == 4`,
and `split_at(2)` then lands inside it and panics instead of returning `None`.

`Time::from_str("0-\u{10348}")` panicked. So did any date-time whose offset
position held such a character.

The severity is the reachability, not the subtlety: parsers exist to consume
text from outside, and this crate's own module header says so. A service
accepting an openEHR document could be stopped by one character in a field it
was about to reject anyway. It is a denial of service reachable before
authentication in any deployment that parses before it authorizes.

**Fixed** by rejecting non-ASCII in the offset before any split, which is the
actual domain constraint — an offset is digits and an optional colon — and so
removes the class rather than that one index.

**Found by** the first run of the new `parsers_never_panic` property. Every
example-based test passed, and had passed since the parser was written: nobody
writes `0-𐍈` as an example. Pinned additionally as
`a16_multibyte_offset_returns_err_and_does_not_panic`, because the property
only finds it while its generator still emits multi-byte characters, and a
later edit could narrow that generator without anyone noticing what was lost.

---

## A-17 — the first property tests passed without testing anything

**Severity:** Medium. **Requirement:** `T13.2`. **Status: fixed.**

The partial-order laws were written over a generator drawing years uniformly
from 0–9999. Every interesting comparison in a partial order happens between
values agreeing on a prefix — same year, different precision — and two
independent draws share a year about once in ten thousand. At proptest's
default 256 cases the `None` branch was reached essentially never.

All four laws passed. They would have passed against almost any implementation.

Caught by mutating `Date::partial_cmp` to compare on the left operand's
precision, which makes the order non-antisymmetric: **every law still passed**.
Narrowing the generator to four values per component makes prefix collisions
the common case, and the same mutation now fails antisymmetry.

Mutation also showed the four laws were jointly satisfiable by a *total* order
— reflexivity, antisymmetry, and transitivity say nothing about when a
comparison must be undecidable, and returning `Some` unconditionally passed all
of them. That is the entire purpose of the type. Two laws were added:
incomparability of a value with its own refinement, and its complement, that
differing known components stay decidable so `None` cannot come to mean merely
"different precision".

**The general lesson, recorded because it recurs here:** a passing test is
evidence only after it has been shown capable of failing. This is the third
time in this work that a check reported success while proving nothing — the
others were a mutation that silently failed to apply (`T13.2`) and an
append-only trigger tested against zero rows (`openehr-store/spec/conformance.md`).
The three share one shape: the subject of the test was absent, and absence
reads as success.

---


## A-23 — a version's invariants were checked in one place, and not the one that matters

**Severity:** High. **Requirement:** `V8.1`, `L10.9`, `J9.9`. **Status: fixed.**

Found while classifying the 75 invariants
[`assets/invariant-coverage.md`](../../assets/invariant-coverage.md) reports as
unnamed — a list whose own header says distinguishing out-of-scope from vacuous
from unenforced "needs a human".

**Found.** `OriginalVersion::new` checks `Lifecycle_state_valid` and
`Data_valid`. The type derives `Deserialize`, which writes the fields straight
in, and no `Validate` implementation existed for a version at all — `validation.rs`
did not mention `OriginalVersion` once. The store validated
`version.data()`, the composition *inside* the envelope, and never the envelope.

So a version arriving as JSON was checked by nothing. Measured, not inferred: a
document naming lifecycle state `9999` and carrying no data at all deserialized
to `Ok`, and `openehr-loco`'s `POST` accepted exactly that shape.

**A third invariant was enforced nowhere.**
`VERSION.Preceding_version_uid_validity` — `uid.version_tree_id.is_first xor
preceding_version_uid /= Void` — was in neither the constructor nor the store.
The store refuses a rootless successor only by comparing against the container's
head, so committing **version 2 into an empty container** succeeded and produced
a history whose first entry says it is not the first.

Two things make it credible that this went unnoticed:

- The conformance suite tested the rootless successor *with* a head present.
  One case, and the guard was only as wide as it.
- A unit test in `common.rs` built version 2 with no predecessor while testing
  something else entirely. The impossible version was easy enough to construct
  that a test did it by accident, and enforcing the invariant is what surfaced
  it.

**Fixed.** Three parts, because the gap had three:

1. `OriginalVersion::new` enforces `Preceding_version_uid_validity`.
2. `impl Validate for Version<T>` covers `Lifecycle_state_valid`, `Data_valid`,
   and `Preceding_version_uid_validity`, and descends into the data. This is the
   half that covers deserialized input.
3. `commit_composition` validates the **version**, not the composition inside
   it.

Deserialization stays lenient rather than being made to refuse, because `J9.9`
says so and the reason holds: a document that cannot be read cannot be inspected,
repaired, or reported on. What was missing was any way to ask whether it was
valid, and now there is one.

`Attestations_valid` and `Other_input_version_uids_valid` are named in the new
impl and deliberately not checked: both are `X /= Void implies not X.is_empty`,
and a `Vec` has no way to be present and empty in the openEHR sense. Named so a
reader finds the reason rather than concluding they were missed.

**Residual.** None. The other 74 were classified by `A-24`, which this finding
prompted — and that work found `A-25` in turn.


## A-24 — seventy-five invariants nobody had looked at

**Severity:** Medium. **Requirement:** `L10.4`, `L10.9`, `W0.4`. **Status:
classified; three sub-findings open.**

**Found.** `assets/invariant-coverage.md` reported 75 of RM 1.1.0's 155
invariants as "not named in the crate's source", and said in its own header that
telling out-of-scope from vacuous from unenforced "needs a human, and this file
does not attempt it".

That sentence was true for as long as nobody did it. While it stood, a genuine
gap and a class this crate deliberately does not model looked identical, which
is `W0.4` exactly: a gap not written down reads as a pass. `A-23` was found in
the first hour of doing the work.

**Classified.** Every one is now dispositioned in `openehr-assets`, and the
build fails if an invariant is named nowhere and dispositioned nowhere — or if a
disposition outlives the invariant it explains.

| | Count |
| --- | --- |
| Out of scope | 29 |
| Cannot fail in Rust | 17 |
| Enforced under another name | 1 |
| **Not enforced** | **25** |

"Cannot fail in Rust" is the largest honest answer and the least obvious one.
Most are `X /= Void implies not X.is_empty`, and a `Vec` has no way to be
present and empty — openEHR's absent case *is* the empty collection. The rest
are predicates derived from the field they constrain: `is_null()` returns
`value.is_none()`, `is_archetype_root()` returns `archetype_details.is_some()`,
`is_merged()` returns `!other_input_version_uids.is_empty()`. Each is true by
construction, and each would have read as a missing check forever.

**Three sub-findings, open:**

- ~~**`DV_INTERVAL.Limits_consistent` is enforced under the wrong name.**~~
  **Fixed.** `Interval::new` refused `lower > upper` and reported `INTERVAL`
  with prose. `L10.4` requires openEHR's own name so a reader can find the rule
  in the class definition — the same defect `A-20` fixed fifteen times,
  surviving because the grep that found those looks for names that *are* used
  and this one used none.

- **`TERM_MAPPING.Purpose_valid` is unenforced although the crate ships the code
  set.** `term_mapping_purpose::GROUP` exists and is registered; nothing checks
  a mapping's purpose against it. Same shape as `A-22`, which found three
  `DV_MULTIMEDIA` invariants unenforced "despite the crate shipping their code
  sets".

- ~~**`VERSION.owner_id` is not modelled at all.**~~ **Resolved 2026-08-02, and
  the suspicion was wrong.** This said confirming it needed the BMM's attribute
  lists read rather than inferred from, and that the register does not guess
  (`W0.3`). The BMM was then read.

  `VERSION` declares three **properties** — `contribution`, `signature`,
  `commit_audit` — and `owner_id` is not among them. It is a **function**, whose
  documentation says: *"Copy of the owning `VERSIONED_OBJECT._uid_` value;
  extracted from the local `_uid_` property's `_object_id_`."* So
  `Owner_id_valid` constrains a derived value against the thing it is derived
  from, and cannot fail. The crate is right not to store it.

  Reading the BMM for that one question answered six more. `is_simple`,
  `purpose`, `type` on `PARTY`, `ADDRESS`, `CONTACT`, `PARTY_IDENTITY` and
  `PARTY_RELATIONSHIP` are all derived functions — `purpose` and `type` are
  documented as *"taken from the value of the inherited `name` attribute"* —
  so `Is_simple_validity`, `Purpose_valid`, `Type_valid` and `Type_validity`
  are definitional too. **Unenforced fell from 25 to 18.**

  The distinction the register had been missing is now vendored as
  `assets/rm-1.1.0-attributes.json`, so the next classification does not have
  to re-derive it: an invariant constraining a *property* is a rule, and one
  constraining a *function* is usually a definition.

**Also recorded rather than fixed:** the four `PARTY`/`PARTY_RELATIONSHIP` graph
invariants need a demographic *repository* — an object store that can be asked
for the reverse of a relationship. The crate models demographics as values with
no back-references. That is a legitimate exclusion and it is **not declared
anywhere**, which `C0.16` calls a defect in its own right. It needs a numbered
requirement beside `S1.4` and `S1.6`.

**Fixed alongside the classification.** The four `EHR` reference-collection
rules — `Compositions_valid`, `Contributions_valid`, `Folders_valid`,
`Directory_valid` — now have an `impl Validate for Ehr`, and the store validates
an EHR before it writes one.

That fix is `A-23` in a second class, and worth stating as such. `A-21` made
`Ehr::new` check `Ehr_status_valid` and `Ehr_access_valid`. The four collections
are filled by **infallible** `with_*` builders, which no constructor can see,
and `Deserialize` is derived — so an EHR read from JSON reached none of the six
checks. Both of `A-21`'s rules are therefore repeated in the `Validate` impl
rather than assumed, and a test deserializes an EHR whose status and access
references are both typed `"EHR"` and asserts both violations come back.

Every one of these is an `OBJECT_REF`. Rust cannot tell a reference to a
composition from one to a contribution, so the type name is the only thing that
can — which is why the rules exist and why nothing else would have caught a
`compositions` list naming a `CONTRIBUTION`.

**Since.** `ENTRY.Is_archetype_root` and `ENTRY.Subject_validity` are now
enforced, and `VERSIONED_OBJECT.Latest_version_valid` joined its two siblings as
definitional — `latest_version()` returns `versions.last()`.

Enforcing `Is_archetype_root` found **seven fixtures that violated it**,
including the README's own example of "a composition another openEHR
implementation wrote". Every one carried the archetype id as its
`archetype_node_id` and no `archetype_details`, which `LOCATABLE.Archetyped_valid`
makes the same statement as not being an archetype root. That is `A-21`'s shape
for the third time: the fixtures were built from what an entry looks like rather
than from what the model requires, and nothing compared the two.

`Subject_validity` is the more interesting of the pair, because openEHR gets it
free and this crate does not. The BMM documents `subject_is_self` as *"True if
this Entry is about the subject of the EHR, in which case the subject attribute
is of type PARTY_SELF"* — an implication that holds by construction there.
Here `PartyProxy::is_subject` also answers true for a `PARTY_RELATED` whose
relationship is `self`, so an entry could claim to be about the patient while
naming a related party, and the two readings of "who is this about" diverged in
silence.

**Since, again.** `REFERENCE_RANGE.Range_is_simple` and `Is_archetype_root` on
`EHR_ACCESS` and `PARTY` are enforced, and the two `EVENT` timing rules turned
out to be definitional — the BMM makes `offset` and `interval_start_time`
derived functions, and this crate stores neither.

**Ten unenforced invariants remain, and none of them is merely undone.** Nine
need external code sets the crate deliberately does not carry — ISO 639, ISO
3166, IANA character sets and media types, the `A-19` decision. The tenth,
`EHR_ACCESS.Scheme_valid`, is a **declared departure**: openEHR derives `scheme`
from the concrete `settings` and requires it non-empty, so an `EHR_ACCESS` must
always carry a policy. `EhrAccess::new` deliberately records none, because "no
access policy has been set" and "the policy is deny-all" are different facts and
collapsing them would invent one.

That is the end of the classification `A-24` began. Every one of RM 1.1.0's 155
invariants is now either cited by the crate, definitional, out of scope,
enforced under another name, or unenforced **for a stated reason** — and the
build fails if a new one appears with no answer.

**Residual.** None. Both departures this finding recorded are now declared:
`S1.19` excludes the demographic repository the four `PARTY` graph invariants
constrain, and `S1.20` declares the `EHR_ACCESS.Scheme_valid` departure. `L10.11`
adds the register `L10.9` had only in the other direction — every openEHR
invariant the crate does not enforce, with its reason — and `openehr-assets`
fails the build when that register and the generated report disagree, in either
direction.


## A-25 — the measurement was wrong, and wrong in the flattering direction

**Severity:** High. **Requirement:** `L10.4`, `W0.3`, `W0.4`. **Status: fixed.**

Found by `A-24`'s own staleness guard. After enforcing `TERM_MAPPING.Purpose_valid`
the build refused to proceed, naming ten dispositions it said were now
unnecessary — among them `CONTACT.Purpose_valid` and `PARTY_IDENTITY.Purpose_valid`,
which nothing had been done to.

**Found.** `invariant_coverage` decided an invariant was named by asking whether
the crate's source *contained the string*. Two things follow, and both inflate:

1. **The class was ignored.** openEHR reuses 15 invariant names —
   `Language_valid` belongs to seven classes, `Value_valid` to six,
   `Is_archetype_root` to five, `Purpose_valid` to four. Naming one marked all
   of them.
2. **Comments counted.** A doc comment saying an invariant is *not* checked, and
   why, made it count as named. `A-24`'s own careful notes — "`Attestations_valid`
   and `Other_input_version_uids_valid` are named here and deliberately not
   checked" — did exactly that.

Both are false passes, in a file written to remove that kind of ambiguity.

**Fixed.** An invariant counts as named when the source cites the
`(class, name)` **pair**, using the same parser that already reads
`ParseError::invariant(...)` and `ctx.violation(...)` for the `L10.4` divergence
check. Comments cannot satisfy it; a rule belonging to another class cannot
satisfy it.

| | Before | After |
| --- | --- | --- |
| Named | 83 | **69** |
| Not named | 72 | **86** |

Twenty-four invariants nobody had examined were revealed, and are now
dispositioned: 6 enforced under another name, 5 that cannot fail, 4 out of
scope, and 7 genuinely unenforced.

**Two of them I called unenforced, and they were not.** `ATTESTATION.Reason_valid`
and `PARTICIPATION.Function_valid` are checked by `check_optional_group`, which
takes the class and invariant as literals and reports through them. The parser
matched two call forms — `ParseError::invariant(` and `.violation(` — and a
rule enforced by a helper was invisible to both. I wrote "genuinely unenforced,
with their groups already shipping" into this finding before reading the code
that enforces them, which is `W0.3` in the register that exists to enforce it.

An enumerated list of call forms is a guard only as wide as its list, and that
shape had now bitten twice in one finding. The parser matches the **pair** —
any two adjacent string literals shaped like a class and an invariant — so a
helper added tomorrow is covered without anyone remembering to add it.

Making that change needed a second correction. Comments must not count, and the
first attempt skipped them by searching for `//` — which truncated `"https://…"`
mid-literal and put every quote after it out of phase, silently *un*naming forty
rules that were fine. Replaced with a scanner that tracks whether it is inside a
string, a line comment, or a block comment.

**It also caught my own code.** The first `impl Validate for Ehr` raised its six
violations through a closure taking the invariant name as a *variable*. The
citation parser reads literals, so all four new `EHR` rules still counted as
unnamed. Rewritten with the class and invariant spelled out at each call site —
which is what makes them findable by a human grepping too, and is now noted in
the code so the next person does not tidy it back.

**The honest reading.** This did not make the crate worse; it made the report
true. Sixteen invariants that had been counted as covered never were, and the
number had been quoted in a generated file that says at the top it is evidence.

## A-26 — a total that was derived once

**Severity:** Low. **Requirement:** `C0.7`, `C0.20`. **Status: fixed.**

Found by checking `db:D-09`'s defect against the other tree.

**Found.** The matrix said, of itself:

> Counted mechanically from the tables below, with every requirement id in
> `spec/*.md` checked to appear exactly once — 291 ids, 291 covered, none
> missing. A hand-written total in a file like this is a number nobody
> rechecks; this one was derived from the rows.

It was derived from the rows, once. Six requirements added afterwards had no row
at all: `S1.18`, `S1.19`, `S1.20`, `L10.9`, `L10.10`, `L10.11` — three of them
added the same day, the other three earlier in the same sequence of work. The
sentence warning against a number nobody rechecks was itself the number nobody
rechecked.

Milder than `D-09`: nothing contradicted anything, and the *covered* figure was
right. What was wrong was the denominator, and the claim of completeness that
rested on it.

**Fixed.** The six have rows, the header records the version it was actually
assessed against, and **CI re-derives the count on every push** — expanding the
ranges in the `Id` column, comparing against the requirements the specification
defines, and failing on a requirement with no row, a row for a requirement that
does not exist, or an id covered twice.

**A near miss worth recording.** The first measurement reported nine missing,
including `R4.12a`–`R4.12c`. Those are covered, by a row reading
`R4.12a–R4.12c`; the range expander dropped letter suffixes and collapsed it to
`R4.12`. Three of nine reported gaps were defects in the instrument, which is
`A-25` in miniature and the second time in two days that a measurement of this
specification has been wrong before the specification was.

## A-36 — a URI checked at one gate, and a panic behind the other

**Found 2026-08-20**, by writing a fuzz target for a type that had none and
running it. Two defects, one cause.

**The panic.** `DvUri::scheme()` read:

```rust
self.value.split_once(':').map(|(s, _)| s).expect("constructor guarantees a scheme")
```

with rustdoc saying `# Panics — Never: the constructor guarantees a colon is
present`. The constructor does. `Deserialize` is derived, writes `value`
straight in, and calls no constructor — which is `L10.1a`, stated in this
crate's own specification and in `CLAUDE.md`, and the sentence in the rustdoc
was written as though only one construction path existed.

```rust
let u: DvUri = serde_json::from_str(r#"{"value":"nocolon"}"#).unwrap();  // Ok
u.scheme();  // panicked at src/rm/data_types/uri.rs:86
```

`rest()` was the same function with the other half of the tuple.

**The silent half, which is worse.** `DvEhrUri` is `#[serde(transparent)]` over
`DvUri`. Its whole reason for existing is that `LINK.target` is typed
`DV_EHR_URI` so that *a link cannot point out of the record without saying so*
(`D3.31`, `M5.9`) — and the type's own doctest asserts that
`"https://example.org/x".parse::<DvEhrUri>()` is an error. It is. The JSON path
is not:

```rust
let u: DvEhrUri = serde_json::from_str(r#"{"value":"https://example.org/x"}"#).unwrap();
assert_eq!(u.scheme(), "https");   // passed
```

No panic, no error, no violation — a link out of the record, in a record, with
the type system satisfied.

**Why nothing reported it.** `impl Validate for DataValue` ends in `_ => {}`.
`Uri` and `EhrUri` fell into it. This is the hazard `CLAUDE.md` records about
`Node::children` — *a path that resolves to nothing is not an error* — appearing
in the validation table instead of the navigation one: an absent arm is
indistinguishable from a value with nothing to check. And `LINK.target` was
reached by no validation at all, on any class.

The invariant scanner did not catch it either, and could not have: it asks
whether an invariant is **named** somewhere in the crate.
`DV_URI.Value_valid` was named — as a *disposition*, `Renamed`, reading "the
URI parser refuses invalid text and reports itself". True of the parser. The
disposition described gate one and was read as covering both.

**Consequence.** `openehr-loco` deserializes documents from HTTP. A composition
carrying one malformed link target was a panic in a request handler, and one
carrying a well-formed `https://` target was accepted and stored. Neither
required anything unusual — a hand-written JSON document reaches both.

**Fixed**, in four parts:

1. `scheme()` and `rest()` are **total**: no colon means an empty scheme, which
   compares unequal to `ehr` and to every other real scheme, so a caller that
   dispatches on it fails closed. `D3.30a` now requires this of any accessor on
   a type whose `Deserialize` is derived.
2. `check_uri` and `check_ehr_uri` in `validation.rs`, reached from the
   `DataValue` arms that used to fall through. `check_uri` re-runs `DvUri::new`
   rather than restating its rules, so the two gates cannot drift (`W0.1`).
   Emptiness reports openEHR's own `DV_URI.Value_valid`; the scheme and
   character rules report `Uri_well_formed`, registered as a crate addition
   under `L10.9` because openEHR's `Value_valid` is only `not value.is_empty`.
3. Links are validated on `LOCATABLE`, so every node of every structure is
   covered by one call, at path `/links[N]/target`.
4. The `DV_URI.Value_valid` disposition is removed. `openehr-assets` **refused
   the build** until it was — "dispositions for invariants the crate now names"
   — which is `A-24` working as designed.

**Reproduced before and after.** Four tests in `tests/guarantees.rs`, each
naming its failure mode, plus the `uri` fuzz target, which finds the panic from
an empty corpus.

**What this says about the class.** Every finding here is some version of "a
claim was written once and never re-checked". This one is narrower and worth
naming separately: **a mitigation was recorded against the gate it worked at,
and read as covering the gate it did not.** The disposition was not wrong. It
was scoped, and nothing carried the scope.

## A-35 — ten types whose equality and order contradicted each other

**Opened** while fixing `A-32`, as a note that the base-layer defect had the
same shape one level up, in `DvDate`, `DvTime`, `DvDateTime`, `DvDuration`, and
`DataValue`. Left open because closing it looked like it meant rippling a trait
removal through `Interval<T>`'s bound.

**Closed 2026-08-21**, and the survey that closed it found the record was wrong
about both the scale and the cause.

**The cause is not lexical form.** That is where it was first seen. Every
`DV_ORDERED` descendant carries `OrderedAttrs` — normal range, normal status,
other reference ranges — and every one derives `PartialEq` over all its fields
while comparing only its magnitude. Run against each type in turn:

| These two values | `==` | `partial_cmp` |
| --- | --- | --- |
| `DV_DATE_TIME` `11:00:00Z` and `12:00:00+01:00` | false | `Some(Equal)` |
| `DV_TIME` `11:00:00Z` and `12:00:00+01:00` | false | `Some(Equal)` |
| `DV_DURATION` `PT60M` and `PT1H` | false | `Some(Equal)` |
| `DV_QUANTITY` `5 mg` with `precision` 1 and with 2 | false | `Some(Equal)` |
| `DV_QUANTITY` `5 mg` with and without `units_display_name` | false | `Some(Equal)` |
| `DV_COUNT` `5` with and without a normal range | false | `Some(Equal)` |
| `DV_PROPORTION` `1/4` with `precision` 1 and with 2 | false | `Some(Equal)` |
| `DATA_VALUE` wrapping any of the above | false | `Some(Equal)` |

Five types were named in the finding. Ten had it — the four temporal wrappers,
`DV_QUANTITY`, `DV_COUNT`, `DV_ORDINAL`, `DV_SCALE`, `DV_PROPORTION`, and
`DATA_VALUE` — and the three that the original note did not reach have nothing
to do with ISO 8601.

**What was actually wrong.** Rust requires `a == b` if and only if
`partial_cmp(a, b)` is `Some(Equal)`. Every row above ships `a != b` together
with `a <= b` and `a >= b`. Inside this crate that is invisible, because every
comparison here goes through the ordering consistently — which is exactly why it
survived two audits. It surfaces in a caller: `binary_search` can return a hit
that is not `==` to the needle, `dedup_by` leaves adjacent "equal" elements,
`sort_by` and `max_by` are underdetermined.

**Neither trait could move**, which is why the fix is to drop one:

- Making `==` semantic would make `DvCount::new(5).with_normal_range(r)` equal
  to `DvCount::new(5)`, and would let a canonicaliser that rewrote `1.10` as
  `1.1` pass its own round-trip test. That is `db:D-08` reintroduced, in the
  crate rather than in a database.
- Making `partial_cmp` return `None` where the values are ordered-equal but not
  `==` would satisfy the contract and break reference ranges: an interval of
  `[11:00Z, 13:00Z]` would stop containing `12:00+01:00`. A wrong clinical
  answer in exchange for a satisfied trait.

**Fixed** as `D3.18a` was, one level up:

1. No `DV_ORDERED` implements `PartialOrd`, and neither does `DataValue`.
   Comparison is `DvOrdered::semantic_cmp`, a required method so a new
   `DV_ORDERED` cannot forget it, and `DataValue::semantic_cmp` for the enum.
2. `INTERVAL<T>` is bounded on `SemanticOrd` rather than `PartialOrd`
   (`D3.18c`), with explicit impls and deliberately no blanket one — a blanket
   `impl<T: PartialOrd> SemanticOrd for T` collides under coherence, and the
   explicit list is what stops a type with this defect reaching `INTERVAL<T>`
   again without anyone deciding it should.
3. `Interval::contains` is rewritten against `semantic_cmp`. The operators it
   used read "not comparable" as "not greater, therefore below", which is a
   wrong answer rather than a missing one.

**No behaviour changed.** The comparison logic is the same logic; what changed
is which trait it is reachable through. Verified by the suite, by the doctests,
and by the four downstream crates, which needed **no edits at all** — the whole
blast radius was inside `openehr`, and every affected call site was a compile
error rather than a silent change. That is the argument for the trait removal
over the alternatives: the compiler enumerated the work.

**Breaking for callers**, and cheaply: `a < b` becomes
`a.semantic_cmp(&b) == Some(Ordering::Less)`, `a.partial_cmp(&b)` becomes
`a.semantic_cmp(&b)`, and `DvOrdered` has to be in scope. Recorded in
`CHANGELOG.md`; the next release is not a patch.

**Pinned** by `guarantees::equality_and_order_disagree_by_design_and_neither_is_partial_ord`,
which asserts both halves for each shape — that the values are `!=`, and that
they order `Equal` — so restoring `PartialOrd` fails the suite rather than
quietly reintroducing the contradiction. And by
`guarantees::a_reference_range_is_unmoved_by_how_an_instant_is_spelled`, because
the rewrite of `contains` is the part that could have changed an answer.

## A-37 — a red fuzz job nobody read, and two ways an AQL query changed meaning

**Found 2026-08-21**, not by fuzzing but by *looking at the last CI run on
`main`*. It was a failure, from 2026-08-04, seventeen days old:

```
thread '<unnamed>' panicked at fuzz_targets/aql.rs:33:9:
assertion `left == right` failed: AQL normalisation is not idempotent
```

**The finding before the findings.** `CLAUDE.md` said "CI is green".
`openehr-fuzz/README.md` said "No crashes. All seven targets run in CI on every
push." Both were written when they were true and neither was re-checked, and a
red job on the default branch is about as visible as a signal gets. `W0.3` is
usually about a claim nobody could check; this one anybody could, in one
command, and for seventeen days nobody did.

Two independent defects were behind it.

### The lexer widened UTF-8 bytes into characters

```rust
value.push(bytes[i] as char);   // one byte -> one char
```

Scanning the input as bytes is correct — the only bytes the lexer examines are
ASCII delimiters, and an ASCII byte never occurs inside a multi-byte UTF-8
sequence. **Copying** by byte is not. Every non-ASCII character in a string
literal came out as Latin-1 mojibake:

| Written | Lexed as |
| --- | --- |
| `'Müller'` | `'MÃ¼ller'` |
| `'日本語'` | `'æ—¥æœ¬èªž'` |

`WHERE c/name/value = 'Müller'` therefore parsed, checked clean, and asked
about a string nobody is named. There was no error to see: the query was valid,
it was simply about something else. This is the same shape as `db:D-08`, where
MySQL rewrote a magnitude of `1.10` as `1.1` — a silent transformation of
clinical data by a layer that was only supposed to carry it.

Fixed by appending **slices** of the input, tracking the start of each run
between escapes.

### The renderer omitted the parentheses its own grammar needs

`FROM` puts `CONTAINS`, `AND` and `OR` at one precedence level, and `CONTAINS`
takes the whole remainder as its right operand — `containment` calls itself
there. The renderer wrote `{left} CONTAINS {right}` with no parentheses:

| Tree | Rendered | Re-parsed as |
| --- | --- | --- |
| `Or(Contains(a, b), c)` | `(a CONTAINS b OR c)` | `Contains(a, Or(b, c))` |

The caller asked for *either (a containing b) or c*. What came back was *a
containing either b or c*. Both are valid AQL, both select records, and they
select **different** records.

`Q12.15` said a rendered query must re-parse to an "equivalent" query, and by
that word the renderer passed: the output parses. The word was doing no work.
It now says **equal** — the same tree — and the test compares trees rather than
strings.

Fixed by parenthesising any operand that is not a bare class, which leaves the
ordinary `EHR e CONTAINS COMPOSITION c` unchanged.

### A third, found while fixing the second

`Literal::Display` wrote `'{v}'` with no escaping, so a string containing a
quote rendered as `'it's'`. Rendering now escapes `'` and `\` — and only
those, because the lexer's rule is "a backslash introduces the next character
literally" and not a C-style table, so a rendered `\n` would mean the letter
`n`.

**Verified.** The `aql` target, which reproduced all of this from an empty
corpus in under two minutes, now runs seven minutes clean. Pinned by
`guarantees::an_aql_string_literal_is_not_mangled_by_the_lexer` and
`guarantees::aql_rendering_round_trips_through_the_parser`, the second of which
asserts tree equality and not only text equality — the assertion `Q12.15`'s
original wording could not make.

**What this says about the process.** Every finding in this register was found
by running something. This one was found by *reading the result of something
that had already run* — which had been sitting in the open, red, on the default
branch. The fuzz target did its job on 2026-08-04. The gap was between the job
failing and anyone looking, and no amount of additional checking closes that
one.

## A-38 — `serde_json` reads back a number it did not write

**Found 2026-08-21** by the `data_value` fuzz target, on its **first CI run**,
which is the outcome that justifies writing a fuzz target at all.

```
assertion `left == right` failed: canonical JSON is not a fixed point
  left:  …,"value":1.5777777777770001}
 right:  …,"value":1.577777777777}
```

**It is not this crate's arithmetic.** Reduced:

| Operation | Result |
| --- | --- |
| `core::str::parse::<f64>("1.5777777777770001")` | `0x3ff9_3e93_e93e_863b` |
| `serde_json::from_str::<f64>("1.5777777777770001")` | `0x3ff9_3e93_e93e_863a` |

One ULP apart. `serde_json` 1.0.151 serialises `…863b` as
`1.5777777777770001` — correctly, since `core` reads that string back as
`…863b` — and then its own parser reads it as `…863a`. **The serializer and the
parser disagree.**

**The consequence is drift, not a single lost bit.** The first version of this
finding said the value "converges on the second application and stays there".
That was written from one example and was wrong; the fuzz target disproved it
within two minutes of the claim being made. Iterating on a `DV_QUANTITY`
magnitude:

```text
4.4444444444444444e-7  →  4.4444444444444454e-7  →  4.444444444444446e-7  →  stable
```

Three applications there. **No bound is established** — it settles in the cases
observed, and nothing here proves it must. Each serialise-and-reparse cycle can
move a magnitude, and a record that is read, amended in one field, and
re-committed passes every *other* field through such a cycle.

**What this costs, exactly.** Measured rather than reasoned about:

| Question | Answer |
| --- | --- |
| Are the stored bytes stable? | **Yes.** `1.5777777777770001`, written once. |
| Is the digest over them stable? | **Yes.** |
| Can `verify_versions` raise a false tamper alarm? | **No.** |
| Is the value read back equal to the value written? | **No.** One ULP low. |
| Do re-canonicalised bytes match the stored bytes? | **No.** |

The first three are "no" **because of a decision already made**.
`db:M3.43` requires canonical JSON to be stored in a byte-preserving type, and
`openehr_store::integrity::hashed_bytes` hashes `row.data_json` **as bytes**
rather than re-deriving them from the parsed value. Had the integrity check
re-canonicalised — the obvious implementation — a clinical record carrying a
high-precision number would report `ContentAltered` with nobody having altered
it.

That requirement was written for `db:D-08`, where MySQL rewrote a magnitude of
`1.10` as `1.1`. It turns out to defend against the JSON library too. A rule
that pays twice for reasons its author did not know about is the argument for
writing rules about *properties* rather than about the specific thing that went
wrong.

**Reported upstream 2026-08-21** as
[serde-rs/json#1336](https://github.com/serde-rs/json/issues/1336), *"Bug? float
parser is not the inverse of its own serializer."* Open, unlabelled, no
maintainer response yet.

**Still open here, because a report is not a fix.** Nothing in this repository
can make `serde_json`'s parser agree with its serializer, and this finding stays
open until either the upstream issue is resolved and the dependency moves, or
this repository takes option 2 below. Three responses exist and none is free:

1. **Leave it**, with the containment above and this finding. The residual is
   that a caller who reads a value back and compares it to what they wrote
   finds them unequal in the last bit. **This is the current position.**
2. **`serde_json`'s `arbitrary_precision`**, which keeps a number as its
   original text and would preserve the input digits exactly. It changes the
   type of every numeric Reference-Model field from `f64` to something
   text-backed — a large API change with its own arithmetic questions.
3. ~~**Report it upstream**~~ — **done**, see above. Worth doing regardless of
   1 or 2, and it is the only one of the three that helps anybody outside this
   repository.

**What to watch for.** If #1336 is fixed, the pinning test below starts failing
— by design, because it asserts the drift is *present*. That failure is the
signal to bump `serde_json`, delete the test, and close this finding. Do not
"fix" the test by relaxing it.

Choosing between them is a design decision, not a repair, so it is written down
rather than made silently (`W0.19`).

**The fuzz targets assert only that canonical form re-parses** (`W0.31`: a
target must not report a documented limitation as a finding). Convergence was
tried first and is not assertable — see above; the attempt is why the drift is
described accurately here instead of by extrapolation from one example.

The behaviour is pinned instead by
`guarantees::canonical_json_drifts_on_a_high_precision_float`, which fails when
the drift **stops** — an upstream fix or a move to `arbitrary_precision` — so
whoever closes this finding is told what it was. That test also asserts the
containment rather than restating it: a digest over the stored bytes is stable
because it is taken over bytes.

## A-27 — a sign the lexer could not have added

**Closed 2026-08-21.** Opened as a limitation with the decision explicitly
unmade: `WHERE o/value/magnitude > -2.5` is ordinary clinical AQL — a base
excess, a temperature difference, a scale scored below zero — and this parser
refused it outright.

**Why it stayed open.** `-` is also the character that separates the parts of an
archetype id. Adding a sign to the number scanner means deciding what
`openEHR-EHR-COMPOSITION.encounter.v1` is when it follows an operator, and the
finding said so rather than guessing. `CLAUDE.md` carried the warning in as many
words: *do not "fix" it by adding a sign to the number scanner*.

**The decision.** The sign is resolved by the **parser**, at a position where an
operand is expected — never by the lexer's number scanner. The lexer gains `-`
as an ordinary symbol and nothing else changes there.

The ambiguity then cannot arise, and not by care: an archetype id begins with a
**letter**, so it is scanned as a *word*, and the word scanner already absorbs
its own hyphens. It never reaches the symbol branch. A `-` stands alone only
where no word claimed it, and the parser looks at that position and asks a
single question — is a number next?

| Input | Result |
| --- | --- |
| `WHERE o/value/magnitude > -2.5` | a comparison against −2.5 |
| `WHERE c/v MATCHES {-1, 0, 1}` | a three-element set; `MATCHES` parses operands like everything else |
| `COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1]` | unchanged, and asserted to be |
| `WHERE c/v > -openEHR-EHR-…` | **error**: "expected a number after `-`" |

**A dead guard, made honest.** `Parser::integer`'s `v >= 0` was unreachable —
`Token::Integer` starts at a digit and never carries a sign — and mutation
testing could have replaced it with `true` unnoticed. It is gone. In its place
`LIMIT`/`OFFSET` refuse a sign **deliberately** (`Q12.9d`), because what used to
be refused incidentally as `unexpected character` stopped being refused that way
the moment a sign became lexically well formed. The message now names the
reason: *LIMIT and OFFSET are counts and must not be negative*. A `LIMIT` that
clamped `-5` to `0` would return an empty result set that looks like an answer
(`db:P6.15`).

**And it uncovered an older one.** The fuzz target, run against the widened
grammar, found `SELECT -0.0` rendering as `-0` and reparsing as `Integer(0)`.
The sign was not the defect: `format!("{v}")` writes `0` for `0.0`, and this
lexer reads digits with no `.` as an **integer**, so `Number(0.0)` had always
round-tripped to `Integer(0)` — a literal changing type, which `Q12.15`'s tree
equality forbids. It was invisible because the *text* matched; `-0.0` is the
first value where it does not. Fixed by `Q12.9e`: a real renders with a decimal
point.

Two defects, one of them years older than the other, and the second only
reachable because the first was fixed. That is the ordinary shape of this work
and is worth noting against the instinct to treat a new failure after a change
as evidence the change was wrong.

**Verified.** `aql::a_sign_is_a_number_where_a_value_belongs_and_nowhere_else`
drives all four rows above; the `aql` fuzz target ran clean over the widened
grammar. The test that pinned the limitation was **rewritten rather than
deleted**, and its doc comment says what it used to assert — a pinned limitation
and a pinned capability are the same test with the sign flipped, and the history
is the part worth keeping.

## A-39 — two silent matches and a branch that never ran

**Found 2026-08-21** by the retrospective mutation pass that `W-18` left as a
residual — nine commits had reached `main` without the `mutants` job ever
running on them. Nothing had failed. The code was correct; the tests were not
checking it.

### Deleting a match arm is silent, again

`CLAUDE.md` carries this warning about `path.rs`:

> **A path that resolves to nothing is not an error.** … The consequence is that
> **deleting a match arm from the navigation table is silent** … Fifty such arms
> had no test (`A-28`).

The same shape, in a different file, unnoticed. `DataValue::semantic_cmp` is a
match over nine same-class pairs ending in `_ => None`, and **six of the nine
arms could be deleted with the whole suite still green**: `Ordinal`, `Scale`,
`Proportion`, `Date`, `Time`, `Duration`. `DataValue::is_strictly_comparable_to`
is the same match again and was worse — every arm deletable, and the entire
function replaceable with `false`.

**The wrong answer is not an error, which is why it is quiet.** A deleted arm
falls to `_ => None` — *not comparable* — which is a correct answer for a
quantity against a count and a wrong one for two dates. And it does not stop
there:

`DvOrdered::is_abnormal` asks `normal_range.contains(&DataValue::…)`, and
`Interval::contains` reads "not comparable" as "not inside" — deliberately, so
that an undecidable comparison never admits a value. So a `DV_DATE` **outside**
its recorded normal range would report as **not abnormal**, and that method's
own documentation says what happens next: *"a dashboard that renders the first
as the second is reassuring for the wrong reason."*

**Fixed** by one table-driven test with a row per arm, asserting `Less`,
`Greater` and `Equal` in both directions, plus `is_strictly_comparable_to` for
the same rows, plus the cross-class `None` that makes the `_` arm's own job
visible. It asserts `rows.len() == 9`, so adding a variant to either match
without adding a row fails — the same instruction `CLAUDE.md` gives for
`path.rs`.

### A branch that could not change its own output

`trim_float` read:

```rust
if v.fract() == 0.0 && v.abs() < 1e15 { format!("{v:.0}") } else { format!("{v}") }
```

Every mutation of that comparison survived — `<` replaced by `==`, by `>`, by
`<=`. That reads as an untested bound. It is not: **the two branches produce the
same string for every finite `f64`**, measured across 3,952 values spanning
`1e-20` to `f64::MAX` in both signs, with zero differences.

Both halves were solving a problem `Display` had already solved. `{:.0}` existed
to drop the trailing `.0` — `Display` writes `184`, not `184.0`. And
`v.abs() < 1e15` existed to stop `{:.0}` printing a large float's *exact binary
value*: `9876543209999998976` where `Display` writes `9876543210000000000` —
sixteen digits of noise implying precision the value does not have. The guard
protected a branch that was never needed.

**A test could not have fixed this**, and that is the point worth keeping. Two
branches with identical output are indistinguishable by any assertion on the
output, so the surviving mutants were **equivalent mutants over dead code**
rather than a coverage gap. The remedy was to delete the branch and pin the
*property* instead: a whole number renders without `.0`, a huge one gains no
false digits. That test stays, because the property is not this crate's to
guarantee — it rests on `Display for f64` choosing the shortest round-tripping
form, and if that ever changed a chart would start reading `184.0 mm[Hg]`.

### What this says about the method

Every finding in this register was found by running something. This one was
found by running something **at the code rather than at the behaviour** — the
suite passed, the fuzz targets passed, CI was green, and three of these
functions were still not doing anything a test would notice. Mutation testing is
the only check here that asks *what would break if this were wrong*, and it was
switched off for the branch these changes landed on (`W-18`).

## A-19 — a departure whose advice nobody had checked was followable

**Classified 2026-08-22.** Opened because `COMPOSITION.Territory_valid` and
`Language_valid` were neither enforced nor declared; the declaration landed as
`S1.18` and the row has read *"enforcement open"* ever since, which implied
work that `S1.18` argues against.

**The register and the specification disagreed, quietly.** `S1.18` does not say
enforcement is pending. It says implementing it in this crate would be **wrong**:
ISO 3166-1 and ISO 639-1 are closed, small, and *mutable*, a table compiled into
a library is wrong from the day a country changes, and validating against a
stale copy rejects conformant data — which `D3.5`'s own reasoning calls the
worse failure. That is a decision, and `A-02` and `A-08` are already carried as
"open, by decision" for departures of exactly this kind.

**What was actually open** is the sentence the departure ends on:

> A deployment that needs the check should do it where the tables can be
> updated.

That is only true while a caller can **reach** every code the crate declines to
check, and reachability is not free. `A-34` is the finding where
`DV_ENCAPSULATED`'s `charset` and `language` round-tripped perfectly and could
not be read at all — `EncapsulatedAttrs` was exported and no type returned one.
The accessors that make `S1.18`'s advice followable were *added by that
finding*, two departures later, by accident of looking at something else.

**Checked, and it holds.** All nine codes behind the ten unenforced invariants
are reachable through the public API:

| Code | Reached by |
| --- | --- |
| `COMPOSITION.language`, `.territory` | direct accessors |
| `ENTRY.language`, `.encoding` | `Entry::entry_attrs()` — one indirection, the `A-34` shape |
| `DV_TEXT.language`, `.encoding` | optional accessors; absent is a fact and is asserted too |
| `DV_ENCAPSULATED.charset`, `.language` | `encapsulated()`, added by `A-34` |
| `DV_MULTIMEDIA.media_type` | direct accessor |

`guarantees::a_caller_can_read_every_code_the_crate_declines_to_check` pins it.
The failure mode it guards is specific: an accessor with no caller can be
deleted or narrowed with every test still green, and at that moment the
departure becomes **silently worse than declared** — the crate does not do the
check and the caller can no longer do it either.

The encapsulated codes are reached by deserializing, not by building, because
`EncapsulatedAttrs` has no builder. That is the right path to test anyway: a
code the crate will not check is a code that came from outside it.

## A-38 — an upstream defect that was a missing feature

**Closed 2026-08-22**, by being handed a specification naming a `serde_json`
feature I had not looked for.

The finding was accurate about the behaviour and wrong about everything else.
`serde_json` did parse `1.5777777777770001` one ULP below `core::str::parse`,
its parser was not the inverse of its serializer, and a magnitude did drift
across repeated canonical round trips. What was wrong was the conclusion:

> **Open, and upstream.** Nothing in this repository can make `serde_json`'s
> parser agree with its serializer.

`serde_json` has a **`float_roundtrip`** feature that makes exactly that true,
and this repository had not enabled it. The fix is one word in thirteen
manifests. With it the two agree bitwise and canonical form is a fixed point
from the first application:

```text
before   4.4444444444444444e-7 → …4454e-7 → …446e-7 → stable
after    4.4444444444444444e-7 → stable
```

**Three responses were written down and the real one was not among them.** The
entry listed: leave it contained, adopt `arbitrary_precision`, report upstream.
All three took as given that the crate could not fix it. That premise was never
checked — it came from reducing the defect to a minimal case, confirming it,
and stopping. Nobody read the feature list of the dependency the finding was
about.

The upstream report stands and is not wasted: the default parser really is not
the inverse of the default serializer, which is worth someone's attention
whatever this repository does. But it was filed as *the* remedy rather than as
a courtesy, and it delayed the fix by a day.

**`arbitrary_precision` is separately refused**, with evidence, as `SJ2`. It is
incompatible with this crate's `#[serde(tag)]` and `#[serde(flatten)]` layout —
four round-trip tests fail with `invalid type: map, expected f64` — and its
benefit reaches only `serde_json::Number`, where the Reference Model stores
magnitudes as `f64` fields.

**What the pinning test did.** `canonical_json_drifts_on_a_high_precision_float`
asserted the drift was **present** and failed the moment it stopped, with the
message *"If serde-rs/json#1336 was fixed, that is the good outcome: bump
`serde_json`, delete this test, and close A-38."* It fired on the first run
after the feature was enabled and told the next reader what to do. It is kept,
inverted: the property now rests on a cargo feature staying enabled, which is
one careless edit from silently reverting to drift nothing else would notice.

The two fuzz targets are restored to asserting canonical form is a **fixed
point**, which was weakened to "must re-parse" while this was open.

## Closed findings

**A-01** and **A-03** are fixed and kept above with their evidence, because the
evidence is the reason each fix is trusted and because each leaves a residual
that is still live — an inference about `DV_SCALE`, and unbounded recursion
depth. A finding is not deleted when it is fixed; it is marked.

## A-40 — an entire section in force, and nothing behind it

**Severity: Medium. Status: open — object model built 2026-08-26; validation
against an in-memory archetype, and repository resolution of a filled slot,
built 2026-08-30; no parser, flattening, or template expansion.**

**What happened.** `S1.4` — *the crate MUST NOT implement the Archetype Model* —
was withdrawn on 2026-08-26 and replaced by `S1.21` and
[§15](15-archetypes.md): AOM2 as types, ADL 2 parsing, ADL 1.4 ingestion,
specialisation and flattening, template expansion, operational templates,
validation against an operational template, and a repository abstraction for
retrieval. Thirty-one requirements in §15 plus `S1.21`, all in force from the
day they were written.

**The gap, as it stands.** `K15.1`–`K15.4` are implemented and tested:
`openehr::am` is the AOM2 object model — `ARCHETYPE`, the constraint tree,
multiplicities, and archetype terminology — with construction-time checking of
the AOM2 validity conditions decidable from one artefact (`VARDT`, `VATDF`,
`VACDF`, `VATCD`, `VOKU`), a lossless JSON round trip, and an `Unsupported`
primitive-constraint variant so an unmodellable constraint is carried rather
than dropped. `K15.18`–`K15.23` are implemented and tested since 2026-08-30:
`openehr::am::validate` walks a Reference Model instance against an
`Archetype`'s definition — existence, cardinality, occurrences, RM class and
node identity, and primitive value constraints — reports a construct it cannot
check (a slot filler, an unmodelled primitive kind, a `C_STRING` pattern) as
*unchecked* rather than passing it, and keeps the verdict a distinct type from
[`crate::validation`]'s, per `K15.19`. `K15.24`–`K15.27` are implemented and
tested since 2026-08-30 as well: `openehr::am::repository` defines the
retrieval abstraction (`openehr` itself performs no I/O, `K15.25`), and
`validate_with_repository` resolves a `C_ARCHETYPE_ROOT` filler through it —
verifying the repository answered the identifier asked for, requiring the
caller to opt in before validating against a result with no established
provenance (`K15.26`), and never treating a retrieval failure as a pass
(`K15.27`).

**Eighteen requirements have no code.** No ADL 2 parser (`K15.5`–`K15.7`), no
ADL 1.4 ingestion (`K15.8`–`K15.10`), no flattening (`K15.11`–`K15.13`), and no
template expansion or operational template (`K15.14`–`K15.17`). For a caller,
the practically important sentence is now narrower still: this crate can tell
you whether a `COMPOSITION` conforms to an archetype it already holds in
memory or can retrieve through a repository it is given, and still cannot tell
you whether it conforms to the *published* archetype named on the instance,
because nothing here reads ADL or merges a specialisation's inherited
constraints in first. A bare `ARCHETYPE_SLOT` stays unchecked regardless of a
repository: which archetype fills it lives on the instance's own
`ARCHETYPED.archetype_id`, which `crate::path::Node` does not expose — a
residual named in its own right below, not folded into this count because it
is a gap in `crate::path`, not in §15.

**Why this is a finding rather than a plan.** `C0.9` — a gap that is not written
down reads as a pass. A specification section with no code is the most flattering
possible document about a crate, and this repository has already been caught
believing its own documentation twice (**W-09**, **A-26**). The register is where
the distance between the specification and the code is kept visible, and 32
requirements was the largest such distance this crate has carried.

**Evidence a reader can check.** `grep -rln "K15\." openehr/src` returns the
`am` module and `lib.rs`, and nothing else — no parser module, no flattening, no
retrieval. Twenty-two §15 rows in
[`conformance-matrix.md`](conformance-matrix.md) read `spec`, a status that did
not exist before this reversal and means exactly this; fourteen read `•` and
name the tests that earn them.

**What holds in the meantime.** `K15.30`: every entry point that would implement
an unbuilt part of §15 refuses explicitly, and no documentation may state or
imply that this crate validates against a *published* archetype — only against
one already held in memory, and `openehr::am::validate`'s own module
documentation says so before it says anything else. `openehr::am`'s own module
header carries the table of what is and is not built, and `validation`'s header
keeps the sentence that passing Reference-Model validation does not mean an
instance conforms to its archetype. `K15.31`: a partial implementation is not
described as more archetype support than it is — validation without a parser or
flattening is validation of what is handed to it, and must be called that.
`L10.2`, amended, keeps the sentence that a passing composition may still
violate its archetype in place until the matrix says every requirement in §15 is
satisfied.

**Residual, and it is real.** The scope reversal leaves citations elsewhere in
the repository that point at a withdrawn requirement while stating a reason that
is no longer the reason: `openehr-store/src/store.rs`,
`openehr-store/src/schema.rs`, `openehr-fuzz/README.md`, and the engine crates'
"not implemented anywhere (`lib:S1.4`)" rows. Every one of them remains
**factually** correct about the code — nothing implements archetypes — and every
one now cites a withdrawn decision instead of an unbuilt requirement. They are
not rewritten here on purpose: `W0.6` says a citation's whole value is that it
does not change, and a mass rewrite during a specification change is how a
citation stops meaning anything. They are re-pointed when the code they describe
changes, and this paragraph is the record that they are known.

**Residual, added 2026-08-30.** `validate_with_repository` resolves a
`C_ARCHETYPE_ROOT`, but a bare `ARCHETYPE_SLOT` stays unchecked with or
without a repository, and this is a gap in a different module than the one
this finding is about: which archetype fills a slot is recorded on the
instance's `ARCHETYPED.archetype_id`, and `crate::path::Node` — built for AQL
and path resolution, before archetype support existed — exposes only
`archetype_node_id`, the short code, never that attribute. Closing it means
adding a variant or a method to `crate::path::Node`, which is `path.rs`'s
surface, not `am`'s; `openehr::am::validate`'s own module documentation states
the gap rather than working around it with something that looks like a
resolution and is not one.

## A-41 — the matrix's totals went stale, again

**Severity: Low. Status: fixed — re-derived mechanically to 344 on 2026-08-26.**

**What happened.** [`conformance-matrix.md`](conformance-matrix.md) stated three
different totals at once: a sentence claiming *300 ids, 300 covered*, a totals
table summing to **291**, and rows covering **311**. All three were written to be
derived from the tables, and none was re-derived after the requirements that
followed them.

**Why the existing guard did not catch it.** The `claims` job re-derives
*coverage* — every requirement has exactly one row — which is what **A-26**
asked for, and it passed the whole time. Coverage and tally are different
claims, and only one of them was mechanised. That is **A-26** one level down: the
boast moved from "every id has a row" to "here is how many of each", and the new
boast had no check.

**Fix.** Both numbers re-derived by expanding every `Id` cell and counting
statuses: 344 requirements, 344 covered, and a per-status tally that now includes
the `spec` and `withdrawn` statuses this reversal introduced.

**Residual.** The tally is still hand-transcribed into the file. The honest
statement is in the matrix itself — it names the method, so the next reader can
re-run it — and mechanising the tally the way coverage is mechanised is the
better fix, not yet done.

## A-42 — three more invariants checked at construction and nowhere else

**Severity: Medium. Status: fixed.**

Found by re-reading the canonical RM's own invariant list against what
`Validate` actually walks, the same method `A-23` used — not by a fuzzer or a
bug report, and not restating `A-23` itself: this is three fresh instances of
its exact shape, in three different classes.

**Found.**

1. `AUDIT_DETAILS.System_id_valid` and `Change_type_valid`. `AuditDetails::new`
   checks both, but `impl Validate for Version<T>` (the fix `A-23` added)
   never visited `self.commit_audit()` — it covered the version's own
   envelope invariants and stopped short of the audit record every commit
   carries. `check_attestation` was no better: it checked an attestation's
   committer (`Committer_valid`) but not the same `AUDIT_DETAILS`'s
   `system_id`/`change_type`. Measured: a `VERSION` deserialized from JSON with
   `"system_id":""` and a `change_type` code outside `audit_change_type`
   reported no violation for either.
2. `ISM_TRANSITION.Transition_valid`. `IsmTransition::with_transition` checks
   that `transition` is from the `instruction_transitions` group; the `Action`
   arm of `impl Validate for Entry` checked `current_state` and never
   `transition`, immediately beside it in the same struct.
3. `INTERVAL_EVENT.Math_function_validity`. `IntervalEvent::new` checks that
   `math_function` is from the `event_math_function` group; the `Interval` arm
   of `impl Validate for Event` called `check_coded_text` on it (the
   `DV_CODED_TEXT`-level rubric check) but never the group-membership check
   that is `Math_function_validity` itself.

All three are `A-23`'s shape exactly: a rule a constructor enforces, on a type
that also derives `Deserialize`, which writes the field straight in and calls
no constructor (`db:V9.8`, `lib` side).

**Fixed.** A shared `check_audit_details` helper (checking all three
`AUDIT_DETAILS` invariants, including `Committer_valid`) is called from both
`impl Validate for Version<T>` and `check_attestation`, replacing the
narrower, duplicated `check_party` call the latter used alone. `Transition_valid`
and `Math_function_validity` are each a direct group-membership check beside
the sibling check already there, matching that sibling's own style rather
than introducing a new one.

`openehr/tests/canonical_json.rs`'s `kitchen_sink()` fixture already builds a
real `ISM_TRANSITION` and `INTERVAL_EVENT` through their checked constructors;
a new test corrupts each in a serialized copy and confirms `validate()`
reports it, then confirms the unmodified fixture stays clean — a check strict
enough to fail only on the corrupted copy, not the original. A second new
test extends the existing `a_version_envelope_is_checked_on_data_that_arrived_as_json`
JSON-literal test (the one `A-23` added) with a bad `system_id` and a bad
`change_type`, the same way that test already covers `Lifecycle_state_valid`
and `Data_valid`.

**Residual.** None found in this pass. The method — re-reading invariants
against `Validate`'s actual walk rather than trusting a prior fix's scope —
is the same one that found `A-23`'s two other invariants inside itself; it was
not re-run over the rest of the RM exhaustively, so more instances of this
shape may remain uncounted.

## A-43 — `INTERVAL<T>` had no interval-vs-interval operations

**Severity: Low. Status: fixed.**

Found while cross-checking `openehr::base` against the canonical BASE
foundation types (`openEHR/specifications-BASE`,
`org.openehr.base.foundation_types.interval.adoc`), which declares three
abstract functions on `INTERVAL`: `has(e: T)` (element membership),
`intersects(other: INTERVAL)`, and `contains(other: INTERVAL)`. This crate's
`Interval<T>` had only the first, under the name `contains` — a name already
public and not renamed to make room for the other two, since a rename is a
breaking change for a method that already does what it says.

**Why this is Low rather than Medium.** Nothing in this crate calls for
either missing operation yet: `am::multiplicity::MultiplicityInterval::narrows`
hand-rolls the same interval-vs-interval logic for a different type, because
`base::Interval` offered it nowhere generically — a duplicated rule waiting to
diverge, in the shape CLAUDE.md's Gregorian-leap-rule note warns about, but not
yet a defect anyone has hit.

**Fixed.** `contains_interval` (all points of `other` are points of `self`)
and `intersects` (at least one limit of `other` falls strictly inside `self`).
Both compare bounds directly through `SemanticOrd` rather than reusing
`contains` on `other`'s raw bound values, which gets one case wrong: `self =
(0, 10)`, `other = (0, 5)` share the excluded point `0`; neither interval
contains it, so `self.contains(0)` alone answers `false` and a check built on
it would wrongly conclude `other` is not contained, when every point `other`
actually has (everything strictly between `0` and `5`) is a point of `self`.
Worked through by hand before writing the implementation, not found by a
failing test — the naive approach was never committed — and then confirmed
by a test built for exactly this shape.

Six new tests, including the shared-excluded-boundary case above, an
open-vs-closed-at-the-same-point case, one-sided-unbounded intervals (both
directions), and `intersects`'s "touching but not overlapping" case
(`(0, 10)` and `(10, 20)`, open at the shared point on at least one side, so
neither interval contains it and they do not intersect).

**Residual.** `MultiplicityInterval::narrows` was not rewritten to use the new
methods — it operates on a different type (`am::multiplicity`'s own interval,
not `base::Interval`), so there is no direct duplication to remove, only a
parallel implementation that could in principle be expressed the same way if
the two types were ever unified. Not attempted here.

## A-44 — two cardinality/occurrences agreement rules `C_ATTRIBUTE` did not check

**Severity: Low. Status: fixed.**

Found by diffing `openehr::am`'s implemented surface against AOM2's own
class definitions (`openEHR/specifications-AM`,
`docs/AOM2/master04.5-constraint_model-class_definitions.adoc`), not against
anything the crate's own documentation had flagged — `am/mod.rs` and
`spec/15-archetypes.md` already disclose the large gaps (no parser, no
flattening, `VASID`/`VACSD` unchecked) accurately; these two were not among
them.

**Found.**

1. `VACMCU`. Where a container attribute's cardinality states a finite upper
   bound, every child's own occurrences, where finite, must have an upper
   bound no greater than it. `CAttribute::container` already summed the
   children's *lower* bounds against the cardinality's upper bound
   (`a_cardinality_that_cannot_hold_its_children_is_refused`), but never
   compared an individual child's *upper* bound against it — so a cardinality
   of `0..2` accepted a single child declared `0..10` without complaint,
   which no runtime resolving that constraint against real data could
   satisfy.
2. `VACSO`. A single-valued attribute's child occurrences must not have a
   finite upper bound greater than `1`. `CAttribute::single` validated only
   that the attribute name was non-empty; a child declared `0..3` under a
   single-valued attribute — which by definition holds at most one object —
   was accepted.

**Fixed.** `CAttribute::container` gains the `VACMCU` check alongside its
existing lower-bound-sum one. `CAttribute::single` gains the `VACSO` check —
which required extracting a private `new_raw` constructor shared by both
`single` and `container`, since `container` had been built *on* `single`:
adding `VACSO` inside `single` directly would have made every container
attribute's children subject to a rule that exists specifically because a
single-valued attribute cannot hold more than one object, wrongly refusing
container children that legitimately occur more than once. Caught before it
shipped, by reasoning through what `container`'s existing delegation to
`single` implied for the new check, not by a failing test.

Four new tests, including one confirming a child `VACSO` refuses under
`single` is accepted unchanged under `container` — the case that would have
silently broken had the two checks not been separated.

**Residual.** The AM object model's other agreement rules between
`C_ATTRIBUTE`'s cardinality and its children's constraints — `VACMLB` (the
existing lower-bound-sum check) already accounted for — were the two checked
here. This pass did not re-derive the full AOM2 validity-rule list
exhaustively against every `C_ATTRIBUTE`/`C_OBJECT` construction path;
`A-40`'s residual (specialisation, `VASID`/`VACSD`, flattening) remains the
larger, already-declared gap this finding does not touch.

## A-45 — the four temporal primitive constraint kinds were entirely unmodelled

**Severity: Medium. Status: fixed.**

Found while cross-checking `openehr::am::CPrimitive` against AOM2's own
constraint-kind list: `C_DATE`, `C_TIME`, `C_DATE_TIME`, and `C_DURATION`
had no variant at all. A node governed by any of the four fell into
`CPrimitive::Unsupported` and was always reported `Unchecked` — disclosed
accurately (`CPrimitive::Unsupported`'s own doc comment names this as
deliberate, and `K15.20` requires exactly this rather than a silent pass),
but with a practical cost the disclosure did not size: most realistic
clinical archetypes constrain at least one date or time field (event time,
onset, therapeutic frequency), so `validate_against_archetype` reported at
least one `Unchecked` node — and therefore `is_conformant() == false` — on
nearly every real archetype, not an edge case.

**Why it had not been done already.** `base::Interval<T>` requires `T:
SemanticOrd`, and none of `base::Date`/`Time`/`DateTime`/`Duration`
implemented it — each has its own inherent `semantic_cmp` (used internally
by the RM's `DvDate`/`DvTime`/etc. wrappers), but no `SemanticOrd` impl, so
`Interval<Date>` was a compile error before this, not a missing check
someone chose to skip.

**Fixed**, in two parts:

1. `SemanticOrd` implemented for all four `base` temporal types, each
   delegating to the type's own already-tested inherent `semantic_cmp` —
   `base::interval`'s own doc comment is explicit that this trait has **no**
   blanket impl, deliberately, so a type reaching `Interval<T>` needs this
   decision made for it, not inherited from satisfying some other bound.
2. `CPrimitive::Date`/`Time`/`DateTime`/`Duration` added, each `{ range:
   Vec<Interval<T>>, pattern: Option<String> }` — a *list* of ranges, matching
   AOM2's own `C_DATE.constraint: List<Interval<Iso8601_date>>` shape, not an
   approximation of `C_INTEGER`/`C_REAL`'s discrete-list-plus-one-range shape.
   `pattern` is carried and never evaluated, the same choice already made for
   `C_STRING`'s own pattern field, for the same reason (no pattern-matching
   implementation exists) — reported `Unchecked` rather than silently
   ignored, same as `C_STRING`'s.

No change was needed in `crate::path`: it already exposed
`DV_DATE`/`DV_TIME`/`DV_DATE_TIME`/`DV_DURATION`'s `value` attribute as
`Scalar::Str` of the ISO 8601 lexical text, which is exactly what the new
`check_temporal` parses and compares.

Ten new tests: four in `base::iso8601` proving `Interval` over each of the
four types uses the semantic comparison (not a lexical one — `PartialOrd` is
deliberately not implemented for these types, `lib:A-32`); six in
`am::validate` — a table-driven test covering all four kinds' range checks
in one pass, plus a dedicated pattern-is-unchecked test matching
`C_STRING`'s own equivalent test.

**Residual.** `pattern` matching itself remains unimplemented for all five
kinds that carry one (`C_STRING` and the four here) — `valid_iso8601_date_
constraint_pattern`-style matching is its own, separately scoped piece of
work, not attempted in this pass.

## A-46 — `C_PRIMITIVE_OBJECT` had no way to carry a `node_id`

**Severity: Low. Status: fixed.**

Found while cross-checking `openehr::am::CObject`'s node-id story end to end.
Every `C_OBJECT` has a `node_id` (`org.openehr.am.aom2.c_object.adoc`:
`1..1`), and `CObject::node_id`'s own dispatcher already read
`CPrimitiveObject`'s field — `Self::Primitive(o) => o.node_id.as_deref()` —
but nothing existed to ever set that field to anything but `None`.
`CPrimitiveObject::new` always left it `None`, and there was no
`with_node_id` to call afterwards, unlike `CArchetypeRoot`, which already had
one.

**Fixed** — `CPrimitiveObject::with_node_id`/`node_id()` added, mirroring
`CArchetypeRoot`'s own pair. Also added: `CPrimitiveObject::PRIMITIVE_NODE_ID`,
the constant `"Primitive_node_id"` — AOM2's own sentinel
(`org.openehr.am.aom2.c_primitive_object.adoc`: "the `_node_id_` attribute
will have the special value `Primitive_node_id`" for a `C_PRIMITIVE_OBJECT`
written inline in ADL with no node id of its own). The sentinel is a literal
string, not coded id-/at-/ac- syntax, so a bare `NodeIdSyntax::of` check
would reject it; `with_node_id` short-circuits on an exact match against the
constant before falling back to `NodeIdSyntax::of` for every other value.

Three new tests: a real `at`-code set and read back through both
`CPrimitiveObject::node_id` and `CObject::node_id`; the sentinel itself
accepted (confirming `NodeIdSyntax::of` alone would have rejected it, so the
short-circuit is doing real work, not standing in for a case that already
passed); and a malformed value — neither a valid code nor the sentinel —
refused with `ParseError`, the same shape as the existing malformed-node-id
test for `CArchetypeRoot`.

**Not attempted.** `C_PRIMITIVE_OBJECT.assumed_value` — the default value
offered to template authoring and archetype-editor UIs — remains unmodelled;
it has no bearing on the conformance-checking path this crate implements and
was ranked below this finding in the same research pass for that reason.

## A-47 — `Terminology_code`/`Terminology_term` did not exist

**Severity: Low. Status: fixed.**

Found while researching BASE for classes this crate had never checked
against its own source, rather than assuming coverage from the identifiers
and foundation types it already has. `Terminology_code`
(`org.openehr.base.foundation_types.terminology_code.adoc`) is not
`CODE_PHRASE`: `CODE_PHRASE.terminology_id` is a structured
`TerminologyId`, while `Terminology_code.terminology_id` is a bare
namespace `String`, and `Terminology_code` additionally carries an optional
`terminology_version` and an optional `uri`. Neither it nor
`Terminology_term` — the term-text/concept-reference pairing built on it —
existed in this crate under any name.

**Why this one, on its own.** `Terminology_code` is the declared type of
`AUTHORED_RESOURCE.original_language`, `RESOURCE_DESCRIPTION_ITEM.language`,
and `TRANSLATION_DETAILS.language` (`org.openehr.base.resource.*.adoc`) —
three classes this crate does not model, and `S1.1`'s own package list
(`RM Data Types, Data Structures, Common, EHR, and Demographic`) does not
name the `resource` package they belong to, so building the three of them is
a separate scope decision this finding does not make. `Terminology_code`
itself is independently well-formed and useful on its own terms — it is a
correct, small addition regardless of whether the three classes above are
ever built — which is why it is recorded here rather than held back until
they are.

**Fixed** — `openehr::base::TerminologyCode` and `TerminologyTerm` added.
Neither class declares an invariant in its own BASE class definition (unlike
`CODE_PHRASE`'s `Code_string_valid`), so both constructors are infallible —
adding a non-empty-string check by analogy with `CODE_PHRASE` without a
cited invariant to require it would be exactly the kind of unverified claim
`W0.3` exists to catch. `uri` is carried as a plain, unvalidated `String`:
this crate has no BASE-level `Uri` type distinct from the Reference Model's
`DV_URI`, and building one was out of scope for this finding.

Five new tests: construction with and without the optional fields, a
`TerminologyTerm` pairing, and canonical-JSON round-tripping including that
absent optional fields are omitted from the JSON rather than written `null`.

**Not attempted.** `AUTHORED_RESOURCE`, `RESOURCE_DESCRIPTION`,
`RESOURCE_DESCRIPTION_ITEM`, and `TRANSLATION_DETAILS` remain entirely
unmodelled. Whether this crate should model them at all is an open scope
question this finding does not resolve, since `S1.1` does not commit the
crate to the `resource` package the way it does to Data Types, Data
Structures, Common, EHR, and Demographic.

## A-48 — `C_PRIMITIVE_OBJECT.assumed_value` had no field

**Severity: Low. Status: fixed.**

Found in the same AM research pass that produced `A-46`, and ranked below it
there for the reason repeated in this crate's own module documentation:
`assumed_value` — the default a template author or a form generator would
offer when data supplies none — has no bearing on
`validate_against_archetype`'s tree walk, which checks values *present* in a
real instance and never looks at an archetype's own defaults. It remained
unaddressed until now for that reason, not because it was forgotten.

`assumed_value: Any` (`org.openehr.am.aom2.c_primitive_object.adoc`) also
carries its own invariant, `Inv_valid_assumed_value: valid_value
(assumed_value)` — the value must conform to the same node's `constraint`.

**Fixed, with the invariant explicitly not enforced.** `PrimitiveValue`
added — `Boolean`, `Integer`, `Real` (via `base::Real`, not `f64`, for the
`D3.18d` reason), and one `Text` variant standing in for `C_STRING`,
`C_DATE`, `C_TIME`, `C_DATE_TIME`, `C_DURATION`, and `C_TERMINOLOGY_CODE`
alike, the same collapsing `crate::path::Scalar::Str` already makes for the
corresponding `DataValue`s. `CPrimitiveObject::with_assumed_value`/
`assumed_value()` attach and read it. `Inv_valid_assumed_value` is carried
unchecked, the same choice already made for `C_STRING`'s `pattern`
(`A-45`'s residual) — a `Boolean` assumed value attached to a `C_INTEGER`
constraint is accepted exactly as given, not refused. Checking it for real
would need the same per-kind conformance logic `am::validate`'s
`walk_primitive`/`check_temporal` already have, decoupled from the
`Ctx`/reporting machinery those are built around — a larger piece of work
than this finding's own scope, and not attempted here.

Five new tests: a matching-kind assumed value attached and read back; a
mismatched-kind one accepted rather than refused, confirming the
non-enforcement is real and not accidental; canonical-JSON round-tripping,
including that an absent `assumed_value` is omitted rather than written
`null`; and the untagged-enum ordering that lets a whole-number JSON literal
read as `Integer` rather than falling through to `Real`.

**Not attempted.** `Inv_valid_assumed_value` itself, as above. AOM2's other
`C_PRIMITIVE_OBJECT` function, `has_assumed_value()`, is not added — a
caller can already ask `assumed_value().is_some()`, and a second, redundant
accessor would be exactly the kind of duplicated logic this repository's own
history (`lib:A-33`) warns against maintaining in two places.

## A-49 — the ADL header readers used the wrong archetype-identifier grammar

**Severity: Medium. Status: fixed, residual documented.**

Found while implementing `ARCHETYPE_HRID` as its own type (AOM2's
`org.openehr.am.aom2.archetype_hrid.adoc`) and checking it against
`openEHR/adl-antlr`'s real grammar files, rather than assuming the type this
crate already had — `base::ArchetypeId` — was what the header actually
names. It is not. Both `adl14.g4`'s `archetype` rule and `adl2.g4`'s
`authored_archetype` rule name the header's own identifier `ARCHETYPE_HRID`,
a lexer token distinct from and richer than `ArchetypeId`'s grammar:

```text
ARCHETYPE_HRID       : ARCHETYPE_HRID_ROOT '.v' ARCHETYPE_VERSION_ID ;
ARCHETYPE_HRID_ROOT  : (NAMESPACE '::')? IDENTIFIER '-' IDENTIFIER '-' IDENTIFIER '.' LABEL ;
ARCHETYPE_VERSION_ID : DIGIT+ ('.' DIGIT+ ('.' DIGIT+ (('-rc'|'-alpha'|'-beta') ('.' DIGIT+)?)?)?)? ;
```

`ArchetypeId::from_str` accepts neither the optional `namespace::` prefix
nor a prerelease suffix (`-rc.4`, `-alpha`, `-beta`) on the version — both
committed this session in `adl14.rs`/`adl2.rs` (`428dfb2`, `161a193`), both
citing `ARCHETYPE_HRID` in their own error messages without checking the
text actually parsed to that grammar. A real archetype header using either
form would have been refused by a reader whose own module documentation
already named the grammar it should have accepted.

**Fixed** — `openehr::am::ArchetypeHrid` and `VersionStatus` added, parsed
against the grammar above (including its own laxity relative to AOM2's
class-level invariant, `Inv_release_version_validity`, which declares a
strict three-part version the grammar itself does not require — the same
`I2.15`-shaped departure this crate already made for `ArchetypeId`, made
explicit in `ArchetypeHrid`'s own module documentation rather than repeated
silently). `Adl14Header.archetype_id` and `Adl2Header.archetype_id` now hold
an `ArchetypeHrid`; eleven new tests cover the namespace prefix, the three
prerelease suffixes, the departure case, and four refusal shapes. All 13
pre-existing `adl14`/`adl2` tests pass unchanged, since every fixture they
use is plain classic-form text that both grammars accept identically.

**Residual, documented rather than silently left.** `specializes` on both
header types is unchanged — still `Option<ArchetypeId>`. ADL 1.4's
`specialization_section` names its parent with a third, different token,
`ARCHETYPE_REF` (namespace prefix allowed, no prerelease suffix, an
unbounded chain of `.DIGIT+` version segments); ADL 2's own
`specialize_section` allows *either* `ARCHETYPE_HRID` or `ARCHETYPE_REF`
(`cadl2.g4`: `archetype_ref: ARCHETYPE_HRID | ARCHETYPE_REF`).
`ArchetypeId` is closer to `ARCHETYPE_REF` than to `ARCHETYPE_HRID` but is
not exactly either — it lacks the namespace prefix both allow, and caps the
version at three parts where `ARCHETYPE_REF` does not. Reconciling
`ArchetypeId` itself against `ARCHETYPE_REF`, or building a true union type
for ADL 2's `archetype_ref` parser rule, is a separate piece of work: unlike
the header's own identifier, `ArchetypeId` is a `base` type used pervasively
across Reference Model data (`ARCHETYPED.archetype_id`), so widening it
is a larger, more consequential change than adding a new, narrowly-scoped
type was, and was not undertaken in this pass.

## A-50 — `C_COMPLEX_OBJECT` had nowhere to put a tuple constraint

**Severity: Medium. Status: fixed.**

Found while comparing this crate's `am::constraint` module against the full
class list `openEHR/specifications-AM`'s `docs/UML/classes/` directory names,
rather than against the subset of AOM2 this crate already claimed to model.
Several `am.aom2.constraint_model` classes have no counterpart here —
`SIBLING_ORDER`, `C_COMPLEX_OBJECT_PROXY`, `CONSTRAINT_STATUS`, and the
`C_SECOND_ORDER` family among them — and this finding is about one member of
that family specifically: `C_SECOND_ORDER`'s two concrete children,
`C_ATTRIBUTE_TUPLE` and `C_PRIMITIVE_TUPLE`, ranked above the others because
of what they attach to, not because they were the only gap found.

**Why this one, and why it ranks above `SIBLING_ORDER` or `RM_OVERLAY`,
the AOM2 research pass's other candidates.** `C_ATTRIBUTE_TUPLE` is not a rare
corner of the model. AOM2's own second-order-constraints section states
plainly that it "replaces all domain-specific constraint types defined in
ADL/AOM 1.4, including `C_DV_QUANTITY` and `C_DV_ORDINAL`"
(`openEHR/specifications-AM`,
`docs/AOM2/master04.3-constraint_model-second_order.adoc`) — the mechanism
every `DV_QUANTITY` archetype node uses to pair a unit with the magnitude
range that unit implies (`"deg F"` with `32.0..212.0`, `"deg C"` with
`0.0..100.0`, never the two crossed), and every `DV_ORDINAL` node uses to
pair a numeric value with its coded symbol. `DV_QUANTITY` and `DV_ORDINAL`
are two of the most common leaf types in the published archetype corpus.
Without this field, an archetype using the tuple form for either one could
not be represented in this crate's object model at all: not accepted and
checked, not accepted and reported unchecked, not even round-tripped through
JSON — `CComplexObject` had no field to deserialize the constraint into, so
it silently vanished on read, the same silent-loss shape `A-46` and `A-48`
each found in `C_PRIMITIVE_OBJECT`. `SIBLING_ORDER` by contrast only matters
inside a specialised archetype (`K15.11`–`K15.13`, not implemented) — a real
gap, but not reachable by anything this crate can build yet. `RM_OVERLAY` is
reachable today (it is an attribute of `ARCHETYPE` itself, not gated on
templates), but is a gap in a different class — `Archetype`, not
`CComplexObject` — and closing it is not this finding's scope; see **Not
attempted** below.

**Fixed.** `openehr::am::CPrimitiveTuple` and `CAttributeTuple` added to
`am::constraint`, and `CComplexObject::with_attribute_tuples` attaches them —
a builder rather than a `new` parameter, the same choice `A-46` and `A-48`
made for `C_PRIMITIVE_OBJECT`'s own late-added fields, since most callers
never use this attribute and `#[serde(default)]` on the new field keeps
JSON written before this existed readable. One structural invariant is
checked at construction: every row in `CAttributeTuple`'s `tuples` must
supply exactly as many values as `members` names attributes, cited directly
from AOM2's own text — `C_PRIMITIVE_TUPLE.members`' description states "each
member... corresponds to one of the `C_ATTRIBUTEs` referred to by the owning
`C_ATTRIBUTE_TUPLE`" (`openEHR/specifications-AM`,
`docs/UML/classes/org.openehr.am.aom2.c_primitive_tuple.adoc`) — a row naming
three values for a two-attribute tuple has a value with nothing to
correspond to. `CPrimitiveTuple::members` is refused empty for a
narrower reason: AOM2 marks it `1..1` where `C_ATTRIBUTE_TUPLE.members` and
`.tuples` are both `0..1`, and a Rust `Vec` has no `Void` to carry that
distinction, so the mandatory case is translated the way
`ArchetypeTerminology::new` already translates one elsewhere in this module —
refusing empty rather than letting it silently stand in for absent.

`am::validate::walk_complex` now visits `CComplexObject::attribute_tuples`
and reports each one `Unchecked`, naming the attributes it covers, rather
than the two other options: silently ignoring it (what happened before this
finding, by omission) or silently treating it as satisfied (what `K15.20`
forbids). Checking it for real would mean picking the one `C_PRIMITIVE_TUPLE`
row whose values match the instance's actual values across every named
attribute *at once* — a different shape of check than `walk_attribute`'s
per-attribute walk, and a larger piece of work than this finding's own scope.

Eight new tests: the units/magnitude example from AOM2's own
second-order-constraints document built and read back; the arity-mismatch
invariant refused, naming the reason; the `1..1` empty-members case refused;
the `0..1` empty-tuples case accepted; the builder's default (absent unless
attached) checked through both `CComplexObject` and `CObject`;
canonical-JSON round-tripping; and a fixture written as though from before
this field existed — literal JSON with no `attribute_tuples` key at all —
still deserializing, confirming `#[serde(default)]` is doing real work and
not merely present. `am::validate` gained one more: the units/magnitude
example wired into a real `EVALUATION`/`ELEMENT`/`DV_QUANTITY` walk,
confirming the report names
`"C_ATTRIBUTE_TUPLE co-varying constraint is not evaluated"` with
`"units, magnitude"` as the detail, and that no violation is raised for data
the tuple was never consulted against.

**Not attempted.** `C_ARCHETYPE_ROOT` does not gain the field, though AOM2
declares it a `C_COMPLEX_OBJECT` subtype and so inherits `attribute_tuples`
formally. This crate's own `CArchetypeRoot` already carries the matching
asymmetry for `attributes` — always empty, with no builder to populate it,
matching AOM2's own note that "In all uses within source archetypes and
templates, the `_children_` attribute is `Void`" (`openEHR/specifications-AM`,
`docs/UML/classes/org.openehr.am.aom2.c_archetype_root.adoc`) — so adding
`attribute_tuples` there alone, with no way to populate `attributes` either,
would model an inheritance relationship this crate does not otherwise
represent. `SIBLING_ORDER` remains unmodelled and, unlike this finding, is
genuinely gated on specialisation: its own class documentation states it
applies only "on a `C_OBJECT` within a container attribute in a specialised
archetype", which is `K15.11`–`K15.13`, not implemented. `RM_OVERLAY` is a
different shape of gap and this finding does not close it either: it is an
optional attribute of `ARCHETYPE` itself, the same level as `rules`
(`EXPR_CONSTRAINT`, also unmodelled) — reachable today, not gated on
specialisation or templates — and `Archetype` in `am::archetype` has no field
for either one. That is a separate, `ARCHETYPE`-level gap, not a
`C_COMPLEX_OBJECT`-level one, and is left for its own finding rather than
folded into this one. Actually evaluating a tuple constraint during
validation — matching an instance's values to one row — is deferred for the
same reason `Inv_valid_assumed_value` was in `A-48`: a larger piece of work,
decoupled from `walk_complex`'s existing per-attribute shape, and not
attempted here.

## A-51 — a soft terminology constraint was checked as though it were required

**Severity: Medium. Status: fixed, residual documented.**

Found while reading `C_TERMINOLOGY_CODE`'s own primary source
(`openEHR/specifications-AM`,
`docs/UML/classes/org.openehr.am.aom2.c_terminology_code.adoc`) to compare
this crate's `CPrimitive::TerminologyCode` variant against it — the same
per-class comparison that found `A-50` — rather than assuming the variant
already committed to this repository's history was complete because it had a
name and a citation-free existence.

**What was missing, and what it did in the meantime.** AOM2's
`C_TERMINOLOGY_CODE` carries a `constraint_status` attribute
(`CONSTRAINT_STATUS`: `required`, `extensible`, `preferred`, `example`) that
this crate had no field for at all. `openEHR/specifications-AM`,
`docs/ADL2/master04.5-cadl_primitive_types.adoc` states plainly what the
three non-`required` values mean: "Formally, all three of these statuses are
the same as a value constraint specifying only the RM type as being a
terminology code... which is to say, at the archetype level, validity of the
data instance is achieved by supplying *any terminology code*." An
`extensible`, `preferred`, or `example` terminology constraint is satisfied
by any coded value whatsoever — the whole reason ADL offers the three
statuses is to let an archetype suggest a value set without binding the
instance to it. With no field to carry `constraint_status`, `am::validate`'s
`walk_primitive` had no way to know a constraint was soft, and checked every
`C_TERMINOLOGY_CODE` as though it were `required`: a real archetype using
`extensible [ac2]` — the openEHR specification's own recommended pattern for
handling terminology gaps in a novel condition, named explicitly in
`master04.5-cadl_primitive_types.adoc`'s own soft-constraint section — would
have every conformant instance whose code is not already in the value set
reported as a `C_TERMINOLOGY_CODE` violation. That is a false verdict on
conformant clinical data, the same class of defect `A-01` found in three
quantity rules, not merely an absent field.

**Fixed.** `openehr::am::ConstraintStatus` added — `Required`, `Extensible`,
`Preferred`, `Example` — with `is_required()` mirroring AOM2's own
`constraint_required()` for the non-`Void` case (`Void`/`None` is read as
`Required` at the call site, matching AOM2's own stated default "in a
top-level archetype", the only kind this crate builds). `CPrimitive
::TerminologyCode` gained a `constraint_status: Option<ConstraintStatus>`
field, additive and `#[serde(default)]` so JSON written before this existed
still deserializes. `walk_primitive`'s `C_TERMINOLOGY_CODE` arm now checks
`constraint_status` first: anything but `Required` skips the `code_list`/
`ac`-code check entirely rather than reporting a violation, matching AOM2's
own stated semantics exactly rather than approximating them — and, per that
same semantics, is not reported `Unchecked` either, since a soft constraint's
outcome is fully determined (always satisfied by any code), not merely
unevaluated.

Four new tests: `ConstraintStatus::is_required` true only for `Required`;
canonical-JSON round-tripping including the field's omission when absent;
a fixture written as though from before `constraint_status` existed —
literal JSON with no such key — still deserializing; and, in `am::validate`,
an `extensible` constraint naming one code accepting an instance carrying a
completely different one, with neither a violation nor an unchecked entry,
confirming the fix is a real behavioural change to `walk_primitive` and not
merely a field that round-trips.

**Not attempted — the residual named in `CPrimitive::TerminologyCode`'s own
module documentation.** AOM2's `constraint` attribute is a single `String` —
one `at`-code, or one `ac`-code naming a value set — never a list; ADL's own
way of offering several alternative codes is several sibling `C_OBJECT`s
under one attribute, the same alternative-matching shape every other node
kind already uses via `CAttribute::children`. This variant's own
`code_list: Vec<String>` field has no counterpart in AOM2 at all, was not
re-derived from the primary source when it was first written, and predates
this pass. Correcting the shape — most likely removing `code_list` in favour
of the sibling-alternative pattern — is a breaking change to a type that
shipped in a published version (`openehr` 0.7.0 brought the Archetype Model
into scope), and deciding how to land a breaking AM change is `agents
/publishing.md`'s process, not a decision this finding makes unilaterally.
Recorded here rather than fixed silently or left for a future reader to
rediscover (`C0.9`).

## A-52 — `ARCHETYPE.rm_overlay` did not exist

**Severity: Low. Status: fixed.**

Found immediately after `A-51`, reading `ARCHETYPE`'s own class definition
(`openEHR/specifications-AM`,
`docs/UML/classes/org.openehr.am.aom2.archetype.adoc`) while checking the
`Archetype` struct against it attribute by attribute rather than assuming
the four attributes this crate already modelled — `archetype_id`,
`parent_archetype_id`, `definition`, `terminology` — were the whole class.
`rm_overlay: RM_OVERLAY` (`0..1`) had no counterpart at all: `RM_OVERLAY`,
`RM_ATTRIBUTE_VISIBILITY`, and `VISIBILITY_TYPE` did not exist in this crate
under any name, so an archetype declaring visibility or an alias for an
RM attribute outside its constrained structure would lose that information
silently on JSON read, the same shape of loss `A-46` and `A-50` each found —
`Archetype` had no field to deserialize it into.

**What this is not.** `rm_overlay` carries no conformance meaning: hiding an
attribute from an authoring tool, or aliasing it, does not change whether an
instance conforms, and `org.openehr.am.aom2.rm_overlay.adoc` names no
invariant connecting the two. This is authoring-tool metadata, and
`am::validate::validate_against_archetype` does not read it — the new
module's own documentation says so before it says anything else, the same
discipline `am::validate`'s own module header already applies to what it
does and does not check.

**Fixed.** `openehr::am::RmOverlay`, `RmAttributeVisibility`, and
`VisibilityType` added in a new `am::rm_overlay` module. `RmAttributeVisibility
::new` checks AOM2's own `Inv_alias_validity` (`alias /= Void implies
visibility /= Void`) — an alias with no stated visibility names an attribute
without saying anything a tool can act on. `RmAttributeVisibility::alias` is
typed `openehr::base::TerminologyCode`, matching AOM2's own attribute type
exactly rather than approximating it with a bare `String` — the first
consumer of the type `A-47` added for exactly this reason, since `A-47`
itself noted the three classes it was building the type *for*
(`AUTHORED_RESOURCE`, `RESOURCE_DESCRIPTION_ITEM`, `TRANSLATION_DETAILS`)
remained unmodelled; `RM_ATTRIBUTE_VISIBILITY` reaches it first.
`Archetype::with_rm_overlay` attaches one, `#[serde(default,
skip_serializing_if = "Option::is_none")]` keeping JSON written before this
existed both readable and unchanged in shape when the field is unused.

Six new tests: `Inv_alias_validity` refused and then satisfied once a
visibility accompanies the same alias; a visibility with no alias needing no
alias check at all; one overlay carrying two independent path statements;
canonical-JSON round-tripping; the builder's default (absent unless
attached) on `Archetype` itself; and a fixture written as though from before
`rm_overlay` existed — an `Archetype`'s own JSON with no such key — still
deserializing.

**Not attempted.** `rm_visibility`'s path keys are carried as written and not
checked against `crate::path::Node` or resolved in any way — the class's own
description says a path may be "at deeper non-constrained RM paths from an
object or the root", which by definition are paths this crate's own
constraint tree does not describe, so there is nothing in an `Archetype` to
validate a path against even in principle.

**A residual noticed in passing, checked in the next pass and cleared.**
`parent_archetype_id` is typed `Option<ArchetypeId>` here, but AOM2's own
attribute is a bare `String`, described as "may take the form of an
archetype interface identifier, i.e. the identifier up to the major version
only, or may be a full archetype identifier". Checked against
`openEHR/adl-antlr`'s own grammar (`src/main/antlr/adl/base_lexer.g4`):
`ARCHETYPE_REF: ARCHETYPE_HRID_ROOT '.v' INTEGER ('.' DIGIT+)*` — every form
of an archetype reference, "interface" included, requires `.v` followed by
at least the major version digit; there is no version-less form to be
narrower than. "Up to the major version only" means exactly what
`ArchetypeId::from_str` already accepts — one to three numeric version
components, one being the minimum — so `parent_archetype_id`'s typing is not
a new defect. The one real gap in the same identifier is already on record:
`ARCHETYPE_HRID_ROOT` allows an optional `NAMESPACE '::'` prefix
`ArchetypeId::from_str` does not, which is `A-49`'s own residual, not a new
one this parent-identifier check adds.

## A-53 — `C_COMPLEX_OBJECT_PROXY` did not exist under any name

**Severity: Medium. Status: fixed, residual documented.**

Found continuing the same per-class sweep of `am.aom2.constraint_model`
`A-50` started: `SIBLING_ORDER`, `CONSTRAINT_STATUS`, and
`C_COMPLEX_OBJECT_PROXY` were the classes that finding named as having no
counterpart here. `CONSTRAINT_STATUS` is closed — `A-51`'s own
`ConstraintStatus` is exactly it, confirmed by checking that AOM2 uses the
enumeration nowhere but `C_TERMINOLOGY_CODE.constraint_status`, so nothing
further is missing on that account. This finding is the second of the
remaining two, and the only one of them buildable without specialisation
support this crate does not have — `SIBLING_ORDER` remains open for the
reason `A-50` already gave it.

**What was missing.** `C_COMPLEX_OBJECT_PROXY` is `C_OBJECT`'s fourth
concrete descendant alongside `C_COMPLEX_OBJECT`, `C_PRIMITIVE_OBJECT`, and
`ARCHETYPE_SLOT` — a node that, instead of stating its own constraint tree,
names another node in the same archetype by path and means "the same
constraint as that one". `openEHR/specifications-AM`,
`docs/UML/classes/org.openehr.am.aom2.c_complex_object_proxy.adoc`: "A
constraint defined by proxy, using a reference to an object constraint
defined elsewhere in the same archetype." With no counterpart at all, an
archetype using a proxy node — the mechanism a real archetype uses to avoid
repeating an identical subtree, the same economy `C_ATTRIBUTE_TUPLE` gives
co-varying constraints — could not be represented, checked, or round-tripped
through JSON: `CObject` had no variant to deserialize it into, the same
silent-loss shape `A-50` found for `attribute_tuples`.

**Fixed.** `openehr::am::CComplexObjectProxy` added as a new `CObject::Proxy`
variant — `#[non_exhaustive]` on `CObject` means an external caller matching
only the four existing variants keeps compiling, by design (see the enum's
own documentation on that point), but every exhaustive match inside this
crate itself needed a new arm, and got one: `rm_type_name`, `node_id`,
`occurrences`, `attributes`, `attribute_tuples`, and `am::validate`'s own
`walk_object` dispatch. A node governed by a proxy is reported `Unchecked`,
naming `target_path`, rather than resolved — resolving it means looking up
another node in the *same archetype's own constraint tree* by archetype
path, a capability distinct from anything `crate::path::Node` does (that
walks Reference Model *data*, not an `am::constraint` tree) and not
attempted here.

**Not attempted — the residual named in `CComplexObjectProxy`'s own module
documentation.** AOM2 lets a proxy's `occurrences` be `Void`, meaning
`use_target_occurrences()`: use the target node's own occurrences instead of
stating any locally, the class's second defining feature alongside
`target_path` itself. Representing that needs `CObject::occurrences()` to
return `Option<&MultiplicityInterval>` rather than `&MultiplicityInterval`,
and every one of the four existing variants already commits to the
non-`Option` shape as published API. Changing it is the same kind of
breaking change `A-51` declined to make to `code_list` without going through
`agents/publishing.md`'s process, and is declined here for the same reason:
`CComplexObjectProxy::occurrences` is required, not optional, so a proxy
that would defer to its target cannot be built with this type as it stands.
Two open, undecided breaking changes to the same enum family — this one and
`A-51`'s — are now on record together rather than each looking like an
isolated shape choice.

Four new tests: an empty `target_path` refused; `rm_type_name`/`node_id`/
`occurrences`/`attributes`/`attribute_tuples` all reachable identically
through `CObject::Proxy` as they are through every other variant;
canonical-JSON round-tripping; and, in `am::validate`, a proxy governing a
real `ELEMENT` reported `Unchecked` with its `target_path` as the detail,
never as a silent pass.

## A-54 — `CObject::occurrences()` could not represent a deferred value

**Severity: Low. Status: fixed.** Closes `A-53`'s own residual.

**Deciding, rather than deferring again.** `A-51` and `A-53` each declined a
breaking fix to already-published API and recorded the decision as a
residual instead of making it — the same choice twice in a row, which is
itself worth noticing: leaving both open indefinitely is itself a decision,
just an unstated one. `agents/publishing.md`, read in full for the first
time this pass rather than assumed to gate source changes as well as
releases, turns out to describe only release *mechanics* — version tables,
publish ordering, a checklist — not a design-approval process, and this
repository's own history (`0.6.0`'s `f64`→`Real` migration, `0.4.0`'s
`PartialOrd` removal) already establishes that a breaking API change is an
ordinary commit here, version-bumped only when someone actually runs
`cargo publish`. That act needs credentials and manual confirmation neither
of these findings required; making the fix did not.

**The fix.** `CObject::occurrences()` now returns `Option<&MultiplicityInterval>`.
The four existing variants — `CComplexObject`, `CPrimitiveObject`,
`ArchetypeSlot`, `CArchetypeRoot` — are unaffected in every other respect:
their own struct fields stay `MultiplicityInterval`, not `Option`, and the
dispatcher simply wraps each in `Some`. Only `CComplexObjectProxy::occurrences`
itself becomes genuinely `Option<MultiplicityInterval>`, `None` now meaning
AOM2's own `use_target_occurrences()` — the residual `A-53` named. Two
construction-time checks in `CAttribute::single`/`container` read
`child.occurrences()` and needed a rule for a deferred child: AOM2 states one
directly, in `C_OBJECT.effective_occurrences()`'s own text — "If local
`occurrences` not set, always assume 0 as the lower bound" — cited and
applied to the `required`-count sum in `container`; the upper-bound checks in
both functions (`VACSO`, `VACMCU`) exclude a deferred child entirely, since
its effective upper bound depends on a target this crate does not resolve
and guessing one would be exactly the kind of unverified claim `W0.3` exists
to catch. `am::validate::walk_attribute`'s own occurrences check gained the
matching case: a deferred alternative is reported `Unchecked`, naming
`use_target_occurrences`, rather than causing a `match` to not compile or a
panic to reach a caller.

Two new tests: a proxy built with `None` occurrences reads
`use_target_occurrences() == true` and `occurrences() == None` through both
`CComplexObjectProxy` and `CObject`; and, in `am::validate`, a proxy with
deferred occurrences governing one matched instance node is unchecked
*twice* — once from `walk_object`'s own per-node handling (`A-53`), once
from the separate alternative-occurrences loop — confirmed as two distinct,
correctly-worded reports rather than one masking the other.

## A-55 — `CPrimitive::TerminologyCode::code_list` had no AOM2 counterpart

**Severity: Low. Status: fixed.** Closes `A-51`'s own residual, made for the
reason `A-54`'s own opening paragraph gives.

**The fix.** `code_list: Vec<String>` removed. `constraint: Option<String>`
now carries either kind of code AOM2's own single-valued `constraint`
attribute can — a required `at`-code or a value-set `ac`-code — distinguished
at check time by AOM2's own leader convention, `is_value_set_code`:
`a_code.starts_with (Value_set_code_leader)`, `Value_set_code_leader = "ac"`
(`openEHR/specifications-AM`,
`docs/UML/classes/org.openehr.am.aom2.adl_code_definitions.adoc`), not by
which of two Rust fields a caller happened to populate. `am::validate`'s
`C_TERMINOLOGY_CODE` arm was rewritten around that distinction: an `ac`-code
checks value-set membership as before; an `at`-code now checks exact
equality with the instance's own coded value, a check `code_list` never
correctly performed in the first place (it checked *membership in a list*,
which happened to coincide with equality only for a single-element list).
`archetype.rs`'s own `VACDF` sweep (`terminology_constraints`) is narrowed to
match: it used to treat every non-empty `constraint` as an `ac`-code needing
a value set, which was safe only because `constraint` was never anything
else; now that an `at`-code can appear there too, the sweep filters on the
same `"ac"` leader before demanding a value set for it, so an `at`-code
constraint is no longer wrongly refused for lacking one.

`AOM2`'s own convention for "no constraint" is an empty string, not `Void`
— this crate keeps translating that to `None` for the idiomatic reason
`constraint: Option<String>` already gave, but now also accepts `Some("")`
as the same case, defensively: `Deserialize` does not run this crate's own
constructors, so a foreign or hand-written payload may use either spelling
and both must be read the same way.

**Multiple alternative codes are unaffected in what they can express, only
in how.** `code_list: ["at1", "at2"]` on one node is now two sibling
`CObject::Primitive` alternatives under the same `C_ATTRIBUTE`, each with
its own single-code `constraint` — the shape `CAttribute::children` already
gives every other node kind for exactly this purpose (`A-50`'s own tuple
alternatives are a second instance of the same pattern). No expressive power
is lost; what changes is which part of the tree carries the alternation.

Two new tests: an `at`-code constraint checked for exact equality, both
directions (the matching code passes, a different one violates); and
`constraint: None` and AOM2's own `Some(String::new())` spelling both
reported unchecked with the same reason, proving the two are read as one
case rather than one being silently favoured.

## A-56 — `Inv_valid_assumed_value` was never checked

**Severity: Low. Status: fixed.** Closes `A-48`'s own residual.

**Why this one, now.** `A-48` deferred `Inv_valid_assumed_value` — AOM2's
own validity condition that `C_PRIMITIVE_OBJECT.assumed_value` conforms to
the same node's `constraint` — reasoning that "nothing calls this before a
template author or a form generator would, and neither exists in this
crate." That reasoning was about *runtime* consumers of an assumed value,
which is correct and remains true; it did not consider that the archetype
*itself* can be checked for internal consistency independent of any
consumer, the same way `VARDT`/`VATDF`/`VACDF`/`VATCD` already are. An
archetype whose own stated default cannot satisfy its own stated constraint
is malformed on its own terms, whether or not anything downstream ever asks
for the default.

**Where the check lives, and why not at the leaf.** `CPrimitiveObject
::with_assumed_value` builds one node in isolation and has no terminology in
scope — `Inv_valid_assumed_value` for a `C_TERMINOLOGY_CODE` naming an
`ac`-code needs the archetype's own value set, which only exists once a
`CComplexObject` and an `ArchetypeTerminology` are assembled together. So
the check is in `Archetype::check` (`am::archetype`), the same place
`VACDF` already needed the terminology for the same reason.
`CPrimitiveObject::with_assumed_value` itself is unchanged — still
infallible, still carries a mismatched value exactly as given, exactly as
its own documentation already said — because that is correct in isolation,
not merely undone.

**The check itself.** `assumed_value_conforms` — AOM2's own `valid_value`
— matches `(CPrimitive, PrimitiveValue)` pairs: `Boolean` against
`allow_true`/`allow_false`; `String` against `list` (empty means
unconstrained, matching every other kind's own convention); `Integer`/`Real`
against `list`/`range`, `Real` compared by `semantic_cmp` for the `D3.18d`
reason every other numeric comparison in this crate already gives;
`Terminology_code` against an exact `at`-code or, for an `ac`-code, the
terminology's own value set (`A-55`'s `"ac"`-leader convention, reused
rather than re-derived); the four temporal kinds by parsing
`PrimitiveValue::Text` via the relevant `base` type's own `FromStr` and
checking it against `range`. A kind mismatch — `PrimitiveValue::Boolean`
paired with `CPrimitive::Integer`, say — does not conform, full stop:
AOM2's `valid_value` has no notion of "close enough".

**`C_UNSUPPORTED` is excluded, not guessed at.** This crate cannot interpret
a constraint kind it does not model, so it makes no claim about whether an
assumed value conforms to one — neither "passes" (which would be
unverified) nor "fails" (which would refuse an archetype for a reason this
crate cannot actually establish). The same reasoning `VASID`/`VACSD` already
state for the two conditions this crate cannot check at all.

Five new tests: a matching, in-range assumed value accepted; an
out-of-range one refused, naming `Inv_valid_assumed_value`; a
kind-mismatched one — `Boolean` on `C_INTEGER` — refused at the archetype
even though `CPrimitiveObject` alone still carries it unchecked, confirming
the two levels genuinely disagree on purpose rather than one silently
overriding the other; a `Terminology_code` assumed value checked against a
real value set, both directions; and a `C_UNSUPPORTED` constraint's assumed
value building a conformant archetype regardless of what the value actually
is, confirming the exclusion is real rather than an accidental pass.

## A-57 — a side-effecting `debug_assert!` broke header parsing in release builds

**Severity: High. Status: fixed.**

**Found by the release process itself, not by a research pass.** Preparing
0.9.0 for publication, `agents/publishing.md`'s own checklist — `cargo test`
and `cargo clippy` clean, then CI on the actual commit — ran green on
everything except one job: `benchmarks still run / openehr`
(`cargo bench --benches -- --test`, `W0.35`'s smoke-test-only run). Two
tests failed there and nowhere else: `am::adl2::tests
::reads_meta_data_and_specialisation` and `am::adl14::tests
::skips_archetype_header_metadata`, both refusing a well-formed `(adl_version
=2.4.0; ...)` `meta_data` clause as `"unterminated (...) metadata"`.
Reproduced locally with the identical command before touching any code, per
`W0.3` — the failure was real, not a shared-runner artefact of the kind
`agents/publishing.md` already warns benchmark jobs can produce.

**The bug.** `adl_lexer::Lexer::skip_parenthesised` — used by both ADL 2 and
ADL 1.4's header readers (`am::adl2::parse_header`, `am::adl14::parse_header`,
`A-49`) to skip an optional `meta_data` block — opened with:

```rust
debug_assert!(matches!(self.next(), Some(Token::Symbol('('))));
```

`self.next()` advances the lexer — a side effect this function's own
correctness depends on, consuming the opening `(` before the loop below
starts counting nested parens. `debug_assert!`'s argument **is not evaluated
at all** when `debug-assertions` is off, which `cargo bench`'s profile — like
`cargo build --release`, and like any downstream consumer's own release
build — sets by default. In that build, `self.next()` is never called, the
opening `(` is never consumed, and the loop reads it again as a *nested*
paren — one extra level of depth with no closing paren to match it, so a
perfectly well-formed `meta_data` clause is refused as unterminated.

**Why nothing had caught it.** `cargo test` — every invocation of it in this
repository's own history, and CI's own `test` job — always builds in the
`dev` profile, where `debug_assert!` is live and the bug is invisible: the
side effect happens, the assertion passes, the function works. Only a
release-profile build exercises the broken path, and the only CI job that
builds one is `benchmarks still run`, which `W0.35`/`W0.36` deliberately do
not gate merges on ("run, never gated on") — so this had been red, unnoticed,
since the header readers were first added (`428dfb2`, `161a193`), through
every commit and every green `test` run since.

**Severity.** Both header readers are the only ADL entry points this crate
has (`K15.5`/`K15.8`, both still otherwise unimplemented, `A-40`), and
`meta_data` is not a rare clause — real published archetypes carry an
`adl_version`/`uid` block routinely (`openEHR-EHR-CLUSTER.device.v1.0.0.adls`,
cited in `am::cadl`'s own tests, opens with exactly this shape). A caller
building this crate into a release binary — the normal way to ship Rust —
would have every such header refused, silently, with no `test`-profile
signal ever suggesting a problem. Rated `High`: a defeated control, not a
gap in coverage — the function's own doctests and unit tests all pass, and
still do, in the one profile that does not exercise the bug.

**Fixed.** The opening token is consumed into a binding unconditionally;
`debug_assert!` checks only the already-computed value, which has no side
effect of its own to elide:

```rust
let opening = self.next();
debug_assert!(matches!(opening, Some(Token::Symbol('('))));
```

Verified both ways: `cargo bench --benches -- --test` (release profile) and
`cargo test` (dev profile) both pass, 389 of 389, where before the fix the
former failed exactly the two tests above and the latter passed all along —
confirming the fix closes the actual gap between the two rather than
changing behaviour either build already had right.

**No residual.** `grep -rl debug_assert --include="*.rs"` across all eighteen
crates returns exactly one file: `adl_lexer.rs`, the one fixed here. This
was not one instance of a pattern repeated elsewhere — it was the only
`debug_assert!` in the entire tree.

## A-58 — a `C_ATTRIBUTE_TUPLE` was carried but never evaluated

**Severity: Low. Status: fixed.** Closes `A-50`'s own residual.

**The gap.** `A-50` gave `CComplexObject` somewhere to put a
`C_ATTRIBUTE_TUPLE`/`C_PRIMITIVE_TUPLE` constraint and made `walk_complex`
visit it — but only to report it `Unchecked`, unconditionally, naming the
attributes it covers. Nothing resolved the instance's actual `units` and
`magnitude` (or `value`/`symbol`) and asked whether any permitted row
accepted them together. `A-50`'s own text named this explicitly as deferred:
"a larger piece of work, decoupled from `walk_complex`'s existing
per-attribute shape, and not attempted here." Left as it stood, a
`C_ATTRIBUTE_TUPLE` constraint could not fail — every instance, conformant or
not, produced the same unchecked report, which `K15.20` requires this crate
to never let read as a pass.

**Fixed.** `walk_complex` now calls a new `walk_attribute_tuple` for each
`CAttributeTuple` it visits. It:

1. Resolves each co-varying attribute (`tuple.members()`) to exactly one
   instance value via `Node::children`. An attribute that is not
   single-valued in the instance — absent, or repeating — cannot be resolved
   to one value to compare, so the whole tuple is reported `Unchecked`
   there, naming the unresolved attribute's own path and how many values it
   actually had. AOM2's own tuple examples (`units`/`magnitude`,
   `value`/`symbol`) are always single-valued attributes; this is not a
   shape the check guesses at.
2. Evaluates every row's every column by calling `walk_primitive` itself —
   the same function `walk_attribute` already uses for an ordinary
   `C_PRIMITIVE_OBJECT` — against a scratch `Ctx` whose result is inspected
   and discarded, rather than re-implementing the six primitive kinds'
   comparison rules a second time. `lib:A-33` is the standing reason a rule
   stays in exactly one place in this crate; a tuple's row is nothing more
   than several ordinary primitive constraints evaluated together.
3. Combines each column's three-valued outcome (`Conforms`/`Violates`/
   `Unchecked` — a new, function-local `TupleVerdict`, not `ConstraintStatus`,
   which is AOM2's own carried-in-the-archetype status rather than a computed
   verdict) into a row verdict by AND (`Violates` anywhere wins; `Unchecked`
   anywhere else keeps the row open), then into a tuple verdict by OR
   (`Conforms` anywhere wins; `Unchecked` anywhere else keeps the tuple open).
   Only when every row definitely violates does the tuple itself become a
   violation — the instance's values do not match any permitted combination.
4. An empty `tuples` list (AOM2 declares it `0..1`, so absence is legal) has
   no row to compare against at all, and stays `Unchecked` rather than
   becoming a guessed `Conforms` or `Violates` — this crate has no stated
   reading of what a tuple naming zero rows means for conformance, and is
   not inventing one here.

**Tests.** Four in `am::validate`, replacing the single always-unchecked test
`A-50` added: the zero-rows edge case (renamed
`an_attribute_tuple_with_no_rows_is_unchecked`, asserting the new message and
detail — the old test's doc comment described the pre-fix "nothing walks
into a tuple's own rows" behaviour, which this fix makes false in general,
true only for this specific degenerate input); a real matching row
(`an_attribute_tuple_matching_a_row_is_conformant`, `140 mm[Hg]` against
`mm[Hg]`/`kPa` rows — no violation, nothing unchecked); a real non-matching
set of rows (`an_attribute_tuple_matching_no_row_is_a_violation`, `120
cm[H2O]` against the same two rows — exactly one violation naming
`C_ATTRIBUTE_TUPLE`); and an unresolvable column
(`an_attribute_tuple_column_that_does_not_resolve_is_unchecked`, an
`accuracy` attribute `DV_QUANTITY` has no children for — unchecked, naming
the column's own path and `"0 value(s)"`).

**No residual.** Every branch `walk_attribute_tuple` can take — unresolvable
column, definite match, definite non-match, and the zero-rows edge case — has
a test exercising it.

## A-59 — `ARCHETYPE_SLOT.is_closed` had no counterpart

**Severity: Low. Status: fixed.**

Found while comparing this crate's `am::constraint` module against
`org.openehr.am.aom2.archetype_slot.adoc` directly, the same method `A-50`,
`A-52`, and `A-53` each used to find their own gaps, rather than assuming
this crate's existing `ArchetypeSlot` — added before this pass, and already
carrying `includes`/`excludes` — was complete because it had a name and two
of its three fields.

**The gap.** AOM2 declares `ARCHETYPE_SLOT` with three attributes —
`includes: List<ASSERTION> [0..1]`, `excludes: List<ASSERTION> [0..1]`, and
`is_closed: Boolean [1..1]` — plus one function, `any_allowed(): Boolean`,
`"True if no constraints stated, and slot is not closed."` This crate's
`ArchetypeSlot` had the first two and neither the third nor the function. An
archetype closing a slot — `"closed to further filling either in further
specialisations or at runtime"`, the class's own words for `is_closed` —
could not say so under any representation: not accepted and checked, not
accepted and carried unchecked, not even round-tripped through JSON, the
same silent-loss shape `A-46`/`A-48`/`A-50`/`A-52` each found in a different
class.

**Fixed.** `is_closed: bool` added, defaulting `false` — AOM2's own stated
default, so a slot left alone stays open, matching every existing fixture
and caller without change. `closed()` is a builder, one-directional like
`CComplexObjectProxy`'s own occurrence builder: there is no `open()`,
because `false` is already what building a slot and calling nothing extra
produces. `is_closed()` reads it back, and `any_allowed()` is `AOM2`'s own
function verbatim: `includes.is_empty() && excludes.is_empty() &&
!is_closed`. `#[serde(default)]` on the new field keeps JSON written before
this pass existed readable, the same choice `A-46`/`A-48`/`A-50` made for
their own late-added fields.

**Not enforced, and not able to be from where this crate currently
validates.** `am::validate::walk_object`'s existing `CObject::Slot` arm
reports every slot `Unchecked` regardless — added before this finding, for
a reason `is_closed` does not touch: which archetype, if any, fills a slot
is recorded on the *instance's* own `ARCHETYPED.archetype_id`, and
`crate::path::Node` — the interface every walk function in `am::validate`
reads instance data through — does not expose it at all. `is_closed`
answers a different question (is filling permitted here, at authoring or
specialisation time) than a filler check would (what, if anything, actually
filled it, at instance-validation time), so even resolving the `Node` gap
would not make this field enforceable by itself; the two are separate
pieces of work. Carrying it unchecked, rather than not representing it at
all, is the same choice `A-48` made for `assumed_value` before `A-56`
checked its own invariant three findings later.

**Tests.** Four, in `am::constraint`: the default-open case and each of the
three ways `any_allowed()` can become false — `closed()`, one inclusion
assertion, one exclusion assertion — checked separately, since folding them
into one assertion would hide which restriction actually flipped the
result; canonical-JSON round-tripping with the field set; and a fixture
written as though from before `is_closed` existed — literal JSON with no
such key at all — still deserialising, and reading as the AOM2-stated
default.

## A-60 — `ARCHETYPE_SLOT` could state `is_closed` and still never catch a violation of it

**Severity: Medium. Status: fixed.**

**The gap.** `A-59` gave `ArchetypeSlot` an `is_closed` field and its
`any_allowed()` function, but its own "Not enforced" section was explicit
about why that alone changed nothing observable: `am::validate::walk_object`
reports every `CObject::Slot` `Unchecked`, unconditionally, because which
archetype — if any — fills a slot is recorded on the *instance's* own
`ARCHETYPED.archetype_id`, and `crate::path::Node`, the interface every walk
function in `am::validate` reads instance data through, exposed no way to
read it. A closed slot the instance data filled anyway — the one case
`is_closed` exists to forbid — produced exactly the same report as an empty
one: `Unchecked`. Rated `Medium`, not `Low` like `A-59` itself, because this
is not a missing representation but a missing *check*: real non-conformant
data (a closed slot filled regardless) passed through
`validate_against_archetype` reported no differently from data this crate
genuinely cannot evaluate, which is the same understating-of-`false`
direction `A-45` was rated `Medium` for.

**Fixed, in two parts.**

1. **`Node::archetype_details()`**, in `crate::path`, added alongside the
   existing `Node::archetype_node_id()` it is modelled on — same match arms,
   same reasoning about which `Node` variants can answer and which cannot
   (a bare data value never can; `Locatable::archetype_details` already
   states `None` for the same reason on the RM side). `Entry`,
   `ItemStructure`, and `Event` — the three `Node` variants that wrap an enum
   rather than a `Locatable`-implementing struct directly — gained their own
   inherent `archetype_details()` dispatcher next to the `locatable()`
   dispatcher they already had, rather than routing through
   `.locatable().archetype_details()` as the existing `archetype_node_id()`
   code does: `archetype_details` is a [`Locatable`]-*provided* trait
   method, with no inherent counterpart on `LocatableAttrs` the way
   `archetype_node_id` has, so going through `.locatable()` does not resolve
   it. Caught by the compiler, not by a review — `E0599`, three times, one
   per enum.
2. **`walk_object`'s `CObject::Slot` arm** replaced with a new
   `walk_slot`, which resolves what filled the position (if anything) via
   the new accessor and reasons over the three constraints AOM2 actually
   states, without parsing a single assertion:
   - No filler at all (`archetype_details()` is `None`): the slot was left
     open. `is_closed`, `includes`, and `excludes` all restrict what may
     fill a slot; none of them restricts leaving it unfilled, which is
     occurrences' own job, checked separately by the attribute the slot
     sits under. Nothing to violate and nothing to leave unchecked.
   - A filler is present and `is_closed()`: definite violation, naming
     `ARCHETYPE_SLOT.is_closed`, regardless of what the filler is or
     whether `includes`/`excludes` might otherwise have allowed it — a
     closed slot forbids filling outright, so presence alone settles it.
   - A filler is present and `any_allowed()` (open, nothing stated):
     definite pass — an unrestricted slot's filler needs no further check,
     whatever it turns out to be.
   - A filler is present, open, but `includes` and/or `excludes` name an
     assertion: `Unchecked`, naming the filler's own archetype id as
     detail — this crate does not parse `ASSERTION` expressions (`K15.10`),
     so whether *this* filler actually satisfies *this* assertion remains
     genuinely unknown, and stays reported that way rather than guessed at
     either direction.

**Not attempted.** `includes`/`excludes` themselves remain unparsed strings;
this finding does not touch `K15.10`. A slot whose only restriction is
`is_closed` — the one AOM2 attribute expressible without any assertion
grammar at all — is now checked exactly as fully as `any_allowed()` already
let a fully open slot be; a slot restricted by `includes`/`excludes` is no
better checked than before `A-59`, only correctly distinguished from the
closed and unrestricted cases rather than folded into the same `Unchecked`
report all three used to share.

**Tests.** The single pre-existing slot test asserted a fixture — a plain
`ELEMENT` with no `archetype_details` at all — that this fix reveals was
never actually a *filled* slot to begin with, only ever the open-and-empty
case; it is replaced with four, one per outcome above:
`an_open_slot_left_unfilled_is_fully_conformant`,
`a_closed_slot_that_was_filled_anyway_is_a_violation`,
`an_unrestricted_open_slots_filler_is_fully_conformant`, and
`a_restricted_open_slots_filler_is_unchecked_not_silently_passed` — the last
of these is the only one still carrying the `K15.20` discipline the old,
single test's name claimed for all four cases at once. A shared `filled()`
helper in `am::validate`'s own test module builds an `Element` with
`archetype_details` set, the shape every "was this slot actually filled"
question in this finding turns on.

## A-61 — `ARCHETYPE_TERM.other_items` had no counterpart

**Severity: Low. Status: fixed.**

Found comparing `am::terminology` against
`org.openehr.am.aom2.archetype_term.adoc` directly, the same method
`A-50`/`A-52`/`A-53`/`A-59` each used to find their own gaps — this crate's
`TermDefinition` already modelled `code` (as the enclosing map's own key,
not a duplicated field — a reasonable choice, not a gap), `text`, and
`description`, and had for as long as the type existed, so it read as
complete. It was not: AOM2 states a fourth attribute, `other_items:
Hash<String, String> [0..1]`, described as a "Hash of keys and
corresponding values for other items in a term, e.g. provenance." Nothing
in this crate could hold one — not accepted and checked, not accepted and
carried unchecked, not even round-tripped through JSON — the same
silent-loss shape every one of the findings above found in a different
class.

**Fixed.** `other_items: BTreeMap<String, String>` added, defaulting empty.
`with_other_item(key, value)` is a builder, matching
`ArchetypeTerminology::with_binding`'s own one-entry-at-a-time shape for its
external bindings; `other_items()` reads the whole map back.
`#[serde(skip_serializing_if = "BTreeMap::is_empty", default)]` keeps JSON
written before this field existed both readable and unchanged in shape when
unused, the same choice `A-46`/`A-48`/`A-50`/`A-59` made for their own
late-added fields.

**Not enforced, deliberately.** AOM2 does not name a fixed set of
recognised keys for `other_items` — "e.g. provenance" is one example, not
an enumeration — so there is nothing to validate a key or value against.
This is the same position `am::terminology`'s own module documentation
already states for `ArchetypeTerminology`'s external bindings: "A binding
to SNOMED CT or LOINC names a terminology this crate cannot reach... those
are carried, reported as unchecked, and never reported as satisfied"
(`S1.10`, `K15.22`). `other_items` is carried on the same terms.

**Tests.** Three: absent by default and carried once attached via the
builder; canonical-JSON round-tripping, both bare and with entries, and
confirming `other_items` is omitted from the JSON entirely when empty
rather than written as `{}`; and a fixture written as though from before
this field existed — literal JSON with no `other_items` key at all — still
deserialising, reading as an empty map.

## A-62 — `am::cadl` refused three constructs its own types already modelled

**Severity: Medium. Status: fixed.**

**The gap.** `am::cadl`'s own module documentation grouped `ARCHETYPE_SLOT`
(`allow_archetype`), `C_ARCHETYPE_ROOT` (`use_archetype`), and
`C_COMPLEX_OBJECT_PROXY` (`use_node`) under one blanket refusal, reasoning
that `allow_archetype`'s own `include`/`exclude` clauses need the assertion
language `K15.10` covers and this parser does not — true, but stated as the
reason for refusing all three, when only one of them actually touches an
assertion at all. `c_archetype_root`'s own grammar
(`archetype_ref: ARCHETYPE_HRID | ARCHETYPE_REF`) and
`c_complex_object_proxy`'s (`ADL_PATH`, a single trailing token) need
nothing from `K15.10` — both types already existed in this crate (`A-50`,
`A-53`), already had constructors ready to take exactly what the grammar
offers, and archetypes using either construct were refused for a reason
that, on inspection, did not apply to them.

**Fixed, in three parts.**

1. **`use_archetype` → `C_ARCHETYPE_ROOT`**, fully. The only real obstacle
   was lexical, not grammatical: `ARCHETYPE_HRID`/`ARCHETYPE_REF` do not
   lex as one `Word` token in `cadl_lexer::Lexer` — its word-scanner stops
   at `-` (needed elsewhere, so `at`/`id`-codes and RM type names tokenize
   correctly), which splits `openEHR-EHR-CLUSTER.device.v1` into five
   tokens. Rather than adding a second, `-`-tolerant scanning rule only
   this one construct would use, `Lexer::text_since(start)` slices the
   original source between two offsets a caller has already bounded —
   `c_archetype_root`'s own grammar puts nothing but the reference between
   its leading `,` and the closing `]`, so "everything up to `]`" is exact
   for this call site, not a guess. `CArchetypeRoot::new` takes the
   reconstructed text as a plain `String` (it was never typed narrower
   than that — `A-49`'s own `ARCHETYPE_REF`-vs-`ArchetypeId` residual does
   not apply here), so no further validation is invented.
2. **`use_node` → `C_COMPLEX_OBJECT_PROXY`**, fully, including the case its
   own `occurrences` is absent. `Lexer::read_raw_path()` reads raw,
   un-tokenized text to the next whitespace — `ADL_PATH`'s own grammar
   (`base_lexer.g4`) contains no unescaped whitespace, so this is exact,
   not approximate, and simpler than tokenizing `/`-separated segments only
   to immediately re-join them. A new `parse_optional_occurrences`, distinct
   from the existing `parse_occurrences`, builds `None` rather than
   refusing an absent `occurrences` here specifically:
   `C_COMPLEX_OBJECT_PROXY.occurrences` is the one field in this crate an
   absence is *meaningful* for (`use_target_occurrences()`, `A-53`), not
   something to guess a value for or treat as an omission.
3. **`allow_archetype` → `ARCHETYPE_SLOT`**, for its unrestricted form only
   — occurrences stated or not, no `matches` clause. Two narrower refusals
   remain, each real: `closed` is refused because its own grammar
   production (`archetype_slot: ... (( c_occurrences? (...)? ) |
   SYM_CLOSED )`) carries no `c_occurrences` at all alongside `closed`, and
   `ArchetypeSlot` — unlike `CComplexObjectProxy` (`A-54`'s own scope
   decision) — stores occurrences as a plain, non-deferrable
   `MultiplicityInterval`, so there is no value to build one from without
   inventing one; `matches { include ... exclude ... }` is refused because
   `K15.10` genuinely applies there — each assertion is the full BEOM
   `boolean_expr` grammar (quantifiers, arithmetic, function calls), and
   this parser lexes no part of it.

**Not attempted.** `K15.10` itself — the BEOM expression grammar
`allow_archetype`'s `matches` clause needs — remains out of scope, as does
`closed`'s own occurrences gap; both are refused by name, not silently
accepted with an empty assertion list or a guessed multiplicity, exactly
the discipline the module documentation already states for every other
refusal in this parser.

**Tests.** The one pre-existing `allow_archetype` test asserted a fixture
that, under this fix, hits a different (also correct) refusal first —
"occurrences omitted" fires before the parser ever reaches the `matches`
clause it was written to exercise, since occurrences precedes `matches` in
the grammar and the old fixture stated neither. It is renamed and given
occurrences so the refusal it actually names fires. Five new tests:
`use_archetype` parsed into a `C_ARCHETYPE_ROOT`, using exactly the
`-`-split fixture that proves the slice reconstruction is exact;
`use_node` with no stated `occurrences`, confirming it builds `None`
rather than refusing; an unrestricted `allow_archetype` parsed into an
`ArchetypeSlot` with `any_allowed()` true — the case `A-60`'s
`walk_slot` can now fully check on data reached through real ADL text, not
only through direct Rust construction; and the `closed` and `matches`
refusals, each naming its own reason. Two more in `cadl_lexer`:
`text_since` reconstructing a `-`-split archetype reference, and
`read_raw_path` stopping at whitespace after skipping trivia, plus its
end-of-input case.
