# openehr-rust — documentation index

Every entry point in the repository, in one place. If you know what you want,
this is the fastest route; if you do not, start with [`README.md`](README.md).

Eighteen crates, each its own Cargo workspace: the [`openehr`](openehr/)
Reference Model library, the engine-agnostic [`openehr-store`](openehr-store/),
six `openehr-<engine>/` dialect crates, the [`openehr-loco`](openehr-loco/)
HTTP service, the asset generator, and eight fuzz harnesses. Eight are
published to crates.io; [`agents/publishing.md`](agents/publishing.md) tracks
the version. [`spec/index.md`](spec/index.md) is the root of every
specification and says which one governs which code.

## By what you are doing

### Evaluating

| | |
| --- | --- |
| [openehr-rust.github.io](https://openehr-rust.github.io) | the project's landing page |
| [README](README.md) | what this is, in five minutes |
| [Conformance matrix — library](openehr/spec/conformance-matrix.md) | what the Reference Model crate actually satisfies today |
| [Conformance matrix — databases](spec/databases/conformance-matrix.md) | per-engine status; the only document that distinguishes the six |
| [PHI.md](PHI.md) | what the software does with patient data, in plain language, for a privacy or security review |
| [COMPARISONS.md](COMPARISONS.md) | how this relates to other openEHR software |
| [BENCHMARKS.md](BENCHMARKS.md) | what is measured, and what a number here does not claim |
| [INSTALL.md](INSTALL.md) | installing and first use |
| [The audit registers](spec/audit.md) | what has been found wrong, with evidence — also [library](openehr/spec/audit.md) and [databases](spec/databases/audit.md) |

### Building something

| | |
| --- | --- |
| [Reference Model tutorials](openehr/examples/) | five runnable examples: build, validate, paths and AQL, versioning, redaction |
| [Persistence tutorial](openehr-sqlite/examples/01_store_a_record.rs) | store and read a record — in `openehr-sqlite`, the only crate with a `Store` |
| [`openehr-store`](openehr-store/README.md) | schema, projection, commit rules, the conformance suite |
| [Adding an engine](agents/adding-an-engine.md) | what a dialect owns, and the five steps |

### Contributing

| | |
| --- | --- |
| [CONTRIBUTING.md](CONTRIBUTING.md) | ways to help, and the claims rule |
| [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) | conduct, including the claim-accuracy clause |
| [SECURITY.md](SECURITY.md) | reporting a vulnerability; never send patient data |
| [RFC.md](RFC.md) | the open questions this project wants answered |
| [GOVERNANCE.md](GOVERNANCE.md) | who decides, on what basis |
| [MAINTAINERS.md](MAINTAINERS.md) | one maintainer, stated plainly |
| [AGENTS.md](AGENTS.md) | how to work here — the operational guide |
| [Topic guides](agents/index.md) | engines, auditing, conformance, publishing, openEHR concepts |
| [AI_STATEMENT.md](AI_STATEMENT.md) | how this was written, machine assistance included |

### Implementing or auditing

| | |
| --- | --- |
| [Specification root](spec/index.md) | crate map, identifier namespaces, the conformance ladder, publishing |
| [Library specification](openehr/spec/index.md) | the Reference Model, `lib:` ids, sections 0–15 |
| [Database specification](spec/databases/index.md) | persistence, `db:` ids, sections 0–16 |
| [Compliance mappings](openehr/spec/14-compliance-mapping.md) | regulation → requirement → evidence — also [databases](spec/databases/13-compliance-mapping.md) |
| [Professionalization](spec/professionalization/index.md) | the rules this repository holds itself to, trademark notice included |
| [plan.md](plan.md) · [tasks.md](tasks.md) | the workstreams, and the verified execution queue |

## The crates, by conformance level

The level has one owner,
[`spec/databases/conformance-matrix.md`](spec/databases/conformance-matrix.md);
this table restates it and is checked against it.

| Crate | Level |
| --- | --- |
| [`openehr-sqlite`](openehr-sqlite/) | **Verified** |
| [`openehr-postgresql`](openehr-postgresql/) | **Schema** |
| [`openehr-mysql`](openehr-mysql/) | **Schema** |
| [`openehr-mariadb`](openehr-mariadb/) | **Schema** |
| [`openehr-mssql`](openehr-mssql/) | **Dialect** |
| [`openehr-oracle`](openehr-oracle/) | **Dialect** |

`openehr` and `openehr-store` are libraries outside the engine ladder;
`openehr-loco` states evidence rather than a level (`W0.32`).

## Reading order, if you have an hour

1. [README](README.md) — 5 min
2. [`openehr-store`'s README](openehr-store/README.md) — 15 min, the storage
   idea everything rests on
3. [The persistence tutorial](openehr-sqlite/examples/01_store_a_record.rs) —
   15 min, hands on
4. [The conformance matrices](spec/databases/conformance-matrix.md) — 10 min,
   what is actually true
5. [The audit registers](spec/audit.md) — 15 min, what was found wrong and how

## Trademarks

openEHR® is the registered trademark of the openEHR Foundation and is used with
the permission of openEHR International. Use of the trademark does not
constitute endorsement of this product by openEHR International or openEHR
Foundation.
