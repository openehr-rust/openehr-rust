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

**Assessed 2026-08-02.** Anything not in this file is not claimed.

This file contradicted itself until 2026-08-02 — two requirements were marked
satisfied *and* listed as absent, and two rows described gaps that had been
closed. See [`audit.md`](audit.md) **D-09**. A CI check now refuses the first of
those; the rest is still assessed by hand.

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
| `R4.2` lossless round-trip incl. lexical instants | • | including the four `VERSION`/`AUDIT_DETAILS` attributes `D-07` found dropped |
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
| `M3.16` tamper-evidence chain | • | per container; a rewritten row fails to recompute |
| `M3.16c` checkpoint over the chain | • | a truncated chain verifies clean; only the checkpoint notices |
| `M3.39`–`M3.42` digest is SHA-256, 32 raw bytes | • | `ColTy::Digest`, binary in all six dialects |
| `O10.15` schema version recorded, mismatch refused | • | three states, all tested |
| `M3.33` projection refuses a non-archetype-root | • | |
| `M3.16d` content verified from the **stored bytes** | • | `tests/tamper.rs` edits a row through a second connection with the triggers dropped; `integrity`'s own unit tests catch 15 of 15 viable mutants (`lib:A-09`) |
| `M3.43` canonical JSON in a byte-preserving column | • | the store round-trips it; the per-engine claim is below |
| `M3.34` anonymous committer stored as `NULL` | • | |

## Service requirements

`openehr-loco` only. It sits **outside the conformance ladder** — every rung
there is defined by DDL, a `Store`, or a database server (`W0.32`) — so it
states evidence and takes no level. Publish status is a separate question
from ladder membership: `openehr-loco` has been on crates.io since
2026-09-01 (`W16.1`, amended), currently at `0.8.1` — `0.8.0` also shipped
that day but carries two RUSTSEC advisories permanently
(`agents/publishing.md`, `spec/audit.md` **W-20**), so it is not the version
to depend on.

| Requirement | Status | Note |
| --- | :-: | --- |
| `S1.20` deleted is `410`, never-existed is `404` | • | `tests/http.rs`, mutation-checked |
| `PR12.13` verifies an assertion, does not authenticate | • | relying party; no credential is held |
| `PR12.14` PASETO `v4.public`, never JWT | • | a `v4.local` token offered as `v4.public` is refused |
| `PR12.15` verification key only, no signing path | • | by construction; no secret key is loaded |
| `PR12.16` refuses to start with no verification key | • | observed by hand, and the verifier is built before the store |
| `PR12.17` no non-expiring token; audience may be bound | • | both tested |
| `PR12.18` verification is not authorization | • | no route consults who the token names |
| `PR12.19` committer must be the verified caller | • | `403` for another party, `422` for an unidentifiable one; mutation-checked |
| `PR12.21` no header may stand in for a token | • | spoofed identity headers, alone and alongside a token; mutation-checked |
| `PR12.5`, `PR12.6` read auditing, durability stated | • | recorded and flushed **before** the body is returned; a read that cannot be recorded is refused |
| `PR12.22` an access record names ids, never content | • | the response carries the composition, the log does not |
| `PR12.23` a failed read is recorded, distinguishably | • | `not_found`, `gone`, `refused` |
| `PR12.20` a token is not an audit trail | • | auditing is opt-in and `/metadata` says which |
| `H5.15` update requires a precondition; `412` not `409` | • | stale, absent, and `*` all tested |
| `H5.16` both `W/"uid"` and the bare uid accepted | • | |
| `PR12.12` tamper detection | — | the store's, not the service's |

## Cross-cutting

| Requirement | Status | Note |
| --- | :-: | --- |
| `X15.15` no two dialects emit the same DDL | • | all six compared |
| `X15.16` the comparison's coverage is asserted | • | `ENGINE_CRATES` count; added after **W-01** |
| `X15.19` types that differ in reality differ in code | • | booleans ≥ 4 spellings, JSON ≥ 3 |
| `T11.9` fuzzing, run not merely committed | • | 21 targets across 8 fuzz crates, seeded, in CI on every push; the seed corpora are themselves checked (`W0.30`, **W-15**) |
| `M3.22` schema declared once | • | |
| `M3.23` foreign keys point backwards only | • | |
| `M3.27` every `_text` has a nullable `_utc` partner | • | asserted over the whole schema |
| `M3.30` `ColTy` not `non_exhaustive`, no wildcard arms | • | by construction |
| `P6.18` no index over a column an engine cannot search | • | schema test; refuses `LongText`/`Json`, and a new `ColTy` fails to compile |
| `P6.13` every index records the query it exists for | • | schema test over `Index::note` |
| `W16.19` one licence expression, one licence file | • | checked in CI |
| `W16.15` `repository` names the real repository | ~ | correct since 0.1.1 and in the current 0.2.0; `openehr` **0.1.0 is published with the wrong one and is immutable** (**W-03**) |

## Not implemented in the store

Specified, and absent **from `openehr-store` and its engines**. A requirement
satisfied by `openehr-loco` above them can still appear here — `PR12.5` does —
because a program embedding the store directly gets the store's behaviour and
not the service's.

Listed so the gap is visible rather than inferred from silence (`W0.4`).

| Requirement | Subject | Note |
| --- | --- | --- |

| `M3.18` | GDPR Art. 17 erasure | no erasure operation |
| `PR12.5`, `PR12.6` | read auditing **in the store** | `openehr-loco` records reads above it (see the service table); a program embedding `openehr-store` directly still records none, which is the case `PR12.5` was written for |
| `O10.14` | schema migration | no migration mechanism, and none before 1.0 by decision. The applied version **is** recorded — see `O10.15` above |


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
