# openEHR concepts, especially if you know FHIR

Not normative. [`openehr/spec/`](../openehr/spec/index.md) is.

This exists because the persistence specification in this repository was
imported from a FHIR project and text-substituted, and the result reads as
openEHR while requiring FHIR. Knowing where the two models genuinely differ is
how you spot the rest of that.

## The one difference everything follows from

**FHIR fixes the shape of a resource at specification time. openEHR does not.**

A FHIR `Patient` has the fields the specification says, so you can generate a
relational schema from the specification, shred a resource into typed columns,
and index anything.

An openEHR `COMPOSITION` contains whatever its **archetype** says. Archetypes are
authored by clinicians, published separately, and arrive long after the software
ships. There is no fixed shape to shred into.

So the storage model here is:

- The **canonical JSON is the record**.
- The relational part is an **index** over the attributes the Reference Model
  itself fixes — who committed, when, which archetype, which category, which
  setting.

A schema shredded from the Reference Model alone would have one column per RM
attribute and a key/value table for everything clinically interesting: a document
store with extra joins, plus a migration every time an archetype is published.

If you see a requirement demanding generated per-attribute tables, thousands of
tables, or names like `patient_name_given`, it is FHIR text that survived the
substitution.

## Vocabulary

| FHIR | openEHR | Note |
| --- | --- | --- |
| Resource | `COMPOSITION`, `EHR_STATUS`, `FOLDER`, … | openEHR has ~90 RM classes, not resource types |
| Profile / StructureDefinition | **Archetype** (ADL), **Template** | partly implemented: the object model, validating an instance against one already in memory, and a reader for an archetype's `definition` section that parses 967 of 1,379 published ADL 2 files as of 2026-09-04 (`openehr::am`, `openehr/spec/corpus.md`); no whole-archetype parser, flattening, or template expansion (`lib:A-40`) |
| FHIRPath | **openEHR path**, and **AQL** for query | paths are implemented; AQL parses but does not execute (`lib:S1.5`) |
| Search parameter | — | the composition index, filtered as an AQL `FROM` would |
| `meta.versionId` | `OBJECT_VERSION_ID` — `object::system::tree` | carries the *creating system*, which is what keeps two offline systems' "version 2" distinct |
| Provenance / AuditEvent | `AUDIT_DETAILS`, `CONTRIBUTION`, `ATTESTATION` | part of the model, not a separate resource |
| Bundle transaction | `CONTRIBUTION` | one change set over several versions |
| R4 / R5 | RM **1.0.2 / 1.1.0** | openEHR has no "R5". Seeing one is a substitution artefact |

## Things that will catch you

### Times are strings, and partial precision is a fact

`2024-05` is a date known to the month — a birth date on a refugee's record, a
diagnosis recalled as "sometime in May". It is **not** `2024-05-01`.

Storing it in a native `TIMESTAMP` silently completes it, which fabricates a
clinical fact, and normalises the lexical form, which breaks round-tripping. So
every stored instant occupies two columns: `…_text`, authoritative and exact, and
`…_utc`, derived and **nullable**.

The derived column is `NULL` whenever the instant is not established — a local
time with no offset, a date with no time — because that is the same answer
`DateTime::diff_seconds` gives in Rust. A column that guessed would make SQL
disagree with the library about one record.

### Comparison is partial, and refuses rather than guesses

A month-precision date is not ordered against a day inside that month. `5 mg` is
not comparable with `5 mL`. A path matching three elements fails rather than
returning the first. Each has a plausible wrong answer no downstream reader could
detect.

Expect `Option`, not `bool`, from comparisons.

### Absence is structured

openEHR has four null flavours — nobody looked, somebody looked and could not
find out, the value is withheld, the question does not arise. They are four
different clinical facts and the library will not let them collapse into one.

### Versions are append-only

A correction is a **new version**; the version it corrects stays. This is the
whole change-control model, and it is enforced in the database by triggers rather
than in application code — a guarantee that a SQL console can walk around is not
one.

`openehr_version` and `openehr_contribution` are append-only tables.

### Nothing prints PHI

No `Display` renders an identifier or a media blob. No error echoes a submitted
value. Redaction masks rather than deletes, and counts rather than names what it
withheld.

Store errors name identifiers, tables, and rules — never a patient's data —
because a store error is the one that reaches a connection-pool log, an APM
trace, and a paging alert at once.

## What is deliberately not implemented

| Not here | Why |
| --- | --- |
| AQL **execution** | needs archetype path resolution; AQL parses and statically checks, and returns no rows |
| UCUM unit conversion | a wrong conversion is a thousand-fold dosing error |
| External terminology lookup | needs a terminology server; codes are carried opaquely |
| HL7 `GTS`/`PIVL` timing evaluation | a partial timing engine produces a dosing schedule that is right most of the time |

This is the **library's** scope. `openehr-loco` is a separate, optional,
unpublished crate putting a narrow REST API in front of the store — it adds no
archetype validation, AQL execution, or anything else in this table; see its
own README.

**Archetypes and templates are a different case: not a permanent exclusion,
an open one.** `S1.4` — the decision to exclude the Archetype Model
outright — was withdrawn 2026-08-26, and `openehr::am` now implements the
object model, validating an instance against an archetype already in
memory or resolved through a repository you supply, and (`openehr::am::cadl`)
reading an archetype's `definition` section — 967 of the 1,379 ADL 2 files
in `openEHR/adl-archetypes` as of 2026-09-04, the rest refused by name
([`../openehr/spec/corpus.md`](../openehr/spec/corpus.md)). No
whole-archetype parser, no ADL 1.4 body, no flattening, no template
expansion — that gap is real and tracked as `A-40`
in [`../openehr/spec/audit.md`](../openehr/spec/audit.md), not asserted as a
permanent decision the way the four rows above are.

Where an operation is defined and not implemented, the code returns an
`Unsupported` error naming the spec section that records the exclusion. It never
returns a plausible default.

## Further reading

- [specifications.openehr.org](https://specifications.openehr.org/) — the
  authority. Note that the **rendered pages omit every class-definition table**;
  they `include::` them from a UML export, and the model itself is a binary
  `.mdzip`. Anything written from the prose alone is a guess — four findings in
  `openehr/spec/audit.md` came from exactly that. Go to the published PDF and the
  amendment record.
- [`openehr/spec/index.md`](../openehr/spec/index.md) — what this implementation
  decided, where openEHR left it open.
- `openehr/examples/` — five runnable tutorials.
- [`../openehr-skill/SKILL.md`](../openehr-skill/SKILL.md) — the same
  vocabulary as a Claude Code Skill, for a reader without a FHIR background
  to map from.
