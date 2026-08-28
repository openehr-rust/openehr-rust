# 5. Common and archetyping

Requirement prefix: `M5`.

## Where these invariants come from

As in [§3](03-data-types.md) and [§6](06-ehr.md): read from the **Release 1.0.2
publication** (*Common Information Model*, Rev 2.1.1, 20 Dec 2008, §3.2, §4.3,
§6.5), because the rendered RM page omits its class-definition tables.

## `LOCATABLE`

- **M5.1** Every clinical node MUST carry the `LOCATABLE` attributes: `name`,
  `archetype_node_id`, and optionally `uid`, `links`, `archetype_details`, and
  `feeder_audit`.
- **M5.2** `archetype_node_id` MUST be non-empty. A node without one cannot be
  reached by any path, which makes it invisible to AQL and to templates alike.
- **M5.3** `name` MUST be able to be a `DV_CODED_TEXT` as well as a `DV_TEXT`,
  because openEHR types it `DV_TEXT` and permits the subtype at runtime.
- **M5.4** Reading a `name` that carries a `defining_code` MUST NOT discard the
  code. Where `_type` is absent, the presence of `defining_code` MUST decide.
- **M5.5** `is_archetype_root` MUST be exactly "`archetype_details` is present",
  and `concept` MUST answer only at a root. Offering an internal node's name as
  a concept would misreport where the archetype boundaries are.

## `ARCHETYPED`

- **M5.6** `ARCHETYPED` MUST carry a parsed `ARCHETYPE_ID`, an optional
  `TEMPLATE_ID`, and a non-empty `rm_version`.
- **M5.7** Validation MUST report an `ARCHETYPED` whose archetype id constrains
  a different RM class than the node it annotates — an `OBSERVATION` archetype
  id on an `EVALUATION`. It is the commonest structural error in hand-built and
  transform-produced instances, nothing downstream detects it, and the data
  looks fine.
- **M5.8** `rm_version` MUST be carried and MUST NOT be enforced (`S1.16`).

## `LINK` and `FEEDER_AUDIT`

- **M5.9** `LINK.target` MUST be a `DV_EHR_URI` (`D3.31`).
- **M5.10** `FEEDER_AUDIT.original_content` MUST accept only a
  `DV_ENCAPSULATED` descendant. Accepting a `DV_TEXT` would invite a lossy
  stringification of the one thing that exists to be lossless.
- **M5.11** `FEEDER_AUDIT_DETAILS.system_id` MUST be non-empty: recording that
  data came from somewhere without saying where is the one fact it exists to
  carry.

## Parties

- **M5.12** `PARTY_PROXY` MUST admit `PARTY_SELF`, `PARTY_IDENTIFIED`, and
  `PARTY_RELATED`, and MUST infer the class where `_type` is absent by looking
  for `relationship`, then `name`/`identifiers`, then defaulting to
  `PARTY_SELF`.
- **M5.13** Reading a `PARTY_RELATED` as a `PARTY_IDENTIFIED` MUST NOT be
  possible: the relationship is the attribute that says whom the data is about.
- **M5.13a** `PARTY_RELATED.relationship` MUST come from the openEHR
  `subject_relationship` group (`Relationship_valid`). The primary constructor
  takes a **code**, so the rule holds by construction; a relationship coded
  against another terminology is readable and is checked by validation instead.
  This is the attribute that says whom an entry is about, so an unrecognised
  code means a finding may be attributed to the wrong person.
- **M5.14** `PARTY_IDENTIFIED` MUST require at least one of `name`,
  `identifiers`, or `external_ref`. A party with none of them identifies nobody.
- **M5.15** "Is this the record subject?" MUST answer true for `PARTY_SELF`
  **and** for a `PARTY_RELATED` whose relationship is `0|self|`. Both spellings
  occur, and code that checks only the first treats a self-related entry as
  being about somebody else.
- **M5.16** `PARTY_SELF` with no `external_ref` MUST be representable. It is how
  openEHR supports an anonymous record, and it is the representation a research
  extract needs — not a degenerate case to normalise away.

## Participation

- **M5.17** `PARTICIPATION` MUST carry `function`, `performer`, and optionally
  `mode` and a time interval.
- **M5.18** `PARTICIPATION.function` MUST be a `DV_TEXT` rather than a coded
  type, because openEHR types it that way: the useful vocabulary lives in
  external terminologies, and the openEHR group defines exactly one code.
- **M5.18a** Where `function` or `mode` **is** coded from openEHR's own
  terminology, it MUST be in the `participation_function` or
  `participation_mode` group respectively (`Function_valid`, `Mode_valid`). The
  check MUST NOT apply to a code from another terminology: rejecting a
  SNOMED-coded participation function would reject the commonest real case.
