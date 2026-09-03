# Draft: reply in the FerroEHR thread, and the email to its author

**Status: draft, not sent.** Written 2026-09-03 for the maintainer to send by
2026-09-05 (`tasks.md` P0; `help/outreach/index.md` §11). Posting and emailing
are the maintainer's actions, not a tool's (`GOVERNANCE.md` §Machines do not
decide). Every factual claim below is one the tree makes today and
`scripts/check-docs.py` checks; nothing here says *safe*, *compliant*,
*certified*, *clinically*, or *fast* (`help/outreach/index.md` §1).

Thread: [FerroEHR – a new Rust-based openEHR CDR, looking for
testers](https://discourse.openehr.org/t/ferroehr-a-new-rust-based-openehr-cdr-looking-for-testers/17230)
(Implementation category, 21 posts as of 2026-08-27).

## Discourse reply (a reply in their thread, not an announcement)

> Ruben — congratulations, and thank you for post #16 in particular. "We wrote
> to spec" being *forbidden* as a way to close a failing conformance case, with
> the three-way attribution to server, runner, or catalogue, is the most useful
> sentence about conformance I have read on this forum.
>
> I maintain [openehr-rust](https://github.com/openehr-rust/openehr-rust),
> which is deliberately a different shape from FerroEHR: Rust libraries, not a
> CDR. The Reference Model as types (constructors validate, `Deserialize` does
> not and says so), paths, AQL parsing — parsing only, it does not execute —
> an engine-agnostic store, and DDL for six SQL engines, with each engine
> stated by level rather than by feature: SQLite is **Verified** (the full
> store, re-checked in CI on every commit); PostgreSQL, MySQL, and MariaDB are
> at **Schema** (a real server executed the DDL and the append-only tables
> were observed refusing `UPDATE`); SQL Server and Oracle are at **Dialect**
> (no server has parsed it yet). The Archetype Model is in scope since August:
> the AOM2 object model, validation of a composition against an archetype
> already in memory, and a reader for an archetype's `definition` section
> that, as of 2026-09-03, parses 774 of the 1,379 ADL 2 files in
> `openEHR/adl-archetypes` and refuses the rest by name, with every refusal
> category recorded — no whole-archetype parser, no OPT, no flattening yet.
> The matrix that owns
> every one of those claims is
> [here](https://github.com/openehr-rust/openehr-rust/blob/main/spec/databases/conformance-matrix.md),
> and the register of our own defects is next to it.
>
> Two things you may care about regardless of the crates:
>
> 1. MySQL's `JSON` column type rewrote a stored `DV_QUANTITY.magnitude` of
>    `1.10` as `1.1` — a clinical precision loss independent of any digest
>    (our `db:D-08`, with the reproduction). Our store refuses that column
>    type and the two others that reorder or rewrite bytes, because the
>    canonical JSON *is* the record. If your PostgreSQL 18 storage keeps
>    JSONB anywhere, the same question applies to it.
> 2. Our RM reals are a `Real` type that keeps the written text, not `f64`, so
>    `1.50 mg` and `1.5 mg` stay different records and hash differently.
>
> An offer, if it is useful: your conformance corpus has published JSON
> Schemas and fixtures. I would like to run every fixture through
> `openehr`'s canonical-JSON reader and `validate()`, and every CKM archetype
> you used through our `definition` reader, and report back what each side
> refuses — with the requirement id we refused it under, so you can attribute
> the disagreement to spec, to you, or to us. Where the released text is
> silent we keep the same kind of register you describe; the `is_modifiable`
> ordering in post #6 is exactly the shape of thing we would adopt, with your
> #2673 cited, rather than re-adjudicate.
>
> One question back: how does FerroEHR store a partial date such as
> `2024-05`? We use two columns — an authoritative exact text and a derived,
> nullable UTC instant — because `2024-05` is a date known to the month, not
> `2024-05-01`, and I have not found a second implementation to compare that
> against.
>
> Licence: [`LICENSE.md`](https://github.com/openehr-rust/openehr-rust/blob/main/LICENSE.md).
> How it was written, machine assistance included:
> [`AI_STATEMENT.md`](https://github.com/openehr-rust/openehr-rust/blob/main/AI_STATEMENT.md).

**Before posting, the maintainer checks:** the two links resolve on `main`;
the ladder sentence matches `spec/databases/conformance-matrix.md` on the day
of posting (it is the one file that owns levels, `W0.40`); and the account has
answered at least two other threads first (§5.1: "earn the post").

## Email to the author (§9: a question, not an announcement)

> Subject: how does FerroEHR store a partial date like `2024-05`?
>
> Hi Ruben — I maintain openehr-rust, a Rust implementation of the Reference
> Model plus SQL persistence (libraries, not a CDR; I replied in your thread
> today). We store every instant as two columns, an authoritative exact text
> and a derived nullable UTC one, because `2024-05` is known to the month and
> is not `2024-05-01`, and I would like to know what FerroEHR does on
> PostgreSQL 18. Two things I can offer in return: a measured case where
> MySQL's `JSON` type rewrote a stored `1.10` as `1.1` (our `db:D-08`), and a
> run of your conformance fixtures through our RM reader with every refusal
> attributed to a requirement id. Happy to share either way. — Joel

## What this draft does not do

- It does not compare performance. This tree runs benchmarks and asserts
  nothing about wall-clock, on purpose (`W0.35`, `W0.36`); if asked, that is
  the answer, not a number from a laptop.
- It does not promise dates for the ambiguities register, AQL execution, or
  OPT — all in `tasks.md`, none scheduled.
- It does not describe `openehr-loco` as an openEHR REST API; it has eleven
  endpoints and sits outside the conformance ladder (`W0.32`).

## Trademarks

openEHR® is the registered trademark of the openEHR Foundation and is used with
the permission of openEHR International. Use of the trademark does not
constitute endorsement of this product by openEHR International or openEHR
Foundation.
