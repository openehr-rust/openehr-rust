# News

Release notes with their reasoning live in [`CHANGELOG.md`](CHANGELOG.md); this
page is the short form, plus what is coming and how to reach a human.

## Specification change — the Archetype Model is in scope, 2026-08-26

`S1.4` excluded archetypes, ADL, templates, and archetype-constraint validation
from this crate since its first release. It is **withdrawn**, and
[`openehr/spec/15-archetypes.md`](openehr/spec/15-archetypes.md) now specifies
what was excluded: AOM2, ADL 2 and ADL 1.4, flattening, template expansion,
operational templates, validation against an operational template, and
retrieval from a repository such as CKM.

**The object model is built; the rest is not.** `openehr::am` implements AOM2 as
Rust types (`K15.1`–`K15.4`), checked at construction against the AOM2 validity
conditions one artefact can decide. **Twenty-eight of the thirty-two
requirements have no code** — no ADL parser, no flattening, no template
expansion, no operational template, no retrieval, and no validation of data
against an archetype. The matrix marks each `spec`, `lib:A-40` tracks them, and
until they close this crate validates at Reference Model level only, and says
so.

The exclusion's reasoning was kept rather than deleted: a partial constraint
engine would let "valid" mean "the parts I understood were satisfied". §15 is
written to answer that — every unimplemented construct, incomplete lineage,
unresolved artefact, and unreachable repository is a refusal, never a pass.

## Latest release — 0.7.0, 2026-08-26

**The Archetype Model is in scope, and its object model exists.** `S1.4` — the
requirement that this crate must *not* implement archetypes — is withdrawn, and
`openehr::am` implements AOM2 as Rust types: archetypes, the constraint tree,
multiplicities, and archetype terminology, with the AOM2 validity conditions one
artefact can decide checked at construction.

**Four of thirty-two requirements.** No ADL parser, no flattening, no template
expansion, no operational template, no retrieval — and no way to check that a
`COMPOSITION` conforms to the archetype it names. `lib:A-40` tracks the rest.

`unsafe_code` is now forbidden in all eighteen crates twice over: in every
manifest and at every crate root and fuzz target. The eight fuzz crates had no
lint table at all before this, so it had been forbidden in none of the 21 fuzz
targets while the documentation said the tree forbids it.

The release stopped at the mutation-testing gate first: 43 of 147 mutants
survived in the new module, including one whose failure would have refused
ordinary archetypes. Three tests killed them, and 0.7.0 went out from a run
where all 32 CI jobs passed.

## Previous release — 0.6.0, 2026-08-22

**The Reference Model's reals keep the digits they were written with.**
`DV_QUANTITY.magnitude`, `DV_SCALE.value`, `DV_PROPORTION.numerator` and
`.denominator`, and the accuracy fields are `openehr::base::Real` rather than
`f64`, so `1.50 mg` and `1.5 mg` are different records and hash differently
(`lib:D3.18d`).

The `f64` accessors are unchanged — `magnitude()` still returns `f64`, and
`magnitude_real()` is the new one — so code that reads magnitudes compiles
untouched. What changes is serialization: a document carrying `1.50` now
round-trips as `1.50`. That is why it is 0.6.0 and not 0.5.1. Records already
stored are unaffected; canonical JSON is byte-preserving (`db:M3.43`) and
verification hashes the bytes that were stored.

The release also waited on a red build: the `mutants` job caught
`DvQuantity::accuracy_real` surviving mutation — an accessor nothing tested —
and everything else was green. A published version is immutable, so it went out
after that was fixed rather than before.

## Recent releases

| Version | Date | Headline |
| --- | --- | --- |
| **0.7.0** | 2026-08-26 | the Archetype Model is in scope (`lib:S1.21`, §15); `openehr::am` is its object model |
| 0.6.0 | 2026-08-22 | reals preserve their digits (`lib:D3.18d`) |
| 0.5.0 | 2026-08-21 | AQL accepts negative numeric literals |
| 0.4.0 | 2026-08-21 | `PartialOrd` removed from every `DV_ORDERED` (`lib:A-35`); MSRV moved to the N−3 formula |
| 0.3.0 and earlier | 2026-08 | see [`CHANGELOG.md`](CHANGELOG.md) |

Eight crates are published and versioned in lockstep: `openehr`,
`openehr-store`, `openehr-sqlite`, `openehr-postgresql`, `openehr-mysql`,
`openehr-mariadb`, `openehr-mssql`, and `openehr-oracle`.

## How to follow this project

There is no mailing list and no newsletter. Watch
[releases on GitHub](https://github.com/openehr-rust/openehr-rust/releases),
follow the crates on [crates.io](https://crates.io/crates/openehr), or read
[`CHANGELOG.md`](CHANGELOG.md), which carries the reasoning that a release note
usually leaves out.

## Coming up

Dates below belong to other people's calendars and were checked on 2026-08-26.
Nothing here is a sponsorship, a partnership, or an announcement of a talk.

- **EHRCON26**, the annual openEHR International conference — 22–23 September
  2026, Meervaart Theatre, Amsterdam, with a pre-conference clinical modelling
  workshop on 21 September.
- **openEHR Collabrathon** — a two-day hybrid build starting 5 November 2026.
- Planned work in the repository itself is tracked as findings in
  [`spec/audit.md`](spec/audit.md); the open ones are the honest roadmap.

## Press and media contact

**Joel Parker Henderson** — joel@joelparkerhenderson.com — sole maintainer
([`MAINTAINERS.md`](MAINTAINERS.md)). One person, replying on one person's
schedule; say what you need and by when.

Available for: technical interviews on openEHR persistence, clinical data
precision in SQL engines, or specification-first development; review of a draft
that mentions this project; a written statement.

### Facts you can quote, and where each one is checkable

Every claim below is backed by something in the repository that can be run or
read. Nothing else about this project should be quoted as verified.

- Eight crates on crates.io at 0.7.0, released 2026-08-26, implementing the
  openEHR Reference Model in Rust with SQL persistence for six engines.
- **`openehr-sqlite` is at conformance level Verified** — a complete store,
  re-checked in continuous integration on every commit. Three dialects
  (PostgreSQL, MySQL, MariaDB) are at **Schema**: a real server executed the DDL
  and the append-only tables were observed refusing `UPDATE` and `DELETE`. Two
  (SQL Server, Oracle) are at **Dialect**: no server has parsed the DDL. The
  matrix that owns those levels is
  [`spec/databases/conformance-matrix.md`](spec/databases/conformance-matrix.md).
- **MySQL's `JSON` column type rewrote a stored clinical magnitude of `1.10` as
  `1.1`** — a precision loss independent of any digest, measured and recorded as
  `db:D-08`, and refused by the dialect checker as a result.
- The project publishes its own register of known defects
  ([`spec/audit.md`](spec/audit.md)) and a disclosure of how the code was written
  ([`AI_STATEMENT.md`](AI_STATEMENT.md)).
- It is maintained by one person, has no production deployment anyone has
  reported, and says so.

**What this project will not say**, whoever asks: that it is safe for patient
data, clinically validated, certified, compliant, or production-proven. None of
those is a property of a library, and the words have regulatory meanings.

### Citing this work

Use [`CITATION.cff`](CITATION.cff); GitHub renders it as a ready-made citation.

## Correcting something

If a claim on this page, or anywhere else in this repository, does not survive
checking, that is a report the project wants: open an issue and cite the file.
`W0.4` — a gap that is not written down reads as a pass.

## Trademarks

openEHR® is the registered trademark of the openEHR Foundation. Use of the
trademark does not constitute endorsement of this product by openEHR
International or openEHR Foundation.
