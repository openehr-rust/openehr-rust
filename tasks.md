# Tasks

Triaged backlog for taking this repository from "honest library and embeddable
store" to something a professional can put in front of patient data. Rationale
and workstreams live in [`plan.md`](plan.md). A `[x]` here means the work is
**verified done**, not intended — check items off in the same change that
completes them, with the evidence named.

**This file is not engineering status.** Capability is read from
[`openehr/spec/conformance-matrix.md`](openehr/spec/conformance-matrix.md) and
[`spec/databases/conformance-matrix.md`](spec/databases/conformance-matrix.md);
open engineering findings live in the three audit registers
(`spec/audit.md`, `openehr/spec/audit.md`, `spec/databases/audit.md`). Nothing
here speaks for the matrices, and a task that adds capability starts with a
requirement (`W0.19`), never with code.

**Rewritten 2026-09-03** against the FerroEHR announcement thread on the openEHR
Discourse — [*FerroEHR – a new Rust-based openEHR CDR, looking for
testers*](https://discourse.openehr.org/t/ferroehr-a-new-rust-based-openehr-cdr-looking-for-testers/17230),
21 posts, 2026-08-24 to 08-27, read in full — because that thread is the
clearest available record of what openEHR's implementers, SEC members, and
integrators actually inspect when a new Rust implementation appears. The
earlier version of this file (the professionalization checklist, nearly all
`[x]`) is condensed into §Done below; its full evidence text is in
`git show 4761700:tasks.md` (its last revision, 2026-08-30) and, restated, in
`plan.md`, `SECURITY.md`, `MAINTAINERS.md`, and `NEWS.md`.

## What the thread says a professional checks

Each row names the post it comes from and what this repository has today. The
tasks below are ordered by these gaps.

| Checked by the community | Thread | Here, today |
| --- | --- | --- |
| **Conformance that is not self-graded**: a machine-readable catalogue, every case citing the spec section, negative twins for every refusal, cross-run against EHRbase, a bring-your-own-server runner, a public ambiguities register | #15 (Haarbrandt, SEC), #16 | A conformance *ladder* and two matrices, self-checked counts, three audit registers — strong on honesty, but no external corpus has ever been run through this code, no test cites a spec section by id, and `db:D-11` says 144 of 221 database requirements have never been assessed at all |
| **Which spec generation, pinned where**: RM/BASE/LANG releases coupled as openEHR publishes them; a `stable` profile refusing surfaces the released specs do not define | #7 (Iancu, SEC), #8 | Release targets exist per requirement (`S1.16`, `K15.2`) and terminology provenance is recorded to the file and date, but there is no one table of every specification release this tree is transcribed from |
| **Lifecycle edge cases**: `is_modifiable` toggled inside one contribution | #5, #6 | **Done at the `Store` level, 2026-09-12** (`db:H5.17`): `SqliteStore::commit_composition` refuses content while deactivated and admits it again once reactivated, even later in the same contribution. `openehr-loco`'s `GET`/`PUT …/ehr_status` HTTP endpoints remain undone |
| **Performance**, with an outsider offering to rerun the numbers | #4, #14 | Benchmarks exist and are run-never-gated (`W0.35`); no store commit latency or HTTP round-trip has ever been published |
| **Runs in sixty seconds**: `docker compose up`, a Codespace, a hosted sandbox with Swagger, and a *correct* quickstart | #9, #10, #11, #13, #21 | `Dockerfile`, compose file, and devcontainer exist and are verified (`curl /openehr/v1/metadata` → 200); no OpenAPI document yet, and `openehr-loco` is still `publish = false`, SQLite-only |
| **A REST surface tools can talk to**: ITS-REST 1.1.0, AQL over HTTP, templates, EHR_STATUS, an admin scheme — openEHR Explorer added FerroEHR as a server type within three days | #1, #21 | Eleven endpoints: EHR, contribution, composition commit/read/history/vread, one index search. No `/query/aql`, no template endpoints, no `EHR_STATUS`, `DELETE` answers `501` |
| **AQL that executes** | #1 | AQL is parsed and statically checked and **not executed** — a deliberate rule (`S1.5`, reaffirmed `K15.29`) that a CDR reader will read as the headline gap |
| **Templates**: OPT 1.4, ADL 2.4, WebTemplate, FLAT/STRUCTURED, validated at upload | #1 | AOM2 types, `am::validate` against an in-memory archetype, and a `definition`-only cADL reader (`A-40`, `A-62`–`A-69`). No OPT, no template, no flattening, no ADL 1.4 body |
| **Strict readers and typed errors**: undeclared/duplicate JSON keys refused, every error naming path and rule | #1, #21 | Constructors validate and `validate()` runs on JSON ingress (`lib:A-23`); duplicate keys **are** already refused (`serde`'s own derived behavior, unconditionally). Undeclared keys are not — and a stated posture already exists, the opposite way: `J9.9` requires ignoring them, deliberately, for forward compatibility with future openEHR minor releases |
| **One edition, security included**: RBAC/ABAC, ATNA audit, multi-tenancy all in the open-source build | #18 | Tamper-evident audit chain, PHI redaction, PASETO auth; no RBAC, no read audit at the store (`db:D-04`), no SBOM, no TLS statement |
| **Upstreaming spec defects** — SEC asked for the 228 inconsistencies rather than let them be discarded | #18, #19 | This repository adjudicates spec silences inside its audit registers and has reported none of them upstream |
| **Transparent AI disclosure** — praised first, before anything technical | #2, #7, #19 | `AI_STATEMENT.md` exists and is candid; keep it that way |

## Triage

**P0** — this week; credibility and the outreach date. **P1** — the
capabilities a CDR reader looks for first; each begins with a requirement.
**P2** — operability and evidence. **P3** — larger scope or awaiting a
decision. Size: S (hours), M (days), L (weeks), XL (a track).

### P0 — land what exists, and stop saying stale things

- [x] **Push the twelve unpushed commits and read the CI run.** Local `main`
      is twelve commits ahead of `origin/main` (`A-58`–`A-69`, 2026-09-02);
      `gh run list` shows CI last ran on the 0.9.0 publication commits. Two
      of the twelve are **breaking** (`A-63`, `A-69`), so the next release is
      0.10.0 by this repository's own rule, not 0.9.1. *Evidence:* `git
      status -sb` shows no `ahead`; `gh run list --limit 1` green on the
      pushed head; `agents/publishing.md` untouched until the release is
      actually cut. — **S**
      - Done 2026-09-03. The first push (`bd17de3`, run 33775455452) was
        **red** on one job: `changed lines are mutation-tested (openehr)`,
        14 missed mutants and 3 timeouts, every one a test gap in the
        `A-58`–`A-69` code. Fixed in `e461b99`, with the survivors re-run
        locally by name (a push is mutated against `event.before..HEAD`
        only, so CI would not have re-checked them) and 14 further
        survivors found by mutating the whole of `assumed_value_conforms`
        rather than its changed lines. Run 33781305464 on `e461b99`: 33
        of 33 jobs green. `A-70` and the corpus runner went in the same
        push; `A-71` (breaking, a third reason the next release is 0.10.0)
        followed in `caccca1`/`4725d8c`, run 33785547358 green.
- [x] **Correct the claims the last two weeks made false.**
      `COMPARISONS.md:43` still says "**no validation of data against an
      archetype**" and "28 of those 32 requirements have no code";
      `openehr::am::validate` has existed since 0.8.0 and `A-40`'s own status
      line now lists what is built. Re-derive every `openehr/spec/
      conformance-matrix.md` row dated 2026-08-02 that the `am` work touched
      (`W0.10` says undated re-checks are the defect). *Done 2026-09-03: the
      matrix's `K15.18`–`K15.23` row cited two test names renamed by `A-60`
      and `A-63` — found by checking every cited name against `cargo test
      -- --list` (630 tests), which nothing in CI does; corrected.
      `K15.6`–`K15.7` moved to **•** with nine named tests (the refusal
      discipline is a property of a parser, and `am::cadl` plus the two
      header readers now exist to be held to it), so eighteen unsatisfied
      became sixteen. The tally was re-derived mechanically by expanding
      every `Id` cell: 270 •, 33 doc, 16 spec, 13 type, 8 —, 3 ?, 1
      withdrawn, 344 total, matching the table. Restatements fixed in
      `15-archetypes.md` (§ preamble and `K15.5`'s own refused-list), the
      `A-40` paragraph, `README.md`, `COMPARISONS.md`, `am/mod.rs`'s
      capability table, and two rustdoc comments still saying `Node` does not
      expose `ARCHETYPED.archetype_id`; a dated `NEWS.md` entry added rather
      than the 2026-08-26 entry rewritten. `check-docs.py`, `check-trademarks
      .py`, CI's own exact-once coverage script, `cargo doc`, clippy at
      `-D warnings`, and 437 tests all green.* — **S**
- [x] **Remove the stray build directories.** `openehr-cdr-{mariadb,mssql,
      mysql,oracle,postgresql,sqlite}/` contain only `target/` and are not
      crates; `openehr-loco-fuzz/` is untracked. Either add the fuzz crate
      with a target and a CI row (`W-13`: a guard is only as wide as its
      list) or delete it. *Done 2026-09-03: `openehr-loco-fuzz/` held only a
      cargo-fuzz `corpus/` (2,324 inputs) and two empty `artifacts/`
      directories for targets named `paseto_token` and `ehr_json`, which
      exist in no crate in the tree — run output from a harness that was
      never committed, not a crate, and with nothing found; deleted along
      with the six `openehr-cdr-*` directories. `git status --short` shows no
      untracked entry; `check-docs.py` still counts 18 crates and 8 fuzz
      harnesses.* — **S**
- [ ] **Answer the thread, and email its author, by 2026-09-05** — the date
      `help/outreach/index.md` §11 already set. The right content is what
      this repository does that FerroEHR does not claim: `base::Real`
      precision (`lib:D3.18d`, `db:D-08`), a file-based store with no daemon,
      a conformance ladder whose lowest rungs are stated. Offer to run their
      published conformance corpus's JSON fixtures through `openehr`'s RM
      reader and report what refuses. Never "safe", "compliant", "certified",
      or "clinically" (outreach §1). *Evidence:* the post and the reply,
      linked from `NEWS.md`. *Draft written 2026-09-03,
      `help/outreach/drafts/2026-09-ferroehr-thread-reply.md`; updated
      2026-09-05 (the due date) to report the external corpus run actually
      completed in the meantime (`A-70`–`A-77`, 178 to 969 of 1,379 files,
      `openehr/spec/corpus.md`) rather than only offer one. Every claim
      re-checked against the tree; sending is the maintainer's action and
      has not happened.* — **S**
- [ ] **Decide the `regex` dependency.** `K15.10`'s remainder — evaluating a
      carried `ARCHETYPE_SLOT` assertion against a filler's identity — needs a
      regex engine, and `openehr/Cargo.toml` has never had one; every
      dependency there carries a justifying comment. Options: add `regex`
      (safe, well-audited, heavy), hand-roll a documented subset, or leave
      assertions carried-not-evaluated and say so in the matrix. This is a
      maintainer decision, not code. *Evidence:* a dated line in `plan.md`
      §Open decisions and, if adopted, the comment in `Cargo.toml`. — **S**

### P1 — the capabilities a CDR reader looks for, specification first

- [ ] **AQL execution over the SQLite store.** `S1.5`/`K15.29` forbid it
      today; amend under `C0.19` with the reasoning kept, then specify in
      `spec/databases/` before writing code: candidate compositions selected
      through the archetype index (`db:P6.12`), `CONTAINS`/`WHERE` evaluated
      in process by `crate::path::Node` over the stored canonical JSON, the
      unsupported constructs `Q12.9` lists refused at planning time (the
      thread's own praise for FerroEHR: "rejects unsupported constructs
      explicitly"). SQLite first — it is the only crate at **Verified**.
      *Evidence:* new `Q12.x`/`db:` requirements, a conformance-suite test
      per operator, the matrix row, and `POST /query/aql` in `openehr-loco`.
      — **XL**
- [ ] **OPT 1.4 ingestion and validation at commit.** `K15.16` is in force
      with no code. Every client in the thread — EHRbase, FerroEHR, openEHR
      Explorer — uploads an operational template 1.4 XML first and commits
      compositions against it. Read OPT 1.4 into the AOM2 types this tree
      already has, then have `openehr-store` refuse a composition its
      template does not admit, with the violation naming path and rule
      (`K15.20`, `K15.21`). Template validation *at upload*, not deferred
      (thread #1). *Evidence:* a CKM-published OPT round-trips; `am::
      validate` runs against it in the commit path; matrix rows for `K15.16`,
      `K15.18` flip. — **L**
- [ ] **`is_modifiable` at commit, order-independent.** Thread #5/#6 found
      the sequencing bug and #6 published the adjudication with spec
      citations (FerroEHR's own tracker, `#2673`; the `versions` List-vs-Set
      defect it reports upstream is their `#2674`): refuse content members
      only when the EHR is deactivated *and* the contribution does not
      reactivate it. Specify it as a `db:H5.x` commit rule, then test both
      orderings against `openehr-sqlite`. *Evidence:* the requirement, two
      conformance cases, and `EHR_STATUS` read/update in the `Store` trait
      and `openehr-loco` (`GET`/`PUT …/ehr_status`). — **M**
      - **Scope corrected 2026-09-06, before writing any of it.** `Ehr.ehr_status`
        (`openehr::rm::ehr`) is only an `ObjectRef` — a pointer to a
        `VERSIONED_EHR_STATUS` elsewhere, per RM — and `openehr-store`'s
        `Store` trait has no method to create, commit, or read an
        `EhrStatus` version at all; `openehr_version`'s own `rm_class`
        column already admits `EHR_STATUS` generically (`schema.rs`), but
        nothing calls it that way. So this task's real prerequisite is
        **versioned `EhrStatus` persistence** — a `Version<EhrStatus>`
        counterpart to `commit_composition`/`get_version`/`latest_version`,
        the same shape composition versioning already has — not merely the
        one commit-rule check the task's own headline names. That
        prerequisite is close in size to the composition-versioning work
        itself, not a small addition to it. Re-sized **L**, and the `db:H5.x`
        commit rule itself is unaffected — it is one check on data this
        prerequisite must exist to supply.
      - **2026-09-12, the prerequisite itself, done — smaller than the
        2026-09-06 estimate.** Investigation before writing anything found
        most of the "composition-versioning work" was already generic:
        `VersionRow::project<T: Serialize>`, and every read method
        (`get_version`/`latest_version`/`all_versions`/`version_at_time`/
        `chain_checkpoint`) — none names `Composition` in its own
        implementation, so none needed to change. The one real gap was a
        write path: `Store::commit_ehr_status`, added alongside
        `commit_composition`, sharing its commit-rule check
        (`openehr_store::check_commit_rules`, factored out — behaviour-
        preserving, every existing test unchanged) rather than duplicating
        it. `openehr-store::conformance::run_ehr_status` is the new shared
        test — commits two `EHR_STATUS` versions, reads each generic method
        back, refuses a duplicate — and passes against `openehr-sqlite`.
        Mutation-tested (`cargo mutants --in-place`, `agents/auditing.md`'s
        own documented workaround for this crate's sibling path
        dev-dependencies): `check_commit_rules` had no direct test in
        `openehr-store` itself before this — added six, now catches both
        mutants that survived; `commit_ehr_status`'s own one candidate
        mutant is unviable for the same reason `commit_composition`'s
        already-accepted one is (`CommitOutcome` has no `Default`), not a
        new gap. **Breaking** (`CHANGELOG.md`): a new required trait
        method. **Not done in this pass**: the `db:H5.x` commit rule itself
        (refuse content only when deactivated *and* the contribution does
        not reactivate) and `openehr-loco`'s `GET`/`PUT …/ehr_status` — the
        prerequisite was the whole scope, deliberately, so it could be
        verified on its own rather than folded into a larger, harder-to-
        review change.

        **Found and fixed pushing it: a real CI gap, `db:W-20`.** The
        `mutants (openehr-sqlite)` job turned red on this push —
        `cargo build failed in an unmutated tree` — because
        `cargo-mutants`'s isolated copy cannot resolve this crate's five
        sibling dev-dependencies (`agents/auditing.md` already documented
        the cause and the `--in-place` remedy; the CI workflow itself had
        never used it, for any crate). This was the first direct push in a
        long time to change mutable code in this crate's own `src/` rather
        than a test, a doc, or a comment, so nothing had surfaced it before.
        Fixed in `.github/workflows/ci.yml`, scoped to this one crate in the
        matrix. A second, smaller gap closed the same way: `run_ehr_status`
        (the new shared conformance test) has no `Store` to call inside
        `openehr-store` itself, so its own mutation could not be caught
        there either — excluded via a new `openehr-store/.cargo/mutants.toml`,
        alongside `run` (the pre-existing composition half of the same
        suite, carrying the identical characteristic unnoticed until now).
        Both reproduced locally against the exact failing commit before
        trusting the fix. Full account in `spec/audit.md`'s new **W-20**.
      - **2026-09-12, the commit rule itself, done.** Specified as
        `db:H5.17` in `spec/databases/05-versioning-and-history.md`: a
        content commit is refused when the EHR's current `EHR_STATUS` has
        `is_modifiable = false`, checked fresh on every call rather than
        cached — the exact sequencing `#2673` names. `SqliteStore` gained a
        private `ehr_is_modifiable`, called from `commit_composition`
        before the write; `StoreError::NotModifiable` is the new refusal.
        `openehr-loco`'s `status_for` maps it to `409 Conflict`, beside
        `Conflict`/`Commit`, for the reason both already are (well-formed
        request, conflicting state). Two conformance orderings, both in
        `openehr_store::conformance::run_is_modifiable_gate`, run against
        `openehr-sqlite` in CI: content refused while deactivated, and
        admitted once reactivated earlier in the same `CONTRIBUTION`.
        Matrix row added (`spec/databases/conformance-matrix.md`).
        **Breaking** (`CHANGELOG.md`): existing callers whose workflows
        relied on committing against a deactivated record will now be
        refused.

        **A real fixture defect surfaced writing this, before any commit
        landed.** `sample_ehr()`'s `EHR.ehr_status` `ObjectRef` and
        `sample_version()`'s composition container had shared one uid
        (`RECORD`) since the fixture was first written — inert as long as
        nothing ever looked up "whatever is committed at the container
        `EHR.ehr_status` names". `ehr_is_modifiable` does exactly that
        lookup, so any test committing both a composition and an
        `EHR_STATUS` retrieved the composition's own JSON and failed to
        parse it as an `EhrStatus` (`missing field 'subject'`) — surfaced
        first as two failing `openehr-sqlite` concurrency tests. Fixed by
        giving `EHR_STATUS` its own distinct container constant,
        `EHR_STATUS_RECORD`, in `openehr-store::conformance` — the
        production-correct shape, since two different `VERSIONED_OBJECT`s
        never share a uid. All tests pass, including the two that regressed.

        **Not done in this pass, deliberately**: `openehr-loco`'s
        `GET`/`PUT …/ehr_status` endpoints. The commit-rule guarantee holds
        for every caller of the `Store` trait directly, and for
        `openehr-loco`'s existing composition endpoints, without them; what
        remains is exposing `EHR_STATUS` itself — reading the current one
        and toggling `is_modifiable` — over HTTP, a separate, self-contained
        slice left for a follow-up rather than folded into this one.
