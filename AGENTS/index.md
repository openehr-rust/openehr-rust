# AGENTS

Topic guides for working in this repository. Start with
[`../AGENTS.md`](../AGENTS.md), which covers the layout, the build commands, and
the rules that matter most.

**None of this is normative.** These files describe how to work. The
specifications decide what must be true (`W0.2`):
[`spec/index.md`](../spec/index.md),
[`spec/databases/`](../spec/databases/index.md),
[`openehr/spec/`](../openehr/spec/index.md).

## Guides

| Guide | Read it when |
| --- | --- |
| [adding-an-engine.md](adding-an-engine.md) | Adding a SQL engine crate, or changing a dialect. |
| [conformance.md](conformance.md) | Claiming or checking a conformance level. |
| [publishing.md](publishing.md) | Publishing to crates.io. |
| [openehr-concepts.md](openehr-concepts.md) | You know FHIR, or you are new to openEHR. |
| [auditing.md](auditing.md) | Auditing the tree, or you found something wrong. |

## The one-paragraph version

Eight crates, each its own Cargo workspace. `openehr` is the Reference Model.
`openehr-store` holds everything about persistence that is not a SQL spelling —
the five-table schema, the projection onto rows, the commit rules, and the
conformance suite. Six engine crates each supply one `Dialect`, owning exactly
four things: type spellings, identifier quoting, placeholder style, and
append-only enforcement. Only `openehr-sqlite` also has a `Store`.

## The failure this architecture is shaped by

The sibling FHIR monorepo gave each of six ports a full copy of the DDL
generator. One copy spent that fork's entire life emitting another engine's
types, because nothing ever compared the copies.

So here the shared logic lives once and the dialects own only spellings. That is
necessary and it was not sufficient: this repository reproduced the same defect
in `openehr-mariadb`, which was a name-substituted copy of `openehr-mysql`
emitting byte-identical DDL while claiming a MariaDB server had accepted it. The
guard that exists to catch exactly that compared five dialects, and this was the
sixth.

Two lessons are worth carrying into anything you do here:

1. **A guard is only as wide as its input list.** Check the guard's coverage, not
   just its result.
2. **A claim nobody re-ran is a claim nobody checked.** Every finding in
   [`spec/audit.md`](../spec/audit.md) was found by running something, never by
   reading something.
