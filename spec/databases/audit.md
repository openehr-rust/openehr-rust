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

### D-01 — No engine crate has a dialect annex — **Medium, fixed**

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

**Fixed 2026-08-01.** All six annexes written, each addressing the nine
subjects `X15.6` names, and each carrying status **proposed** (`X15.9`) — so
none may be cited as evidence for a conformance level.

Writing them surfaced things that had not been written down anywhere:

- **Oracle's engine floor is 12.2**, and now for a stated reason. Identifiers
  were 30 bytes before 12.2 and 128 after; several generated names here exceed
  30, so the schema is not installable below it. That closes the open row in §1.
- **Four departures** now exist as numbered `M14.x` requirements where before
  they were undeclared: PostgreSQL and SQLite discard the `Id`/`Text` length
  bound (`M14.1`, `M14.5`); MySQL's drop-then-create trigger leaves a window
  (`M14.3`); Oracle cannot `=`-compare a `CLOB` and would need
  `DBMS_LOB.COMPARE` to satisfy `M3.42` (`M14.8`). `C0.16` calls an undeclared
  departure a defect; these are now declarations.
- **SQL Server's and Oracle's missing live runs** are recorded as `M14.6` and
  `M14.7` rather than left as an absence.

**Residual.** All six are **proposed**, not ratified. `X15.9` requires a
ratified annex before it counts as evidence, and ratification for the two
Dialect-level crates needs a live run that no available machine can provide.

### D-02 — Two store requirements were unverifiable as written — **Medium, fixed**

**Required.** `R4.5` (a multi-row read sees one snapshot) and `H5.4` (concurrent
commits produce one winner).

**Found.** `openehr-sqlite` reads inside a transaction and the unique index of
`H5.10` exists, so both are plausibly satisfied. **Nothing exercises either.**
No test in this repository runs two threads against one store.

Both are recorded as `?` in the [conformance matrix](conformance-matrix.md)
rather than `•`, which is the correct handling (`C0.20`) and not a fix.

**Fixed 2026-08-01.** `openehr-sqlite/tests/concurrency.rs` drives both, with a
**file** database and a connection per thread — an in-memory database is private
to its connection, so a concurrent test against `in_memory()` would run N
independent databases and pass without testing anything.

- `R4.5` — a reader loops against a writer committing 24 versions and asserts
  every version it can see has its index row visible too. **Passed first time**;
  the commit transaction of `R4.4` was already doing its job.
- `H5.4` — eight writers race for one position in a version tree. Exactly one
  wins. **This one failed**, and is recorded as `D-06`.

### D-06 — A concurrent commit refusal was reported as an engine error — **Medium, fixed**

Found by the test that closed `D-02`, which is the point of writing it.

**Found.** Eight writers racing for one position in a version tree produced
exactly one winner — so the guarantee held, enforced by the unique index of
`H5.10`. The seven losers received:

```
StoreError::Engine("SQLite: UNIQUE constraint failed: openehr_version.uid")
```

**Why that is a defect and not cosmetic.** `H5.9` requires the commit refusals
to be **distinguishable by the caller**. A caller told `Commit` knows another
writer took the position and can re-read the head and retry. A caller told
"UNIQUE constraint failed" knows only that something went wrong — and cannot
tell it from a corrupt schema, a disk error, or a bug. A version tree is
precisely where guessing is not allowed.

It is only reachable under concurrency: the single-threaded path checks the
commit rules before inserting, so both writers pass the check, and only the
database catches the second. Every existing test drove one store from one
thread, which cannot distinguish "the rules hold" from "the rules hold when
nothing else is happening".

**Fixed.** The SQLite store now translates a uniqueness violation into the
refusal it is, and the two indexes mean different things:

| Index | Means | Maps to |
| --- | --- | --- |
| `openehr_version.uid` | the same version identity committed twice | `CommitError::DuplicateVersion` |
| `ix_version_container_trunk` | a *different* identity took that position | `CommitError::NotLatest` |

**Residual.** Only SQLite is fixed and only SQLite is tested — it is the only
crate with a `Store`. The other five engines will need the same translation when
they gain one, and their drivers report constraint violations differently. The
requirement is `H5.9`; the shared conformance suite cannot check it until a
second store exists.

### D-07 — The store silently dropped four VERSION attributes — **High, fixed, properly**

