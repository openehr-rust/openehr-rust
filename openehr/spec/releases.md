# Specification release provenance

Non-normative. `S1.16` and `K15.2` are.

This file exists because "which openEHR specification release does this crate
implement" turned out to have six different, mostly unrecorded answers rather
than one. `terminology.rs` names its source, its file, and the date it was
read; nothing else in the tree did, until this file was built by checking each
area directly against `openEHR`'s own specification repositories rather than
assuming a citation existed because a similar one did elsewhere.

**Checked 2026-09-06** against the GitHub API — a repository's tags/releases
endpoint for the latest numbered release, and its commits endpoint, per file,
for whether that file has changed on `master` since that release was cut. A
file that has changed since the last tag is not a claim that the change
matters to this crate; it is the fact a re-vendor check needs to decide
whether it does.

## The table

| Area | Crate module(s) | What is cited | Read/pinned as | Latest tagged release |
| --- | --- | --- | --- | --- |
| **RM** | `src/rm/`, `src/base/` (partially) | No `openEHR/specifications-RM` file citation exists anywhere in the tree | Only a version *target*: "RM 1.1.0" (`S1.16`), no file or date | `Release-1.1.0`, 2020-09-29 — matches the targeted number, but no citation ties any specific RM file to any specific read |
| **BASE** | `src/base/` | `openEHR/specifications-BASE`, named files (e.g. `org.openehr.base.resource.*.adoc`, `org.openehr.base.foundation_types.interval.adoc`) | Repo + filename only, no date | `Release-1.2.0`, 2021-04-09 — **the `interval.adoc` file this crate cites does not exist at that tag.** It exists only on `master`, last changed 2026-07-23 — nearly five years after the last numbered release. Whatever was read was draft content, not a release |
| **AM** | `src/am/` | `openEHR/specifications-AM`, many named `.adoc` files; `openEHR/adl-antlr`'s grammar files; `openEHR/adl-archetypes` for the corpus | Repo + filename, no date, **except** the corpus run, which is commit-pinned (`093c77ea003742b9540e3dd377d615e2b26f2996`, `openehr/spec/corpus.md`) | `Release-2.3.0`, 2024-03-20 — two sampled `.adoc` files this crate cites (`c_primitive_object`, `archetype_hrid`) exist at that tag, but both have since changed on `master` (last commit 2026-03-31, a different blob than the tag's) |
| **TERM** | `src/terminology.rs` | `openEHR/specifications-TERM`, `computable/XML/en/openehr_terminology.xml`, **read 2026-07-31** — the one citation in the tree with both a file and a date | Dated, not commit-pinned | `Release-3.0.0`, 2023-06-26 — the cited file does not match that tag either (different blob; last real change 2024-09-22, over a year after the tag). The 2026-07-31 read was of `master`, and `master` has not moved since, so re-reading it today reproduces exactly what was read then |
| **QUERY** (AQL) | `src/aql.rs`, `src/path.rs` | **Nothing.** No file, repo, or date names any AQL specification source anywhere in the tree | — | `Release-1.1.0`, 2021-05-14 |
| **ITS-REST** | `openehr-loco` | **Nothing.** No file, repo, or date names an ITS-REST specification source anywhere in the tree, crate or spec | — | `Release-1.1.0`, 2026-07-19 — **six weeks old as of this file.** `openehr-loco`'s own base-path uncertainty (`/openehr/v1` here, `/rest/openehr/v1` expected by tooling — `tasks.md`'s "ITS-REST completeness" item) was never checked against this or any other ITS-REST release |

## What this does and does not close

**Closes**, as evidence: the table itself, referenced from `S1.16` and `K15.2`.

**Does not close `K15.2`.** The requirement asks the crate to name the AOM
release it targets, the way `S1.16` names RM 1.1.0. The honest answer, checked
today, is that it does not target a numbered release at all — every AM
citation reads `master` at an unrecorded time, and `master` has moved past the
last tag (`Release-2.3.0`) since. Stating "AOM 2.3.0" here would be exactly the
kind of claim this repository's own culture forbids: a number nobody checked
the crate's actual behaviour against. `K15.2` stays open, now for a specific,
verified reason rather than a general one.

**Does not close `S1.16`'s own file-level gap.** RM 1.1.0 is the targeted
*version*, correctly, but no `specifications-RM` file backs any specific piece
of `src/rm/`. The one dated RM note in the tree
([`03-data-types.md`](03-data-types.md)) cites a different kind of source
entirely — the *Data Types Information Model* Release 1.0.2 print publication,
Revision 2.1.1, 20 Nov 2008 — for the Quantity classes only, not the RM
generally.

**Names two areas with no citation at all**, which is worse than a stale one:
QUERY and ITS-REST. Nothing in this tree has ever been checked against an AQL
grammar or an ITS-REST specification file. `openehr-loco`'s REST surface and
`src/aql.rs`'s grammar were both written from general knowledge of openEHR
rather than against a named, dated source — a fact the "ITS-REST completeness"
item in `tasks.md` already treats as work to do, and this file is what
confirms exactly how little grounding that work starts from.

## Re-vendor check

When re-reading a module against its specification, in the order a change is
most likely to matter:

1. **ITS-REST** — the only area whose latest release postdates this crate's
   own work on it (`Release-1.1.0`, 2026-07-19). Read it before writing another
   line of `openehr-loco`'s REST surface, rather than after.
2. **QUERY** — no citation exists to go stale; the first read is also the
   first citation.
3. **BASE** and **AM** — `master` has diverged from the last tagged release in
   both, confirmed by sampling one file in each. A full re-vendor pass would
   need to check every cited file, not just the ones sampled here, and decide
   file by file whether the divergence changes anything this crate enforces.
4. **RM** — targeted version (1.1.0) is stable and matches the latest tag
   exactly; lowest risk of the six, but also the one with no file citations to
   even check for drift.
5. **TERM** — already the best case: dated, and confirmed unchanged since that
   date. Nothing to do until the next scheduled read.

## Trademarks

openEHR® is the registered trademark of the openEHR Foundation and is used
with the permission of openEHR International. Use of the trademark does not
constitute endorsement of this product by openEHR International or openEHR
Foundation.
