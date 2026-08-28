# 6. EHR and composition

Requirement prefix: `E6`.

## Where these invariants come from

As in [§3](03-data-types.md), the rendered RM page omits its class-definition
tables. They were read from the **Release 1.0.2 publication** (*EHR Information
Model*, Rev 5.1.1, 16 Aug 2008, §4.7, §5.4, §8.3) and the EHR amendment record
checked for every change since: releases 5.1.2 through 5.2.1 changed
documentation, added `EHR.folders`, and added tags. None altered the invariants
below.

## `EHR`

- **E6.1** `EHR` MUST hold `OBJECT_REF`s to its status, access settings,
  compositions, contributions, and folders — not the objects themselves. A
  composition is a versioned object, and an EHR that embedded them would embed
  the whole record.
- **E6.2** `EHR.directory` MUST be the first element of `folders`, and setting
  folders from an empty list MUST be refused rather than leaving the two
  disagreeing.

## `EHR_STATUS`

- **E6.3** `is_modifiable` and `is_queryable` MUST be independent flags.
- **E6.3a** An `EHR_STATUS` MUST be an archetype root (`Is_archetype_root`).
  One without `archetype_details` cannot say which archetype shaped it, so
  nothing downstream can validate it against one.
- **E6.4** Documentation MUST state that deactivation — `is_modifiable = false`,
  on death, merge, migration, or opt-out — normally leaves the record
  **queryable**, because the history stays clinically and legally live. Code
  that treats deactivation as deletion removes a record law requires be
  retained.
- **E6.5** `EHR_STATUS.subject` MUST be a `PARTY_SELF`, permitting an anonymous
  record (`M5.16`).

## `COMPOSITION`

- **E6.6** `category` MUST come from the openEHR `composition_category` group,
  and the constructor MUST take a code rather than a `DV_CODED_TEXT`.
- **E6.6a** A `COMPOSITION` MUST be an archetype root (`Is_archetype_root`),
  for the reason given in `E6.3a`.
- **E6.6b** A **persistent** composition MUST NOT carry an `EVENT_CONTEXT`
  (`Is_persistent_validity: is_persistent implies context = Void`). A persistent
  composition is a running summary across many encounters — a problem list, a
  medication list — so attaching one encounter's context asserts that the whole
  list belongs to that visit. The specification states this twice: as a formal
  invariant, and in prose as *"Persistent Compositions do not have an Event
  context."*
- **E6.7** `language` and `territory` MUST be required. A composition that does
  not say what language it was recorded in cannot be safely rendered or
  translated.
- **E6.7a** openEHR additionally requires `language` and `territory` to be
  drawn from the `languages` and `countries` **code sets**
  (`Language_valid`, `Territory_valid`). This crate does **not** enforce
  either: both are external code sets, and resolving them is out of scope
  (`S1.10`). The codes are carried opaquely and checked only for
  `CODE_PHRASE` well-formedness. Declared here rather than left as a silent
  gap (`C0.12`).
- **E6.8** `content` MUST admit both `SECTION` and every `ENTRY` subclass, and
  the serialized form MUST be one flat `_type` dispatch rather than a nested
  one. A nested encoding makes a reader buffer the whole subtree per level.
- **E6.9** A traversal reaching every `ENTRY` however deeply nested in sections
  MUST be provided. An entry three sections deep is exactly as clinically
  significant as one at the top.
- **E6.10** `SECTION` MUST be documented as **navigational**: moving an entry
  between sections must not change what it says, and software deriving meaning
  from section membership breaks at the next template revision.

## `EVENT_CONTEXT`

- **E6.11** `setting` MUST come from the openEHR `setting` group.
- **E6.12** `end_time` MUST be refused when it is established to be before
  `start_time`, and MUST be accepted when the two are not comparable (`D3.14`),
  with validation reporting rather than the constructor guessing.
- **E6.12a** `location` MUST be absent or non-empty (`location_valid`). A
  present-but-empty location is indistinguishable from an absent one to every
  reader.

## Entries

- **E6.13** The five entry classes — `OBSERVATION`, `EVALUATION`,
  `INSTRUCTION`, `ACTION`, `ADMIN_ENTRY` — MUST be modelled, with the `ENTRY`
  attributes shared and the `CARE_ENTRY` attributes shared among the four
  clinical ones.
- **E6.14** An entry's `subject` MUST be explicit at construction. An entry that
  is about a family member and does not say so reads as a finding about the
  patient, and that error is invisible in every rendering.
- **E6.15** `ADMIN_ENTRY` MUST be distinguishable from the `CARE_ENTRY`
  subclasses, because a research extract that keeps clinical entries and drops
  administrative ones is making a defensible cut and one that cannot tell them
  apart is not.
- **E6.16** `OBSERVATION.data` MUST be a `HISTORY` even for a single reading, as
  openEHR requires. That is what makes a one-off blood pressure and a continuous
  arterial trace the same shape.
- **E6.17** `EVALUATION` MUST NOT have a `HISTORY`. An evaluation is an opinion
  held at the moment of recording; superseding it is a new version, not a new
  event.

## Instructions and actions

- **E6.18** `INSTRUCTION.narrative` MUST be required, and MUST be documented as
  the human-readable order rather than a summary of the activities. A narrative
  generated from structured activities records what the software understood, not
  what the prescriber meant.
- **E6.19** `INSTRUCTION.activities` MUST be non-empty.
- **E6.20** `ACTIVITY.action_archetype_id` MUST be carried as a **pattern**, not
  parsed as an archetype id: openEHR writes `/.*/` when any action will do, and
  validating it as an id would reject the commonest value there is.
- **E6.21** `ISM_TRANSITION.current_state` MUST come from the openEHR
  `instruction_states` group and `transition` from `instruction_transitions`.
- **E6.22** Terminal states MUST be identifiable, so that an `ACTION` recorded
  against an already-completed order can be flagged.
- **E6.23** `INSTRUCTION_DETAILS.activity_id` MUST be non-empty: an action that
  names the instruction but not the activity cannot say which step of a
  three-times-daily order it fulfilled.

## Folders

- **E6.24** `FOLDER` MUST hold references, so that more than one folder tree can
  classify the same composition without duplicating it.