Found by reading the openEHR **BMM** for RM 1.1.0 rather than the code, which is
what `D-05` asked for. The rendered specification pages omit every class table —
they `include::` them from a UML export — so
`specifications-ITS-BMM/components/RM/json/openehr_rm_1.1.0.bmm.json` is the
machine-readable source of record.

**Found.** RM 1.1.0 gives `VERSION`, `ORIGINAL_VERSION`, and `AUDIT_DETAILS`
attributes the `openehr` crate models as struct fields and the store persists
**nowhere**:

| Attribute | RM type | Modelled? | In the schema? |
| --- | --- | --- | --- |
| `AUDIT_DETAILS.description` | `DV_TEXT` | yes | no |
| `ORIGINAL_VERSION.other_input_version_uids` | `OBJECT_VERSION_ID` | yes | no |
| `ORIGINAL_VERSION.attestations` | `ATTESTATION` | yes | no |
| `VERSION.signature` | `String` | **only on `IMPORTED_VERSION`** | no |

All are optional in the BMM. **Optional is not droppable.**

The fourth row was a different defect and belonged to the library. A first pass
of this finding recorded `VERSION.signature` as modelled and dropped; checking
the field's owner before writing the fix showed it sat on `ImportedVersion`
alone, while the BMM puts `signature` on `VERSION`, which `ORIGINAL_VERSION`
inherits — so a locally created version could not be signed at all. Split out as
`lib:A-18` and since **fixed**: `OriginalVersion` now has the field, a
`with_signature` builder, an accessor, and a round-trip test.

Closing `A-18` made the fourth row real. A signature can now exist, and this
schema still has no column for it — so `refuse_unpersistable` rejects it too,
and the count here is four.

`data_json` holds the version's *content* — the `COMPOSITION` — not the `VERSION`
envelope, which is decomposed into columns (`M3.19`, `R4.8`). So an attribute
without a column has nowhere to go, and `commit_composition` returns `Ok`.

**Why this is High.** Two of the four are legally meaningful. An `ATTESTATION` is
a clinician asserting that content is what they signed off; `VERSION.signature`
is the signature over the version. A store that accepts an attested version,
returns success, and cannot give the attestation back has lost the part of the
record that made it evidence — silently, with no error and no documentation
saying so.

`AUDIT_DETAILS.description` is the free-text reason for a change: "corrected
after telephone call with the lab". It is often the only thing that explains why
a correction exists.

**What it does and does not violate.** `R4.2` requires an openEHR object written
and read back to equal the original. The `Store` trait does not offer read-back
as a `Version<T>` at all — `get_version` returns a `VersionRow` — so the store
never *claims* the round-trip for the envelope. The defect is therefore not a
broken promise so much as an **accepted input that can never be returned**,
undocumented, and reported as success. `S1.11` says an operation this layer does
not implement must refuse rather than return a silent success; that is the rule
being broken.

**Two remedies, and the choice is not mine to make quietly.**

1. **Refuse.** `VersionRow::project` rejects a version carrying any of the four,
   with `StoreError::Unsupported`. No schema change, no migration, and silent
   loss becomes an explicit refusal. It is a **behaviour change on published
   crates**: commits that succeed today would start failing.
2. **Persist.** Four columns, an `Attestation` needs its own table or a JSON
   column, and there is no migration mechanism (`O10.14`) for the eight crates
   already on crates.io at 0.1.1.

**Fixed by remedy 2 — the columns.** Refusal was the smaller evil and shipped
first, in 0.2.0. The schema now has the columns instead: `audit_description`,
`signature`, `attestations_json`, and `other_input_version_uids_json`. An empty
collection stores `NULL` rather than `[]`, because "not a merge" and "merged
nothing" are the same fact and SQL has one way to say it.

Verified against PostgreSQL 18, MySQL 8.4, and MariaDB 11.4: parses, idempotent,
append-only enforced, row intact. The seed in `verify-schema.sh` failed first —
it omitted the new `NOT NULL` chain columns — which is the check doing its job.

*Superseded text:* `VersionRow::project` refused a version carrying any
of the four, with `StoreError::Unsupported` naming the attribute and citing this
finding. It sits in the **shared** projection, so all six engines inherit it
rather than each needing the same check (`M3.35`).

Two tests: one asserting an audit description is refused, and a control
asserting a version without one still projects — because a refusal that rejected
everything would be indistinguishable from a broken projection (`T11.10`).

