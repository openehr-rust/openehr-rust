# Conformance matrix

**Non-normative.** This records what is true today, not what is intended. It is
the file to distrust last: if it disagrees with the code, the code wins and this
file has a defect.

**Assessed:** 2026-08-02, against `rustc 1.97.1`, `openehr` 0.2.0. Rows touched
by **A-36** were re-derived 2026-08-20 against `openehr` 0.3.0 and `rustc 1.98.0`;
the rest carry the earlier date and have not been re-checked since, which is
stated rather than papered over (`W0.10`). The `D3.18b`/`D3.18c` rows were
derived 2026-08-21 with **A-35**. The `K15.18`–`K15.23` row was derived
2026-08-30 against `openehr` 0.8.0, when `openehr::am::validate` landed.
**Method:** each requirement read against the code that implements it and the
test that exercises it; test names below are real and runnable with
`cargo test <name>`.

## Key

| | Meaning |
| --- | --- |
| **•** | Implemented **and** verified by a named test (`C0.7`) |
| **?** | Implemented, no test exercises it directly (`C0.8`) |
| **doc** | A documentation requirement; satisfied by prose, not by code |
| **type** | Enforced by the type system. A runtime test for it could not fail (`T13.2`), so writing one would be theatre. |
| **—** | Deliberately not implemented; see [`01-scope.md`](01-scope.md) |
| **spec** | Specified and **not implemented**: the requirement is in force, no code claims it, and the gap is tracked in [`audit.md`](audit.md). Distinct from **—**, which is a decision not to implement, and from **open**, which is a defect against something the crate purports to do |
| **withdrawn** | The requirement no longer stands. Its identifier is permanent (`C0.5`) and its text is kept where it was, marked, so a citation to it still resolves (`C0.18`, `C0.19`) |
| **open** | Not satisfied; tracked in [`audit.md`](audit.md) |

Doctests count as tests: they compile and run in CI (`T13.8`).

## Totals

Counted mechanically from the tables below, with every requirement id in
`spec/*.md` checked to appear exactly once — **345 ids, 345 covered, none
missing** (344 before `K15.32`, 2026-09-03).

A hand-written total in a file like this is a number nobody rechecks, and this
one proved the point: it said 291 of 291 while six requirements added after the
assessment had no row at all — `S1.18`, `S1.19`, `S1.20`, `L10.9`, `L10.10`,
`L10.11`. "Derived from the rows" was true once and then was not.

**CI now re-derives it** on every push, expanding the ranges in the `Id` column
and comparing against the requirements the specification defines. A new
requirement with no row fails the build; so does a row for a requirement that
does not exist, and so does an id covered twice.

**Re-derived 2026-08-26**, mechanically, from the rows below — and the previous
numbers were wrong, which is the second time this table has drifted from its own
tables. It said 291 total against 300 in the sentence above while the rows held
311, because CI re-derives *coverage* and nothing re-derived the *tally*.
**A-41** is the finding; the counts below were produced by expanding every `Id`
cell and counting statuses, and re-running that is how they should be checked
again.

**Re-derived again 2026-08-30**, the same way, after `K15.18`–`K15.23` moved
from **spec** to **•**: six ids, `openehr::am::validate`. **Re-derived a
third time the same day**, after `K15.24`–`K15.27` made the same move: four
ids, `openehr::am::repository` and `openehr::am::validate`'s
`validate_with_repository`.

**Re-derived a fourth time, 2026-09-03**, after `K15.6`–`K15.7` moved from
**spec** to **•**: two ids, the refusal discipline `am::cadl` and the two
header readers are now held to and tested for (`A-62`–`A-67`). The same pass
found that two test names the `K15.18`–`K15.23` row cited had been renamed by
`A-60` and `A-63` without this file following — corrected below, because this
file promises every name in it is runnable, and for four days two were not.
Later the same day `K15.32` was added and satisfied together (`A-71`, after
the first external corpus run): 345 ids, 271 **•**; the two `K15.6`–`K15.7`
test names that asserted the old refusal of an omitted `occurrences` were
replaced by the names that now assert the parse.

| Status | 2026-09-03 | 2026-08-30 | 2026-08-26 | 2026-07-31 |
| --- | --- | --- | --- | --- |
| • verified | 271 | 268 | 258 | 175 |
| doc | 33 | 33 | 33 | 30 |
| **spec** — in force, unimplemented | 16 | 18 | 28 | — |
| type | 13 | 13 | 13 | — |
| — out of scope | 8 | 8 | 8 | 6 |
| ? implemented, untested | 3 | 3 | 3 | 54 |
| withdrawn | 1 | 1 | 1 | — |
| open | 0 | 0 | 0 | 4 |
| **total requirements** | **345** | **344** | **344** | 269 |

The **spec** rows are §15 plus `S1.21`, added on 2026-08-26 when `S1.4` was
withdrawn and the Archetype Model brought into scope. There were 32 of them that
day; `K15.1`–`K15.4` — the AOM2 object model — were implemented and tested the
same day, leaving 28. `K15.18`–`K15.23` — validating a Reference Model instance
against an archetype already held in memory, as a verdict kept separate from
`L10.x` and with no partial pass — followed on 2026-08-30, leaving 22.
`K15.24`–`K15.27` — the repository abstraction, and resolving a
`C_ARCHETYPE_ROOT` filler through one — followed the same day, leaving 18.
`K15.6`–`K15.7` — the refusal discipline: an unimplemented construct is
refused by name at its offset, never skipped, defaulted, or returned as a
partial tree — followed on 2026-09-03, once `am::cadl` existed to be held to
it (`A-62`–`A-67`), leaving **16**. `K15.32` — an unstated `occurrences`
stays unstated and is inferred only by AOM2's own rule — was added later
that day and satisfied in the same commit (`A-71`), so it never joined the
block: 33 ids in §15, 16 of them still **spec**.
They remain a large block of unsatisfied requirements this crate has carried
since 2026-08-26, and they are counted here rather than described elsewhere so
that the size of the gap is a number a reader can see.

