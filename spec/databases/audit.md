# Persistence audit findings

**Rewritten 2026-08-01.** This file previously held ~70 KB of findings imported
from a FHIR monorepo and mechanically renamed. They described defects in a
different codebase — a shredder, a search-parameter compiler, six copied `map/`
trees — and included claims about this software that were never true of it, such
as "all **7,399 official OpenEHR example resources** (R3 + R4 + R5) round-trip".
Retaining them would have meant a findings register whose findings were fiction.
See [`spec/audit.md`](../audit.md) **W-04**.

**Non-normative.** This is the register of known gaps between what
[`spec/databases/`](index.md) requires, what the persistence documentation
claims, and what the code does. Repository-wide findings — anything spanning
crates or sitting above either domain — live in [`spec/audit.md`](../audit.md)
as `W-xx`. Library findings live in
[`openehr/spec/audit.md`](../../openehr/spec/audit.md) as `A-xx`.

A finding stays here until it is fixed or a requirement is amended to match
reality. Deleting one because it is inconvenient, or because the text that stated
it was rewritten, is the failure this file exists to prevent (`C0.20`).

## Numbering

- **D-xx** is this register's prefix, chosen because `F-xx` is taken (below) and
  `A-xx` belongs to the library.
- Identifiers are permanent and never reused (`C0.5`).

## `F-xx` refers to a different project

The code and documentation in this repository cite findings **F-01**, **F-06**,
**F-07**, **F-08**, **F-25**, and **F-26**. Those are **not** findings against
this software. They belong to the sibling FHIR monorepo, and they are cited here
because this repository's architecture is a direct response to them.

The citations are preserved rather than rewritten, because a citation's value is
that it does not change (`C0.5`), and because the lessons are the reason several
requirements exist. What they refer to:

| Id | The sibling monorepo's defect | What it caused here |
| --- | --- | --- |
| **F-01** | Six READMEs claiming a working store, a CLI, and 7,399 losslessly round-tripped resources — in ports where two had no store at all and none had ever had a CLI. | `C0.9`, `C0.11`: a crate states its level in the first screenful and may not describe a capability above it. |
| **F-06** | Two ports whose database CI jobs invoked a test target that did not exist, so they could not have passed and did not say so. | `C0.13`, `T11.13`: a check must fail rather than skip, and a self-skipping test is not evidence. |
| **F-07** | One port derived its hash-chain pre-image from `jsonb`, so no other port could verify its chains. | `X15.2`, `R4.11`: canonical form is computed in Rust, never delegated to the database. |
| **F-08** | An Oracle DDL emitter that produced MySQL types for the fork's entire life, because six ports each owned a full copy of the generator and nothing compared them. | The whole architecture: `X15.13`, `X15.14`, `M3.22`, `G2.8`. And `X15.15` — the comparison that catches it. |
| **F-25**, **F-26** | A migration path that could never have executed, in a port with no store to notice. | `C0.8`: **Dialect** level explicitly does not mean the DDL runs. |

**F-08 was reproduced in this repository anyway.** See `W-01`. That is the most
useful thing in this table: an architecture designed against a specific defect
still admitted it, because the guard that would have caught it was incomplete.

## Findings recorded elsewhere

Two sets of findings are persistence findings that live in another register, and
are **not** duplicated here. Renumbering them would break citations (`C0.5`).

| Ids | Register | Subject |
| --- | --- | --- |
| `A-13`, `A-14`, `A-15` | [`openehr/spec/audit.md`](../../openehr/spec/audit.md) | Found by running generated DDL against PostgreSQL 18 and MySQL 8.4: MySQL rejects `CREATE INDEX IF NOT EXISTS`, and three dialects enforced append-only nowhere. Recorded in the library register because that was the only register that existed at the time. |
| `W-01`, `W-02`, `W-04` | [`spec/audit.md`](../audit.md) | `openehr-mariadb` as a copy of `openehr-mysql`; the claimed-but-absent CI; this directory's import. All three span crates, so they belong to the repository register. |

They are listed rather than moved. A finding that changes number is a finding
whose citations in commit messages, test names, and code comments quietly stop
resolving.

## Open findings

### D-01 — No engine crate has a dialect annex — **Medium, open**

**Required.** `X15.6` requires every engine crate to carry
`spec/14-<engine>-dialect.md`, addressing nine subjects explicitly and by name,
with "not applicable" acceptable and silence not.