**This is a behaviour change on published crates.** Commits that succeeded at
0.1.1 and silently lost an attestation will now fail. That is the intended
direction: a caller told `Unsupported` can act, and a caller whose attestation
vanished cannot.

**Residual.** Refusing is the smaller evil, not a good outcome. openEHR permits
these attributes and this store still cannot hold them; remedy 2 — columns, and
a table or JSON column for attestations — remains the real fix, and needs a
migration mechanism this project does not have (`O10.14`).

### D-10 — three fuzz properties that could be deleted without failing anything — **Medium, fixed**

**Found 2026-08-21** by the retrospective mutation pass `spec/audit.md` **W-18**
left as a residual.

`check_projection`, `check_stored_instant`, and `check_verify_versions` are the
properties the `openehr-store-fuzz` targets drive. Each was **replaceable with
`()`** with nothing in this repository failing.

**Why that is worse than an ordinary coverage gap.** These functions are called
only from fuzz targets, and `cargo test` does not run fuzz targets. A property
that asserts nothing never crashes, so the `fuzz` job stays green — over three
properties that had been deleted. The gate and the thing it gates would both
have reported success.

That is `T11.10` exactly, and `W0.28` states it as a requirement: *a fuzz
property MUST be shown to fail against a deliberately broken implementation. A
check that cannot fail is indistinguishable from a control that works.* The
properties were written on 2026-08-20 and that demonstration was never done.

**Fixed, in three parts, and the third is a limitation rather than a fix:**

1. **`check_verify_versions` was too weak to fail at all.** Its assertions —
   empty means `Empty`, and the verdict is a function of its input — hold for
   any input given a correct `verify_versions`. It now **provokes** the answer
   it is about: for a history that verifies, editing each version's content in
   turn must make the chain report `ContentAltered` at that version. That is the
   entire purpose of `M3.16`, and it is falsifiable by construction rather than
   by imagination.

2. **`conformance::property_tests` now calls all three**, with input that is
   right and input that is not. `check_stored_instant`'s failing input is
   constructible — a `StoredInstant` whose derived UTC column disagrees with its
   authoritative text, which is the defect `M3.31` names — and the test builds
   one and asserts the property rejects it. That mutant is killed.

3. **The other two now report what they checked**, which is what makes them
   killable. `check_projection` returns whether the composition projected;
   `check_verify_versions` returns how many versions had their tamper detection
   provoked. `property_tests` asserts the count equals the history's length, so
   a stubbed-out property fails a real test.

   **A `mutants.toml` skip was written first and then deleted.** It was the
   weaker answer: a skip says *this cannot be checked*, and a return value makes
   it checkable. The counts are not an artifact of the testing either — the
   second distinguishes "verified a hundred-version history and re-broke every
   one of them" from "returned early", which the fuzz target could not otherwise
   tell apart.

   Asserting only the `true` case left `check_projection -> bool` replaceable
   with `true`, so the test asserts the other answer too: a composition with no
   `archetype_details` has nothing to put in `archetype_id` and is refused,
   which is a **result** and not a failure.

**Residual.** Three mutants in `dialects_are_distinct` and `check_dialect`
survive, unrelated to this finding and pre-dating it: those helpers are killed
when measured from `openehr-sqlite`, whose tests call them, and the library
conformance matrix's `T13.2` row already records that `openehr-store`'s own test
target does not.

### D-09 — The conformance matrix contradicted itself — **Medium, fixed**

Found by comparing the matrix against the specification it summarises, after a
day of changes had landed under it.

**Found.** The file that says *"Where it disagrees with a crate's documentation,
this file is the one to trust"* disagreed with **itself**:

- `M3.39` and `M3.42` were marked **•** in the store-level table — "digest is
  SHA-256, 32 raw bytes", with `ColTy::Digest` named as evidence — and listed
  under **Not implemented** as "no digest is stored anywhere yet, and adding one
  needs a new `ColTy` variant". Both statements were in the same file, and the
  second had been true when written.
- `O10.14`'s row said "no migration mechanism and **no applied-version
  metadata**" while `O10.15` was marked **•** for recording exactly that
  metadata and refusing a mismatch.
- `R4.2` was marked **~** because "four `VERSION`/`AUDIT_DETAILS` attributes are
  accepted and silently dropped — `D-07`", months after `D-07` was fixed. The
  matrix understated the crate.
- `PR12.5`/`PR12.6` were listed as absent after `openehr-loco` implemented read
  auditing above the store.

