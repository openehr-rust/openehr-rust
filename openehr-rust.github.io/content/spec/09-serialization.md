# 9. Serialization

Requirement prefix: `J9`.

openEHR canonical JSON (ITS-JSON) is what every openEHR implementation
exchanges. A type that serializes but does not read back cannot leave the
process.

## Round trip

- **J9.1** Every modelled type MUST round-trip: build → serialize → deserialize
  → compare equal.
- **J9.2** A second round trip MUST be byte-identical to the first. A serializer
  that changes the bytes on the second pass normalised something on the first,
  and normalising a clinical record silently is what must not happen.
- **J9.3** Partial dates, negative durations, fractional seconds, and explicit
  UTC offsets MUST survive verbatim (`D3.9`, `D3.10`).

## `_type`

- **J9.4** `_type` MUST be emitted wherever the declared attribute type is
  abstract or has subclasses, and on classes exchanged standalone —
  `COMPOSITION`, `EHR_STATUS`, `EHR_ACCESS`, `FOLDER`, `HISTORY`,
  `EVENT_CONTEXT`, `ACTIVITY`, `ISM_TRANSITION`, `INSTRUCTION_DETAILS`.
- **J9.5** `_type` MUST be **required** on input for `DATA_VALUE`. Guessing is
  not available: `{"value": "P1D"}` is a syntactically valid `DV_TEXT`,
  `DV_URI`, `DV_DATE`, and `DV_DURATION`, and picking one would turn a
  measurement into a string in a way nothing downstream could detect.
- **J9.6** Where `_type` is absent and the choice **is** decidable by shape, the
  crate MUST decide by the attribute that carries meaning and MUST NOT lose it:
  `defining_code` for `DV_TEXT` versus `DV_CODED_TEXT` (`M5.4`), `relationship`
  for `PARTY_RELATED` (`M5.13`), `::` count for `UID_BASED_ID` (`I2.20`).
- **J9.7** A `_type` naming a different class than the declared abstract type
  admits MUST be an error. A `_type` on a concrete class MAY be ignored, because
  there it is redundant.

## Reading is lenient, writing is canonical

- **J9.8** The crate MUST accept identifiers written as bare strings where
  openEHR specifies `{"value": …}` objects. Template tooling emits them, and
  rejecting them converts a cosmetic divergence into an import failure.
- **J9.9** The crate MUST ignore attributes it does not model rather than
  rejecting the document. openEHR adds attributes between minor releases, and a
  strict reader rejects tomorrow's payload for containing something it does not
  need.
- **J9.10** The crate MUST always **write** the canonical form, so that leniency
  normalises on round trip rather than propagating.
- **J9.11** Null and empty-list attributes MUST be omitted on output, as
  ITS-JSON requires.

## Canonical bytes for digests

- **J9.12** A canonical byte form MUST be defined for hashing (§11): object keys
  sorted by Unicode scalar value, no insignificant whitespace, array order
  preserved.
- **J9.13** Numbers MUST NOT be renormalised in the canonical form. Measured
  precision is data.
- **J9.14** Key ordering MUST NOT depend on locale. A locale-dependent digest is
  not reproducible across hosts.

## Recursion

- **J9.15** Deserialization is recursive and its stack cost is a function of
  document depth and build profile. The crate MUST document this and MUST NOT
  silently impose a depth limit (`S1.15`). The measured requirement and the
  method of measuring it are recorded in [`audit.md`](audit.md).
