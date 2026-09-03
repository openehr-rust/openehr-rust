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
| **Lifecycle edge cases**: `is_modifiable` toggled inside one contribution | #5, #6 | The RM models the flag and its semantics; **nothing in `openehr-store` or `openehr-loco` reads it on commit** |
| **Performance**, with an outsider offering to rerun the numbers | #4, #14 | Benchmarks exist and are run-never-gated (`W0.35`); no store commit latency or HTTP round-trip has ever been published |
| **Runs in sixty seconds**: `docker compose up`, a Codespace, a hosted sandbox with Swagger, and a *correct* quickstart | #9, #10, #11, #13, #21 | No `Dockerfile`, no compose file, no devcontainer, no OpenAPI document; `openehr-loco` is `publish = false`, SQLite-only, and started by `cargo run` |
| **A REST surface tools can talk to**: ITS-REST 1.1.0, AQL over HTTP, templates, EHR_STATUS, an admin scheme — openEHR Explorer added FerroEHR as a server type within three days | #1, #21 | Eleven endpoints: EHR, contribution, composition commit/read/history/vread, one index search. No `/query/aql`, no template endpoints, no `EHR_STATUS`, `DELETE` answers `501` |
| **AQL that executes** | #1 | AQL is parsed and statically checked and **not executed** — a deliberate rule (`S1.5`, reaffirmed `K15.29`) that a CDR reader will read as the headline gap |
| **Templates**: OPT 1.4, ADL 2.4, WebTemplate, FLAT/STRUCTURED, validated at upload | #1 | AOM2 types, `am::validate` against an in-memory archetype, and a `definition`-only cADL reader (`A-40`, `A-62`–`A-69`). No OPT, no template, no flattening, no ADL 1.4 body |
| **Strict readers and typed errors**: undeclared/duplicate JSON keys refused, every error naming path and rule | #1, #21 | Constructors validate and `validate()` runs on JSON ingress (`lib:A-23`); duplicate-key refusal and a stated unknown-field posture are not in place |
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
      linked from `NEWS.md`. *Draft written 2026-09-03 —
      `help/outreach/drafts/2026-09-ferroehr-thread-reply.md`, the reply and
      the email both, every claim checked against the tree; sending is the
      maintainer's action and has not happened.* — **S**
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
- [ ] **PostgreSQL `Store`.** Every CDR in the thread runs on PostgreSQL
      18; this repository's only `Store` is SQLite. Implement
      `openehr-postgresql`'s store against the existing DDL, run
      `conformance::run` against a real server in CI (the `schema` job
      already provisions one), and promote in `spec/databases/
      conformance-matrix.md` — the one file that owns a level (`W0.40`).
      *Evidence:* the matrix row moves Schema → Store → Verified with the
      job named. — **L**
- [ ] **MSSQL and Oracle parsed by a real server.** Two of six dialects have
      "never been parsed by a server" (`spec/databases/conformance-matrix
      .md`). Both now run in containers — `mcr.microsoft.com/mssql/server`
      and `gvenzl/oracle-free` — so `verify-schema.sh` can gain both, the
      `M14.6`/`M14.7` departures close, and the two annexes can move from
      *proposed* to *ratified* (`X15.9`). *Evidence:* the `schema` matrix in
      `.github/workflows/ci.yml` lists six engines; two rows promote to
      Schema. — **M**
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
- [ ] **Strict readers.** Thread #1's strictness list is the bar: refuse
      undeclared keys and duplicate keys on the canonical-JSON ingress path,
      and make every refusal name the JSON path and the requirement. Decide
      `deny_unknown_fields` per RM class as a stated policy (`lib:`
      requirement), not crate-by-crate accident; duplicate-key refusal needs
      a custom `serde_json` map visitor. *Evidence:* invalid twins for each
      refusal beside the valid fixture, the way #16 describes. — **M**
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
      - [ ] The JSON half (SDK examples, web-template compositions).
      - [ ] `spec_refs` per conformance case, and the index.
      - [ ] The regression job. Blocked on a corpus this tree may carry:
        `adl-archetypes` has no licence file, so it is read where it is,
        never vendored (see `corpus.md` §Licence).
- [ ] **Close the corpus's open candidates, largest first** (`corpus.md`
      §Candidates; each needs its test and grammar reading before an `A-`
      number, `W0.19`). (4) An unwrapped interval's kind: `odin_values.g4`
      builds `integer_interval_value` from `INTEGER` tokens and
      `real_interval_value` from `REAL`, so `|0..100|` is a `C_INTEGER` and
      `A-67`'s "cannot be told apart" is wrong — 184 files, 120 of them
      CKM/NEHTA clinical archetypes. (5) Unwrapped temporal literals and
      `*_CONSTRAINT_PATTERN`s taken for RM type names — 36 files; needs a
      `CPrimitive` pattern form first (`A-63` modelled the value forms
      only). (1) `DATE_CONSTRAINT_PATTERN` refused under the wrong name —
      24 files. Then the Reference Model multiplicity decision in
      `plan.md`. *Evidence:* run 3 in `corpus.md`, dated, with the counts.
      — **M**

