# 7. Demographics

Requirement prefix: `G7`.

## Separation from the EHR

- **G7.1** The EHR MUST reference parties by `PARTY_REF` and MUST NOT embed
  them. This is a privacy boundary as much as a modelling one: a clinical
  extract can be shipped with the demographic service left behind, and what
  travels is a record about an opaque identifier.
- **G7.2** Documentation MUST state that collapsing demographics into the EHR
  removes the ability to make that separation later, when it is needed and
  expensive.

## Parties

- **G7.3** `PARTY` MUST require at least one `PARTY_IDENTITY`.
- **G7.4** `PARTY` MUST require `uid`. Every reference from a clinical record is
  by uid, so a party without one is written and then unreachable.
- **G7.5** `PERSON`, `ORGANISATION`, `GROUP`, and `AGENT` MUST be distinct
  types, and `ROLE` MUST be a `PARTY` and not an `ACTOR`.
- **G7.6** `AGENT` MUST be documented as the class for devices and software. An
  automated ECG interpretation authored by a person is a misattribution of a
  machine reading.
- **G7.7** `GROUP` MUST be documented as distinct from `ORGANISATION`: a group
  is created by another party and has no independent legal existence.

## Names, contacts, capabilities

- **G7.8** `PARTY_IDENTITY.details` MUST be an archetyped `ITEM_STRUCTURE`
  rather than fixed name fields. Given/family splits, patronymics, mononyms, and
  generational suffixes are all legitimate, and a fixed schema excludes some of
  them.
- **G7.9** `CONTACT.addresses` MUST be non-empty.
- **G7.10** `CAPABILITY.was_valid_on` and `ROLE.was_held_on` MUST return
  *unknown* when no validity period is recorded, and MUST NOT return *yes*.
  These answer "was this clinician registered when they signed?", and silence is
  not a yes.
- **G7.11** `CONTACT.is_valid_on` MAY treat an absent period as always valid,
  matching openEHR's reading of an absent interval, because the consequence of
  being wrong is a failed contact attempt rather than a false assurance.

## Relationships

- **G7.12** `PARTY_RELATIONSHIP` MUST reference both parties by `PARTY_REF`, and
  MUST be stored with the source party as openEHR specifies.
