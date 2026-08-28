# 2. Identifiers

Requirement prefix: `I2`.

Identifiers are how openEHR names records, versions, archetypes, and
terminologies. They are exchanged as text, compared as text, and appear in every
REST path and every reference, so the rules about what text is accepted and what
text is produced are load-bearing rather than cosmetic.

## Parsing

- **I2.1** Every identifier type MUST be parsed into its parts at the boundary,
  not held as an opaque string. An unparsed `OBJECT_VERSION_ID` makes
  `preceding_version_uid` unverifiable, which is what lets a client commit a
  version whose parent belongs to a different versioned object.
- **I2.2** A parsed identifier MUST print back to exactly the text it was parsed
  from, except where a requirement below explicitly normalises.
- **I2.3** Input that would not survive `I2.2` MUST be refused rather than
  normalised.

## Primitive UIDs

- **I2.4** `UID` MUST be resolved to one of `UUID`, `ISO_OID`, or `INTERNET_ID`
  by lexical shape, tried in that order.
- **I2.5** `ISO_OID` MUST be tried before `INTERNET_ID`, because an all-digit
  dotted string satisfies both grammars and openEHR uses OIDs for issuing
  authorities.
- **I2.6** `UUID` MUST be five hexadecimal groups of lengths 8-4-4-4-12.
- **I2.7** `UUID` equality and hashing MUST be case-insensitive, and the stored
  text MUST keep the case it was given. Normalising case would change an
  identifier that a caller round-trips; comparing case-sensitively would fail to
  find a record keyed under the other case.
- **I2.8** `ISO_OID` arcs MUST be non-empty and all digits.
- **I2.9** `INTERNET_ID` labels MUST be non-empty, MUST consist of
  `[A-Za-z0-9_-]`, and MUST NOT begin or end with a separator.

## `OBJECT_ID` descendants

- **I2.10** `HIER_OBJECT_ID` MUST parse as `uid [ '::' extension ]`. The
  extension MUST be non-empty when present and MUST NOT contain `::`, because
  either would make the printed form re-parse into different parts.
- **I2.11** `OBJECT_VERSION_ID` MUST parse as exactly three `::`-separated
  parts: `object_id`, `creating_system_id`, `version_tree_id`.
- **I2.12** `VERSION_TREE_ID` MUST parse as `trunk` or
  `trunk.branch_number.branch_version`, every component a positive integer.
- **I2.13** `VERSION_TREE_ID` MUST refuse a leading zero. `01` and `1` denote
  one version, and accepting both would make two distinct strings name it.
- **I2.14** `ARCHETYPE_ID` MUST parse as
  `rm_originator '-' rm_name '-' rm_entity '.' domain_concept '.' version`, with
  the domain concept containing no `.`.
- **I2.15** `ARCHETYPE_ID` MUST accept a version of one, two, or three numeric
  components after the `v`. This **departs** from the BASE grammar, which gives
  one; ADL 2 archetypes carry three and the CKM publishes both, so the narrower
  grammar would reject identifiers that appear in real instance data. The
  guarantee that survives: the major version is always extractable, and the text
  round-trips (`I2.2`).
- **I2.16** `TEMPLATE_ID` MUST be non-empty and free of whitespace. openEHR
  gives its lexical form as "to be determined"; inventing a stricter grammar
  would reject valid identifiers from conformant tools, and accepting anything
  would let a missing template id look present.
- **I2.17** `TERMINOLOGY_ID` MUST parse as `name [ '(' version ')' ]` with both
  parts non-empty when present.
- **I2.18** `GENERIC_ID` MUST require both `value` and `scheme` to be non-empty,
  so that an identifier in an uncontrolled scheme at least says which scheme.
- **I2.19** `UID_BASED_ID` MUST admit only `HIER_OBJECT_ID` and
  `OBJECT_VERSION_ID`, as openEHR's typing of `LOCATABLE.uid` requires.
- **I2.20** Where `_type` is absent, `OBJECT_ID` MUST be inferred only between
  `HIER_OBJECT_ID` and `OBJECT_VERSION_ID`, by counting `::` separators. It MUST
  NOT guess `ARCHETYPE_ID`, `TEMPLATE_ID`, or `GENERIC_ID`, which are not
  distinguishable by shape.

## References

- **I2.21** `OBJECT_REF` namespaces MUST match
  `[a-zA-Z][a-zA-Z0-9_.:/&?=+-]*`.
- **I2.22** `local` and `unknown` MUST remain distinguishable: `local` asserts
  this system's identifier space, `unknown` records that the space was never
  captured. Collapsing them turns missing provenance into a claim of provenance.
- **I2.23** `PARTY_REF.type` MUST be one of `PERSON`, `ORGANISATION`, `GROUP`,
  `AGENT`, `ROLE`, `PARTY`, `ACTOR`, or `ANY`, enforced at construction rather
  than at validation. A `PARTY_REF` naming a clinical class is a call that
  should not have compiled.
- **I2.24** `LOCATABLE_REF.path` MUST be absent rather than empty to denote the
  root object, and the two MUST NOT both be representable.
- **I2.25** `LOCATABLE_REF` MUST render as the URI `namespace ':' id [ '/' path ]`
  without doubling a separator when the path already begins with `/`.