Three things moved these numbers, and they are not the same thing.

**Requirements gained tests.** `tests/invariants.rs` closed **A-06**: 32 tests,
five of them mutation-verified by disabling the check and watching the test
fail.

**Thirteen were reclassified `type`** — enforced by the compiler, where a
runtime test could not fail and writing one would be theatre (`T13.2`). Calling
them `type` rather than `•` keeps the verified count meaning what it says.

**Twenty-two requirements were added**, and this is the part worth reading. The
rendered openEHR pages omit every class-definition table — the sources
`include::` them from a UML export generated at build time — so anything
implemented from the prose alone is a guess. All four Reference Model packages
have now been read from the published PDFs, and **all four times the primary
source contradicted what had been implemented**: three rules backwards, sixteen
invariants missing, five reported under names openEHR does not use, and one
place where **openEHR contradicts itself** (`V8.7a`). A rising total here is the
specification catching up with the one it implements.

The three remaining `?` are named individually below rather than lumped: a
constant-time claim nobody has timed, an error path that cannot be provoked, and
mutation verification being five checks rather than a policy.

## §0 Conformance — `C0`

Process requirements; they govern this specification rather than the code.

| Id | Status | Evidence |
| --- | --- | --- |
| C0.1–C0.3 | doc | this directory's prose |
| C0.4–C0.6 | • | every requirement in this directory carries an id; no gaps or reuse |
| C0.7–C0.11 | doc | this file, and [`audit.md`](audit.md) |
| C0.12–C0.14 | • | `I2.15` and `R4.9` are the two declared departures |
| C0.15–C0.18 | doc | commit conventions |
| C0.19 | doc | `S1.4` is the first exclusion reversed under it — withdrawn in place, reason kept, §15 named |

## §1 Scope — `S1`

| Id | Status | Evidence |
| --- | --- | --- |
| S1.1–S1.3 | • | `canonical_json::a_composition_covering_every_modelled_class_round_trips` |
| S1.4 | withdrawn | reversed 2026-08-26; the Archetype Model is in scope under `S1.21` and §15. Text kept in place, marked (`C0.19`) |
| S1.5–S1.7 | — | not implemented, by decision |
| S1.8 | • | `time_specification::tests::unimplemented_accessors_refuse_rather_than_guess` |
| S1.9 | • | `quantity::tests::different_units_are_not_comparable_in_either_direction` |
| S1.10 | • | `text::tests::rubric_checking_reports_unchecked_separately_from_valid` |
| S1.11 | doc | `security` module header; `V8.18` |
| S1.12 | • | `guarantees::unimplemented_operations_refuse_and_cite_the_spec` |
| S1.13 | • | `canonical_json` round trip covers `DV_PARSABLE` and time specifications |
| S1.14 | doc | `security` module header |
| S1.15 | doc | `J9.15`; bounded by `canonical_json::reading_a_composition_stays_within_a_small_stack` |
| S1.21 | spec | §15 in full — seventeen of the thirty-three ids it brought into scope are **•** and sixteen **spec** (2026-09-03, `K15.32` added and satisfied the same day); **A-40** |
| S1.16–S1.17 | • | `terminology::tests::the_codes_that_disagree_between_terminology_repositories_are_the_current_ones` |
| S1.18 | — | declared departure: ISO 3166 and ISO 639 are not carried, so `COMPOSITION.Territory_valid` and `Language_valid` are not checked. Its closing requirement — that a caller can do the check instead — **is** verified: `guarantees::a_caller_can_read_every_code_the_crate_declines_to_check` (`A-19`) |
| S1.19 | — | declared exclusion: no demographic repository, so the four `PARTY` graph invariants cannot be checked from a value in hand |
| S1.20 | — | declared departure: an `EHR_ACCESS` may record no policy, so `Scheme_valid` is not checked. `X11.24`'s fail-closed default is why that is safe |

## §2 Identifiers — `I2`

| Id | Status | Evidence |
| --- | --- | --- |
| I2.1 | • | `object_id::tests::version_id_round_trips_exactly`, `…::archetype_id_round_trips_and_splits` — the parts are asserted, not just the text |
| I2.2–I2.3 | • | `object_id::tests::version_id_round_trips_exactly`, `…::archetype_id_round_trips_and_splits` |
| I2.4–I2.5 | • | `uid::tests::oid_wins_over_internet_id_for_all_digit_text` |
| I2.6 | • | `uid::tests::rejects_near_misses` |
| I2.7 | • | `uid::tests::uuid_case_is_preserved_but_not_significant`, `…::uuid_hash_agrees_with_eq` |
| I2.8–I2.9 | • | doctests on `IsoOid`, `InternetId` |
| I2.10 | • | `object_id::tests::hier_object_id_rejects_a_double_colon_extension` |
| I2.11 | • | `object_id::tests::version_id_round_trips_exactly` |
| I2.12–I2.13 | • | doctest on `VersionTreeId` |
| I2.14–I2.15 | • | `object_id::tests::archetype_id_rejects_malformed_forms` |
| I2.16 | • | doctest on `TemplateId`; `object_id::tests::a_template_id_may_contain_whitespace` (`A-79`) |
| I2.17 | • | doctest on `TerminologyId` |
| I2.18 | • | doctest on `GenericId` |
| I2.19 | • | `invariants::a_uid_field_admits_only_the_two_uid_based_classes` |
| I2.20 | • | `object_id::tests::uid_based_id_infers_by_separator_count` |
| I2.21–I2.22 | • | `object_ref::tests::namespace_grammar_is_enforced` |
| I2.23 | • | `object_ref::tests::party_ref_refuses_a_non_demographic_class` |
| I2.24 | • | `invariants::a_locatable_ref_refuses_an_empty_path` |
| I2.25 | • | `object_ref::tests::locatable_ref_uri_does_not_double_the_slash` |

