# Contributing

This project is one person ([`MAINTAINERS.md`](MAINTAINERS.md)) and a large
specification tree. That combination means contributions are welcome and that
the bar is unusual: **the rules here are written down, machine-checked, and not
negotiable per patch**. Reading this page first is faster than discovering them
in review.

## What would help most, in order

1. **Run a dialect against your engine.** Five of the six SQL dialects are below
   **Store** level, and two — SQL Server and Oracle — have never had their DDL
   parsed by a server. `sh openehr-store/scripts/verify-schema.sh postgresql`
   (or `mysql`, `mariadb`) is one command; for SQL Server and Oracle there is no
   script yet, and a transcript of what the server said to
   `cargo run --example ddl` output would be a genuine contribution. The
   conformance ladder is the point of this project, and only evidence moves a
   crate up it.
2. **Review the DDL for the engine you administer.** Not the Rust — the SQL. A
   DBA who says "that index will not be used" or "that type does not preserve
   bytes on this version" is worth more here than a feature. `db:D-08` — MySQL
   rewriting a stored `1.10` as `1.1` — is exactly the class of finding this
   asks for, and it came from running something rather than reading it.
3. **Work on the Archetype Model** (`lib:A-40`). The specification is written
   ([`openehr/spec/15-archetypes.md`](openehr/spec/15-archetypes.md)), the AOM2
   object model exists as `openehr::am`, and 28 requirements have no code: ADL 2
   parsing, ADL 1.4 ingestion, flattening, template expansion, operational
   templates, validation of data against an archetype, and retrieval. Each of
   those is a self-contained piece of work with its requirements already stated.
4. **Use it and say what broke.** There is no reported production deployment.
   The most valuable issue this project can receive is a real one.
5. **Tell it what it has got wrong.** See [`RFC.md`](RFC.md).

## The five rules a patch is held to

They are the repository's own, and CI enforces most of them:

1. **Specification first** (`W0.19`). The requirement is written down before the
   commit lands. Discovering a requirement while implementing is normal; adding
   the behaviour without adding the requirement is not.
2. **Never claim more than is verified** (`W0.3`). Not in a README, not in
   rustdoc, not in a commit message. "The same code path works elsewhere" is not
   evidence.
3. **A gap not written down reads as a pass** (`W0.4`). If you find something
   wrong and cannot fix it now, it goes in the audit register with evidence.
4. **A test that cannot fail proves nothing** (`C0.10`). Show that it fails when
   the behaviour is removed or inverted — that is what `cargo mutants` checks on
   every pull request, and it has already stopped a release.
5. **The tree is at zero warnings.** Keep it there.

## Working in the repository

```sh
git clone https://github.com/openehr-rust/openehr-rust.git
cd openehr-rust/openehr && cargo test

# There is no root workspace: each of the eighteen crates is its own.
# `RUSTFLAGS` matters — a lint that only fires under `-D warnings` passes
# locally and fails in CI without it.
RUSTFLAGS="-D warnings" cargo clippy --all-targets

# Before opening a pull request, from the repository root:
for d in openehr openehr-store openehr-sqlite openehr-postgresql \
         openehr-mysql openehr-mariadb openehr-mssql openehr-oracle \
         openehr-loco openehr-assets; do
  (cd "$d" && cargo test --quiet \
     && RUSTFLAGS="-D warnings" cargo clippy --all-targets --quiet) \
    || echo "FAIL $d"
done
python3 scripts/check-docs.py          # counts, versions, levels, shared blocks
(cd openehr-assets && cargo run -- check)   # the committed DDL and coverage assets
```

[`AGENTS.md`](AGENTS.md) is the full operational guide and applies to human and
machine contributors alike; [`agents/`](agents/index.md) holds the topic guides —
adding an engine, conformance, publishing, openEHR concepts, auditing.

**Cite requirement ids** in code comments, commit messages, and test names, and
qualify them `lib:` or `db:` where the domain is not obvious (`W0.5`) — the two
specification trees allocate some of the same identifiers. A comment that
explains *why* a decision was made is the house style; when you change such
code, update the reasoning rather than deleting it.

**Disclose AI assistance** in the pull request description — which tool, and what
it did — per [`AI_STATEMENT.md`](AI_STATEMENT.md) §8. Nobody here will hold it
against you; an undisclosed one, discovered later, is a different conversation.

## Contributing time without writing Rust

- **Clinical and modelling review.** Whether the terminology groups, the null
  flavours, and the path semantics match how records are actually authored.
- **Specification review.** The tree is public and normative
  ([`spec/`](spec/index.md), [`openehr/spec/`](openehr/spec/index.md)). A
  requirement that is wrong, unenforceable, or missing is worth reporting even
  with no patch attached.
- **Documentation.** The tutorials run as tests; if one is confusing, that is a
  defect in a checked artefact.
- **Triage and reproduction.** Turning a vague report into a failing test is the
  work that most often unblocks a fix.

## Money

**There is no funding vehicle, and nothing here is asking for donations.** No
GitHub Sponsors, no Open Collective, no fiscal host, no legal entity, and no
account to send money to. Saying so plainly is better than an unmaintained
sponsor button.

If you want to fund work rather than wait for it, the useful shapes are:

- **Pay for verification you need.** The gap between **Schema** and **Store** on
  an engine is engineering time on a database you may already run. A commercial
  engagement to close it for your engine is a conversation:
  joel@joelparkerhenderson.com.
- **Contribute infrastructure rather than cash.** Access to a licensed SQL
  Server or Oracle instance would move two crates off the bottom rung of the
  ladder, which no amount of code review can do.
- **Fund the archetype work** (`lib:A-40`). It is scoped, specified, and large.

Any such arrangement is disclosed the way everything else here is: if a
requirement, a level, or a release was funded by someone with an interest in the
outcome, that fact belongs beside the claim.

## Licensing of contributions

Inbound matches outbound. By contributing you offer your work under the same
five-way choice the project publishes ([`LICENSE.md`](LICENSE.md)): MIT,
Apache-2.0, BSD-3-Clause, GPL-2.0-only, or GPL-3.0-only, at the recipient's
option. **There is no CLA and no copyright assignment.**

Do not contribute code you cannot license that way, including material copied
from another project or produced by a tool under terms that restrict it
(`AI_STATEMENT.md` §6).

## Conduct, and what to expect

Be straightforward and assume competence. Technical disagreement is welcome and
is settled by evidence — a specification citation, a failing test, or a server's
output — not by seniority or volume.

[`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) governs conduct — Contributor
Covenant 2.1, plus one clause specific to this project: overstating what the
software does is a conduct problem, not only a technical one. Security-relevant
reports go to joel@joelparkerhenderson.com rather than to a public issue, under
[`SECURITY.md`](SECURITY.md) — which also says what to expect, and what to do if
this project goes quiet on you. Who decides what, and how a disagreement is
settled, is [`GOVERNANCE.md`](GOVERNANCE.md).