- [ ] **PostgreSQL `Store`.** Every CDR in the thread runs on PostgreSQL
      18; this repository's only `Store` is SQLite. Implement
      `openehr-postgresql`'s store against the existing DDL, run
      `conformance::run` against a real server in CI (the `schema` job
      already provisions one), and promote in `spec/databases/
      conformance-matrix.md` — the one file that owns a level (`W0.40`).
      *Evidence:* the matrix row moves Schema → Store → Verified with the
      job named. — **L**
- [x] **MSSQL and Oracle parsed by a real server.** Two of six dialects had
      "never been parsed by a server" (`spec/databases/conformance-matrix
      .md`). Both now run in containers — `mcr.microsoft.com/mssql/server`
      and `gvenzl/oracle-free` — `verify-schema.sh` gained both, the
      `M14.6`/`M14.7` departures are met, and both rows promote to Schema.
      *Evidence:* `spec/databases/conformance-matrix.md`; `schema / oracle`
      run [34040865467](https://github.com/openehr-rust/openehr-rust/actions/runs/34040865467);
      `schema / mssql` run
      [34045294037](https://github.com/openehr-rust/openehr-rust/actions/runs/34045294037). — **M**

      **2026-09-06, closed.** `openehr-oracle` needed two script fixes and no
      dialect change: `gvenzl/oracle-free` (a public image, unlike the
      official ones the blocker was originally about) let its DDL reach a
      real server, where it parsed, was idempotent, round-tripped canonical
      JSON byte-exact, and enforced append-only on the first real attempt.
      `openehr-mssql` took seven rounds to reach the same result, six of them
      fixing `verify-schema.sh` itself (two `set -eu` bugs that swallowed the
      real error, two different readiness-race diagnoses against `MSSQL_DB`'s
      asynchronous database creation, a host/container filesystem-boundary
      mistake — `sqlcmd -i` names a file for the *container* to open, not the
      host running `mktemp` — and `sqlcmd`'s own column padding hiding behind
      what looked like a clean byte-exact JSON comparison) and one a genuine
      defect in the dialect: `append_only_sql`'s `CREATE OR ALTER TRIGGER`
      shared a batch with every statement before it, which SQL Server refuses
      outright (`Msg 111`). Recorded as `spec/databases/audit.md` **D-12**,
      fixed by giving `MssqlDialect` its own statement terminator (`\nGO`,
      the same technique `OracleDialect` already uses). Both crates are
      **Schema**; **no engine crate remains at Dialect**. Neither dialect
      annex was ratified — the three earlier Schema engines' annexes are
      still *proposed* too, so ratification and reaching Schema are
      independent axes, and ratifying either was left as a separate,
      editorial judgement rather than made here. Updated everywhere the level
      is restated: `conformance-matrix.md`, both dialect annexes, both
      crates' `lib.rs`/`README.md`, `AGENTS.md`, `CLAUDE.md`, `README.md`,
      `index.md`, `openehr-store`'s `README.md`/`spec/conformance.md`,
      `agents/conformance.md`, the CI `claims` job's guard, `spec/audit.md`
      and `spec/databases/audit.md`'s dated corrections, `CHANGELOG.md`,
      `RFC.md`, `COMPARISONS.md`, and the outreach draft/index.
- [ ] **ITS-REST completeness, and say which release.** State the ITS-REST
      version `openehr-loco` targets and its base path (`/openehr/v1` here;
      `/rest/openehr/v1` is what tooling expects — thread #21 had to add a
      per-server URL scheme). Add `EHR_STATUS`, `VERSIONED_COMPOSITION`
      (`…/versioned_composition/{uid}`), `POST /query/aql` (blocked on the
      first P1 item), and the definition endpoints for OPT 1.4 (blocked on
      the second). Replace `DELETE → 501` with the deletion version openEHR
      actually specifies. Publish an OpenAPI 3 document generated from the
      routes and check it in. *Evidence:* openEHR Explorer connects with no
      server-specific branch; every endpoint has an `http.rs` test. — **L**

      **2026-09-08: three more concrete divergences, found running the new
      conformance runner against a real EHRbase 2.35.1**
      (`openehr-loco/spec/conformance-runs.md`), not read off its docs:
      `POST /ehr` expects a caller-built `Ehr` object here, against
      query parameters or an `EHR_STATUS` body there — a real request-shape
      mismatch, not only a base-path one; the real ITS-REST history and
      version-read resource is `versioned_composition/{uid}/...`, confirmed
      against EHRbase's own `/v3/api-docs`, not `composition/{uid}/_history`
      as this crate has it; and `GET /composition` (search) is not part of
      ITS-REST at all — absent from EHRbase's own OpenAPI paths — so it is
      this crate's own invention with nothing to check it against, and
      belongs off this item's list rather than on it.
- [ ] **Strict readers.** Thread #1's strictness list is the bar: refuse
      undeclared keys and duplicate keys on the canonical-JSON ingress path,
      and make every refusal name the JSON path and the requirement. Decide
      `deny_unknown_fields` per RM class as a stated policy (`lib:`
      requirement), not crate-by-crate accident; duplicate-key refusal needs
      a custom `serde_json` map visitor. *Evidence:* invalid twins for each
      refusal beside the valid fixture, the way #16 describes. — **M**

      **Scope corrected 2026-09-06, before landing any of it.** Both halves
      of this task's own premise turned out to be wrong, in opposite
      directions.

      **Duplicate-key refusal needs no code at all.** `serde`'s own derived
      struct/enum `Deserialize` already refuses a repeated key by default —
      verified against a scratch crate and then against
      `openehr::rm::ehr::Composition` directly: `{"name":"a","name":"b",…}`
      returns `Err("duplicate field `name`")`, with no custom visitor,
      through `#[serde(flatten)]` and through `#[serde(tag = "_type")]`
      internally-tagged dispatch alike (`DataValue`, `ContentItemWire`). This
      has evidently been true throughout; the "needs a custom map visitor"
      line was never checked. The only path that loses this guarantee is
      deserializing into `serde_json::Value` first — which nothing on the
      ingestion path (`axum::Json<Version<Composition>>` in
      `openehr-loco`) does.

      **`deny_unknown_fields` cannot be adopted as this task describes,
      because a stated policy already exists and says the opposite:
      `J9.9`** — "The crate MUST ignore attributes it does not model rather
      than rejecting the document. openEHR adds attributes between minor
      releases, and a strict reader rejects tomorrow's payload for
      containing something it does not need." This is not an oversight to
      correct; it was deliberately reaffirmed once already, in `lib:A-23`'s
      own fix ("Deserialization stays lenient rather than being made to
      refuse, because `J9.9` says so and the reason holds: a document that
      cannot be read cannot be inspected, repaired, or reported on").
      `J9.7` names the specific case this task's own JSON would hit: "A
      `_type` on a concrete class MAY be ignored, because there it is
      redundant" — `CODE_PHRASE` is a real example that does not model its
      own `_type`, and adding `deny_unknown_fields` across
      `openehr::rm`'s ~85 `Deserialize`-deriving types (confirmed
      mechanically achievable — `#[serde(deny_unknown_fields)]` composes
      correctly with `#[serde(flatten)]` in every arrangement tested,
      contrary to a real historical `serde` limitation this task may have
      been written against) broke it immediately: 7 existing tests failed,
      two of them asserting `J9.9` itself by name
      (`validation::tests::a_version_envelope_is_checked_on_data_that_arrived_as_json`,
      `security::redact::tests::a_redacted_composition_is_still_valid`).
      Reverted rather than landed.

      So this is not an implementation gap; it is two stated goals in
      direct conflict — Thread #1's "refuse the undeclared" against this
      crate's own forward-compatibility guarantee — and **choosing between
      them is a maintainer decision, not code**, the same footing
      `tasks.md`'s own regex item already stands on. One shape that
      resolves it without repealing `J9.9`: strictness as an opt-in posture
      at the ingestion boundary (`openehr-loco`, or a `Store::commit_*`
      flag) rather than baked into the RM's own `Deserialize` — refusing at
      the edge an operator chooses to run strict, while the library itself
      keeps reading tomorrow's payload. Not decided here.
- [ ] **Run an external corpus, and cite the spec per test.** The single
      strongest answer to #15: fixtures nobody here wrote. Feed the openEHR
      SDK's canonical JSON examples and Better's web-template test
      compositions through `serde_json → validate()`, and **every CKM
      archetype** through `am::cadl::parse_definition`, recording per file
      whether it parsed, refused by name (`K15.6`), or failed for a reason
      this tree does not yet state. Add a `spec_refs` line to every
      conformance case naming the section it tests, and generate an index
      from it. *Evidence:* a committed results table with dated counts, and
      a CI job that fails when a previously-parsing file stops parsing.
      — **M**
      - [x] 2026-09-03: the archetype half, first run.
        `openehr/tests/adl_corpus.rs` (ignored; `OPENEHR_ADL_CORPUS`) over
        `openEHR/adl-archetypes` at `093c77ea`, results in
        [`openehr/spec/corpus.md`](openehr/spec/corpus.md). Two findings
        from it fixed the same day: `A-70` (differential-form attributes)
        and `A-71` (an unstated `occurrences`, two thirds of every refusal;
        `K15.32`), taking parsed `.adls` from 178 to 774 of 1,379. Run 2
        recorded there with the next candidates.
      - [x] 2026-09-05: the JSON half, first run.
        `openehr/tests/json_corpus.rs` (ignored; `OPENEHR_JSON_CORPUS`) over
        `ehrbase/openEHR_SDK`'s own canonical-JSON reference fixtures at
        `e57511c`, results in
        [`openehr/spec/json_corpus.md`](openehr/spec/json_corpus.md).
        Two findings from it: `A-78` (a comma decimal sign in a
        fractional second, refused though openEHR's own ADL grammar names
        it — 21 of 57 fixtures) and `A-79` (`TEMPLATE_ID` refusing
        whitespace a real template name needs) fixed the same day; one
        needed no new number — the run also falsified one of the two
        grounds `A-02` (already open, "ISO 8601 basic format refused")
        gives for its own refusal, corrected in place rather than left
        standing on a claim now known false; a fourth, `A-81`, examined
        every invariant `validate()` reported on a parsing file and found
        one real gap — a correct Spanish rubric reported as a violation,
        not *unchecked*, because the rubric table this crate carries is
        English-only — left open by decision, with a new `D3.7a`. "Better's
        web-template test compositions" named in this item's own text
        turned out not to fit: WebTemplate is a
        simplified, non-canonical format this crate has no reader for
        (`K15.14`–`K15.17`, not implemented), so running it through
        `validate()` would test nothing real — noted rather than forced.
      - [x] `spec_refs` per conformance case, and the index.
        **Scoped 2026-09-06, before starting.** "Conformance case" is
        undefined here, and the two readings are far apart in size. Narrow:
        the corpus runners' own disposition tables already name a section
        per file/finding (`corpus.md`, `json_corpus.md` — e.g. `K15.6`,
        `D3.13a` per row), so this sub-item may already be substantially
        met for the corpus half specifically, and "the index" could be a
        small, mechanical reverse-mapping generated from those two tables.
        Wide: every `#[test]` in the tree — 95 in `openehr/tests/*.rs`
        alone, before the far larger count of `#[cfg(test)] mod tests`
        unit tests inside `src/` (630+ tests total, `A-58`'s own count) —
        annotated with a *new*, machine-checked citation tag, when an
        *informal* version of exactly this already exists and is already
        common: 74 distinct requirement ids cited in backtick-quoted doc
        comments across `openehr/tests/guarantees.rs`, `invariants.rs`, and
        `properties.rs` alone, unchecked by anything. Building the
        wide reading properly needs a new scanner (`openehr-assets`
        already has one shape of this, `regex_citations`, but for RM
        invariant calls, not doc-comment requirement ids; it is not a
        drop-in), a decision about what counts as "a conformance case" at
        all, and a decision about whether to formalise the citations
        already there or add a second, parallel convention beside them —
        each its own design question, not a mechanical add. Sized on the
        wide reading, this is its own item, not a sub-bullet of this one;
        left unscoped rather than started on a guess. The narrow reading
        (corpus-table index) is achievable and not yet attempted.

        **2026-09-08, the narrow reading.** `scripts/generate-corpus-index.py`
        (`--write`/check, the same convention as `generate-llms-files.py`)
        scans `corpus.md` and `json_corpus.md` for backtick-quoted ids —
        findings (`A-71`) and requirements (`K15.6`, `D3.13a`) — and
        generates `openehr/spec/corpus-index.md`: a reverse map from id to
        every place a run cites it, by file, section, and the citing line.
        Adds no new citation and no new convention; it makes the ones the
        two run reports already carry askable in the other direction. 24
        ids, 74 citations, first run. Wired into the `claims` CI job (a new
        step, not a new job) so a corpus run that adds a citation and
        forgets to regenerate the index fails the build the same way a
        stale `llms.txt` does. Both `corpus.md` and `json_corpus.md` gained
        a one-line cross-reference to it. Rows in `AGENTS.md` and
        `spec/audit.md`'s `claims` job descriptions updated to mention it;
        `check-docs.py`'s "12 CI jobs have a row" count is unaffected — a
        step, not a job. `check-trademarks.py` unaffected — the generated
        file does not use the openEHR mark in prose.
      - [ ] The regression job, either half. Blocked on a corpus this tree
        may carry: `adl-archetypes` has no licence file (`corpus.md`
        §Licence); `openEHR_SDK` is Apache-2.0 and could be vendored, but
        is read where it is instead, for consistency with the other half
        (`json_corpus.md` §Licence) rather than because the licence
        requires it.
- [x] **Close the corpus's open candidates, largest first** (`corpus.md`
      §Candidates; each needs its test and grammar reading before an `A-`
      number, `W0.19`). *Done 2026-09-03, run 3 (916 of 1,379 parsed):*
      (4) the unwrapped interval's kind, decided by token (`A-72`); the
      closed slot `A-71` had unlocked (`A-73`); the relop interval
      spelling `|>=0.0|` that surfaced behind it (`A-74`). *Done
      2026-09-04, runs 4–6 (969 of 1,379 parsed):* (1) and (5) together,
      temporal patterns and unwrapped temporal literals (`A-75`); (2),
      `primitive_kind`'s case-insensitive match, reproduced for real
      chasing `A-75`'s own residual (`A-76`); (7), a negative unwrapped
      number had no dispatch at all (`A-77`); (8), the `VACMCU` refusal
      `A-76` surfaced examined and closed with no code change — seven
      files share one boilerplate `SECTION` whose own cardinality and
      mandatory children contradict each other (the archetype's own
      defect, correctly refused), the eighth is the reference suite's own
      invalid fixture. *Remaining, correctly left open rather than forced*:
      (3) refusal names correct but unhelpful, and (6) the unused `+/-`
      spelling — neither has a next action a corpus run can supply; the
      Reference Model multiplicity decision moved to `plan.md`'s own open
      decisions, where a design call belongs. *Evidence:* run 6 in
      `corpus.md`, dated, with the counts. — **M**

### P2 — operability and evidence

- [x] **`Dockerfile`, `docker-compose.yml`, `.devcontainer/`.** Thread #9
      lost an afternoon to a compose override; #21's whole review was "one
      `docker compose up` and it's running". Build `openehr-loco` as a
      static binary in a multi-stage image, compose it with nothing but a
      volume, and add a devcontainer so a Codespace boots the published
      image. *Evidence:* `docker compose up` answers `curl /openehr/v1/
      metadata` on a clean machine; the quickstart in `INSTALL.md` is the
      command, not a paragraph. — **M**

      **2026-09-08.** Two scope corrections against the task's own words,
      both because there is no publishing pipeline for `openehr-loco`
      images (it is `publish = false`, `AGENTS.md`) and standing one up is
      a maintainer-level release decision, not this task's to make:
      "static binary" is scoped to what is actually true — `rusqlite`'s
      `bundled` feature means no `libsqlite3` at runtime, but the binary
      still links `glibc`, so the runtime stage is `debian:bookworm-slim`,
      not `scratch` (the `Dockerfile`'s own header comment says so, rather
      than let "static" overclaim). "Boots the published image" became
      "the devcontainer builds from the local `Dockerfile`" —
      `.devcontainer/devcontainer.json` uses `rust:1.98-bookworm` (the
      builder stage's own base, since a Codespace needs the compiler the
      runtime stage deliberately drops), not an image nowhere exists to
      pull.

      Two real defects found only by actually building and running the
      image, not by reading the Dockerfile:

      1. No `.dockerignore` existed, so `COPY openehr ./openehr` (and the
         three crates beside it) sent each crate's `target/` — a combined
         ~37 GB on this machine — into the build context. The first build
         attempt looked hung for that reason; it was copying tens of
         gigabytes nothing downstream can use (a host `target/` cannot be
         reused by a different toolchain and path inside the image).
         Fixed by adding `.dockerignore` (`**/target/`, `.git/`); the four
         `COPY` steps went from indefinite to instant.
      2. Loco's own default environment is `development`
         (`loco_rs::environment::DEFAULT_ENVIRONMENT`) unless `LOCO_ENV`
         says otherwise, and nothing in the first version of the
         `Dockerfile` set it — so the container would have silently loaded
         `config/development.yaml`, whose `binding: localhost` is exactly
         the mistake `config/production.yaml` (this task's own prerequisite
         fix, `development.yaml` bound to `localhost` being unreachable
         from outside a container) exists to avoid. Fixed with
         `ENV LOCO_ENV=production` in the runtime stage; confirmed by the
         container's own startup banner logging `environment: production`.

      One more found and fixed as a robustness improvement, not a
      network-only workaround: the runtime stage's original
      `apt-get install ca-certificates` duplicates a package the builder
      stage's `rust:*-bookworm` image already carries (it is
      `buildpack-deps`-based). Changed to `COPY --from=builder
      /etc/ssl/certs /etc/ssl/certs`, which removes a second,
      independent package-index fetch from the runtime stage entirely —
      worth doing regardless of any one day's network conditions, though
      it was this session's own flaky path to `deb.debian.org` (`apt-get`
      caught in an `Ign:`/re-`Get:` retry loop on the same 8.6 MB file,
      confirmed not podman-specific by a bare `curl` from the host timing
      out against the same mirror) that surfaced it. What was **not**
      changed in anything committed: verifying the build on this
      machine also needed `podman build --pull=never` plus a bind-mounted
      host `~/.cargo/registry`, because pulls to `docker.io` and
      `crates.io` were, independently, too slow or actively failing
      (`SSL_ERROR_SYSCALL` mid-transfer) today — neither belongs in
      `Dockerfile` or `docker-compose.yml`, since a normal network needs
      neither, and baking in a local cache path would silently break the
      image for anyone without this machine's exact cache.

      Full, real verification, in order: `podman build` (multi-stage,
      producing `openehr-loco:test`) → `podman run` with the port, volume,
      and env vars `docker-compose.yml` declares → `cargo run --example
      generate_test_token` for a throwaway key → `curl
      http://localhost:5150/openehr/v1/metadata` → `200`. Then the actual
      committed `docker-compose.yml` itself, end to end: `podman compose up
      -d` (Podman's Compose v2 delegate) → the same `curl` → `200`, log
      line `environment: production`, `listening on http://0.0.0.0:5150` →
      `podman compose down -v`, clean teardown, no leftover containers,
      volumes, or images. `INSTALL.md` gained a "Run the HTTP service"
      section ending in exactly that `curl` command — the quickstart is the
      command, not a paragraph. `scripts/check-docs.py` and
      `scripts/check-trademarks.py` both clean; no Rust source changed, so
      no crate's `cargo test`/`clippy` run was needed.
- [x] **Publish measured numbers.** Add store commit and read benchmarks
      to `openehr-store/benches/store.rs` and an HTTP round-trip benchmark
      for `openehr-loco`, run them on a named machine, and put the numbers
      with their date and hardware in `BENCHMARKS.md`. Keep `W0.35`/`W0.36`:
      run, never gated. Then take #4's offer. *Evidence:* dated numbers in
      the file, reproducible by the command beside them. — **M**

      **2026-09-07.** The commit/read benchmarks could not go in
      `openehr-store/benches/store.rs` as literally written: that crate has
      no connection to round-trip through (its own bench file says so —
      "everything else in a write is a round trip to a server, which no
      benchmark in this process can measure honestly"), and adding
      `openehr-sqlite` as any kind of dependency of `openehr-store`, even
      dev-only, would be the exact inward-only layering cycle the `layering`
      CI job exists to catch. Went in `openehr-sqlite/benches/store.rs`
      instead — the crate with the real connection — which is consistent
      with, not a departure from, the store crate's own stated reasoning.
      New benches: `commit/composition`, `read/get_version`,
      `read/latest_version` (`openehr-sqlite`) and `http/read_composition`
      (`openehr-loco`, `tower::oneshot` in process, no socket opened).
      Numbers dated 2026-09-07 in `BENCHMARKS.md`, same machine as the
      2026-08-26 measurement it sits beside rather than replaces. `cargo
      test`/`clippy -D warnings` clean in both crates; `check-docs.py`'s
      `bench_crates` count moved from 2 to 4, mechanically, with nothing
      elsewhere in the tree asserting the old number. **Taking #4's offer is
      not done here** — posting to the thread is the maintainer's own
      action (`GOVERNANCE.md` §Machines do not decide), same as the
      Discourse reply draft.
- [x] **Supply chain: SBOM, `cargo-deny`, `cargo-audit`, push protection.**
      `SECURITY.md` names "no SBOM" as an open gap. Add `cargo auditable`
      builds and a CycloneDX SBOM per release artefact, `cargo deny check`
      and `cargo audit` as CI jobs (Dependabot only *alerts*), and turn
      secret-scanning push protection on. Trusted Publishing stays on its
      stated condition (`spec/trusted-publishing/`). *Evidence:* the jobs in
      `ci.yml` with rows in `AGENTS.md` and `spec/audit.md` (the `claims`
      gate requires them); the gap struck through in `SECURITY.md`. — **S**

      **2026-09-06.** Three of four done in full; the fourth narrowed rather
      than forced. Push protection: enabled via the GitHub API, verified with
      a `GET` immediately after (`{"status":"enabled"}`), against a
      repository with zero existing secret-scanning alerts — nothing
      retroactive to worry about. `cargo deny check` and `cargo audit`: a new
      `supply-chain` CI job, all eighteen crates including the eight fuzz
      crates (a check silently skipped on the fuzz crates would be exactly
      `W-13`'s shape again), one shared `deny.toml` at the repository root
      since there is no root workspace for either tool to find a config in on
      its own. Real findings along the way, not a rubber stamp: a yanked
      `chacha20` fixed by `cargo update`; two missing-but-legitimate licences
      added (`0BSD`, `CDLA-Permissive-2.0` for `openehr-loco`'s mail
      dependencies; `NCSA` for every fuzz crate's `libfuzzer-sys`); a `bans`
      false positive from `wildcards = "deny"` misreading this monorepo's own
      unversioned sibling path-dependencies as registry wildcards, reverted
      to the tool's own default; and three RUSTSEC advisories
      (`RUSTSEC-2026-0194`, `-0195`, `-0235`) confirmed unfixable by hand
      (`cargo update --precise` refused each, naming the exact upstream pin
      in `opendal`/`rust_decimal`) rather than assumed so, accepted with
      dated reasons in `deny.toml` and the workflow. **SBOM narrowed, not
      closed:** `cargo cyclonedx` verified and documented as a pre-publish
      step (`agents/publishing.md`), but no release has cut since, so no SBOM
      has yet accompanied a real one. `cargo auditable` (embeds a dependency
      manifest into a *binary*) has no target at all yet — `openehr-loco` is
      the one crate that builds a binary, and nothing distributes it
      anywhere (no Docker image, no release artefact); that gap belongs to
      the `Dockerfile`/`docker-compose.yml` item below, not manufactured
      here. `SECURITY.md` updated to match exactly this state, not more.
- [x] **State the production perimeter.** `openehr-loco` has no TLS, no
      rate limiting, no read audit at the store (`db:D-04`, "fixed above the
      store"), no RBAC. Write the deployment statement — TLS terminated by
      a reverse proxy, what is and is not audited, that one PASETO key set
      is the whole authorisation model — into `openehr-loco/README.md` and
      `PHI.md` §Known limits, and add rate limiting at the router. *Evidence:*
      a reviewer can answer `PHI.md`'s questionnaire section from the
      documents alone. — **S**

      **2026-09-08.** Rate limiting: `tower_governor`'s `GovernorLayer`,
      layered over the whole router in a new `Hooks::after_routes` (nothing
      previously implemented that hook). Keyed on `GlobalKeyExtractor`, not
      the crate's own default (peer IP) — deliberately, and stated as such
      in three places (`after_routes`'s own doc comment, the README, and
      `PHI.md`): behind the reverse proxy this service is meant to sit
      behind, the peer IP this process sees is always the proxy's, so a
      per-IP limiter would silently become a global one anyway, while
      implying more protection than it delivers. Trusting
      `X-Forwarded-For` to recover the real caller was considered and
      rejected — the same shape of mistake `auth.rs` already refuses for
      identity, and not worth reopening for rate limiting. 50 requests
      burst, replenished one per second; verified directly (`tests/http.rs`'s
      own router-building fixture never calls `after_routes`, so it could
      not have caught a limiter wired up wrong or not at all — the same
      gap `install`'s test already closes for `before_run`): a new test in
      `src/app.rs` drives 51 requests through `after_routes`'s own output
      and asserts the 51st, and only the 51st, is `429`. Mutation-checked
      (`cargo mutants --re after_routes`): 1 mutant (the whole function
      body replaced with `Ok(Default::default())`), caught.

      The deployment statement itself: a new "§The deployment perimeter"
      section in `openehr-loco/README.md` (no TLS in this process — a
      reverse proxy terminates it; one PASETO key set is the whole
      authorization model, not authentication *and* authorization; rate
      limiting is global, stated with the reasoning above; read auditing
      is off by default and per-deployment), and four new bullets in
      `PHI.md` §Known limits carrying the same three points plus a
      cross-reference from the existing read-auditing bullet to the HTTP
      edge log `openehr-loco` can turn on. `PHI.md` §If you are filling in
      a questionnaire now points a deployer of `openehr-loco` at its
      README section directly, closing the evidence bar in the task's own
      words: the questionnaire is answerable from the documents, not
      assembled from source by the reviewer.

      Test count moved from 53 to 54 in three places (`README.md`,
      `openehr-loco/README.md`, and the new test itself) — `cargo test`
      confirms 54 (18 lib + 29 http + 7 tasks). `RUSTFLAGS="-D warnings"
      cargo clippy --all-targets` and `cargo deny check` both clean with
      `tower_governor` and its transitive dependencies (`governor`,
      `quanta`, …) added; the pre-existing `winnow`/`windows-sys`
      duplicate-version warnings `cargo deny` reports are unrelated,
      already present before this change, and are warnings under
      `deny.toml`'s policy, not failures. `check-docs.py` and
      `check-trademarks.py` clean.
- [x] **Assess the 144 unassessed database requirements (`db:D-11`).** In
      batches by section, `M3` and `S1` first as the finding recommends,
      then wire `scripts/check-databases-matrix-coverage.py` into CI once it
      would pass on day one. *Evidence:* the script green in CI; `D-11`
      closed. — **L**

      **2026-09-09, batch 1 of N: `M3` and `S1`, 27 of 144.** Read each
      against the real code (`openehr-store/src/schema.rs`, `record.rs`,
      `error.rs`, and every core crate's `Cargo.toml`), not assumed from the
      surrounding architecture. 15 landed `•` (a real test or an
      unambiguous structural fact — e.g. `M3.29` bounded-by-construction via
      `ColTy::Id(n)`/`Text(n)`, `M3.28` a real SQL clause plus `H5.13`'s own
      test), 11 landed `?` (true on inspection, not actively guarded —
      `S1.6`–`S1.12`'s "core MUST NOT" list, `M3.20`/`M3.21`/`M3.38`), 1
      landed `—` (`S1.21`, a statement about the specification's own
      structure, not this crate's code), 0 landed `✗`. Full account,
      row by row, in `spec/databases/audit.md` **D-11**'s own dated
      paragraph; the matrix rows themselves carry the per-requirement
      evidence. Found and fixed in passing: `D-11`'s own text listed `doc`
      as one of this file's marks — this matrix's Legend has only five
      (`•`, `~`, `?`, `✗`, `—`); `doc` belongs to the *library* matrix's
      legend, a different file. `check-databases-matrix-coverage.py`:
      144 → 117 missing. Still **not** wired into CI — 117 remain, so it
      would not pass on day one, exactly the condition this item's own text
      sets for wiring it in.

      **2026-09-09, batch 2: `C0`, 21 of 21 — the whole section.** `D-11`
      itself predicted this section would resolve like the library
      matrix's own `C0` rows, marked `doc`. Checked rather than assumed,
      and it could not resolve that way for the reason batch 1 already
      found: this matrix has no `doc` mark. 7 landed `—` (pure reading
      conventions and editorial-process rules — RFC 2119 interpretation,
      rationale, what counts as a departure, how an amendment is
      conducted — not claims about any crate's code at all), 9 landed `•`
      against real, already-existing checks (the ladder table in this same
      file, `check-docs.py`'s level-consistency and shared-block checks,
      the `schema` CI job running each dialect against its own server —
      the actual fix for `W-01`, `verify-schema.sh`'s row-present insert,
      all six dialect annexes' `M14.x` entries, and the literal absence of
      `07-`/`08-`/`14-` numbered files), 5 landed `?` (id-format and
      no-reuse-and-next-ordinal discipline, cross-directory qualification,
      and departure review — all followed in practice, none enforced by a
      script), 0 landed `✗`. Full account in `spec/databases/audit.md`
      **D-11**'s own dated paragraph. `check-databases-matrix-coverage.py`:
      117 → 96 missing.

      **2026-09-09, batch 3: `W16`, 18 of 18.** Repository/release
      conventions, more concretely checkable than `C0`'s prose. 11 landed
      `•` against real existing checks — `check-docs.py`'s crate counts,
      version-agreement, and level-consistency checks; `X15.15`'s
      cross-dialect DDL comparison (the actual mechanism that found
      `W-01`); `agents/publishing.md`'s documented pre-publish checklist;
      the `examples` CI job; direct inspection for two structural facts
      (every engine crate's `spec/` holds one dialect annex and nothing
      else; all eighteen crates declare their own `[workspace]`). 6 landed
      `?` (true by inspection, none independently tested). 1 landed `~`
      (`W16.12`, once `CHANGELOG.md` was found to exist but cover the eight
      crates as a set, not literally one-per-crate). 0 landed `✗`. **Found
      and fixed in passing, twice**: `16-repository-and-release.md`'s own
      text said the crates "currently share `0.2.0`" (six releases stale —
      `check-docs.py`'s version check reads this file but looks for
      phrasings this sentence does not use, so the drift passed through
      unflagged) and "No crate here has one yet" about a changelog
      (`CHANGELOG.md` exists). Both corrected in place, dated.
      `check-databases-matrix-coverage.py`: 96 → 78 missing.

      **2026-09-09, batch 4: `T11`, 14 of 14 — and a real defect found
      along the way, `db:D-13`.** Tracing `T11.8`'s own "no chain exists
      (`M3.16`)" back to its source turned up a genuine, five-week-stale
      self-contradiction: `03-storage-model.md`'s Digests section said no
      digest was stored anywhere and the chain did not exist, while
      `M3.16`'s own bullet two pages earlier already said *(amended —
      implemented)*, and the store-level table's own `M3.16`/`M3.39`–`M3.42`
      rows were already marked `•`. `openehr_version` has carried real
      `chain_previous`/`chain_content`/`chain_digest` columns since
      `f951556`, 2026-08-02. Fixed in both places the claim appeared
      (`03-storage-model.md` and `T11.8` itself, which also cited a
      three-engine count six weeks out of date and an "each algorithm"
      framing `M3.39` had already made moot) — full account in
      `spec/databases/audit.md`'s new **D-13**.

      The batch itself: 11 of 14 landed `•`, most citing evidence already
      on the matrix for a different id — `T11.15`/`T11.16`/`T11.17`/
      `T11.20` restate `G2.9`/`M3.36`/`X15.15`–`X15.16` exactly; `T11.18`
      restates `X15.18`; `T11.21` restates the `W16.9` row just added;
      `T11.10` cites the `mutants` CI job; `T11.13` cites `C0.8`'s own
      ladder; `T11.19` cites `ci.yml`'s own stated principle of invoking
      `verify-schema.sh` identically to a contributor; `T11.6` cites
      `openehr-sqlite/tests/concurrency.rs`, the same test `H5.4`/`R4.5`
      already use. 1 landed `~` (`T11.8`, corrected as above — real
      tamper-evidence coverage now, minus key-rotation specifically). 2
      landed `?` (`T11.11`'s narrowest-assertion style discipline; `T11.14`,
      vacuously true since no database crate currently has an `#[ignore]`d
      test). 0 landed `✗`. `check-databases-matrix-coverage.py`: 78 → 64
      missing.

      **2026-09-10, batch 5: `O10`, 13 of 13.** Operations — logging,
      install idempotence, schema versioning, connection security, backup,
      release evidence. 7 landed `•`, two of them from an exhaustive,
      direct finding rather than a pointer to an existing test:
      `openehr-store` and `openehr-sqlite` emit **no log line at all** —
      no `tracing`, `log`, `println!`, or `eprintln!` anywhere in either
      crate — which settles both `O10.2` (no stored content in a log) and
      `O10.13` (no bound parameter logged) at once. `O10.16` cites a real
      test (`openehr-sqlite/tests/conformance.rs` deletes the
      schema-version row from a database holding data and gets exactly
      the refusal the requirement names); `O10.4` cites `G2.13` and
      `verify-schema.sh`'s twice-run requirement; `O10.10` cites the
      `supply-chain` CI job and every crate's committed `Cargo.lock`;
      `O10.11` cites `agents/publishing.md`'s tag-as-part-of-publishing
      step; `O10.12` cites `T11.19`, which this requirement's own text
      already names. 5 landed `?` (backup format, connection encryption
      and TLS documentation — the latter two vacuously true, since no
      engine crate opens a live network connection yet — schema-version
      bumping discipline, and point-in-time restore as a logical but
      untested consequence of `M3.17`). 1 landed `—` (`O10.4a`, already
      self-classified *not applicable* in its own text). 0 landed `✗`.
      `check-databases-matrix-coverage.py`: 64 → 51 missing.

      **2026-09-10, batch 6: `X15`, 10 of 10 — plus two more copies of
      `db:D-13`'s stale premise.** Portability/dialect-boundary section,
      largely restating facts this file had already evidenced elsewhere:
      9 landed `•` (`X15.1`/`X15.14` restate `S1.1`/`M3.22`; `X15.2`
      restates `R4.11`; `X15.3` cites the one shared schema declaration;
      `X15.7`/`X15.8` restate `C0.14`/`W16.5`; `X15.12` cites `X15.15`/
      `X15.19`, named in its own text; `X15.13` cites the `Dialect`
      trait's own method list; `X15.20` restates `M3.35`), 1 landed `?`
      (`X15.17`, true after `W-01`'s fix, unenforced against a future
      copy), 0 landed `✗`. Assessing this section surfaced two more
      instances of `db:D-13`'s stale "no chain exists" premise —
      `X15.11` itself, and `openehr-oracle`'s own `M14.8` departure — both
      corrected in place, `D-13` extended to record all four. Full
      account in `spec/databases/audit.md`. `check-databases-matrix-
      coverage.py`: 51 → 41 missing.

      **2026-09-10, batch 7: `P6`, 9 of 9 — the first two genuine `✗` in
      the whole assessment.** Every prior batch (77 requirements) landed
      `0 ✗`. This one found two real, unmet requirements: `P6.16` (every
      search target must declare a kind — the framework exists, but no
      declaration exists for any of the seven real indexed columns) and
      `P6.7` (`find_compositions_by_archetype` returns every match,
      unbounded, no `LIMIT`, no page parameter — `openehr-loco` bounds it
      above the store, the same shape as `PR12.5`, so it joins that table
      rather than getting a bare `✗`). 4 landed `•` (`P6.4`'s seven named
      indexes confirmed in the one shared declaration; `P6.11`'s five
      required query capabilities exercised in `conformance.rs`; `P6.14`
      restates `M3.28`; `P6.19` restates `P6.18`). 3 landed `?`. Full
      account, including why neither `✗` is a documentation defect like
      `db:D-13`'s, in `spec/databases/audit.md`. `check-databases-matrix-
      coverage.py`: 41 → 32 missing.

      **2026-09-10, batch 8: `G2`, 8 of 8.** Schema generation — emission
      order, idempotence, identifiers — mostly structural fact rather than
      runtime behaviour, since the schema is declared, not generated. 7
      landed `•`: `G2.7` against `schema::TABLES`'s own const declaration;
      `G2.10`/`G2.11` against the shared `Dialect::ddl` default (no dialect
      overrides it — tables, then all indexes, then append-only, in that
      order); `G2.12` restates `M3.23`; `G2.14` against the `Idempotence`
      enum's own shape; `G2.17` against `openehr-mysql`'s real
      `index_idempotence` returning `Inline` while `openehr-mariadb` is
      deliberately left at the default; `G2.18` restates `X15.3`. 1 landed
      `?` (`G2.19` — true by inspection, longest identifier today 29
      bytes, nothing would catch a longer one). 0 landed `✗`.
      `check-databases-matrix-coverage.py`: 32 → 24 missing.

      **2026-09-10, batch 9: `V9`, 7 of 7 — every one a bullet.**
      Validation: validate before writing, the two-gate discipline, what
      a refusal reports, what validation does not claim. The first batch
      of the whole assessment where every requirement landed `•`, because
      `lib:A-23`'s own fix — validate the whole `Version` a caller sent,
      not just the `Composition` inside it — already built exactly what
      this section asks for, months before this assessment read it.
      Strongest single piece of evidence: `error::Violation::detail` is
      `&'static str`, so `V9.7` ("never a submitted value") holds
      structurally, not by discipline — a compile-time constant cannot
      embed a runtime value. `V9.9` cites `/metadata`'s own
      `not_implemented` list naming "archetype and template validation"
      by name. 0 landed `?`, `~`, or `✗`. `check-databases-matrix-
      coverage.py`: 24 → 17 missing.

      **2026-09-10, batch 10: `H5`, 7 of 7 — a third genuine gap, found
      in the code's own comment this time.** Versioning and history. 6
      landed `•` (`H5.3` restates `P6.11`; `H5.5`/`H5.6`/`H5.7` against
      `openehr_version`'s own table shape and columns; `H5.9` against
      `CommitError`'s four distinguishable variants; `H5.14` against
      `openehr_contribution` plus `M3.17`). `H5.11` is the batch's own
      `✗`: it requires `is_deleted` to be indexed, and
      `openehr_version`'s own doc comment claimed it was — none of the
      table's three real indexes name it. The comment asserted something
      about its own file the file did not do; corrected in place
      (`openehr-store/src/schema.rs`), `assets/schema.json` regenerated
      to match. Closer to `P6.16`'s shape (written in advance, never
      applied) than `D-13`'s (true once, then drifted) — full account in
      `spec/databases/audit.md`. `check-databases-matrix-coverage.py`:
      17 → 10 missing.

      **2026-09-11, batch 11: `PR12` and `R4`, 10 of 10 — closed.** 9
      landed `•` (`PR12.3a`/`PR12.4`/`R4.9` restate `M3.34`/`M3.15`/`M3.35`;
      `PR12.9` against `AuditDetails.committer`'s required, un-`Default`ed
      type; `PR12.10` against `openehr_contribution`'s own audit columns;
      `R4.3` against the same `V9.9` evidence; `R4.6` against every `Store`
      method taking a typed identifier, never a bare string; `PR12.8` and
      `PR12.11` each found in the code's own words — `openehr-loco/src/
      auth.rs` and `PHI.md` both cite their requirement by number). 1
      landed `?` (`R4.10`, plausible by construction, untested). 0 landed
      `✗`/`~`.

      **All 144 of the originally-unassessed 221 requirements now carry a
      mark** — `check-databases-matrix-coverage.py` reports zero missing
      for the first time since it was written. Three genuinely unmet
      requirements found along the way (`P6.7`, `P6.16`, `H5.11`) and one
      stale claim traced to four places and fixed (`db:D-13`) — real
      findings, not a clean pass manufactured by guessing. Wired the
      script into CI (`claims` job) per the item's own stated condition:
      it would pass on day one, because this is the day it does.
      `AGENTS.md` and `spec/databases/audit.md`'s own `claims`-job rows
      updated; `db:D-11` itself closed, "Medium, fixed". Full account in
      `spec/databases/audit.md`.
- [x] **A specification-release pin table.** One file — `spec/releases.md`
      or a section of `openehr/spec/index.md` — naming the RM, BASE, AM,
      TERM, QUERY, and ITS-REST releases every module here was transcribed
      from, with the date and the source file, the way `terminology.rs`
      already does for one of them. Then a re-vendor check: when a release
      moves, which modules to re-read. Thread #7 and #8 are about exactly
      this risk. *Evidence:* the table; `S1.16`/`K15.2` cite it. — **S**

      **2026-09-06.** `openehr/spec/releases.md`, checked against each
      specification repo's own GitHub API (tags/releases, and per-file commit
      history) rather than assumed from what was already cited in the tree.
      The honest result is not the tidy table the headline implies: only
      `TERM` has both a file and a date, and re-reading it today reproduces
      exactly what was read on 2026-07-31, since the cited file has not
      changed since. `RM` names a version target (1.1.0, matching the latest
      tag exactly) but no file. `BASE` and `AM` cite specific files that
      *have* changed on `master` since the last numbered release — confirmed
      by comparing blob shas at the tag versus `master` for one sampled file
      each, not merely by comparing dates. `QUERY` and `AM`'s own release
      number (`K15.2`) have no citation of any kind; the `AM` corpus (a
      *different* thing — real archetype fixtures, not the AOM2 specification
      itself) is the one part of AM that is genuinely commit-pinned. Both
      `S1.16` and `K15.2` now cite the file, and `K15.2` is corrected in
      place: it stays unmet, now for the specific, verified reason above
      rather than a general one — naming "AOM 2.3.0" would have been exactly
      the kind of unchecked number this repository's culture forbids, so it
      was not named. The "re-vendor check" the item's own text asks for is
      in the file: read `ITS-REST` first (its own latest release, `1.1.0`,
      postdates `openehr-loco`'s work on it by weeks), then `QUERY` (no
      citation to go stale — the first read is the first citation), then
      `BASE`/`AM` (confirmed drifted), `RM` last (stable, matches its tag).
- [ ] **An ambiguities register, and file it upstream.** SEC asked (#19)
      that spec inconsistencies not be discarded. Collect the spec silences
      and contradictions this tree has already adjudicated — `versions` typed
      `List` but described as a set, `is_modifiable` evaluation time, the
      `C_STRING` regex-in-list shape (`A-63`), `ARCHETYPE_HRID` vs
      `ARCHETYPE_REF` (`A-49`), the `TERM` repository that disagrees with
      the computable one — into `openehr/spec/ambiguities.md` with the
      disposition each got, and open one openEHR tracker issue per entry.
      *Evidence:* the file, and issue links beside each entry. — **M**

      **2026-09-09, the register itself.** `openehr/spec/ambiguities.md`,
      all five named entries, each with the specific citation (grammar
      files, class definitions, module docs) rather than a restated
      summary — the `TERM` repository's exact disagreeing codes, the three
      ADL identifier grammars' exact differences, `C_STRING`'s one
      interpretive decision beyond the type fix, `is_modifiable`'s
      not-yet-implemented status stated plainly rather than implied
      finished, and `versions` List-vs-Set noting FerroEHR's own `#2674`
      rather than duplicating it. Linked from `openehr/spec/index.md`.
      **Filing upstream is not done here** — opening issues against an
      openEHR tracker on this project's behalf is a maintainer action
      (`GOVERNANCE.md` §Machines do not decide), the same footing as the
      Discourse reply draft and the benchmarks thread post. Every entry
      says "Filed upstream: not yet" (two — `is_modifiable` and `versions`
      — cite FerroEHR's own existing tracker numbers instead, since those
      were filed by thread #6, not by this project). `check-docs.py` and
      `check-trademarks.py` clean; the latter correctly does not scan
      `openehr/spec/*.md` at all (`ROOT.glob("*.md")` is root-level only),
      so this file joining `corpus.md`/`json_corpus.md`/`releases.md`
      outside its scope is consistent, not a gap.
- [x] **A conformance runner anyone can point at any server.** Thread #16's
      `scripts/conformance.sh` with bring-your-own-SUT is the model: a
      catalogue of HTTP cases against ITS-REST, runnable against
      `openehr-loco`, EHRbase, or FerroEHR, with verdicts committed. Start
      with the eleven endpoints that exist. *Evidence:* the runner, a
      committed run against `openehr-loco`, and one against a stock EHRbase.
      — **M**

      **2026-09-08.** "The eleven endpoints" turned out not to be one shared
      catalogue's worth, checked directly against a real, running EHRbase
      2.35.1 rather than assumed: `POST /ehr`'s request body genuinely
      differs between the two (a caller-built `Ehr` here, query parameters
      or an `EHR_STATUS` body there); every composition and contribution
      case is blocked on template infrastructure this tree does not have
      (`OPT 1.4 ingestion`, a separate P1 item — EHRbase refuses a
      template-less composition outright, confirmed by posting one); and
      `GET /composition` (search) and `GET /metadata` are not part of
      ITS-REST at all, absent from EHRbase's own OpenAPI paths, with nothing
      to check them against. What is genuinely shared, checked and
      landed: reading an EHR that exists, and reading one that does not —
      [`openehr-loco/scripts/conformance.sh`](../openehr-loco/scripts/conformance.sh)
      plus [`openehr-loco/examples/generate_test_token.rs`](../openehr-loco/examples/generate_test_token.rs)
      (a throwaway PASETO keypair and token, for running it by hand), with
      both required runs committed and dated in
      [`openehr-loco/spec/conformance-runs.md`](../openehr-loco/spec/conformance-runs.md)
      — `2 passed, 0 failed` against `openehr-loco` (a keypair from the new
      example, `cargo run -- start`) and against a real EHRbase 2.35.1
      (`podman`, no Keycloak needed despite the upstream compose file making
      it a startup dependency — confirmed by using Basic auth successfully
      without it, not assumed). Three genuinely new, verified divergences
      fed back into the "ITS-REST completeness" item above rather than
      re-stated here. A small claim next to the item's own headline, and
      the honest one — two endpoints, not eleven, with the other nine's
      blockers named precisely enough that closing this further is now a
      matter of sequencing (OPT ingestion; deciding `POST /ehr`'s contract;
      renaming two paths to `versioned_composition/...`), not investigation.
- [ ] **Close `A-40`'s own residual wording and the matrix dates** after
      each P1 item lands, not at the end — the register said this file's
      predecessor "went stale as capability was added underneath it" and
      that is `W0.4` read backwards. — **S**, recurring

      **2026-09-07 pass.** No P1 item landed since the last pass — this
      session's own work (Supply chain, the release pin table) is P2, not
      capability under `am::cadl` — but checked anyway rather than assumed
      clean, and found one real drift: `A-40`'s own "sixteen requirements
      have no code" paragraph still named a closed `ARCHETYPE_SLOT` as
      something `am::cadl` could not read, alongside `SIBLING_ORDER`. `A-73`
      closed that gap on 2026-09-03, the same day, and `src/am/mod.rs`'s own
      module doc was corrected at the time — this one paragraph in
      `openehr/spec/audit.md` was not. Corrected in place, dated, citing
      `A-73`; the sixteen-count itself is unaffected (a closed slot was
      description within `K15.5`'s already-unmet scope, not its own id).
      `conformance-matrix.md`'s `K15.5` row was already current (cites
      `A-72`–`A-77` by name) and needed no change. Checked and found nothing
      else stale in `A-40`'s text against the current `am::` module docs.

      **2026-09-12 pass.** Five `am::`/`base::` capability commits landed
      since the last pass — `A-45` (`C_DATE`/`C_TIME`/`C_DATE_TIME`/
      `C_DURATION` in `am::validate`), `A-46` (`C_PRIMITIVE_OBJECT.node_id`),
      `A-47` (`Terminology_code`/`Terminology_term`, `base::`, not §15),
      `A-48` (`C_PRIMITIVE_OBJECT.assumed_value`), `A-49` (`ArchetypeHrid`,
      fixing the ADL header readers' grammar) — the most since any prior
      pass. Checked each against `A-40`'s own prose and every `K15.*` row
      individually, by diff rather than by assumption, and found **no
      drift this time**: `A-40` already speaks at a level none of the five
      changes: the "unmodelled primitive kind" example in its `K15.18`–
      `K15.23` paragraph was always generic, not naming date/time/duration,
      so `A-45` narrows the real gap without falsifying the sentence
      describing it; the two `node_id`/`assumed_value` additions
      (`A-46`, `A-48`) are pure field additions `A-40` never asserted were
      missing; `A-47` is a `base::` type outside §15 entirely; and `A-49`'s
      header-reader fix changes what type a header field holds, not
      whether the reader still stops at the header — exactly the boundary
      `A-40` and the matrix's `K15.5`/`K15.6`–`K15.7` rows already state.
      `am/validate.rs`'s own module doc, which `A-40` explicitly defers to,
      was itself updated in the same commit as `A-45` and remains
      consistent with `A-40`'s text. No matrix row touched by any of the
      five commits.

### P3 — larger scope, or awaiting a decision

- [ ] **ADL 1.4 body parsing and conversion** (`K15.8`, `K15.9`): the header
      reader exists; the body does not, and CKM's published archetypes are
      ADL 1.4 first. Same "smallest real slice" discipline as `am::cadl`. — **L**
- [ ] **Flattening and specialisation conformance** (`K15.11`–`K15.13`),
      then **template expansion and the operational template** (`K15.14`,
      `K15.15`, `K15.17`). Each is its own track; `SIBLING_ORDER` and
      `closed` slots (`A-62`) are gated on the first. — **XL**
- [ ] **RBAC/ABAC and multi-tenancy** — decide, in `plan.md` §Non-goals or
      as a `db:` requirement, whether this repository will ever carry them.
      Thread #18's argument for one open edition applies to whatever *is*
      shipped; it does not oblige shipping everything. — decision
- [ ] **A hosted sandbox** (thread #13: serverless, nightly wipe, demo data
      only) — only after P2's perimeter statement, rate limiting, and the
      compose image exist, and only with synthetic data (`PHI.md`
      §Development data). — **M**, gated
- [ ] **A second maintainer.** The bus factor is one (`MAINTAINERS.md`), and
      the thread's kindest critique of FerroEHR (#1's own caveat) is the same
      sentence. The Collabrathon on 5 November and EHRCON26 on 22–23
      September are where the conversation starts (outreach §11). — **L**,
      human
- [ ] **BMM-driven cross-check of the RM.** FerroEHR generates its spec
      layer from BMM/XSD/OpenAPI (#8). Generation is out of scope here, but a
      *test* that reads the published BMM and asserts every class and
      attribute has a field in `openehr::rm` — or is listed in `01-scope.md`
      as deliberately absent — would catch the next `db:D-07` (four `VERSION`
      attributes silently dropped) before a reviewer does. — **M**

      **Scope investigated, not started, 2026-09-12.** Two premises checked
      before writing anything, one confirmed easier than expected and one
      confirmed harder.

      **Easier: the BMM itself.** `openEHR/specifications-ITS-BMM` is
      Apache-2.0 (unlike `adl-archetypes`, which has no licence file and is
      therefore read where it sits, never vendored — `openehr/spec/
      corpus.md` §Licence) — a specific `.bmm.json` file could actually be
      committed as a fixture, not only pointed at through an env var. The
      RM's own JSON serialisation (`components/RM/json/
      openehr_rm_1.2.0.bmm.json`, ~90 classes, ~20,000 lines) is a regular,
      parseable schema: `class_definitions.<NAME>.properties.<name>` is
      either `P_BMM_SINGLE_PROPERTY` (`type`, `is_mandatory`) or
      `P_BMM_CONTAINER_PROPERTY` (`type_def.container_type`/`type_def.type`,
      `cardinality`). `serde_json` reads this with no new parser and no new
      dependency.

      **Harder: there is no RM-side list to compare it against.** The
      task's own premise — "asserts every class and attribute has a field
      in `openehr::rm`" — needs, for each of ~90 Rust types, the list of
      field names `serde` actually (de)serialises. Rust has no runtime
      reflection, and this crate derives `Serialize`/`Deserialize` directly
      on hand-written structs rather than through a schema-generating macro,
      so nothing today can answer "what are `Composition`'s field names" by
      inspection. Two ways to get one, both real costs rather than a test
      body:

      1. A `schemars`-shaped dependency, deriving a JSON Schema per RM type
         and reading field names back out of *that*. A genuine new
         dependency in `openehr`'s own `Cargo.toml` — every one there
         carries a justifying comment — which puts this on the same footing
         as the still-undecided `regex` item above: a maintainer decision
         about what this crate depends on, not something to add silently
         inside an unrelated test.
      2. A hand-maintained table, `RM class → field list`, checked by eye
         against `openehr::rm`'s own source once and then trusted. This is
         exactly the shape `W-13` already named a defect in this repository:
         "a guard whose input list was written by hand" — a second,
         independent list that agrees with the code on the day it is
         written and silently stops being checked the day either one
         changes without the other.

      Neither is a small addition to what the task describes; both are
      decisions, not code, the same footing `tasks.md`'s own `regex` item
      already stands on. Re-sized **L**.

      **The narrow pilot this re-scoping would otherwise propose has
      already been run, once, by hand — that is what `db:D-07` is.**
      Checked while investigating this item rather than assumed: `D-07`'s
      own finding text reads the RM 1.1.0 BMM for exactly `VERSION`,
      `ORIGINAL_VERSION`, and `AUDIT_DETAILS`, and found and fixed the same
      four dropped attributes this task's own motivating example names —
      it is not a hypothetical near-miss this task is worried about, it is
      the one real instance of the exact defect class this task exists to
      prevent from recurring. What `D-07` did not do, and what remains the
      actual gap, is make that check *standing*: it read one class family
      once, by a person, rather than every class, automatically, on every
      push. Turning it into that is exactly the two-way dependency decision
      above — a one-off manual read does not need Rust reflection, because
      a person did the field-matching; a repeatable one does.

## Done (condensed; full evidence in `git show 4761700:tasks.md`)

All verified on the dates given; none is re-asserted here without the source.

- [x] Eight crates published, 0.9.0 since 2026-09-02; CI runs test, msrv,
      examples, bench, schema, fuzz, assets, layering, trademarks, claims,
      mutants; the library matrix is machine-derived and the audit counts
      self-check (`claims` job).
- [x] Root document set: README, LICENSE.md, `LICENSES/`, CITATION.cff,
      NEWS, COMPARISONS, BENCHMARKS, INSTALL, CONTRIBUTING, MAINTAINERS,
      CHANGELOG, AI_STATEMENT, GOVERNANCE, SECURITY, PHI, RFC, TRADEMARKS,
      CODE_OF_CONDUCT, `CODEOWNERS` at root, `index.md`, `llms.txt`/`llms.json`.
- [x] Trademarks: openEHR International's permission (2026-08-27), the
      Foundation's prescribed notice on every document and crate, checked by
      `scripts/check-trademarks.py` in CI (`trademarks` job).
- [x] Repository security settings enabled and re-verified live 2026-08-29
      (private vulnerability reporting, Dependabot alerts and security
      fixes, secret scanning); `.github/dependabot.yml` covers every
      workspace with version-update PRs capped at zero. Push protection
      remains off (P2 above).
- [x] Commits and tags SSH-signed from 2026-08-27, verified on GitHub and
      GitLab; release tags `v0.2.0`–`v0.9.0` on all three remotes.
- [x] Trusted Publishing and attestation: decided 2026-08-28, on a stated
      condition (`spec/trusted-publishing/index.md`), not adopted yet.
- [x] Issue templates and a stated response expectation (read within a
      week); `CODE_OF_CONDUCT.md` with the claim-accuracy clause.
- [x] Governance: AI may execute `cargo publish` (2026-09-02) and determine
      that a specific prepared release meets `agents/publishing.md`'s
      checklist; `Co-Authored-By` trailers kept and explained
      (`AI_STATEMENT.md` §4, `GOVERNANCE.md` §Machines do not decide).
- [x] Funding: GitHub Sponsors verified live 2026-08-28; no Open Collective
      (needs a fiscal host the owner must choose).
- [x] Site: `openehr-rust.github.io` live since 2026-08-28, linked from
      README and `index.md`.
- [x] GitHub topics set — `aql`, `ehr`, `healthcare`, `interoperability`,
      `openehr`, `rust`, `sqlite` — verified 2026-09-03 with `gh api
      repos/openehr-rust/openehr-rust/topics`; this closes the last open
      row of the outreach readiness checklist.
- [x] Database matrix: exact-once derivation decided against 2026-08-27
      with reasons; the coverage floor script written, its finding filed as
      `db:D-11` (P2 above).
- [x] Archetype Model: `S1.4` withdrawn, §15 in force, `A-40` open and its
      status line current as of 2026-09-03 (`A-58`–`A-71` closed: tuple
      evaluation, slots, `ARCHETYPED` on `Node`, `use_archetype`/`use_node`/
      `allow_archetype`, assumed values, ISO8601 literals, `CONTAINED_REGEXP`,
      `C_ATTRIBUTE_TUPLE`, `ArchetypeHrid` as the archetype's own identity,
      differential-form attributes, an unstated `occurrences` carried and
      inferred by AOM2's rule under the new `K15.32`). 16 of 33 ids remain
      `spec`.

## Trademarks

openEHR® is the registered trademark of the openEHR Foundation and is used with
the permission of openEHR International. Use of the trademark does not
constitute endorsement of this product by openEHR International or openEHR
Foundation.