## §3 Data types — `D3`

| Id | Status | Evidence |
| --- | --- | --- |
| D3.1 | • | `data_types::tests::every_variant_round_trips_through_its_type_tag` |
| D3.2 | • | `invariants::the_deprecated_paragraph_type_still_round_trips` |
| D3.3 | • | `text::tests::errors_from_text_do_not_echo_the_text` |
| D3.4 | • | `text::tests::plain_no_newlines_is_checked_against_the_value` |
| D3.5 | • | `invariants::a_code_phrase_needs_a_terminology_and_a_code` |
| D3.6 | • | `invariants::a_term_mapping_match_is_one_of_four_characters` |
| D3.7–D3.8 | • | `text::tests::rubric_checking_reports_unchecked_separately_from_valid` |
| D3.7a | open | `A-81` — a non-English rubric is reported as a violation rather than unchecked, the crate's own English-only table having no way to tell the two apart |
| D3.9–D3.10 | • | `iso8601::tests::partial_dates_keep_their_precision_and_text`; `canonical_json::partial_dates_and_negative_durations_survive_verbatim` |
| D3.10b | • | `iso8601::tests::a_comma_decimal_sign_is_accepted_like_a_full_stop`, `…a_durations_comma_decimal_sign_is_accepted_like_a_full_stop` (`A-78`) |
| D3.11 | • | `iso8601::tests::leap_day_validity_follows_the_gregorian_rule` |
| D3.12 | • | `invariants::a_leap_second_is_accepted_and_a_sixty_first_is_not` |
| D3.13 | • | `iso8601::tests::durations_round_trip_and_reject_disorder` |
| D3.13a | doc | recorded as a limitation, **A-02** |
| D3.14 | • | `guarantees::every_undecidable_comparison_answers_none`; `rm::data_types::tests::every_comparable_variant_of_a_data_value_compares`, which covers the **decidable** half — six arms of `semantic_cmp` were deletable in silence until **A-39** |
| D3.14a | • | `guarantees::a_reference_range_is_unmoved_by_how_an_instant_is_spelled`, which asserts an incomparable value is excluded rather than admitted (**A-39**) |
| D3.15 | • | `quantity::tests::different_units_are_not_comparable_in_either_direction` |
| D3.16 | • | `quantity::tests::ordinals_from_different_terminologies_do_not_compare` |
| D3.17 | • | `invariants::proportions_of_different_kinds_do_not_compare` |
| D3.18 | • | `iso8601::tests::offsets_normalise_before_comparison` |
| D3.18a | • | resolved: `PartialOrd`/`Ord` removed from the base ISO 8601 types, semantic order is the plain method `semantic_cmp` — `iso8601::tests::eq_is_lexical_and_semantic_cmp_is_not_the_same_question` (`A-32`, fixed) |
| D3.18b | • | the same resolution over all ten `DV_ORDERED` types and `DATA_VALUE` — `guarantees::equality_and_order_disagree_by_design_and_neither_is_partial_ord` (`A-35`, fixed) |
| D3.18c | • | `INTERVAL<T>` is bounded on `SemanticOrd`; `guarantees::a_reference_range_is_unmoved_by_how_an_instant_is_spelled` covers the `contains` rewrite |
| D3.18d | • | `base::real::tests::a_measured_precision_survives_a_round_trip`; `every_digit_survives_and_only_the_exponent_form_is_normalised`, which asserts the one departure |
| D3.18e | • | `base::real::tests::equality_and_order_answer_different_questions` — lexical equality, numeric order, and no `PartialOrd` |
| D3.18f | • | `base::real::tests::a_real_that_never_saw_a_constructor_is_still_readable` |
| D3.19 | • | `quantity::tests::non_finite_magnitudes_are_refused` |
| D3.19a | doc | declared narrowing; recorded here and in §3 |
| D3.20, D3.20a | • | `quantity::tests::precision_accepts_the_unlimited_sentinel`; `validation::tests::the_unlimited_precision_sentinel_validates` |
| D3.21 | • | `quantity::tests::accuracy_zero_may_not_be_a_percentage`, `…::a_percentage_accuracy_stays_within_a_hundred` |
| D3.21a | • | `quantity::tests::unknown_accuracy_is_a_value_and_not_an_absence` |
| D3.22 | • | verified verbatim against the 1.0.2 class table; `MagnitudeStatus::parse` is the closed set |
| D3.23 | • | `quantity::tests::proportion_kind_rules_are_enforced` |
| D3.23a | • | `quantity::tests::proportion_precision_asserts_integrality_rather_than_forbidding_it` |
| D3.24 | • | `quantity::tests::is_abnormal_distinguishes_unknown_from_normal` |
| D3.24a | • | `validation::tests::a_normal_status_outside_the_code_set_is_reported` |
| D3.24b | • | `validation::tests::a_normal_flag_beside_an_abnormal_number_is_reported` |
| D3.25 | • | `encapsulated::tests::debug_does_not_print_the_bytes`; `guarantees::display_never_reveals_an_identifier_or_a_media_blob` |
| D3.26 | • | `encapsulated::tests::integrity_outcomes_are_distinguished` |
| D3.27 | • | `encapsulated::tests::sha1_is_readable_but_not_writable` |
| D3.28 | • | `invariants::parsable_content_needs_a_formalism` |
| D3.29 | • | `encapsulated::tests::base64_round_trips_including_padding_lengths` |
| D3.30 | • | `uri::tests::schemes_are_checked_not_assumed`; `guarantees::a_uri_that_never_saw_a_constructor_is_reported_rather_than_panicking`; `guarantees::an_empty_uri_is_reported_under_openehrs_own_invariant_name` |
| D3.30a | • | `guarantees::a_uri_that_never_saw_a_constructor_is_reported_rather_than_panicking` |
| D3.31 | • | `uri::tests::ehr_uri_refuses_an_external_target`; `guarantees::an_ehr_uri_deserialized_with_a_foreign_scheme_is_reported`; `guarantees::a_link_target_is_validated_on_every_locatable_that_carries_it` |
| D3.32 | doc | `uri` module header |
| D3.33 | • | `basic::tests::an_empty_identifier_is_refused` |
| D3.34 | • | `basic::tests::display_never_reveals_the_identifier` |

