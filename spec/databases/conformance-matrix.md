# Conformance matrix

**Rewritten 2026-08-01.** The previous matrix recorded six ports satisfying
requirements no crate in this repository implements — lossless shredding
round-trips, snapshot reads, consumption audits, "94.8% of R5 measured on pg
only" — and cited requirement ids that are now withdrawn. It was imported with
the rest of the specification and described a different codebase. See
[`spec/audit.md`](../audit.md) **W-04**.

**Non-normative.** This file records what is true **today**, not what is
intended. Where it disagrees with a crate's documentation, this file is the one
to trust — a README is descriptive (`W0.2`), and READMEs are where the
overstatements have historically been.

**Assessed 2026-08-01.** Anything not in this file is not claimed.

## Legend

| Mark | Means |
| --- | --- |
| **•** | Satisfied, and a test demonstrates it. |
| **~** | Partially satisfied — the cell says how. |
| **?** | Appears implemented; **no test exercises it** (`C0.20`). |
| **✗** | Not implemented. |
| **—** | Not applicable to this crate. |

**?** is not a softer **•**. "The same code path works elsewhere" is not
evidence (`W0.3`).

## Conformance levels

| Crate | Level | Evidence |
| --- | --- | --- |
| `openehr-sqlite` | **Verified** | `conformance::run` against a real in-process database, in CI on every push |
| `openehr-postgresql` | **Schema** | DDL executed against PostgreSQL 18, in CI on every push |
| `openehr-mysql` | **Schema** | DDL executed against MySQL 8.4, in CI on every push |
| `openehr-mariadb` | **Schema** | DDL executed against MariaDB 11.4, in CI on every push |
| `openehr-mssql` | **Dialect** | golden tests only; no server has parsed it |
| `openehr-oracle` | **Dialect** | golden tests only; no server has parsed it |

**`openehr-sqlite` is at Verified** as of green run 30713623082, 2026-08-01. The three Schema
claims are checked by CI on every push rather than attested once; `openehr-mssql`
and `openehr-oracle` stay at Dialect because no reachable server will parse their
DDL ([`spec/audit.md`](../audit.md) **W-02**).

## Per-engine requirements

Columns: **pg** PostgreSQL, **lt** SQLite, **my** MySQL, **ma** MariaDB,
**ms** SQL Server, **or** Oracle.

| Requirement | pg | lt | my | ma | ms | or | Note |
| --- | :-: | :-: | :-: | :-: | :-: | :-: | --- |
| `G2.8` DDL derived from shared schema | • | • | • | • | • | • | no dialect defines a table |
| `G2.9` deterministic emission, golden tests | • | • | • | • | • | • | `tests/ddl.rs` in each crate |
| `G2.13` idempotence declared per statement kind | • | • | • | • | • | • | |
| `G2.15` declared `Guard` actually wraps | — | — | — | — | • | • | only mssql and oracle declare `Guard` |
| `G2.16` emitted clauses match the declaration | • | • | • | • | • | • | cross-dialect test, both directions |
| `G2.20` identifiers quoted by the dialect | • | • | • | • | • | • | |
| `M3.31` `Instant` ≠ `InstantUtc` type | • | • | • | • | • | • | cross-dialect test |
| `M3.36` append-only emitted for both tables | • | • | • | • | • | • | `check_dialect` fails a dialect that inherits the empty default |
| `M3.17` engine **actually refuses** UPDATE/DELETE | • | • | • | • | ✗ | ✗ | verified with a row present; mssql/oracle unparsed |
| `M3.37` no drop-then-create trigger window | • | • | ✗ | • | ? | ? | MySQL must drop first; MariaDB uses `CREATE OR REPLACE` |
| `X15.18` differs from nearest neighbour, tested | • | • | • | • | • | • | mariadb's is the newest and the reason `X15.18` exists |
| `T11.2` DDL executed against a real server | • | • | • | • | ✗ | ✗ | |
| `S1.4` engine floor declared | • | • | • | • | • | • | each stated in its annex, with the dialect fact that sets it |
| `X15.6` dialect annex exists | ~ | ~ | ~ | ~ | ~ | ~ | all six written; all six **proposed**, not ratified (`X15.9`) |

