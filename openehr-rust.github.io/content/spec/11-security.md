# 11. Security

Requirement prefix: `X11`.

openEHR fixes the audit model and deliberately leaves access control open.
This section says what the crate supplies on top, and — at least as
importantly — what it does not.

## The trust boundary

- **X11.1** The crate MUST NOT authenticate, and MUST document that the
  deployment supplies authentication, group membership, transport security, key
  storage, consent capture, and log retention.
- **X11.2** The crate MUST record **who acted** on every version (`V8.13`).
  Authentication is outside on purpose; attribution is not — a perimeter knows
  the identity and only the record knows which nodes were touched, so neither
  can answer an access complaint alone.

## Access control

- **X11.3** `EHR_ACCESS` settings in a scheme the crate does not implement MUST
  be carried unchanged through a read-modify-write, and MUST NOT be evaluated.
- **X11.3a** Dispatch between schemes MUST be by the declared scheme name and
  never by object shape. A shape-based reader would match a foreign policy as
  the reference scheme and discard its attributes, turning an intact
  un-evaluatable policy into an empty one that denies and looks deliberate.
- **X11.4** Every access decision MUST default to deny. There MUST be no
  "unknown" outcome a caller might read as permissive.
- **X11.5** Denial reasons MUST be distinguishable — no policy recorded, scheme
  not implemented, principal not in a permitted group — because each calls for a
  different response, and "denied" leaves an operator nothing to act on.
- **X11.5a** Reading the audit trail MUST be expressible as an operation
  separate from reading clinical content. An information-governance officer
  investigating an access complaint needs the trail and not the record.

## PHI in output

- **X11.6** No `Display` implementation may render PHI. `DV_IDENTIFIER` renders
  its type and issuer; `DV_MULTIMEDIA` has no `Display` and a `Debug` that
  prints shape only.
- **X11.7** No error may echo a submitted value. An error message is the one
  place a value reaches a log, a response, and a support ticket at once.
- **X11.7a** The single exception is a lexical rejection of **design-time
  vocabulary** — an archetype id, a terminology id, a code — where an error that
  will not name the identifier is unactionable. Even there the echoed text MUST
  be truncated, so that a document passed to a parser by mistake is not logged.
- **X11.8** A wrapper MUST be provided for values that must pass through code
  that logs, hiding them from `Display` and `Debug` while keeping them for
  serialization. Protecting serialization would corrupt the record rather than
  protect it; the accidents happen in logs.

## The audit chain

- **X11.9** The chain MUST digest each entry over its predecessor, so that
  altering any entry invalidates every entry after it.
- **X11.10** Documentation MUST state plainly what an **unkeyed** chain buys: it
  detects careless or unaware modification and supports an external witness, and
  it does not stop an informed attacker with write access, because the digests
  are unkeyed over a published pre-image.
- **X11.11** A keyed tag (`HMAC-SHA-256`) MUST be available, and the key MUST
  live in the process rather than in the store it protects.
- **X11.12** Tag comparison MUST be constant-time. A timing oracle lets an
  attacker with write access recover a valid tag byte by byte.
- **X11.13** Only a **tag mismatch** is a tampering finding. A missing tag, a
  tag naming a key this process does not hold, and a malformed tag MUST each be
  reported as what they are. Reporting a key-distribution problem as forgery
  burns an incident response.
- **X11.14** Key identifiers MUST travel with tags, so rotation is additive.
  Without the identifier, rotating would invalidate all history at once —
  indistinguishable from mass tampering.
- **X11.15** Chains MUST NOT be backfilled. A chain assembled after the fact
  attests only that the rows look consistent now, which is exactly what an
  attacker who rewrote them would produce. Beginning late MUST be recordable and
  recorded.
- **X11.16** Chain keys MUST be at least 32 bytes and MUST be zeroized on drop.
- **X11.17** A `Debug` on a key MUST NOT print its material.
- **X11.18** SHA-256, not SHA-1 (`D3.27`).
- **X11.19** A checkpoint form MUST be available that carries **no** patient
  data — counts and digests only — so it can be shipped to a long-retention log
  where clinical data must not go. That separation is what makes a log-based
  witness practical.

## Redaction

- **X11.20** Withheld content MUST be **masked**, not deleted:
  `272|masked|` (`R4.7`). Deleting turns "the patient has withheld their sexual
  health history" into "the patient has no sexual health history", which is a
  clinical statement nobody made.
- **X11.21** A redacted document MUST still satisfy validation. A filter that
  produced something the receiving system rejects has achieved nothing.
- **X11.22** Redaction MUST report **how much** it withheld and MUST NOT name
  **what**. An access log saying "12 elements withheld" is auditable; one saying
  "HIV status withheld" is a disclosure inside the audit trail.
- **X11.23** A recorded redaction reason is written into the document and MUST
  therefore not name the withheld value or its clinical category.
- **X11.24** Redaction MUST fail closed: on error nothing is returned, so a
  caller cannot forward the unredacted original by mistake.
- **X11.25** Redaction rules address `ELEMENT`s. Composer, participations, and
  the audit trail are not withheld by an element rule, and the documentation
  MUST say so — stripping those is de-identification, a different operation with
  different rules.
