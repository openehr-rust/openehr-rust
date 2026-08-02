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
  has its own register with seventeen findings.
- **Performance.** No benchmark exists and none is claimed (`T11.5`, withdrawn).
- **The rewritten requirements themselves** have not been reviewed by a second
  reader.

---

Part of the [openEHR persistence specification](index.md).
