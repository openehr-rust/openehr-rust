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
| `openehr-mssql` | **Schema** | DDL executed against SQL Server 2022, in CI on every push |
| `openehr-oracle` | **Schema** | DDL executed against Oracle Database Free 26ai, in CI on every push |

**`openehr-sqlite` is at Verified** as of green run 30713623082, 2026-08-01. All
five Schema claims are checked by CI on every push rather than attested once —
**no engine crate remains at Dialect** ([`spec/audit.md`](../audit.md)
**W-02**, closed). `openehr-oracle` joined them 2026-09-06 (run
[34040865467](https://github.com/openehr-rust/openehr-rust/actions/runs/34040865467),
job `schema / oracle`) once `gvenzl/oracle-free` — a public, unauthenticated
image — was found; its DDL held up on first real contact, and every attempt
before that found nothing wrong with it, only with the script trying to
verify it. `openehr-mssql` joined them the same day (run
[34045294037](https://github.com/openehr-rust/openehr-rust/actions/runs/34045294037),
job `schema / mssql`), after seven rounds of real fixes — most in
`verify-schema.sh` itself, but one, `M14.6`/`D-12`, a genuine defect in this
crate's own DDL: `append_only_sql`'s `CREATE OR ALTER TRIGGER` shared a batch
with every statement before it, which SQL Server refuses outright. See
`spec/databases/audit.md` **D-12** for the full account.

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
| `M3.17` engine **actually refuses** UPDATE/DELETE | • | • | • | • | • | • | verified with a row present, every engine |
| `M3.37` no drop-then-create trigger window | • | • | ✗ | • | ? | ? | MySQL must drop first; MariaDB uses `CREATE OR REPLACE` |
| `X15.18` differs from nearest neighbour, tested | • | • | • | • | • | • | mariadb's is the newest and the reason `X15.18` exists |
| `T11.2` DDL executed against a real server | • | • | • | • | • | • | mssql and oracle since 2026-09-06, runs 34045294037 and 34040865467 |
| `S1.4` engine floor declared | • | • | • | • | • | • | each stated in its annex, with the dialect fact that sets it |
| `P6.4` all seven named indexes declared and emitted | • | • | • | • | • | • | `openehr-store::schema::TABLES`, one declaration, all six derive from it |
| `G2.7` the schema is compile-time Rust data, not generated | • | • | • | • | • | • | `openehr_store::schema`'s `TABLES` const |
| `G2.10` `ddl()` emits tables, then indexes, then append-only, in that order | • | • | • | • | • | • | the shared `Dialect::ddl` default — no dialect overrides it |
| `G2.11` indexes come after all tables, not per-table | • | • | • | • | • | • | the same `ddl()` default |
| `G2.12` tables emit in `TABLES` order, needing no deferred constraints | • | • | • | • | • | • | the same `M3.23` evidence, plus the same `ddl()` default |
| `G2.14` idempotence declares exactly `IfNotExists`, `Guard`, or `Inline` | • | • | • | • | • | • | the `Idempotence` enum's own shape |
| `G2.17` `Inline` is MySQL's answer; MariaDB does not use it | • | • | • | • | • | • | `openehr-mysql`'s `index_idempotence` returns `Inline`; MariaDB is left at the default |
| `G2.18` table and index names are identical on every engine | • | • | • | • | • | • | the same `X15.3` evidence |
| `G2.19` every identifier fits the tightest engine's limit | ? | ? | ? | ? | ? | ? | true by inspection — the longest name today is 29 bytes, well under Oracle's 30-byte pre-12.2 floor — no test would catch a longer one being added |
| `X15.6` dialect annex exists | ~ | ~ | ~ | ~ | ~ | ~ | all six written; all six **proposed**, not ratified (`X15.9`) |
| `M3.6` `ColTy` declared once, dialect maps it | • | • | • | • | • | • | `col_sql` implemented in all six; `X15.19`/`M3.31` test the mapped differences directly |
| `M3.29` `Id`/`Text` carry a maximum length | • | • | • | • | • | • | bounded by construction — `ColTy::Id(n)`/`Text(n)` take the length as a parameter, not by convention |

## Store-level requirements

Only `openehr-sqlite` implements `Store`, so the other five are **—**
throughout. A dash here means "this crate has no store", not "this crate fails".

| Requirement | sqlite | Note |
| --- | :-: | --- |
| `R4.8` whole canonical JSON stored | • | |
| `R4.11` canonicalized in Rust, not by the engine | • | shared `to_canonical_string` |
| `R4.12` reads reconstruct from JSON, not index columns | • | |
| `R4.13` validate before writing | • | |
| `V9.1` a store validates against the Reference Model before writing, refuses a failure | • | the same `R4.13` evidence |
| `V9.4` terminology validation is out of scope; no `CHECK` constraint enumerates a value set | • | no dialect emits `CHECK` anywhere in the schema — code columns are plain `Id(n)` |
| `V9.5` validation uses `openehr`'s own `validate()`, never a reimplementation | • | `commit_composition`'s `version.validate_ok()` — literally `Validate::validate().into_result()` |
| `V9.6` a refusal is a structured report: path, class, invariant | • | `error::Violation { path, class, invariant, detail }` |
| `V9.7` a validation report never contains a submitted value | • | `Violation::detail` is `&'static str` — a compile-time constant cannot embed a runtime value, structurally |
| `V9.8` construction-time checking alone is not the only guard | • | `commit_composition` validates the whole `Version`, not just its data — the fix `lib:A-23` made for JSON-arrived content that never went through a constructor |
| `V9.9` validation is Reference-Model-level only, never described as archetype/template validation | • | `/metadata`'s own `not_implemented` list names "archetype and template validation" (`lib:S1.4`) explicitly |
| `R4.2` lossless round-trip incl. lexical instants | • | including the four `VERSION`/`AUDIT_DETAILS` attributes `D-07` found dropped |
| `R4.4` commit is one transaction | • | |
| `R4.5` snapshot reads | • | a reader looping against a writer never sees a version without its index row |
| `H5.1` commit appends, never modifies | • | |
| `H5.8` commit rules refuse mis-parented / duplicate / stale | • | |
| `H5.10` unique index makes a duplicate fail in the database | • | |
| `H5.2` deletion is a new version | • | |
| `H5.12` `all_versions` oldest first | • | |
| `H5.13` `version_at_time` skips unestablished instants | • | |
| `H5.11` `is_deleted` is a derived, **indexed** column | ✗ | derived: yes. Indexed: no — `openehr_version`'s own three indexes none include it. The column's own comment claimed it was indexed; corrected in place while assessing this row, 2026-09-10 — no query filters on it yet, so nothing has felt the absence |
| `H5.14` a `CONTRIBUTION` is its own row, append-only | • | `openehr_contribution` (`M3.21`), append-only by the same `M3.17` mechanism |
| `H5.4` concurrent commits produce one winner | • | 8 racing writers, one winner, losers refused by the commit rules (`D-06`) |
| `H5.3` a store offers version-by-id, latest, at-time, every-version | • | the same `P6.11` evidence |
| `H5.5` every version is a row in `openehr_version`, no separate history table | • | one version table in the six-table schema (`M3.21`) — there is no second to split into |
| `H5.6` version identity stored whole and decomposed | • | `openehr_version.uid` (whole) plus `versioned_object_uid`/`creating_system_id`/`trunk_version`/`branch_number`/`branch_version` (decomposed) |
| `H5.7` `creating_system_id` stored, never dropped | • | its own required column, same table |
| `H5.9` commit refusals enforced and distinguishable | • | `openehr::rm::common::CommitError`'s four named variants (`WrongObject`, `DuplicateVersion`, `PrecedingVersionMismatch`, `NotLatest`), wrapped not restated by `StoreError::Commit`; "every engine" is the one engine that has a `Store` today |
| `H5.17` a deactivated `EHR` refuses new content, checked fresh every commit | • | `SqliteStore::ehr_is_modifiable` re-reads `latest_version` of the `EHR_STATUS` container on every `commit_composition`; `StoreError::NotModifiable`; `conformance::run_is_modifiable_gate` covers both orderings — deactivated-then-refused, and reactivated-then-admitted within the same `CONTRIBUTION` |
| `P6.12` archetype lookup served by an index | • | |
| `P6.8` values bound as parameters | • | |
| `M3.16` tamper-evidence chain | • | per container; a rewritten row fails to recompute |
| `M3.16c` checkpoint over the chain | • | a truncated chain verifies clean; only the checkpoint notices |
| `M3.39`–`M3.42` digest is SHA-256, 32 raw bytes | • | `ColTy::Digest`, binary in all six dialects |
| `O10.15` schema version recorded, mismatch refused | • | three states, all tested |
| `M3.33` projection refuses a non-archetype-root | • | |
| `M3.16d` content verified from the **stored bytes** | • | `tests/tamper.rs` edits a row through a second connection with the triggers dropped; `integrity`'s own unit tests catch 15 of 15 viable mutants (`lib:A-09`) |
| `T11.6` concurrency tested adversarially, against a database the threads genuinely share | • | `openehr-sqlite/tests/concurrency.rs` — the same test `H5.4`/`R4.5` above cite; only `openehr-sqlite` has a `Store` to test this way |
| `O10.16` no recorded version but data present is refused, not treated as fresh | • | `openehr-sqlite/tests/conformance.rs`: deleting `openehr_schema_version` from a database holding data yields `SchemaVersionMismatch { found: 0, .. }`, real test |
| `O10.17` the schema version is bumped for a schema-incompatible change, never the crate version | ? | the mechanism (`O10.15`) is real and tested; remembering to bump it whenever a change actually needs it is a discipline, not machine-enforced |
| `O10.19` a point-in-time restore cannot lose a version without losing the rows after it | ? | a true logical consequence of `M3.17`'s own tested guarantee — no backup/restore test exists in this tree to exercise it directly |
| `M3.43` canonical JSON in a byte-preserving column | • | the store round-trips it; the per-engine claim is below |
| `M3.34` anonymous committer stored as `NULL` | • | |
| `PR12.3a` anonymous `PARTY_SELF` committer stored `NULL` | • | the same `M3.34` evidence |
| `PR12.4` every version records `AUDIT_DETAILS` | • | the same `M3.15` evidence |
| `PR12.9` no inferred, defaulted, or synthesized committer | • | `AuditDetails.committer: PartyProxy` is a required field with no `Default` impl — structurally impossible to construct or deserialize without one |
| `PR12.10` a `CONTRIBUTION` carries its own audit, distinct from its versions' | • | `openehr_contribution`'s own `audit_*` columns, separate from `openehr_version`'s |
| `R4.3` content is not rejected for archetyped structure the crate does not interpret | • | the same `V9.9` evidence — no archetype-validation code path exists to reject on |
| `R4.6` identifiers satisfy openEHR's lexical grammars before reaching a column, never normalised | • | `Store` trait methods take typed `HierObjectId`/`ObjectVersionId`, never a bare string — malformed input is refused at parse, before it can reach a column |
| `R4.9` the projection is a pure function, no SQL, shared by every engine | • | the same `M3.35` evidence |
| `R4.10` a projected column is never the only home of a fact | ? | plausible by construction — `project()`'s only input is the same `Composition` object that gets canonicalized to JSON, so nothing it derives can come from elsewhere — but no test re-projects from stored JSON and compares |
| `M3.15` audit attributes on every version/contribution row | • | committing system, change type, committer, and the commit-time pair — the same columns `R4.2`'s own note names |
| `M3.19` canonical JSON is the record, stored whole | • | `M3.43`/`R4.8`/`R4.11` together are this claim, tested |
| `M3.20` the relational part is an index, never shredded content | ? | true today — `openehr_composition_index` carries only RM-fixed attributes (`M3.32`) — but nothing would fail a schema change that added an archetype-specific column |
| `M3.28` ordering and range scans use the derived column | • | the real SQL: `... AND audit_time_committed_utc IS NOT NULL ORDER BY audit_time_committed_utc DESC ...` (`src/store.rs`); the skip is `H5.13`'s own tested claim |
| `M3.32` composition index carries only RM-fixed attributes | • | `openehr_composition_index`'s column list matches exactly: archetype id, template id, category, composer, language, territory, setting, context start/end |
| `M3.35` projection is one function, shared | • | one `project` function in `openehr-store::record`; the guarantee is structural — today only `openehr-sqlite` calls it, since it is the only `Store` |
| `P6.11` a store offers version-by-id, latest, at-time, every-version, and archetype search | • | all five exercised in `openehr-store/src/conformance.rs`, including `find_compositions_by_archetype` |
| `P6.14` time-ranged queries use the derived UTC column | • | the same `M3.28` evidence |
| `P6.15` no silent truncation of a result set | ? | vacuously true at this layer — the store applies no bound to `find_compositions_by_archetype` at all, so nothing here truncates; the caller-facing page/cap (`_count`/`_offset`, capped at 100) is `openehr-loco`'s own, above the store |
| `S1.13` `openehr-sqlite` pins its engine rather than discovering it | • | the `bundled` `rusqlite` feature (`Cargo.toml`) |

## Service requirements

`openehr-loco` only. It sits **outside the conformance ladder** — every rung
there is defined by DDL, a `Store`, or a database server (`W0.32`) — so it
states evidence and takes no level. Nothing here is published.

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
| `S1.19` a service crate implements no clinical behaviour | ? | no controller calls `validate()` or any store-internal check directly (`openehr-loco/src/controllers/`) — true by absence, and an absence is not what a test demonstrates |

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
| `S1.1` one engine-agnostic storage model | • | schema, projection, commit rules, and the conformance suite all live in `openehr-store`, unduplicated; `layering` CI job keeps the six dialect crates and `openehr-sqlite` depending on it rather than reimplementing it |
| `S1.2` targets RM 1.1.0, matching `openehr` | • | inherited from `openehr` (`lib:S1.16`); `Cargo.toml`'s version dependency is what would break first if the two drifted |
| `S1.3` stores every class `openehr` models, without loss | • | `R4.2`'s lossless round-trip, including the four `VERSION`/`AUDIT_DETAILS` attributes `D-07` found dropped |
| `S1.5` no per-attribute shredding | • | six tables total (`M3.21`), none per-attribute; archetyped content stays in `data_json` (`M3.43`) |
| `S1.6` no AQL execution in the core | ? | no AQL executor exists in `openehr-store` — stated on `Store`'s own doc comment — but nothing tests the absence directly |
| `S1.7` no HTTP dependency or server in the core | ? | no `axum`/`hyper`/`tokio`/`tower` line in `openehr`, `openehr-store`, or any of the six engine crates' `Cargo.toml` — checked by hand for this assessment, not yet a named CI check; `openehr-loco` is the stated exception |
| `S1.8` the core does not authenticate or authorize | ? | no credential, key, or principal type anywhere in the core; `openehr-loco` verifies without authenticating (`PR12.13`–`PR12.15`, service table), which is a different crate |
| `S1.9` no terminology resolution, unit conversion, or timing interpretation | ? | inherited exclusion from `openehr` (`lib:S1.8`–`lib:S1.10`); unchecked at this layer specifically |
| `S1.10` no encryption at rest or key management | ? | no such code exists in the core; both belong to the deployment and the engine |
| `S1.11` an unimplemented operation returns `StoreError::Unsupported`, never a default | ? | the variant exists with exactly this shape (`engine`, `what`, `spec_ref`) but is constructed nowhere in the tree — no engine crate below Store level implements enough of `Store` to reach it yet |
| `S1.12` no engine below Store level ships a partial `Store` | ? | true today — only `openehr-sqlite` implements `Store` at all — but nothing would catch a partial implementation appearing on another engine crate |
| `S1.21` sections 7 and 8 stay retired | — | a statement about which specification sections are active, not a claim about this crate's code |
| `M3.21` five data tables plus one metadata table, no more without an amendment | ? | `schema.rs::TABLES` holds exactly these six today; nothing would catch a seventh being added without the amendment this requirement asks for |
| `M3.24` an instant is stored as two columns | • | `every_instant_has_a_derived_partner_and_the_partner_is_nullable` (`openehr-store/src/schema.rs`), the same test `M3.27` cites |
| `M3.25` the `_text` column is authoritative on read | • | `R4.2`'s round-trip returns the exact lexical form from `_text`; `_utc` is a different, non-lexical type, so there is no path by which a read could take the value from it instead |
| `M3.26` `_utc` is nullable, `NULL` when unestablished | • | the same schema test as `M3.24`/`M3.27` |
| `M3.38` a store error never echoes stored content | ? | every `StoreError` variant's fields are identifiers, engine names, or rule names (`src/error.rs`) — true by construction, but no test asserts a stored value can never reach one |
| `C0.1` RFC 2119 keywords, normative only when capitalized | — | a convention for reading this specification, not a claim about any crate's code |
| `C0.2` unmarked prose is rationale, not an obligation | — | same — a reading convention |
| `C0.3` examples/numbers in rationale are illustrative; the requirement governs on conflict | — | same |
| `C0.4` every requirement id follows `<prefix><section>.<ordinal>[<suffix>]` | ? | true of every id in this directory today; a malformed one would not be flagged as malformed — it would simply not match the regex every id-aware script (including this matrix's own coverage checker) relies on, and vanish from tracking silently rather than loudly |
| `C0.5` ids are stable and never reused | ? | followed in practice throughout this tree (withdrawn ids keep their number, e.g. `03-storage-model.md`'s own withdrawn section) — a discipline, not a script |
| `C0.6` section gaps 7, 8, 14 are deliberate, not renumbered | • | `spec/databases/` has no `07-*.md`, `08-*.md`, or `14-*.md` — directly checkable by listing the directory |
| `C0.7` ids in this directory are scoped to it; `openehr/spec/` allocates the same prefixes with different meanings | ? | true — confirmed for `S1.4` specifically while assessing this same batch — but nothing catches an unqualified, ambiguous citation before it is written |
| `C0.8` the four-level ladder | • | this file's own "Conformance levels" table, above, is the ladder — `check-docs.py`'s "every conformance level restated in the tree matches conformance-matrix.md" |
| `C0.9` a crate states its level in its README and crate docs | • | same `check-docs.py` check |
| `C0.10` a level's evidence MUST come from the crate's own engine | • | the `schema` CI job runs each of the five dialects against its own server, separately — the exact fix for `W-01`, the finding this requirement's own rationale names |
| `C0.11` docs MUST NOT describe a capability above the crate's level | • | same `check-docs.py` check as `C0.9` |
| `C0.12` Schema's "with a row present" clause | • | `verify-schema.sh` inserts a row into `openehr_version` before testing `UPDATE`/`DELETE` refusal, every dialect, every run |
| `C0.13` a level is present-tense; Verified needs CI green on `main` | • | this file's own "Conformance levels" section, above, cites the specific green run for each claim |
| `C0.14` a departure is a numbered `M14.x` requirement in the crate's dialect annex | • | all six annexes carry at least one `M14.x` entry |
| `C0.15` a departure MUST NOT weaken an engine-independent invariant | ? | a review-time judgement on any new departure; true of the six that exist today, not machine-enforced |
| `C0.16` an undeclared departure is a defect, not retroactively an amendment | — | a definitional/classification rule for how findings are written up, not a claim about code |
| `C0.17` prose describing an engine is not itself a departure | — | same — definitional |
| `C0.18` a specification change is one copy, one commit | • | `check-docs.py`'s shared-block check (`4 shared blocks match their 1 owners`) is the general enforcement this requirement asks for |
| `C0.19` a new requirement takes the next unused ordinal, never inserted mid-sequence | ? | followed throughout this session's own batch of additions; a discipline, not a script |
| `C0.21` a silent amendment is forbidden; the commit states which crate's behaviour drove it | — | a commit-message discipline, not a property of the code or the document at rest |
| `C0.22` every amendment is checked against the matrix and `audit.md` | — | same — an editorial-process rule |
| `W16.1` eighteen crates, eight published, ten not | • | `check-docs.py`'s own derived counts: `crates=18`, `published=8`, `unpublished=10` |
| `W16.2` an engine crate's name matches the engine it targets | • | `X15.15`'s cross-dialect DDL comparison is the mechanism that would catch a violation of this shape — it is how `W-01` (`openehr-mariadb` emitting `openehr-mysql`'s DDL under its own name) was found |
| `W16.3` a crate's description names its engine and does not overclaim | ? | true by inspection — `openehr-oracle`/`openehr-mssql`'s descriptions read "schema dialect and DDL", not persistence — but nothing beyond the trademark-notice check reads crate descriptions for this specifically |
| `W16.4` a crate declares only the drivers it uses | ? | true by inspection — no database-driver crate (`sqlx`, `tiberius`, …) in any Schema-level engine crate's `Cargo.toml` — not asserted by a dedicated check |
| `W16.5` normative text lives in `spec/` once; an engine crate's `spec/` holds only its dialect annex | • | every engine crate's `spec/` directory holds exactly one `14-<engine>-dialect.md` and nothing else |
| `W16.6` CI verifies no two dialects emit the same DDL, covering every engine | • | `X15.15`/`X15.16`, above — the same rows, the same evidence |
| `W16.7` a shared-behaviour change is one edit in `openehr-store`, not reproduced in engine crates | ? | true by inspection — no commit-rule or projection logic is duplicated in any dialect crate — not actively tested for regression |
| `W16.8` no documentation is text-substituted from another crate | ? | the one known violation (`W-01`) is fixed; nothing automated guards against a recurrence beyond review |
| `W16.9` a documentation code example runs, and CI runs it | • | the `examples` job, plus every rustdoc example being compiled and run as a doctest |
| `W16.10` a measured number names what measured it and when | ? | true of the numbers checked this session (`BENCHMARKS.md`'s convention) — no script scans the whole tree for an unattributed measured number |
| `W16.11` crates may version independently; they currently share one version | • | `check-docs.py`: "versions agree: live 0.9.0 everywhere" — found and fixed in passing while assessing this row: the requirement's own rationale said `0.2.0`, six releases stale (`16-repository-and-release.md`) |
| `W16.12` a changelog describes the crate it sits in | ~ | `CHANGELOG.md` exists (found and fixed in passing: the requirement's own text said "no crate here has one yet") but covers the eight published crates as a declared set, not strictly one crate each — not a substitution, but not literally this requirement's own words either |
| `W16.13` pre-publish: tests and lints pass, the package is clean, every linked file ships | • | `agents/publishing.md`'s documented `cargo publish --dry-run` and `cargo package --list` checklist, followed for every release recorded there |
| `W16.14` a crate is not published above its level, and says which level | • | the same `check-docs.py` level-consistency check as `C0.9`/`C0.11`, applied to the eight published crates specifically |
| `W16.16` every crate is its own Cargo workspace | • | confirmed for all eighteen crates' `Cargo.toml` while assessing this row |
| `W16.17` a sibling dependency's declared version matches the sibling's actual version | • | `check-docs.py`: "versions agree... manifests and inter-crate pins included" |
| `W16.18` no crate is published while a finding against its claims is open | • | `agents/publishing.md`'s documented checklist checks this explicitly before every release |
| `W16.20` a behaviour change bumps the incompatible version, never a patch | ? | followed in practice — 0.9.0's own two breaking changes bumped minor, not patch, per `CLAUDE.md`'s own dated account — no automated semver-diff check enforces it |
| `T11.8` an audit test asserts committing-system/change-type AND database-enforced append-only | ~ | append-only: • on all six dialects (`M3.17`, per-engine). Tamper evidence: truncation-under-checkpoint and content-verification are tested (`M3.16`, `M3.16c`, `M3.16d`); key-rotation additivity is not, at this layer (`db:D-13`) |
| `T11.10` a test proven to fail without its fix | • | `cargo mutants --in-diff`, the `mutants` CI job — the continuous, automated form of exactly this check |
| `T11.11` a regression pinned by the narrowest assertion that catches it | ? | followed throughout this session's own additions (exact values and named sets, not thresholds) — a style discipline, not machine-enforced |
| `T11.12` coverage does not degrade silently; a skip that checks nothing fails | • | the `verify` task's own behaviour: "exits non-zero when the history is not intact, including when it verified nothing" (`openehr-loco/README.md`) |
| `T11.13` a self-skipping test is not the sole evidence for a level | • | `C0.8`'s own ladder, above — Schema and above require a CI job that fails, not skips, without the engine |
| `T11.14` an `#[ignore]`d test has a matrix entry | ? | vacuously true — no test in any database crate is currently `#[ignore]`d — demonstrated correctly in the sibling `openehr` crate's own corpus tests, not exercised here |
| `T11.15` every dialect has golden DDL tests | • | the same `tests/ddl.rs` evidence as `G2.9`, above |
| `T11.16` every dialect is checked by `conformance::check_dialect` | • | the same mechanism `M3.36`'s own row cites |
| `T11.17` every dialect compared against every other, coverage asserted | • | `X15.15`/`X15.16`, above — the same rows |
| `T11.18` a golden test names the specific spellings that would indicate a copy | • | `X15.18`'s own row: compared against the nearest neighbour specifically, the shape that would have caught `W-01` |
| `T11.19` the CI check invokes the same script a contributor runs locally | • | `verify-schema.sh`, invoked identically by `ci.yml` and by a contributor's own terminal — stated as a principle in both places |
| `T11.20` a guard over a list asserts the list's completeness | • | the same `X15.16` row |
| `T11.21` a documentation example is compiled and run | • | the same `W16.9` row |
| `O10.2` no stored content in a log line | • | `openehr-store`/`openehr-sqlite` emit no log line at all — no `tracing`, `log`, `println!`, or `eprintln!` anywhere in either crate, confirmed exhaustively |
| `O10.4` `install()` is idempotent | • | the same `G2.13` evidence (declared per statement kind); `verify-schema.sh` runs every dialect's DDL twice and requires the second run clean |
| `O10.4a` a recompute path for the one derived value, `…_utc` | — | the requirement's own text: *(amended — not applicable)* — no migration machinery exists to need one, and the projection already recomputes `…_utc` from `…_text` |
| `O10.6` no bespoke backup format | ? | true by absence — no backup/restore code exists anywhere in this tree — not guarded against one being added |
| `O10.7` the database connection is encrypted unless in-process | ? | `openehr-sqlite` is exempt by construction (in-process, no connection); no other engine crate opens a live connection yet for the requirement to bind |
| `O10.10` a release ships supply-chain evidence | • | the `supply-chain` CI job (`cargo deny check`, `cargo audit`, all eighteen crates) plus `Cargo.lock` committed in every one |
| `O10.11` a published version matches the source that claims it | • | `agents/publishing.md`'s documented process tags the release as part of publishing, from the exact commit, not after it |
| `O10.12` a contributor's local check and CI run the same script | • | the same `T11.19` row — this requirement's own text cites it |
| `O10.13` no bound parameter logged at a level a production deployment would enable | • | the same zero-logging evidence as `O10.2` |
| `O10.18` an engine crate opening a connection documents its TLS configuration | ? | vacuously true — no crate here opens a network connection yet; the requirement states which crate it would bind once one does |
| `PR12.8` the trust boundary is stated plainly: this layer does not authenticate | • | `openehr-loco/src/auth.rs`'s own module doc cites this requirement by number: "the practical consequence is that `db:PR12.8` still holds" |
| `PR12.11` append-only is not tamper evidence, and documentation must not conflate them | • | `PHI.md`'s own text draws exactly this distinction, citing `db:PR12.11` by number |
| `X15.1` the portable core exists in exactly one crate, not copied | • | the same `S1.1` evidence — `openehr-store` is the one crate; the `layering` job keeps it that way |
| `X15.2` canonical form is computed in Rust, one shared function, never delegated to the database | • | the same `R4.11` evidence: shared `to_canonical_string` |
| `X15.3` table and index names are identical on every engine | • | names come from `openehr-store::schema`'s one declaration (`M3.22`); a dialect's `col_sql`/`quote` affect spelling, never naming |
| `X15.7` a departure cites the core requirement by number and states what holds instead | • | the same `C0.14` evidence — read directly while investigating `db:D-13`'s `M14.8`: every annex entry names its number and states the replacement |
| `X15.8` an annex does not restate core requirements it does not change | • | the same `W16.5` evidence — every engine crate's `spec/` holds exactly its own dialect annex |
| `X15.12` a cross-engine test exists for whatever can be compared without two stores | • | this requirement's own text names `X15.15`/`X15.19`'s own tests as exactly that test |
| `X15.13` a dialect owns exactly `col_sql`, `quote`, `placeholder`, `append_only_sql`, plus idempotence, `guard`, `terminator` | • | the `Dialect` trait's own method list (`openehr-store/src/dialect.rs`) — structurally cannot own more |
| `X15.14` a dialect does not own the schema | • | the same `M3.22` evidence |
| `X15.17` a new engine crate is not created by copying an existing one | ? | true today, after `W-01`'s fix (`openehr-mariadb` rewritten from the `Dialect` trait) — a process discipline, nothing machine-enforced against a future copy |
| `X15.20` the projection is one shared function | • | the same `M3.35` evidence |
| `P6.10` the queryable surface is the index, nothing else | ? | true by inspection — `openehr_composition_index` carries only RM-fixed attributes (`M3.32`); no path or structural index over content exists — not independently tested |
| `P6.16` every search target declares its kind (identity, prefix, range, membership, containment) | ✗ | `search-adjuncts.md`'s `AD1`–`AD2` define the framework, but no declaration exists for any of the seven real indexed columns `P6.4` names — written in advance of the columns it would apply to, not yet applied to them |
| `P6.17` where an engine cannot serve a target's kind, the dialect emits the required adjuncts | ? | vacuous today — `P6.18` already establishes no target currently needs one; nothing exercises the obligation this binds |
| `P6.19` the canonical JSON column gets no structural or path index | • | the same `P6.18` evidence — `Json`/`LongText` cannot be indexed at all, structurally, so neither can be given a path index specifically |

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
| `P6.7` | bounded result sets **in the store** | `find_compositions_by_archetype` returns every match, unbounded, with no opt-in a caller can decline — no `LIMIT`, no page parameter. `openehr-loco` bounds it above the store (`_count`/`_offset`, capped at 100, `P6.15` above); a program embedding `openehr-store` directly gets none of that, the same shape as `PR12.5` |


| `T11.7` | redaction test over emitted logs | |
| `X15.10` | cross-engine logical agreement | untestable: only one store exists |
| `X15.11` | cross-engine chain verification | untestable, same reason as `X15.10`: only one store exists. `M3.16`'s chain is real, not absent — corrected here 2026-09-10, `db:D-13` |

## How to read this file

1. Find the crate you are deploying, not the reference one. A requirement
   verified against SQLite is not thereby verified against Oracle.
2. Treat **?** as unverified. It is recorded rather than promoted precisely
   because promoting it is the failure this repository has committed most often.
3. Check [`spec/audit.md`](../audit.md) for open findings before trusting any
   row.

The single most useful line here, current as of 2026-09-06: **every engine
crate has now had a statement parsed by the engine it names.** Of the five
that reached Schema by an actual live run rather than by import (`sqlite`'s
own history is `W-02`'s), four were found wrong at Dialect level the moment a
real server saw their DDL — PostgreSQL and MySQL (`A-13`, `A-14`, `A-15`),
MariaDB (emitting another engine's script entirely, `W-01`), and SQL Server
(`D-12`: `append_only_sql`'s `CREATE TRIGGER` shared a batch with everything
before it, which the engine refuses outright). **`openehr-oracle` is the one
exception**: its DDL held up on first real contact, and the seven-round arc
that got `openehr-mssql` to Schema found six of its seven real bugs in
`verify-schema.sh` itself, not in the dialect — two `set -eu` bugs that
swallowed the actual error, two different readiness-race diagnoses, a
host/container filesystem-boundary mistake in how the script delivered SQL to
`sqlcmd`, and a display artifact (`sqlcmd`'s own column padding) that could
have hidden a real difference behind a byte-exactness check that looked
green. The count to carry into any future promotion: **five defects found by
running DDL against a real server, one dialect that had none.**

---

Part of the [openEHR persistence specification](index.md).