**Why it happened.** The matrix is prose. Every other claim in this repository
that drifted was prose too, and the ones that stopped drifting are the ones a
build reads — the disposition table, the invariant register, the licence check.
The matrix was assessed once, on 2026-08-01, and nothing has compared it to
anything since.

**Fixed.** Rows corrected, the session's new requirements assessed, and a CI
check added: **no requirement may be both marked satisfied and listed under Not
implemented**. That catches the exact defect and nothing else, which is the most
a check over prose can honestly do.

**Residual.** The matrix still records status by hand. The check proves it is
not self-contradictory; it cannot prove a **•** is deserved. `C0.20` already
says a mark is a claim about evidence, and the evidence is the test named in the
row — a reader who doubts a row should open it.

### D-08 — Two engines cannot return the bytes they were given — **High, fixed**

Found while building the tamper-detection test `PR12.12` demands, by asking what
`ColTy::Json` actually maps to on each engine rather than trusting that a column
called JSON stores JSON.

**Found.** The chain's content digest is `SHA-256` over the version's canonical
JSON (`M3.16`, `M3.23`). Canonical means *these bytes, in this order* — keys in
Unicode-scalar order, no whitespace, numbers as written. `ColTy::Json` mapped to
a **normalizing** column type on three of six engines, so what came back was not
what was hashed, and the digest was unverifiable from the database.

Verified against real engines, not from documentation:

| Engine | `ColTy::Json` was | Round-trips? | Observed |
| --- | --- | --- | --- |
| SQLite | `TEXT` | yes | — |
| **PostgreSQL 18** | `jsonb` | **no** | keys reordered, `": "` inserted |
| **MySQL 8.4** | `JSON` | **no** | keys reordered, **`1.10` → `1.1`** |
| MariaDB 11.4 | `JSON` | yes | alias for `LONGTEXT` |
| SQL Server | `nvarchar(max)` | yes | — |
| Oracle | `CLOB` | yes | — |

Two things make this worse than a digest bug.

**MySQL destroyed a clinical fact.** `1.10` became `1.1`. In openEHR a
`DV_QUANTITY` magnitude's trailing zero is precision — "1.10 mg" asserts
two decimal places and "1.1 mg" asserts one. That loss is independent of the
chain and would have corrupted the record whether or not anything hashed it.
No digest was needed to make this a defect; a digest is only what made it
visible.

**MySQL and MariaDB disagreed, and both spell it `JSON`.** These are the two
crates that `W-01` records as having once been byte-identical copies. The
cross-dialect guard was widened to catch that, and it did — it proves the two
now differ. Nothing checked whether the difference was *correct*. A guard that
asserts two things are not the same says nothing about which of them is right,
and here MariaDB happened to be right by inheritance while MySQL was wrong.

**The comments recorded the assumption that made it wrong.** All three crates
carried a variant of "nothing here relies on binary storage, since the canonical
bytes are regenerated from the parsed object rather than read back from the
column", citing `J9.12`. That was true when written and `D-03`/`D-07` made it
false — the chain now hashes bytes that must be reproducible *from storage* —
and no one revisited the comment, because a comment explaining a decision is not
a thing that fails.

The citation was wrong twice over. `J9.12` is `lib:J9.12` and was cited
unqualified against the `db:` tree, which `W0.5` forbids for exactly this reason
— there is no `db:J9.12`, so the citation resolved to nothing. And `lib:J9.12`
does not say what was claimed: it *defines* the canonical byte form and says
nothing about regeneration.

The requirement actually breached was next to it. **`lib:J9.13`: "Numbers MUST
NOT be renormalised in the canonical form. Measured precision is data."** MySQL
was renormalising them in the column. The rule was written, correct, and
unenforced at the only layer that could break it.

**Fixed.** `ColTy::Json` maps to a byte-preserving text type on every engine,
and `M3.43` now requires it. `SCHEMA_VERSION` is `4`.

The reasoning is the same as `M3.39`–`M3.42`, which forbid storing a digest as
hexadecimal text: a value whose bytes carry the meaning must not be handed to a
type whose contract permits reinterpretation. `jsonb` is the right type for a
document you intend to *query*; the relational columns are this schema's index
and the JSON is never queried as structure (`M3.20`), so nothing is given up.

**What this cost, stated plainly.** PostgreSQL loses `jsonb` operators and GIN
indexing over content. That is a real capability and it is not used here, but a
deployment that reached into `data_json` with `->>` will have to stop.

