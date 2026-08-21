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

**39 findings, 39 in the table below: 6 High, 24 Medium, 9 Low. 32 fixed or
classified, 7 open.** These counts are checked against the table by CI
(`claims` / *the audit summary counts itself correctly*) — if this paragraph
and the table disagree, the table is correct (`W0.3`: never claim more than is
verified), and the check should have failed. Every one of the 7 open findings
is open by a stated reason rather than by omission: **A-02** and **A-08** are
declared departures the crate does not intend to close; **A-05**, **A-10**,
**A-30** and **A-38** are recorded limitations or residuals with the reasoning
for leaving them written beside them — **A-38** is a defect in `serde_json`
that this repository cannot repair, only contain and report; **A-19** is a declared
departure (`S1.18`) whose *enforcement* remains open work. **A-09**
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
| A-19 | Medium | `COMPOSITION.Territory_valid` and `Language_valid` are neither enforced nor declared | **declared** as `S1.18`; enforcement open |
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
| A-38 | Medium | `serde_json` 1.0.151's float parser is **not the inverse of its own serializer** ([serde-rs/json#1336](https://github.com/serde-rs/json/issues/1336)): it reads `1.5777777777770001` one ULP below `core::str::parse`. A magnitude therefore **drifts** across repeated canonical round trips — three applications before it settled in the observed case, with no bound established — and a composition read out of a store is not bit-identical to the one written | open, **upstream** — contained by `db:M3.43`, which stores canonical JSON byte-preserving and digests the stored bytes rather than re-canonicalising, so no false tamper alarm is reachable |
| A-39 | Medium | Two matches in `DataValue` whose arms could be deleted in silence — `semantic_cmp` (6 of 9) and `is_strictly_comparable_to` (all of them, plus the whole function replaceable with `false`) — and `trim_float`, whose guarded branch produced the same string as its `else` for **every finite `f64`**. Found by the retrospective mutation pass `W-18` required, not by anything failing | **fixed** — one table-driven test per arm with a row-count assertion, and the dead branch deleted rather than tested |
| A-35 | Medium | Ten types — every `DV_ORDERED` descendant and `DataValue` — implemented `PartialOrd` while deriving `PartialEq` over all their fields, so `a != b` while `a <= b` and `a >= b` were both true. Recorded as the lexical-vs-semantic shape of `A-32` and scoped to five types; the mechanism is `OrderedAttrs`, which every `DV_ORDERED` carries, and it reached five more | **fixed** — no `DV_ORDERED` implements `PartialOrd`; comparison is `DvOrdered::semantic_cmp`, and `INTERVAL<T>` is bounded on a new `SemanticOrd` (`D3.18b`, `D3.18c`) |
| A-11 | Medium | The Common Information Model was implemented from prose | **fixed** |
| A-12 | Medium | The Data Structures model was implemented from prose | **fixed** |
| A-13 | Medium | One `IF NOT EXISTS` flag covered two statements MySQL treats differently | **fixed**, verified on MySQL 8.4 |
| A-14 | Medium | SQL Server and Oracle documented an idempotence guard that was never emitted | **fixed**, not verified on either engine |
| A-15 | High | Append-only was enforced in the schema on two engines of five | **fixed**, verified on PostgreSQL 18 and MySQL 8.4 |
| A-16 | High | `Time`/`DateTime` panicked on a multi-byte character in the offset | **fixed**, regression pinned |
| A-17 | Medium | The first property tests passed vacuously | **fixed**, mutation-verified |

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

## Closed findings

**A-01** and **A-03** are fixed and kept above with their evidence, because the
evidence is the reason each fix is trusted and because each leaves a residual
that is still live — an inference about `DV_SCALE`, and unbounded recursion
depth. A finding is not deleted when it is fixed; it is marked.
