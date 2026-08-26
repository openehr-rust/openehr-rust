# PHI, privacy, and what this software does with patient data

**Plain-language answers for a privacy officer, a security reviewer, or anyone
filling in a vendor questionnaire.** It cites the normative sources rather than
restating them, so it cannot drift from them: the library's security
requirements are [`openehr/spec/11-security.md`](openehr/spec/11-security.md),
and the regulation-to-requirement mappings are
[`openehr/spec/14-compliance-mapping.md`](openehr/spec/14-compliance-mapping.md)
(the library) and
[`spec/databases/13-compliance-mapping.md`](spec/databases/13-compliance-mapping.md)
(the persistence layer).

**Status: pre-release.** Nothing here is certified by anyone, and no deployment
of this software is known to exist. See "Known limits" before relying on any of
it.

## The short answers

| Question | Answer |
| --- | --- |
| Does this software send data anywhere? | **No.** The model crate performs no I/O at all; no published crate opens a network connection (`lib:S1.11`, `db:O10.15`). The one embedded store writes a local SQLite file you name. |
| Does it phone home, or collect telemetry or analytics? | **No.** There is no such code in the repository. |
| Does it embed or call an AI model? | **No.** AI was used to *write* it, never to run it — [`AI_STATEMENT.md`](AI_STATEMENT.md) §1. |
| Does it hold PHI? | Only in the database *you* run, in tables *you* created. The libraries hold data in memory for the duration of a call. |
| Does it write PHI to logs? | No `Display` renders PHI, no error may echo a submitted value, and a wrapper hides values from `Debug` on the way through logging code (`lib:X11.6`–`X11.8`). |
| Does it record who *changed* a record? | **Yes**, on every version, by design (`lib:X11.2`) — and that record contains user identifiers. See below. |
| Does it record who *read* a record? | **No, not at the persistence layer** (`db:PR12.5`, not implemented). An access investigation asks about reads; this layer cannot answer it yet. |
| Can I erase a patient? | **Not with anything shipped here.** Deletion is a recorded logical version (`lib:V8.10`); no physical-erasure operation exists at the persistence layer (`db:M3.18` row: Not implemented). |
| Is the database connection encrypted? | The requirement is stated (`db:O10.7`); nothing here opens a connection to encrypt, so it falls entirely to your deployment. SQLite has no connection. |
| Is it a medical device? Is it HIPAA/GDPR compliant? | **No, and no.** No crate here is certified, and none claims to be. See "Regulatory framing". |
| Who do I contact? | [`SECURITY.md`](SECURITY.md) for anything sensitive; [`MAINTAINERS.md`](MAINTAINERS.md) otherwise. **Never send real patient data** — a report containing it is deleted, not filed (SECURITY.md §Never send patient data). |

## What the software is

Four kinds of crate, and their PHI posture genuinely differs:

| Crates | What they are | PHI posture |
| --- | --- | --- |
| [`openehr/`](openehr/) — the model | Reference Model types, validation, paths, AQL parsing, and the security module | **Touches no database and performs no I/O.** If PHI passes through it, that is your program's memory, not a store. The audit chain and redaction live here, in-process. |
| [`openehr-store/`](openehr-store/) — the persistence core | schema, projection, commit rules, the conformance suite | Links no driver and opens no socket. It defines the shapes; it does not move data. |
| `openehr-<engine>/` — the six dialects | DDL and SQL for six engines; **SQLite also ships a store** | The SQLite store is the only shipped code that persists anything, into a local file you name. The other five emit DDL for databases *you* operate — check the [conformance matrix](spec/databases/conformance-matrix.md) for what each has been shown to do. |
| [`openehr-loco/`](openehr-loco/) — the HTTP service | Axum/Loco service with PASETO token verification | The only component that listens on a socket. **Not published**, and outside the conformance ladder (`W0.32`). |

## What it does *not* do

Lifted from the trust boundary (`lib:X11.1`). Your deployment must provide
every one of these: **authentication**, group membership, **authorization**
(access-control schemes are carried unchanged and never evaluated —
`lib:X11.3`; every decision the reference scheme does make defaults to deny,
`lib:X11.4`), transport security, key storage, consent capture, and log
retention. Terminology validation is likewise out of scope: external codes are
carried opaquely.

## What it does do, that a reviewer needs to know about

**It records identities on every write, by design.** Every version carries who
committed it, when, and why (`lib:V8.13`, `lib:X11.2`) — a feature answering
HIPAA §164.312(b), with a privacy consequence worth stating plainly: the commit
audit is a record of clinical activity by *users*, and its retention, review,
and erasure are your deployment's responsibility. Reads are not recorded
(`db:PR12.5`), so this is half of an audit-controls story, and the mapping
marks it **Partial** for that reason.

