# Trademarks

The canonical trademark record for this project. The policy behind the
notice, the rules it imposes on every file here, and the check that enforces
them are in [`spec/professionalization/index.md`](spec/professionalization/index.md)
rule 5 and [`scripts/check-trademarks.py`](scripts/check-trademarks.py).

## Notice

> openEHR® is the registered trademark of the openEHR Foundation and is used
> with the permission of openEHR International. Use of the trademark does not
> constitute endorsement of this product by openEHR International or openEHR
> Foundation.

## What this project uses, and how

| Mark | Owner | Registrations | How it is used here |
| ---- | ----- | ------------- | ------------------- |
| openEHR® | the openEHR Foundation (openEHR International) | U.S. 4,272,380; EUIPO 002994853; IP Australia 939279 — verified at [openehr.org/logos/](https://openehr.org/logos/), 2026-08-26 | In the organisation, repository, and crate names (`openehr`, `openehr-store`, the six engine crates), and descriptively throughout, to name the specifications this software implements |

## Permission

**2026-08-27 — permission to use the trademarks granted by openEHR, and
referenced in the notice.** The grant is owner-reported: the correspondence
is held by the maintainer. Later the same day the owner directed that the
notice reference the permission, and the wording adopted is the
**Foundation's own prescribed attribution**, taken verbatim from
[openehr.org/logos/](https://openehr.org/logos/) (re-checked 2026-08-27) —
that page prescribes the exact formulation in the Notice above, so no
wording was composed here. The alternative, minimally editing the previous
owner-specified notice, was not used: where the mark's owner publishes its
own required text, using anything else would be a second formulation to
defend. `scripts/check-trademarks.py` enforces the new wording everywhere,
including every publishable crate's `description`.

Before this grant, the unqualified use of the mark in the project's names was
tracked as this repository's one item of third-party legal exposure
(`plan.md` §Risks) and asked openly in [`RFC.md`](RFC.md) §9. The grant
resolves the exposure question; the notice machinery it prompted stays.

## What this project does not claim

- **No endorsement, affiliation, or certification.** Permission to use a mark
  is not affiliation.
  [`GOVERNANCE.md`](GOVERNANCE.md) §Independence states it in full, and the
  conformance ladder used here — Dialect, Schema, Store, Verified — is this
  project's own, not an openEHR conformance assessment.
- **No logo, badge, or brand element** of the openEHR Foundation appears
  here. The grant recorded above is about the trademarks; nothing visual has
  been adopted.
- **No compliance or regulatory claim** rides on the name. `PHI.md` and
  `SECURITY.md` say what the software does; neither claims certification.

## Other marks

"Rust" and the Rust logo are trademarks of the Rust Foundation. This project
is written in Rust and is not affiliated with or endorsed by the Rust
Foundation.

PostgreSQL, MySQL, MariaDB, Microsoft SQL Server, Oracle Database, and SQLite
are marks or names of their respective owners, used here nominatively to
identify the database engines the dialect crates target. Product names in
[`COMPARISONS.md`](COMPARISONS.md) — EHRbase, FerroEHR, Archie, and others —
are likewise their owners' marks, used to identify the projects compared.

## If we have this wrong

Trademark owners are welcome to write to <joel@joelparkerhenderson.com>. A
correction here is treated the same way as any other defect report: acted on,
not argued with.
