# 0. Conformance

**Rewritten 2026-08-01** to describe this repository. The previous text was
imported from a FHIR specification and named a four-level ladder
(`Scaffold / Schema / Store / Reference`) that competed with the one the code and
crate documentation actually used. See [`spec/audit.md`](../audit.md) **W-06**.

Requirement prefix: `C0`.

This section defines the language the rest of this specification is written in,
and what a claim made in it means.

## Normative language

- **C0.1** The keywords MUST, MUST NOT, REQUIRED, SHALL, SHALL NOT, SHOULD,
  SHOULD NOT, RECOMMENDED, MAY, and OPTIONAL are to be interpreted as described in
  RFC 2119. They are normative only when capitalized.
- **C0.2** Prose carrying no keyword is **rationale**. Rationale explains why a
  requirement exists and MUST NOT be read as imposing an obligation of its own. It
  is kept deliberately: a requirement whose reason is unrecorded is a requirement
  that will be removed by someone who does not know what it was protecting.
- **C0.3** Examples, measured numbers, and file paths in rationale are
  illustrative. Where an example and a requirement disagree, the requirement
  governs and the example is a defect.

## Requirement identifiers

- **C0.4** Every normative statement carries an identifier of the form
  `<prefix><section>.<ordinal>[<suffix>]` — `M3.16b`, `PR12.6`, `T11.12`. The
  prefix is fixed per section and listed in [`index.md`](index.md).
- **C0.5** Identifiers are **stable and never reused**. A requirement that is
  withdrawn keeps its number, marked withdrawn; its number MUST NOT be assigned to
  anything else. A requirement that is amended keeps its number. A requirement
  that is split gains lettered suffixes (`M3.16` → `M3.16a`, `M3.16b`), and the
  parent number continues to exist.

  Reuse is the failure this rule prevents: a citation in a test name, a commit
  message, or an auditor's workpaper is written once and read years later. If
  `M3.16` means something different in 2029 than in 2026, every one of those
  citations silently becomes a lie and nothing reports it.

- **C0.6** Section numbering has gaps — 7, 8, and 14. They are deliberate and MUST
  NOT be closed by renumbering. Sections 7 (REST API) and 8 (CLI) are retired:
  these crates are embeddable libraries, and this repository contains no server
  and no binary. `M14.x` is reserved, in every engine crate, for that crate's own
  dialect annex, so the core MUST NOT define an `M14.x` requirement.

- **C0.7** Identifiers in this directory are scoped to this specification.
  [`openehr/spec/`](../../openehr/spec/index.md) independently allocates `C0.x`,
  `S1.x`, and `R4.x` with different meanings. Where ambiguity is possible, cite as
  `db:M3.4` and `lib:S1.4` (`W0.5`).

## The conformance ladder

- **C0.8** An engine crate sits at exactly one of four levels. The level is a
  claim about what has been **verified**, not about what has been written.

  | Level | Means | Evidence required |
  | --- | --- | --- |
  | **Dialect** | Emits DDL for the shared schema. | The golden DDL tests, and `conformance::check_dialect`. |
  | **Schema** | The engine itself has executed that DDL. | A transcript against that engine's own server: the script applied cleanly, applied *again* cleanly, and the append-only tables were observed refusing `UPDATE` and `DELETE` **with a row present**. |
  | **Store** | Implements `Store` against a real database. | `conformance::run` passing against that engine. |
  | **Verified** | Store level, run in CI against the engine's own server on every commit. | A CI job that provisions the engine and fails — not skips — without it. |

- **C0.9** A crate MUST state its level in its README and in its crate
  documentation, within the first screenful, and MUST NOT claim a level it has not
  earned.
- **C0.10** A crate MUST NOT claim a level whose evidence comes from a different
  engine than the one it targets. Running a crate's verification against a
  substitute engine — because that is the container already in the pipeline —
  produces a green result and no evidence, which is worse than a red one.

  This is not hypothetical. `openehr-mariadb` claimed **Schema** on the strength
  of a run that never happened, against a version of MariaDB that has never
  existed, using DDL that was byte-identical to MySQL's
  ([`spec/audit.md`](../audit.md) **W-01**).

- **C0.11** Documentation MUST NOT describe a capability at a level above the
  crate's. A README text-substituted from a Store-level crate into a Dialect-level
  one asserts, in the new crate's name, results that were never obtained for it.
- **C0.12** The **"with a row present"** clause in Schema is load-bearing, not
  pedantry. A `FOR EACH ROW` trigger on an empty table never fires, so a `DELETE`
  matching zero rows reports a refusal it never performed. The first enforcement
  run in this repository looked like a pass and proved nothing for exactly that
  reason. A check whose subject is absent reports the silence as success.
- **C0.13** A level is a claim about the present, not about an afternoon in the
  past. A level whose evidence is a one-off local run MUST say so, and MUST NOT be
  worded to imply continuous verification. A crate reaches **Verified** only once
  CI has run green on `main`; a committed workflow is not a working one. As of
  green run 30713623082, 2026-08-01, `openehr-sqlite` is at Verified and no other crate is
  eligible ([`spec/audit.md`](../audit.md) **W-02**).

## Departures

- **C0.14** An engine crate that cannot satisfy a core requirement MUST record a
  **departure** in its dialect annex, as a numbered `M14.x` requirement naming the
  core requirement it amends and stating what holds instead.
- **C0.15** A departure MUST NOT weaken an invariant listed in
  [§15](15-portability-and-dialects.md) as engine-independent — the storage model,
  the projection, the commit rules, canonical form, or append-only enforcement.
  Those are the properties that make six crates one product; a crate departing
  from them is a different product wearing the name.
- **C0.16** An undeclared departure is a **defect in the crate**. Discovering that
  a crate has behaved differently all along does not retroactively make it an
  amendment; it makes it a finding.
- **C0.17** Prose that merely describes an engine is not a departure and does not
  license one. A departure cites a number.

## Amending this specification

- **C0.18** A change to a normative statement MUST change its text in this
  directory, in one commit, with the reason stated. There is one copy; amending
  six is no longer possible and no longer permitted.
- **C0.19** A new requirement MUST take the next unused ordinal in its section,
  and MUST NOT be inserted mid-sequence by shifting those after it.
- **C0.20** A requirement MUST be traceable to evidence: a test, a CI gate, or an
  explicit entry in the [conformance matrix](conformance-matrix.md) recording that
  it is unverified. "Specified, implemented, untested" is a state the matrix names
  rather than hides.
- **C0.21** Amending the core to match what a crate already does is permitted and
  expected; doing it **silently** is not. The commit MUST say which crate's
  behaviour drove the change, so a reader can tell a considered generalization
  from a rubber stamp.
- **C0.22** Every amendment MUST be checked against the conformance matrix — does
  it change a status? — and against [`audit.md`](audit.md) — does it close a
  finding, or open one?

## Vocabulary

| Term | Means |
| --- | --- |
| **the core** | this directory: everything not specific to one SQL engine |
| **an engine crate** | one of the six `openehr-<engine>` crates |
| **a dialect** | one implementation of `openehr_store::Dialect` |
| **a store** | one implementation of `openehr_store::Store` |
| **the deployment** | the system a caller runs in: its perimeter, policies, and key material |
| **PHI** | protected health information — anything identifying or clinical about a person |
| **refuse** | return an error rather than a value; never a plausible default |

---

Part of the [openEHR persistence specification](index.md).