### P2 — operability and evidence

- [ ] **`Dockerfile`, `docker-compose.yml`, `.devcontainer/`.** Thread #9
      lost an afternoon to a compose override; #21's whole review was "one
      `docker compose up` and it's running". Build `openehr-loco` as a
      static binary in a multi-stage image, compose it with nothing but a
      volume, and add a devcontainer so a Codespace boots the published
      image. *Evidence:* `docker compose up` answers `curl /openehr/v1/
      metadata` on a clean machine; the quickstart in `INSTALL.md` is the
      command, not a paragraph. — **M**
- [ ] **Publish measured numbers.** Add store commit and read benchmarks
      to `openehr-store/benches/store.rs` and an HTTP round-trip benchmark
      for `openehr-loco`, run them on a named machine, and put the numbers
      with their date and hardware in `BENCHMARKS.md`. Keep `W0.35`/`W0.36`:
      run, never gated. Then take #4's offer. *Evidence:* dated numbers in
      the file, reproducible by the command beside them. — **M**
- [ ] **Supply chain: SBOM, `cargo-deny`, `cargo-audit`, push protection.**
      `SECURITY.md` names "no SBOM" as an open gap. Add `cargo auditable`
      builds and a CycloneDX SBOM per release artefact, `cargo deny check`
      and `cargo audit` as CI jobs (Dependabot only *alerts*), and turn
      secret-scanning push protection on. Trusted Publishing stays on its
      stated condition (`spec/trusted-publishing/`). *Evidence:* the jobs in
      `ci.yml` with rows in `AGENTS.md` and `spec/audit.md` (the `claims`
      gate requires them); the gap struck through in `SECURITY.md`. — **S**
- [ ] **State the production perimeter.** `openehr-loco` has no TLS, no
      rate limiting, no read audit at the store (`db:D-04`, "fixed above the
      store"), no RBAC. Write the deployment statement — TLS terminated by
      a reverse proxy, what is and is not audited, that one PASETO key set
      is the whole authorisation model — into `openehr-loco/README.md` and
      `PHI.md` §Known limits, and add rate limiting at the router. *Evidence:*
      a reviewer can answer `PHI.md`'s questionnaire section from the
      documents alone. — **S**
- [ ] **Assess the 144 unassessed database requirements (`db:D-11`).** In
      batches by section, `M3` and `S1` first as the finding recommends,
      then wire `scripts/check-databases-matrix-coverage.py` into CI once it
      would pass on day one. *Evidence:* the script green in CI; `D-11`
      closed. — **L**
- [ ] **A specification-release pin table.** One file — `spec/releases.md`
      or a section of `openehr/spec/index.md` — naming the RM, BASE, AM,
      TERM, QUERY, and ITS-REST releases every module here was transcribed
      from, with the date and the source file, the way `terminology.rs`
      already does for one of them. Then a re-vendor check: when a release
      moves, which modules to re-read. Thread #7 and #8 are about exactly
      this risk. *Evidence:* the table; `S1.16`/`K15.2` cite it. — **S**
- [ ] **An ambiguities register, and file it upstream.** SEC asked (#19)
      that spec inconsistencies not be discarded. Collect the spec silences
      and contradictions this tree has already adjudicated — `versions` typed
      `List` but described as a set, `is_modifiable` evaluation time, the
      `C_STRING` regex-in-list shape (`A-63`), `ARCHETYPE_HRID` vs
      `ARCHETYPE_REF` (`A-49`), the `TERM` repository that disagrees with
      the computable one — into `openehr/spec/ambiguities.md` with the
      disposition each got, and open one openEHR tracker issue per entry.
      *Evidence:* the file, and issue links beside each entry. — **M**
- [ ] **A conformance runner anyone can point at any server.** Thread #16's
      `scripts/conformance.sh` with bring-your-own-SUT is the model: a
      catalogue of HTTP cases against ITS-REST, runnable against
      `openehr-loco`, EHRbase, or FerroEHR, with verdicts committed. Start
      with the eleven endpoints that exist. *Evidence:* the runner, a
      committed run against `openehr-loco`, and one against a stock EHRbase.
      — **M**
- [ ] **Close `A-40`'s own residual wording and the matrix dates** after
      each P1 item lands, not at the end — the register said this file's
      predecessor "went stale as capability was added underneath it" and
      that is `W0.4` read backwards. — **S**, recurring

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