## §4 Data structures — `R4`

| Id | Status | Evidence |
| --- | --- | --- |
| R4.1 | • | `canonical_json::every_polymorphic_attribute_carries_its_type_tag` |
| R4.2 | type | every structure embeds `LocatableAttrs` |
| R4.3–R4.4 | • | `data_structures::tests::an_element_is_valued_or_null_and_never_both`; `validation::tests::deserialization_bypasses_constructors_and_validation_catches_it` |
| R4.5–R4.7 | • | `data_structures::tests::the_four_null_flavours_stay_four`; `guarantees::the_four_null_flavours_remain_four` |
| R4.8 | • | `invariants::a_cluster_refuses_to_be_empty` |
| R4.9 | • | `invariants::a_ragged_table_is_reported_rather_than_assumed_regular`; doctest on `ItemTable` covers indexing |
| R4.10 | • | `data_structures::tests::element_traversal_reaches_every_leaf_of_every_structure` |
| R4.11 | • | `data_structures::tests::a_history_needs_events_or_a_summary` |
| R4.12 | • | `invariants::a_periodic_history_needs_a_positive_period` |
| R4.13 | • | `validation::tests::an_event_before_its_history_origin_is_reported` |
| R4.14 | doc | `data_structures` module header and `IntervalEvent::time` |
| R4.12a–R4.12c | • | `invariants::a_periodic_history_checks_that_its_events_fall_on_the_period`; `iso8601::tests::differencing_two_instants_crosses_months_and_offsets` |
| R4.15, R4.15a | • | `invariants::an_interval_event_checks_its_width_and_math_function` |
| R4.16 | • | `data_structures::tests::interval_start_time_is_derived_where_it_can_be_and_refused_where_it_cannot`, `…::interval_start_time_crosses_a_day_boundary_correctly` |
| R4.17 | type | `state` and `data` are separate attributes; exercised by the round-trip fixture |

## §5 Common — `M5`

| Id | Status | Evidence |
| --- | --- | --- |
| M5.1 | type | every clinical class embeds `LocatableAttrs` |
| M5.2 | • | `invariants::every_validation_check_fires_on_a_document_that_breaks_it` (`Archetype_node_id_valid`) |
| M5.3–M5.4 | • | `text::tests::a_coded_payload_without_a_type_does_not_lose_its_code` |
| M5.5 | • | `invariants::only_an_archetype_root_has_a_concept` |
| M5.6 | • | doctest on `Archetyped` |
| M5.7 | • | `validation::tests::an_archetype_id_on_the_wrong_class_is_reported` |
| M5.8 | • | `canonical_json::a_composition_covering_every_modelled_class_round_trips` carries `rm_version` unchanged |
| M5.9 | • | `uri::tests::ehr_uri_refuses_an_external_target` |
| M5.10 | • | `invariants::feeder_audit_original_content_must_be_encapsulated` |
| M5.11 | • | `invariants::feeder_audit_details_must_name_a_system` |
| M5.12–M5.13 | • | `common::tests::party_proxy_infers_its_class_without_a_type_tag` |
| M5.13a | • | `invariants::conditionally_coded_attributes_are_checked_only_when_openehr_coded` |
| M5.14 | • | doctest on `PartyIdentified` |
| M5.15 | • | `common::tests::a_self_related_party_counts_as_the_subject` |
| M5.16 | • | `invariants::an_anonymous_subject_round_trips_as_itself` |
| M5.17–M5.18 | type | `PARTICIPATION`'s attributes and the `DV_TEXT` typing of `function` are structural |
| M5.18a | • | `invariants::conditionally_coded_attributes_are_checked_only_when_openehr_coded` |

## §6 EHR — `E6`

