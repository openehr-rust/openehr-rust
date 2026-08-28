# News

Release notes with their reasoning live in [`CHANGELOG.md`](CHANGELOG.md); this
page is the short form, plus what is coming and how to reach a human.

## GitHub Sponsors is open, 2026-08-28

Under the maintainer's personal account —
[github.com/sponsors/joelparkerhenderson](https://github.com/sponsors/joelparkerhenderson) —
verified live rather than assumed, and now the target of `.github/FUNDING.yml`
and the repository's "Sponsor" button. There is still no legal entity behind
this project and still no other channel: **no Open Collective**. A profile
exists on that platform because the maintainer has one, the same as any user
of the site, but no Collective for this project exists and no fiscal host has
been chosen — do not trust a page claiming otherwise. What sponsoring does and
does not buy you is in [`CONTRIBUTING.md`](CONTRIBUTING.md#money).

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

## Latest release — 0.7.4, 2026-08-27

**openEHR granted permission to use their trademarks, and every notice now
says so in the Foundation's own words.** The grant is owner-reported and
recorded in [`TRADEMARKS.md`](TRADEMARKS.md), new at the root. At the
owner's direction, every notice site — the crate descriptions and rustdoc
on crates.io and docs.rs, the crate READMEs, and every root and help
document — carries the attribution openehr.org/logos/ prescribes, verbatim:
"openEHR® is the registered trademark of the openEHR Foundation and is used
with the permission of openEHR International. Use of the trademark does not
constitute endorsement of this product by openEHR International or openEHR
Foundation." Permission is not affiliation: the Independence statement in
`GOVERNANCE.md` stands unchanged. No code changed.

## Previous release — 0.7.3, 2026-08-26

**The trademark notice is everywhere a crates.io reader looks, in the
owner-specified shape.** Every publishable crate's `description` — what
crates.io shows in search results and at the top of the crate page — now
reads `<short description>. <notice> This project is an independent work.`,
the crate READMEs open with the notice as a blockquote, and
`scripts/check-trademarks.py` enforces the description shape in CI.

0.7.2, released earlier the same day, put the notice into the descriptions
but not in that final shape: the closing independent-work sentence was
absent and `openehr-mysql`'s description lacked a full stop before the
notice. A published version is immutable, so 0.7.3 is the remedy. No code
changed in either release.

## Recent releases

| Version | Date | Headline |
| --- | --- | --- |
| **0.7.4** | 2026-08-27 | every notice becomes the Foundation's prescribed attribution, stating the granted permission |
| 0.7.3 | 2026-08-26 | crate descriptions carry the notice in the owner-specified shape, checker-enforced |
| 0.7.2 | 2026-08-26 | the notice reaches the descriptions and gets prominent in the crate READMEs |
| 0.7.1 | 2026-08-26 | the owner-specified trademark notice ships on every crate page |
| 0.7.0 | 2026-08-26 | the Archetype Model is in scope (`lib:S1.21`, §15); `openehr::am` is its object model |
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

- Eight crates on crates.io at 0.7.4, released 2026-08-27, implementing the
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

openEHR® is the registered trademark of the openEHR Foundation and is used with
the permission of openEHR International. Use of the trademark does not
constitute endorsement of this product by openEHR International or openEHR
Foundation.
