# News

Release notes with their reasoning live in [`CHANGELOG.md`](CHANGELOG.md); this
page is the short form, plus what is coming and how to reach a human.

## The `definition` reader meets the published corpus, 2026-09-03

Later the same day, archetypes nobody here wrote: `openEHR/adl-archetypes`
(1,379 ADL 2 files, 593 ADL 1.4) run through `openehr::am::cadl`, with the
results, the corpus commit, and every refusal category recorded in
[`openehr/spec/corpus.md`](openehr/spec/corpus.md). The morning's reader
parsed 178 of the 1,379; by evening **774**, through two findings the run
produced. `lib:A-70`: a differential-form attribute (`/data/events
cardinality matches {…}`, how every specialised archetype states what it
redefines) was refused as a `VOKU` duplicate named `""`. `lib:A-71`
(**breaking**): AOM2's `C_OBJECT.occurrences` is optional and this crate's
was not, so an unstated one — most real nodes — could not be represented
at all and was refused; every `C_OBJECT` constructor now takes an `Option`,
and the new `lib:K15.32` says how the value is inferred and never invented.
Three breaking changes are now unreleased, all for 0.10.0. The corpus is read
where it is, never vendored — it carries no licence — and what it still
refuses is listed largest first, with the grammar reading that says which
refusals are the parser's fault.

## Archetype Model: a `definition` reader and slot checking, 2026-09-03

Twelve findings closed in one pass (`lib:A-58`–`A-69`), unreleased as of
this entry. `openehr::am::cadl` now reads an archetype's `definition`
section — every node kind but a `closed` slot and `SIBLING_ORDER`; every
primitive form including ISO 8601 literals, assumed values, and
`C_STRING` regexes; `C_ATTRIBUTE_TUPLE`; and an `ARCHETYPE_SLOT`'s own
`include`/`exclude` regex assertions — refusing anything else by name at
its offset, never returning a partial tree. `openehr::am::validate` now
evaluates a tuple's rows against instance data and a slot's `is_closed`
rule against whatever actually filled it, which `crate::path::Node` can
finally see. Two of the changes are breaking, so the next release is
**0.10.0**, not 0.9.1: `CPrimitive::String` lost a `pattern` field AOM2
never had, and `Archetype::archetype_id` is an `ArchetypeHrid`, AOM2's own
type for it. **Sixteen of §15's thirty-two requirements now have code;
sixteen do not** — no whole-archetype ADL parser, no ADL 1.4 body, no OPT,
no flattening, no template expansion. The
[matrix](openehr/spec/conformance-matrix.md), not this page, is the count
to trust.

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

## Latest release — 0.9.0, 2026-09-02

**Two breaking Archetype Model changes, decided rather than left open, plus
a real defect caught preparing the release.** `CObject::occurrences()` now
returns `Option<&MultiplicityInterval>`, not `&MultiplicityInterval` —
needed to represent AOM2's `C_COMPLEX_OBJECT_PROXY.use_target_occurrences()`
at all — and `CPrimitive::TerminologyCode` lost its `code_list` field, which
had no counterpart in AOM2's own single-valued `constraint` attribute.
Minor bump, not a patch, for the reason `0.6.0` and `0.4.0` already gave
their own breaking changes within `0.x`.

The rest of `openehr::am`'s growth this release is additive: co-varying
(tuple) constraints, soft terminology constraint statuses, `ARCHETYPE
.rm_overlay`, a proxy-node type, a bounded cADL parser for `definition`'s own
grammar rule (tested against a real published archetype's bytes, not an
invented one), and checking that a primitive object's assumed value actually
conforms to its own constraint. See `CHANGELOG.md`'s `## 0.9.0` entry for
each one, and `openehr/spec/audit.md` **A-50** through **A-57**.

**One real defect, found running the release checklist itself, not by a
research pass.** `cargo bench --benches -- --test` — the release-profile
smoke test `cargo test` never runs — failed two tests `cargo test` always
passed: a side-effecting token read sat inside `debug_assert!`, whose
argument a release build does not evaluate at all, so every ADL archetype
header with a `meta_data` clause silently failed to parse in any
release-profile build since the header readers were first added. Fixed
before the cut; see **A-57**.

## Previous release — 0.8.0, 2026-08-29

**The MSRV floor moves from N−3 to N−2 — 1.95 to 1.96.** Shipped as a minor
version, not a patch: `RV1` in
[`spec/rust-msrv-n-minus-2/index.md`](spec/rust-msrv-n-minus-2/index.md)
now tracks stable two releases back instead of three, and `RV6` forbids
shipping that as anything but a release a consumer expects to break in.
Cargo refuses the build on a floor beneath the new minimum rather than
miscompiling, but a dependency silently dropping support for a toolchain
is a thing worth reading about before it happens, not discovering from a
build error.

`rust-version = "1.96"` in all eighteen manifests, the `msrv` CI job's
derivation moved from `stable − 3` to `stable − 2`, and every prose
statement of the floor updated to match. Verified before publishing, not
only declared: `cargo +1.96 test --all-features` run for real across all
ten buildable crates — including `openehr-sqlite`, which compiles its own
bundled SQLite, and `openehr-loco`, whose `loco-rs` dependency needs only
1.94 — and CI ran green on the exact commit that cut the release before
any crate went out. No public API changed.

## 0.7.4, 2026-08-27

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

## 0.7.3, 2026-08-26

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
| **0.9.0** | 2026-09-02 | two breaking Archetype Model changes decided (`lib:A-54`, `lib:A-55`), plus a release-profile-only defect found and fixed (`lib:A-57`) |
| 0.8.0 | 2026-08-29 | the MSRV floor moves from N−3 to N−2 (`lib:RV1`); 1.95 to 1.96 |
| 0.7.4 | 2026-08-27 | every notice becomes the Foundation's prescribed attribution, stating the granted permission |
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

- Eight crates on crates.io at 0.9.0, released 2026-09-02, implementing the
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