| Id | Status | Evidence |
| --- | --- | --- |
| E6.1 | type | `Ehr` holds `ObjectRef`s throughout |
| E6.2 | • | `invariants::the_directory_is_the_first_folder_or_there_is_neither` |
| E6.3–E6.4 | • | `ehr::tests::deactivating_a_record_does_not_make_it_unreadable` |
| E6.3a | • | `invariants::a_composition_and_an_ehr_status_must_be_archetype_roots` |
| E6.5 | type | typed `PartySelf` |
| E6.6 | • | `invariants::coded_attributes_are_checked_against_their_openehr_group` |
| E6.6a | • | `invariants::a_composition_and_an_ehr_status_must_be_archetype_roots` |
| E6.6b | • | `invariants::a_persistent_composition_may_not_carry_an_event_context` — constructor and validation |
| E6.7 | type | required by the constructor's signature |
| E6.7a | doc | declared not-enforced; out of scope by `S1.10` |
| E6.8 | • | `canonical_json::a_composition_covering_every_modelled_class_round_trips` |
| E6.9 | • | `ehr::tests::entries_are_found_through_nested_sections` |
| E6.10 | doc | `Section` documentation |
| E6.11 | • | `invariants::coded_attributes_are_checked_against_their_openehr_group` |
| E6.12 | • | `ehr::tests::an_event_context_cannot_end_before_it_starts` |
| E6.12a | • | `invariants::an_event_context_location_is_absent_or_non_empty` |
| E6.13 | • | `canonical_json::every_polymorphic_attribute_carries_its_type_tag` |
| E6.14 | • | `ehr::tests::an_entry_about_someone_else_says_so` |
| E6.15 | • | `invariants::administrative_entries_are_distinguishable_from_clinical_ones` |
| E6.16–E6.17 | type | enforced by the types |
| E6.18 | doc | `Instruction::new` documentation |
| E6.19 | • | `ehr::tests::an_instruction_needs_at_least_one_activity` |
| E6.20 | doc | `Activity::new` documentation |
| E6.21 | • | `invariants::coded_attributes_are_checked_against_their_openehr_group` |
| E6.22 | • | `ehr::tests::terminal_ism_states_are_recognised` |
| E6.23 | • | `invariants::an_action_must_say_which_activity_it_fulfilled` |
| E6.24 | type | `Folder` holds `ObjectRef`s |

## §7 Demographics — `G7`

| Id | Status | Evidence |
| --- | --- | --- |
| G7.1–G7.2 | doc | `demographic` module header |
| G7.3 | • | `demographic::tests::a_party_needs_at_least_one_identity` |
| G7.4 | • | `demographic::tests::a_party_without_a_uid_is_refused` |
| G7.5 | • | `demographic::tests::a_role_is_a_party_but_not_an_actor` |
| G7.6–G7.7 | doc | type documentation |
| G7.8 | doc | `PartyIdentity` documentation |
| G7.9 | • | `invariants::a_contact_needs_an_address_and_may_carry_a_validity_period` |
| G7.10 | • | `demographic::tests::an_unrecorded_validity_period_is_not_a_yes` |
| G7.11 | • | `invariants::a_contact_needs_an_address_and_may_carry_a_validity_period` |
| G7.12 | type | `PARTY_RELATIONSHIP` references both parties by `PartyRef` |

## §8 Change control — `V8`

| Id | Status | Evidence |
| --- | --- | --- |
| V8.1 | • | `common::tests::a_version_of_another_object_is_refused` |
| V8.2 | • | `common::tests::a_duplicate_version_id_is_refused` |
| V8.3 | • | doctest on `VersionedObject` |
| V8.4–V8.5 | • | `common::tests::concurrent_writes_are_refused_rather_than_silently_ordered` |
| V8.6 | • | `common::tests::version_at_time_skips_incomparable_commit_times` |
| V8.7, V8.7a, V8.7b | • | `invariants::an_empty_history_has_no_revision_history_at_all` — order, most-recent, and commit time |
| V8.8–V8.9 | • | `common::tests::a_deleted_version_may_have_no_data_and_others_may_not` |
| V8.10 | • | `canonical_json::a_versioned_composition_round_trips_with_its_audit_trail` |
| V8.11–V8.12 | • | `invariants::an_imported_version_keeps_the_identity_it_arrived_with` |
| V8.13–V8.14 | • | doctest on `AuditDetails` |
| V8.15–V8.16 | • | `common::tests::contribution_change_types_are_restricted` |
| V8.17 | • | `invariants::an_attestation_carries_what_was_signed_and_whether_it_is_outstanding` |
| V8.17a | • | `invariants::conditionally_coded_attributes_are_checked_only_when_openehr_coded` |
| V8.18 | doc | `Attestation` documentation |
| V8.19 | • | `invariants::an_attestation_carries_what_was_signed_and_whether_it_is_outstanding` |

## §9 Serialization — `J9`

| Id | Status | Evidence |
| --- | --- | --- |
| J9.1 | • | `canonical_json::a_composition_covering_every_modelled_class_round_trips` |
| J9.2 | • | `canonical_json::round_tripping_twice_is_byte_identical` |
| J9.3 | • | `canonical_json::partial_dates_and_negative_durations_survive_verbatim` |
| J9.4 | • | `canonical_json::every_polymorphic_attribute_carries_its_type_tag` |
| J9.5 | • | `data_types::tests::a_value_without_a_type_tag_is_refused` |
| J9.6 | • | `text::tests::a_coded_payload_without_a_type_does_not_lose_its_code`; `common::tests::party_proxy_infers_its_class_without_a_type_tag` |
| J9.7 | • | `invariants::a_type_tag_naming_the_wrong_class_is_refused` |
| J9.8–J9.10 | • | `canonical_json::a_payload_written_by_another_implementation_reads` |
| J9.11 | • | `invariants::absent_and_empty_attributes_are_omitted_not_nulled` |
| J9.12 | • | `canonical::tests::nested_objects_are_sorted_at_every_level` |
| J9.13 | • | `canonical::tests::measured_precision_is_not_normalised_away` |
| J9.14 | • | `canonical::tests::keys_sort_by_scalar_value_not_by_locale` |
| J9.15 | • | `canonical_json::reading_a_composition_stays_within_a_small_stack`; **A-03** |