**It maintains a tamper-evident history chain — in the process, not in the
database.** Each entry digests its predecessor over SHA-256, so altering any
entry invalidates every entry after it (`lib:X11.9`). What that buys is stated
narrowly, per `lib:X11.10`: **unkeyed, it detects careless or unaware
modification** — a migration, a stray `UPDATE`, a restore from the wrong
backup — and supports an external witness if you publish the head digest
somewhere the database administrator does not control. **It does not stop an
informed attacker with write access.** For that there is an optional keyed
`HMAC-SHA-256` tag whose key lives in the process, never in the store it
protects (`lib:X11.11`), compared only in constant time (`lib:X11.12`), with
key identifiers travelling alongside so rotation is additive (`lib:X11.14`).
A checkpoint form carries counts and digests only — no patient data — so it
can ship to a long-retention log where clinical data must not go
(`lib:X11.19`). The database schema itself has no chain (`db:M3.16`);
append-only is enforced by triggers on the engines the matrix says have been
observed refusing `UPDATE` and `DELETE`, which is integrity, not tamper
evidence (`db:PR12.11`).

**It redacts by masking, never by deleting.** A withheld `ELEMENT` becomes
`272|masked|` — "there is a value here and you are not being shown it" —
because deletion turns "the patient has withheld their sexual health history"
into "the patient has no sexual health history", a clinical statement nobody
made (`lib:X11.20`). A redacted document still validates (`lib:X11.21`);
redaction reports **how much** it withheld and never **what** (`lib:X11.22`),
and it fails closed, so an error cannot leak the unredacted original
(`lib:X11.24`). Element rules are **not** de-identification: composer,
participations, and the audit trail are untouched (`lib:X11.25`).

## Regulatory framing

**These are components, not certified systems.** The persistence mapping puts
it in the form this project uses, and it is the sentence to quote back to
anyone who reads more into a table than it says:

> They cannot make a deployment compliant, and they must not be the reason a
> deployment cannot be.

- **HIPAA.** §164.312(b) audit controls: **Partial** — writes only, reads not
  recorded. §164.312(c)(1) integrity: **Partial** — append-only where
  verified, no hash chain in the schema (the chain is the library's, above).
  §164.312(e) transmission security: **the deployment's** — nothing here
  transmits. §164.502(b) minimum necessary: **out of scope**. All from
  [`spec/databases/13-compliance-mapping.md`](spec/databases/13-compliance-mapping.md);
  the library's contributions per control are in
  [`openehr/spec/14-compliance-mapping.md`](openehr/spec/14-compliance-mapping.md),
  every row of which also says what the deployment must still do.
- **GDPR.** Art. 17 erasure: **not implemented** at the persistence layer;
  the library gives logical deletion only (`lib:V8.10`). Art. 30: **Partial**,
  writes only. Art. 32: **Partial**. Art. 15 access: lossless export exists
  (`lib:J9.1`); assembling and authenticating is yours.
- **IEC 62304 / ISO 13485.** The lifecycle evidence is the specification tree
  itself: permanent requirement identifiers cited from code and tests, the
  conformance matrices, and the audit registers of known problems.
- **openEHR conformance.** The library implements the Reference Model; it does
  not claim conformance to an openEHR conformance profile, because those cover
  a service and this is a library (`lib:S1.7`). The project's ladder is its
  own.

**Not a medical device.** These libraries validate, store, and retrieve
records. A downstream integrator who gives their product a medical purpose
brings *their* product into scope; that classification is theirs to make.

## Development data

No patient data, no personal health information, and no customer data exists
anywhere in this repository — not in source, not in fixtures, not in CI. Test
data is constructed in code or derived from the openEHR specifications' own
examples, which are modelling artefacts rather than records about people
([`AI_STATEMENT.md`](AI_STATEMENT.md) §7). This is a structural property of a
public tree that a reader can check.

## Known limits that bear on this document

Stated here so you find them from this page rather than from an audit:

- **Read access is not audited** at the persistence layer (`db:PR12.5`). The
  version history records changes; an access complaint asks about reads.
- **No erasure operation exists** (`db:M3.18`), and disabling the append-only
  triggers to improvise one is forbidden, because it removes the guarantee for
  every other row.
- **Verification depth varies by engine.** Only SQLite is at **Verified**; the
  [conformance matrix](spec/databases/conformance-matrix.md) is the only
  document that distinguishes the six, and checking it for the engine you
  actually deploy is the audit step that gets skipped.
- **Repository-level posture gaps are self-declared** in
  [`SECURITY.md`](SECURITY.md): commits and tags unsigned, no SBOM, private
  vulnerability reporting disabled as of 2026-08-26.
- **Open findings live in three audit registers** —
  [`spec/audit.md`](spec/audit.md), [`openehr/spec/audit.md`](openehr/spec/audit.md),
  [`spec/databases/audit.md`](spec/databases/audit.md) — and this project
  publishes them deliberately.

## If you are filling in a questionnaire

Cite this file for the posture, `openehr/spec/11-security.md` for the
requirement-level statements, the two compliance mappings for the regulation
rows — their **Partial** and **Not implemented** words are load-bearing — and
the conformance matrix for what your specific engine has been shown to do. If
a question has no answer in those, ask: an unanswered question is more useful
to this project than a guessed one.

## Trademarks

openEHR is a trademark of openEHR International (the openEHR Foundation). This
project is an independent implementation: it is not affiliated with, endorsed
by, or certified by openEHR International.