**Found.** All six `spec/` directories are empty. Every dialect decision — the
`ColTy` bindings, the idempotence declarations, the append-only mechanism, the
engine floor — is documented only in rustdoc, where it is not reviewable as a
diff against the core and where a departure cannot be declared as a numbered
`M14.x` requirement (`C0.14`).

**Consequence.** Undeclared departures are indistinguishable from oversights
(`C0.16`). `openehr-oracle` has an undeclared engine floor — identifiers were 30
bytes before Oracle 12.2 and 128 after, so the schema's names are only safe on
12.2+ and nothing says so.

**Disposition.** Open. Six annexes to write; `X15.6` lists what each must
contain.

### D-02 — Two store requirements are unverifiable as written — **Medium, open**

**Required.** `R4.5` (a multi-row read sees one snapshot) and `H5.4` (concurrent
commits produce one winner).

**Found.** `openehr-sqlite` reads inside a transaction and the unique index of
`H5.10` exists, so both are plausibly satisfied. **Nothing exercises either.**
No test in this repository runs two threads against one store.

Both are recorded as `?` in the [conformance matrix](conformance-matrix.md)
rather than `•`, which is the correct handling (`C0.20`) and not a fix.

**Disposition.** Open. `T11.6` states the test that would close it.

### D-03 — Tamper evidence is specified, built, and unused — **Low, open**

**Found.** `M3.16` requires a tamper-evident chain over committed versions. The
`openehr` crate implements the primitives in full — `Chain`, `ChainEntry`,
`ChainKey`, SHA-256 digests, HMAC tags compared in constant time, key material
zeroized on drop — and `openehr-store` references none of it. `openehr_version`
has no hash or tag column.

So the library carries the cost of a security capability that the persistence
layer does not use, and a reader who finds `security::audit_chain` may
reasonably assume stored history is chained. It is not.

**Why this is Low rather than High.** Nothing *claims* tamper evidence. `M3.16`
is marked unimplemented, `PR12.11` states explicitly that append-only is not
tamper evidence, and the conformance matrix lists it under "not implemented".
The gap is real; the misrepresentation is not.

**Disposition.** Open. Closing it means either wiring the chain into the schema
or recording a decision not to.

### D-04 — Read access is not audited — **Medium, open**

**Found.** `PR12.5` requires that a complete audit of access to clinical data
record reads, not only writes. This layer records no reads at all.

The version history looks like an audit trail and is not one: it records what
changed, and an access investigation asks who *looked*. A deployment that
assumed the history served the purpose would discover the gap during the
investigation.

**Disposition.** Open, and arguably out of scope for a storage layer — but
`PR12.5` states it because the assumption is easy to make and expensive to be
wrong about. A deployment needing read auditing must provide it above this
layer.

## Closed

### D-05 — The specification required the architecture the code rejects — **High, fixed**

**Found.** Every numbered section in this directory was imported from a FHIR
specification and text-substituted, and §2 and §3 therefore required a **shredded
schema** generated from specification packages — "7,355 tables for R5", child
tables per repeating element, `Patient.name.given` → `patient_name_given` — while
`openehr-store/src/schema.rs` argues at length against exactly that, correctly,
because archetypes are authored after the software ships.

A specification requiring the code to be wrong is worse than no specification: it
makes every conformance statement meaningless and invites someone to "fix" the
code toward it.

**Fixed.** All fourteen numbered sections rewritten against the code, with
withdrawn requirements keeping their numbers and listed per section (`C0.5`).
Two cross-cutting documents specifying text-search machinery this layer does not
have are marked withdrawn rather than deleted, because their reasoning would
apply directly if text search were ever added.

**Residual.** The rewrite was done by reading the code, not by re-deriving the
requirements from the openEHR specifications. Where a requirement here is now
merely a description of what the code does, that is a rubber stamp rather than a
considered generalization, and `C0.21` requires the distinction to be visible.
It is not, for this pass. A future review against the primary sources is the way
to close that — and the library register records that every such review so far
found the primary source contradicting what had been implemented.

## What this audit did not cover

Stated so "not examined" and "examined and sound" stay distinguishable (`W0.3`):

- **SQL Server and Oracle DDL** has never been parsed by the engine it names.
  Both crates are at **Dialect**, which is the correct level for that, and the
  gap is in evidence rather than a judgement that the DDL is wrong.
- **The `openehr` crate's Reference Model conformance** was not re-verified; it
  has its own register with seventeen findings.
- **Performance.** No benchmark exists and none is claimed (`T11.5`, withdrawn).
- **The rewritten requirements themselves** have not been reviewed by a second
  reader.

---

Part of the [openEHR persistence specification](index.md).