## §10 Validation — `L10`

| Id | Status | Evidence |
| --- | --- | --- |
| L10.1, L10.1a, L10.1b | • | `validation::tests::deserialization_bypasses_constructors_and_validation_catches_it` |
| L10.2 | doc | `validation` module header |
| L10.3 | • | `validation::tests::every_violation_is_reported_not_just_the_first` |
| L10.4 | • | `validation::tests::a_coded_text_that_contradicts_its_own_code_is_reported` |
| L10.5 | • | `guarantees::a_validation_report_names_paths_and_never_values` |
| L10.5a | • | `invariants::every_validation_check_fires_on_a_document_that_breaks_it` attributes an empty name to `DV_TEXT.Valid_value` |
| L10.6 | • | `invariants::every_validation_check_fires_on_a_document_that_breaks_it` drives twelve of them from JSON; the `DV_ORDERED` pair have their own tests |
| L10.7 | • | `invariants::violations_are_reported_in_document_order_and_that_order_is_stable` |
| L10.8 | • | `text::tests::rubric_checking_reports_unchecked_separately_from_valid` |
| L10.9–L10.10 | • | the crate-added register in [`10-validation.md`](10-validation.md); `openehr-assets` fails the build when it and the generated report disagree |
| L10.11 | • | the unenforced register, same file, same check — in both directions (`D-09` is the same defect one tree over) |

## §11 Security — `X11`

| Id | Status | Evidence |
| --- | --- | --- |
| X11.1 | doc | `security` module header |
| X11.2 | type | `AuditDetails` is required by `OriginalVersion::new`'s signature |
| X11.3 | • | `access::tests::an_unimplemented_scheme_round_trips_unchanged` |
| X11.3a | • | `access::tests::an_unimplemented_scheme_denies_and_names_itself` |
| X11.4 | • | `access::tests::nothing_recorded_denies_rather_than_permits` |
| X11.5 | • | `access::tests::an_unimplemented_scheme_denies_and_names_itself` |
| X11.5a | • | `access::tests::operations_are_separately_permitted` |
| X11.6 | • | `guarantees::display_never_reveals_an_identifier_or_a_media_blob` |
| X11.7, X11.7a | • | `guarantees::no_construction_error_echoes_a_submitted_value` |
| X11.8 | • | `redact::tests::sensitive_hides_from_display_and_debug_but_not_from_serde` |
| X11.9 | • | `audit_chain::tests::removing_an_entry_from_the_middle_breaks_the_link` |
| X11.10 | doc | `audit_chain` module header |
| X11.11 | • | `audit_chain::tests::a_clean_chain_verifies_and_an_edited_one_does_not` |
| X11.12 | ? | `Mac` implements neither `PartialEq` nor `Eq`, so `==` beside the one comparison does not compile; `guarantees::a_forged_tag_is_refused` pins the behaviour. Timing is still not measured, and the absence of the derive is not itself tested — see `Mac`'s documentation for why a `compile_fail` for it passed for the wrong reason |
| X11.13 | • | `audit_chain::tests::an_unheld_key_is_reported_as_such_and_not_as_forgery`, `…::a_forged_tag_under_a_held_key_is_a_finding`, `…::an_unkeyed_chain_does_not_claim_full_verification` |
| X11.14 | • | `audit_chain::tests::key_rotation_is_additive` |
| X11.15 | • | `invariants::a_chain_begins_where_it_begins_and_says_when_it_began_late` |
| X11.16 | • | `audit_chain::tests::short_keys_and_empty_ids_are_refused` |
| X11.17 | • | `audit_chain::tests::key_debug_does_not_print_the_material` |
| X11.18 | • | `encapsulated::tests::sha1_is_readable_but_not_writable` |
| X11.19 | • | `guarantees::a_chain_checkpoint_carries_no_patient_data` |
| X11.20 | • | `redact::tests::a_masked_element_says_the_value_exists` |
| X11.21 | • | `redact::tests::a_redacted_composition_is_still_valid` |
| X11.22 | • | `guarantees::redaction_masks_and_reports_a_count_not_a_category` |
| X11.23 | • | `redact::tests::a_reason_appears_and_does_not_disclose_the_category` |
| X11.24 | ? | `redact` returns `Result` and yields nothing on error, and the error path cannot be provoked. The premise is now tested — `guarantees::no_document_this_crate_can_build_carries_a_non_finite_float` — because `serde_json` writes `null` for a non-finite float rather than failing, so the constructors are the only barrier (**A-10**) |
| X11.25 | • | `guarantees::redaction_masks_and_reports_a_count_not_a_category` |

## §12 Paths and query — `Q12`