### D-03 — Tamper evidence is specified, built, and unused — **Low, fixed**

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

### D-04 — Read access is not audited — **Medium, fixed above the store**

**Found.** `PR12.5` requires that a complete audit of access to clinical data
record reads, not only writes. This layer records no reads at all.

The version history looks like an audit trail and is not one: it records what
changed, and an access investigation asks who *looked*. A deployment that
assumed the history served the purpose would discover the gap during the
investigation.

**Disposition.** *(2026-08-02 — **fixed above the store, still open in it**.)*
`PR12.5` always said a deployment must provide read auditing above the storage
layer. A layer above now exists: `openehr-loco` records every read before
returning it, fails the read when it cannot, and reports in its metadata which
of the two it is doing.

The store still records nothing and still cannot. A principal exists at the
service and nowhere below, so the finding stays open for anyone embedding
`openehr-store` directly — which is the case `PR12.5` was written for and the
one that has not changed.

### D-11 — Most requirements have never been assessed against the matrix — **Medium, open**

Found by writing the coverage check the library matrix already has
(`.github/workflows/ci.yml`, "Every requirement has exactly one row in the
library matrix") and pointing it at this file instead.

**Found.** This file's own header says *"Assessed 2026-08-02. Anything not in
this file is not claimed."* That sentence is safe against overclaiming — an
absent requirement asserts nothing false — but it is silent about how much is
absent, and the answer is most of it:
**144 of 221 requirements defined in `spec/databases/*.md` have zero mentions
anywhere in this file** — not a **•**, not a **✗**, not a **~**, not a **doc**,
not even a row in the "Not implemented" table. `scripts/check-databases-matrix-
coverage.py` reproduces the count.

By section:

| Prefix | Missing | Prefix | Missing | Prefix | Missing |
| --- | --- | --- | --- | --- | --- |
| `C0` | 21 of 22 | `M3` | 13 of 32 | `T11` | 14 of 17 |
| `G2` | 8 of 14 | `O10` | 13 of 15 | `W16` | 18 of 20 |
| `H5` | 7 of 16 | `P6` | 9 of 13 | `X15` | 10 of 18 |
| `S1` | 14 of 16 | `PR12` | 6 of 20 | `R4` | 4 of 11 |
| `V9` | 7 of 7 | | | | |

`C0` and `W16` are largely framework sections — normative language and
repository conventions — and the library matrix's precedent (`lib:C0.1`–`C0.18`
marked **doc**) suggests most of those 39 will resolve the same way once looked
at. That does not shrink the finding: the library matrix actually *has* a `doc`
row for each of its own `C0` requirements, stating that it is prose rather than
code. This file has never stated anything about 21 of its own 22. And the gap
is not confined to framework text — `M3.19`, *"The canonical JSON of a
version's content **is** the record"*, the sentence the whole storage model is
built on, has never been mentioned in the file that is supposed to record what
is true of the storage model today. Neither has `S1.1`, the core's founding
scope statement, nor `V9`, entirely — all seven of its requirements.

**Why it happened.** The library matrix is one linear walk through every
requirement in order, so a missing one is a visible gap in a sequence. This
file is five topic tables assembled by hand — per-engine, store-level, service,
cross-cutting, "not implemented" — and nothing ever walked the specification
section by section and asked, of each requirement, which table it belongs in.
`D-09` fixed this file *contradicting* itself; nothing has ever checked it for
*completeness*, because until `scripts/check-databases-matrix-coverage.py`
nothing could, cheaply, given the multi-table shape (`db:C0.20`'s "not thereby
verified" caveat is doing real work here, and so is
`scripts/check-databases-matrix-coverage.py`'s own docstring on why it checks a
floor rather than exact-once).

**Not fixed.** Closing this means reading each of the 144 requirements against
six engine crates and the store, and recording an honest mark — `•`, `~`, `?`,
`✗`, `—`, or `doc` — which is real assessment work, not something this finding
or a script can manufacture. Filing 144 guessed rows to make a count look
better would be a worse defect than the one being recorded (`W0.3`).
`plan.md`'s "Open decisions" names the recommended path: assess in batches by
section, starting with `M3` and `S1` since they are the requirements most
likely to already be silently satisfied by existing, tested code.

