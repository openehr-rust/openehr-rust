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

Seventeen findings: two High (**A-15**, **A-16**), eight Medium (**A-01**,
**A-03**, **A-06**, **A-11**, **A-12**, **A-13**, **A-14**, **A-17**) and seven
Low. **Twelve are fixed** — A-01, A-03, A-04, A-06, A-07, A-11, A-12, A-13,
A-14, A-15, A-16, A-17 — three of them with a residual recorded. **A-09**
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

**Residual.** Two modules of many, and not in CI: 80 mutants take two minutes
for one file, and a whole crate would be hours. `T13.2` stays **?** — what
changed is that "not systematic" is a measurement with numbers rather than an
impression, the method is written down, and its sharpest use so far was aiming
it at code whose tests live somewhere else.

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

## Closed findings

**A-01** and **A-03** are fixed and kept above with their evidence, because the
evidence is the reason each fix is trusted and because each leaves a residual
that is still live — an inference about `DV_SCALE`, and unbounded recursion
depth. A finding is not deleted when it is fixed; it is marked.