| Id | Status | Evidence |
| --- | --- | --- |
| Q12.1 | • | `path::tests::a_full_path_reaches_a_magnitude`, `…::name_predicates_select_the_right_repeat` |
| Q12.2 | • | `path::tests::malformed_paths_report_where_they_broke` |
| Q12.3–Q12.4 | • | `path::tests::index_predicates_are_zero_based_and_quoted_digits_are_names` |
| Q12.5 | • | `guarantees::an_ambiguous_path_refuses_instead_of_choosing` |
| Q12.6 | • | `path::tests::a_wrong_attribute_is_no_match_not_an_error` |
| Q12.7 | • | `path::tests::a_full_path_reaches_a_magnitude` |
| Q12.7a, Q12.7b | • | `path::tests::reference_ranges_are_navigable`, `…::an_interval_valued_element_is_navigable` |
| Q12.8 | • | `path::tests::predicates_round_trip_through_display_in_long_form` |
| Q12.9 | • | `aql::tests::the_canonical_blood_pressure_query_parses`, `…::aggregates_and_distinct_parse`, `…::not_contains_parses_and_keeps_its_negation`, `…::like_and_offset_parse` |
| Q12.9a | • | `guarantees::aql_refuses_what_it_does_not_model_and_says_where_that_is_recorded` |
| Q12.9b | • | signed numeric literals parse, resolved at operand position — `aql::a_sign_is_a_number_where_a_value_belongs_and_nowhere_else` (`A-27`, fixed) |
| Q12.9d | • | `LIMIT -5` and `OFFSET -1` refused with a message naming the reason — `aql::a_negative_limit_or_offset_is_refused_rather_than_clamped` |
| Q12.9e | • | a real renders with a decimal point so it lexes back a real — `guarantees::aql_rendering_round_trips_through_the_parser`, which compares trees |
| Q12.9c | • | declared limitation: no node-id predicate shorthand — `aql::only_a_dashed_and_dotted_word_standing_alone_is_an_archetype_id` (`A-30`) |
| Q12.10 | — | no execution API exists |
| Q12.11 | • | `aql::tests::keywords_are_case_insensitive` |
| Q12.12 | • | `aql::tests::malformed_queries_report_an_offset` |
| Q12.13 | • | `aql::tests::parameters_are_collected_from_every_clause_and_deduplicated` |
| Q12.14 | • | `guarantees::aql_catches_a_path_rooted_at_an_unbound_alias` |
| Q12.15 | • | `aql::tests::a_parsed_query_reparses_from_its_own_rendering`; `guarantees::aql_rendering_round_trips_through_the_parser`, which compares the **tree** and not only the text; cosmetic spacing difference noted in **A-05** |
| Q12.15a | • | `guarantees::aql_rendering_round_trips_through_the_parser` over the `CONTAINS`/`OR` shapes that broke it (**A-37**) |
| Q12.15b | • | `guarantees::an_aql_string_literal_is_not_mangled_by_the_lexer` (**A-37**) |

## §13 Conformance testing — `T13`

| Id | Status | Evidence |
| --- | --- | --- |
| T13.1 | • | 222 of 276 requirements cite a test; 3 remain `?` and are named above |
| T13.2 | ? | `cargo-mutants` over every module of meaningful size in `openehr` (all now 0 or equivalent survivors — see the `lib:A-09` table in `spec/audit.md` for the full per-module count) plus `openehr-sqlite/{store,dialect}.rs`, `openehr-store/{integrity,record,dialect}.rs`, `openehr-loco`'s auth/controllers/access/app/tasks/views, and the five schema-level engine crates' `Dialect` impls. `openehr-store/conformance.rs` — the logic those numbers actually exercise — is measured from `openehr-sqlite`, not from `openehr-store` itself, because nothing in `openehr-store`'s own test target calls it. Not in CI as a standing gate (the `mutants` job covers only a PR's diff); `openehr-postgresql`/`mysql`/`mariadb`/`mssql`/`oracle`'s `store.rs`-equivalent logic doesn't exist (Schema level, no `Store` impl) and `App::before_run` has one structural residual — see `spec/audit.md` — and the six `-fuzz` crates are untouched — **A-09** |
| T13.3 | • | every test in `tests/guarantees.rs` states its failure mode |
| T13.4 | • | `canonical_json` |
| T13.5–T13.6 | • | `tests/guarantees.rs` |
| T13.7 | • | `canonical_json::a_payload_written_by_another_implementation_reads` |
| T13.8 | • | 79 doctests run in `cargo test` |
| T13.9 | — | no test in this crate self-skips |
| T13.10 | • | this file |
| T13.11 | • | `reading_a_composition_stays_within_a_small_stack` names its toolchain, method, and figures |
| T13.12–T13.13 | • | `cargo clippy --all-targets` is clean with the lint table in `Cargo.toml`, and `#![forbid(unsafe_code)]` at the crate root states it in the source as well — belt and braces, because a manifest edit removes the one and not the other |
| T13.14 | • | every `#[allow]` in the crate carries a reason |

## §15 Archetypes and templates — `K15`

The section was added on 2026-08-26 when `S1.4` was withdrawn. **Seventeen
ids are satisfied and sixteen are not** (thirty-three since `K15.32`,
2026-09-03): `openehr::am` is the AOM2 object model,
`openehr::am::validate` (2026-08-30) checks a Reference Model instance against
an `Archetype` already held in memory, `openehr::am::repository` plus
`validate_with_repository` (2026-08-30, same day) resolve a `C_ARCHETYPE_ROOT`
filler through a caller-supplied repository — `openehr` performs no I/O
itself (`K15.25`) — and `openehr::am::cadl` (2026-09-02, `A-62`–`A-67`) reads
an archetype's `definition` section, refusing what it does not implement by
name (`K15.6`, `K15.7`). No code in this crate reads a *whole* ADL archetype
into an `Archetype`, reads an ADL 1.4 body, flattens a specialised archetype,
or expands a template — so what `validate_with_repository` checks is the
definition each resolved archetype gives as its own, not as a flattened OPT2
would be, and that scope limit is stated in its own module documentation
rather than left for the row below to imply more. A filled `ARCHETYPE_SLOT` is
checked against its own `is_closed` rule through `crate::path::Node::
archetype_details` (`A-60`); one restricted by `include`/`exclude` assertions
carries them (`A-66`) and does not evaluate them (`K15.10`). **A-40** tracks
the rest.

This table exists so that the gap is counted rather than described. A row moves
off **spec** when the code implements it *and* a named test exercises it
(`C0.7`) — not when a parser lands, and not when a README says so (`K15.30`,
`K15.31`).