## Store-level requirements

Only `openehr-sqlite` implements `Store`, so the other five are **—**
throughout. A dash here means "this crate has no store", not "this crate fails".

| Requirement | sqlite | Note |
| --- | :-: | --- |
| `R4.8` whole canonical JSON stored | • | |
| `R4.11` canonicalized in Rust, not by the engine | • | shared `to_canonical_string` |
| `R4.12` reads reconstruct from JSON, not index columns | • | |
| `R4.13` validate before writing | • | |
| `R4.2` lossless round-trip incl. lexical instants | ~ | the **composition** round-trips; four `VERSION`/`AUDIT_DETAILS` attributes are accepted and silently dropped — `D-07` |
| `R4.4` commit is one transaction | • | |
| `R4.5` snapshot reads | • | a reader looping against a writer never sees a version without its index row |
| `H5.1` commit appends, never modifies | • | |
| `H5.8` commit rules refuse mis-parented / duplicate / stale | • | |
| `H5.10` unique index makes a duplicate fail in the database | • | |
| `H5.2` deletion is a new version | • | |
| `H5.12` `all_versions` oldest first | • | |
| `H5.13` `version_at_time` skips unestablished instants | • | |
| `H5.4` concurrent commits produce one winner | • | 8 racing writers, one winner, losers refused by the commit rules (`D-06`) |
| `P6.12` archetype lookup served by an index | • | |
| `P6.8` values bound as parameters | • | |
| `M3.33` projection refuses a non-archetype-root | • | |
| `M3.34` anonymous committer stored as `NULL` | • | |

## Cross-cutting

| Requirement | Status | Note |
| --- | :-: | --- |
| `X15.15` no two dialects emit the same DDL | • | all six compared |
| `X15.16` the comparison's coverage is asserted | • | `ENGINE_CRATES` count; added after **W-01** |
| `X15.19` types that differ in reality differ in code | • | booleans ≥ 4 spellings, JSON ≥ 3 |
| `T11.9` fuzzing, run not merely committed | • | 17 targets across 7 fuzz crates, seeded, in CI on every push |
| `M3.22` schema declared once | • | |
| `M3.23` foreign keys point backwards only | • | |
| `M3.27` every `_text` has a nullable `_utc` partner | • | asserted over the whole schema |
| `M3.30` `ColTy` not `non_exhaustive`, no wildcard arms | • | by construction |
| `W16.19` one licence expression, one licence file | • | checked in CI |
| `W16.15` `repository` names the real repository | ~ | fixed for 0.1.1; `openehr` **0.1.0 is published with the wrong one and is immutable** (**W-03**) |

## Not implemented

Specified, and absent. Listed so the gap is visible rather than inferred from
silence (`W0.4`).

| Requirement | Subject | Note |
| --- | --- | --- |
| `M3.16` | tamper-evident hash chain | the `openehr` crate has the primitives; the store does not use them and the schema has no hash columns |
| `M3.18` | GDPR Art. 17 erasure | no erasure operation |
| `M3.39`–`M3.42` | digest algorithm and storage | SHA-256, 32 raw bytes, binary column — no digest is stored anywhere yet, and adding one needs a new `ColTy` variant |
| `PR12.5`, `PR12.6` | read auditing | only writes are recorded; an access investigation asks about reads |
| `O10.14` | schema migration | no migration mechanism and no applied-version metadata |


| `T11.7` | redaction test over emitted logs | |
| `X15.10` | cross-engine logical agreement | untestable: only one store exists |
| `X15.11` | cross-engine chain verification | follows `M3.16` |

## How to read this file

1. Find the crate you are deploying, not the reference one. A requirement
   verified against SQLite is not thereby verified against Oracle.
2. Treat **?** as unverified. It is recorded rather than promoted precisely
   because promoting it is the failure this repository has committed most often.
3. Check [`spec/audit.md`](../audit.md) for open findings before trusting any
   row.

The single most useful line here: **two of six engine crates have never had a
statement parsed by the engine they name**, and every crate that *did* take that
step was found to be wrong at Dialect level — three of three.

---

Part of the [openEHR persistence specification](index.md).