**Residual.** The diagnostic script is not wired into CI (see its own
docstring): it would fail on every push today for a pre-existing gap rather
than a regression, which is a different kind of red build than every other gate
in this repository asserts. Once the 144 are assessed, wiring it in is
mechanical — the library matrix is the working example.

## Closed

### D-05 — The specification required the architecture the code rejects — **High, fixed**

**Residual narrowed twice.** The rewrite was derived from the code rather than
from openEHR. Two reviews against primary sources have since run:

- **Terminology**, against `specifications-TERM`: five groups checked code for
  code — `audit_change_type`, `version_lifecycle_state`, `event_math_function`,
  `composition_category`, `setting` — all exact. One prose error found and fixed
  (`W-08`).
- **The Reference Model**, against the RM 1.1.0 BMM: `VERSION`,
  `ORIGINAL_VERSION`, `CONTRIBUTION`, and `AUDIT_DETAILS` attribute lists
  checked. RM 1.1.0 confirmed as the current release, so `S1.2` is correct. One
  **High** finding: `D-07`.

- **The RM invariants**, also from the BMM. The earlier note here said these
  would need the published PDF because the BMM only *names* them. That was
  wrong: the BMM carries the **expressions** —
  `Owner_id_valid: owner_id.value.is_equal (uid.object_id.value)` — for 68 of
  155 classes. No PDF was needed.

  Nineteen invariants across the nine classes this layer depends on were checked
  against the code. The result is not a simple pass count, because three
  categories behave differently:

  | Category | Invariants | Status |
  | --- | --- | --- |
  | Enforced, named in the code | `Lifecycle_state_valid`, `System_id_valid`, `Change_type_valid`, `Category_validity`, `Is_archetype_root`, `Setting_valid`, `Items_valid`, `Reason_valid` | ✅ |
  | **Vacuous in Rust** | `Attestations_valid`, `Other_input_version_uids_valid`, `Participations_validity`, `Items_valid` (partly) — all of the form `X /= Void implies not X.is_empty` | not applicable: Rust has no Void-versus-empty distinction for a `Vec`, so an empty vector *is* the absent case and the rule cannot be violated |
  | **Unenforced and undeclared** | `Territory_valid`, `Language_valid` | **`lib:A-19`** |

  `A-19` is the finding. `COMPOSITION` requires `territory` to be a member of
  `Code_set_id_countries` and `language` of `Code_set_id_languages`. The crate
  checks `CODE_PHRASE` well-formedness only, so `ISO_639-1::zz` is accepted
  although `zz` is not a language. These are code sets **openEHR names**, so
  `lib:S1.10` — which excludes external terminologies like SNOMED CT — does not
  cover them.

  Now declared as `lib:S1.18` with the reason: both code sets are mutable, and a
  table compiled into a library is wrong from the day a country changes.
  Validating against a stale copy would reject conformant data, which the
  crate's own `D3.5` reasoning calls the worse failure. Enforcement stays open.

  Also noted, not findings: `VERSION.owner_id` is a derived function in the BMM
  and the crate does not expose it, so `Owner_id_valid` has nothing to violate;
  and `VERSIONED_OBJECT.Uid_validity` (`extension.is_empty`) is satisfied by
  `HierObjectId` construction, which rejects an empty extension outright.

**The rest is now tracked rather than promised.** `assets/invariant-coverage.md`
is regenerated by `openehr-assets` and lists all 155 RM 1.1.0 invariants against
whether the crate names each one: 60 named, 95 not. `rm-1.1.0-invariants.json`
commits the expressions themselves, so the check needs no network and no PDF.

The 95 are **not** 95 findings. They mix three things — out of scope by a
declared exclusion, vacuous in Rust, and genuinely unenforced — and separating
them needs a human, which the report says rather than guessing. A first attempt
at that triage was itself wrong: a shell glob read only one directory level, so
`CODE_PHRASE.Code_string_valid` showed as missing while sitting in
`text.rs:52`. The tool now walks the tree, and that is why it is a committed
tool and not a one-off command.

What remains is the human triage of those 95, class by class.

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
  has its own register, tracked and counted separately (its own summary
  paragraph is checked against its own table by CI) — see
  [`openehr/spec/audit.md`](../../openehr/spec/audit.md).
- **Performance.** No benchmark exists and none is claimed (`T11.5`, withdrawn).
- **The rewritten requirements themselves** have not been reviewed by a second
  reader.

---

Part of the [openEHR persistence specification](index.md).