| Id | Status | Evidence |
| --- | --- | --- |
| K15.1 | • | `am::archetype::tests::a_definition_constraining_the_wrong_rm_class_is_refused`, `…a_node_the_terminology_does_not_define_is_refused`, `…a_code_specialised_deeper_than_its_archetype_is_refused`, `…a_terminology_constraint_naming_no_value_set_is_refused`; `am::constraint::tests::two_constraints_on_one_attribute_are_refused` |
| K15.2 | • | `archetype_model::the_targeted_archetype_model_release_is_named`; `am::AM_RELEASE` is 2.3.0 and an artefact's own declared versions round-trip unenforced |
| K15.3 | • | `archetype_model::an_archetype_round_trips_through_json_unchanged`, `…a_constraint_this_crate_cannot_model_survives_rather_than_disappearing`. **Scope:** the only serialisation this crate accepts today is its own JSON. ADL and the AM ITS forms are `K15.5`, `K15.8`, and `K15.16`, all below |
| K15.4 | • | `archetype_model::an_archetype_is_constructible_without_a_parser` |
| K15.5 | spec | ADL 2 in full. `am::adl2::parse_header` (the header) and `am::cadl::parse_definition` (`definition`'s own `c_complex_object` — every node kind but `SIBLING_ORDER`, `A-62`–`A-67`, `A-72`–`A-77`) read two sections and do not compose into an `Archetype`: no `language`, `description`, `rules`, `terminology`, or `annotations` — **A-40** |
| K15.6–K15.7 | • | `am::cadl::tests::a_slot_assertion_using_a_quoted_string_instead_of_a_regex_is_refused`, `…an_unterminated_contained_regexp_is_refused_naming_it`, `…an_unwrapped_intervals_kind_is_decided_by_its_first_bounds_token` (its four refusals), `…trailing_content_after_the_root_is_refused`, `…a_name_that_is_not_an_identifier_is_refused_by_name`, `…a_relop_interval_is_read_as_one_open_ended_bound` (its two refusals); `am::adl2::tests::a_trailing_concept_section_is_refused_not_silently_accepted`; `am::adl14::tests::missing_concept_section_is_refused_not_defaulted`. **Scope:** proven for the parsers that exist — `am::cadl` and the two header readers — each refusing an unimplemented construct by name at its offset and returning no partial tree; binding on any parser `K15.5` later adds |
| K15.8–K15.10 | spec | ADL 1.4 ingestion, provenance, and the assertion subset |
| K15.11–K15.13 | spec | specialisation, flattening, and the narrowing check |
| K15.14–K15.17 | spec | template expansion and operational templates, both directions |
| K15.18–K15.23 | • | `am::validate::tests::a_matching_instance_is_conformant`, `…a_missing_mandatory_element_is_a_violation`, `…an_unrecognised_node_id_is_a_violation`, `…an_alternative_below_its_own_mandatory_occurrences_is_a_violation`, `…a_closed_slot_that_was_filled_anyway_is_a_violation`, `…a_restricted_open_slots_filler_is_unchecked_not_silently_passed`, `…an_attribute_tuple_matching_no_row_is_a_violation`, `…a_c_integer_range_rejects_a_value_outside_it`, `…a_c_string_pattern_is_unchecked_even_with_no_literals_to_check`, `…a_c_terminology_code_checks_against_the_archetypes_own_value_set`, `…the_wrong_root_rm_class_is_a_single_violation_not_a_cascade`. **Scope:** against an `Archetype` already in memory, not a flattened OPT2 (`K15.11`, `K15.15` are not implemented) — see `openehr::am::validate`'s own module documentation |
| K15.24–K15.27 | • | `am::repository::tests::a_resolved_archetype_without_provenance_says_so`, `…a_resolved_archetype_with_provenance_carries_it`, `…each_failure_kind_names_what_happened`, `…a_fixed_repository_can_stand_in_for_a_real_one_in_a_test`; `am::validate::tests::a_repository_resolved_filler_validates_the_same_subtree`, `…a_retrieval_failure_is_unchecked_and_names_what_happened`, `…a_repository_returning_a_different_archetype_is_unchecked_not_used`, `…unestablished_provenance_is_unchecked_unless_the_caller_opts_in`. **Scope:** resolves `C_ARCHETYPE_ROOT` fillers only. An `ARCHETYPE_SLOT` is checked against its own `is_closed`/`any_allowed()` rule through `crate::path::Node::archetype_details` (`A-60`, `…a_closed_slot_that_was_filled_anyway_is_a_violation`); a slot restricted by `include`/`exclude` assertions carries them (`A-66`) and does not evaluate them — `K15.10`'s remainder — so its filler is reported unchecked, naming the filler's archetype id |
| K15.28–K15.29 | spec | the boundaries this section does **not** move: authoring, publishing, AQL execution |
| K15.30–K15.31 | spec | the honesty gate while the rest is unbuilt — refuse, and do not describe a parser as archetype support |
| K15.32 | • | `am::constraint::tests::effective_occurrences_follows_aom2s_rule_and_defers_only_for_a_proxy`, `…an_unstated_occurrences_is_absent_on_the_wire_and_a_stated_one_round_trips`; `am::cadl::tests::occurrences_omitted_on_a_non_root_node_is_carried_unstated_and_inferred_from_its_owner`, `…a_real_published_archetypes_definition_is_parsed_whole`; `am::validate::tests::an_unstated_occurrences_is_checked_against_the_value_aom2_infers`. **Scope:** the Reference Model multiplicity half of the rule is the one stated assumption — an attribute built without a cardinality is single-valued (`A-71`'s residual) |
