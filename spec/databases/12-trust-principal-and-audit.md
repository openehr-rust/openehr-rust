# 12. Trust, principal, and audit

**Rewritten 2026-08-01.** Most of this section specified how a **service**
establishes a principal — a trusted header, an allow-listed proxy, a
require-principal mode, an `X-Provenance` header. No service exists (`S1.7`), and
openEHR carries attribution inside the model rather than alongside it, which
changes the shape of the answer. See [`spec/audit.md`](../audit.md) **W-04**.

Withdrawn requirements keep their numbers (`C0.5`); new ones start at `PR12.9`
(`C0.19`).

**Amended 2026-08-02.** A service now exists (`S1.7`) and verifies a signed
assertion before it accepts a request. `PR12.13`–`PR12.20` cover it. This does
not restore the withdrawn requirements below and does not weaken `PR12.8`.

Requirement prefix: `PR12`.

## The trust boundary

- **PR12.8** *(amended)* The trust boundary MUST be stated in one place and MUST
  be stated plainly: **this layer does not authenticate.** It records who a
  caller says acted; establishing who they are belongs to the deployment
  (`S1.8`, `lib:X11.1`).

  A library that recorded a principal it had verified and a library that recorded
  one it was handed produce identical rows. The difference is entirely in what
  the deployment did before calling, and a reader of the audit trail cannot tell
  from the data which happened. Saying so is the only honest option.

  *(2026-08-02)* `openehr-loco` now verifies a signed assertion before it
  accepts a request (`PR12.13`). That does not soften this requirement, and it
  is worth being exact about why: the service checks that an issuer signed a
  statement, not that the statement is true. A careless issuer produces
  careless attributions and every signature still verifies.

- **PR12.9** A store MUST NOT infer, default, or synthesize a committer. If the
  caller does not supply one, the commit is refused; it is never attributed to
  the process, the connection user, or "system".

## Verifying the caller's assertion

Added 2026-08-02, when `openehr-loco` gained token verification. These bind a
**service** crate; the core is unaffected and `S1.8` continues to forbid it any
of this.

- **PR12.13** A service crate MAY **verify** an assertion about who is calling.
  It MUST NOT **authenticate**: no credential is checked here, no credential is
  stored here, and no registration or recovery path exists here.

  The distinction is not pedantry, because the two fail differently. An identity
  provider that is wrong has accepted the wrong credential. A relying party that
  is wrong has accepted the wrong *issuer* — and no amount of care at the
  relying party detects a careless issuer. A service MUST describe itself as a
  relying party wherever it describes itself at all.

- **PR12.14** Where a service verifies a token, it MUST use PASETO `v4.public`
  and MUST NOT use JWT.

  JWT negotiates its algorithm inside the token, and the token comes from the
  caller. That one choice is the root of `alg: none`, of RS256 verified as HS256
  against the public key used as an HMAC secret, and of a decade of library
  advisories that are all the same advisory rediscovered. PASETO removes the
  negotiation: the version *is* the algorithm, `v4.public` *is* Ed25519, and
  there is no field in which a caller may propose otherwise.

  This is chosen for the same reason `ColTy` is not `#[non_exhaustive]`
  (`M3.30`) — prefer the design where the dangerous thing cannot be expressed
  over the design where it can be expressed and must be caught.

- **PR12.15** A service MUST hold only the verification key, and MUST contain no
  code path that signs a token.

  Symmetric verification — PASETO `v4.local`, or JWT's HS256 — means the
  verifier holds the key that *mints*. That is ordinarily an accepted trade and
  it is not acceptable here, because a verified subject becomes an
  `AUDIT_DETAILS.committer` inside an append-only, hash-chained history
  (`M3.16`, `M3.17`) whose purpose is to be evidence years later. An attacker
  who reached a minting service could attribute a commit to a clinician who
  never touched the system, and that attribution would then be chained,
  append-only, and indistinguishable from the real ones.

  Asymmetric verification bounds the damage to misuse of tokens actually
  presented. Fabricated evidence and misused evidence are different incidents.

- **PR12.16** A service that verifies MUST **refuse to start** when it has no
  verification key. It MUST NOT start with verification disabled, MUST NOT log a
  warning and continue, and MUST NOT infer a development mode.

  This is the one startup failure worth being absolute about, because the
  failure is silent in exactly the wrong direction: the process starts, the
  health check is green, every request succeeds, and the only symptom is that
  the entire record set is readable by anyone who can reach the port. A service
  that refuses to start produces a page at 03:00; a service that starts open
  produces a breach notification.

  It follows that the verifier MUST be built **before** the store is opened. An
  unconfigured process must not reach a state in which it holds an open
  database.

- **PR12.17** A service MUST refuse a token with no expiry. A token that never
  expires cannot be withdrawn without rotating the issuer's key for every
  caller at once, and no service here has a revocation list to make up the
  difference.

  A service SHOULD also bind the expected audience. Where one issuer serves
  several services and none check `aud`, a token handed to the least sensitive
  of them is replayable against this one. `SHOULD` rather than `MUST` because a
  single-service deployment genuinely does not need it — but the exposure is
  invisible until somebody goes looking, so it MUST be documented wherever the
  configuration is documented.

- **PR12.18** Verification is **not** authorization. A service MUST NOT present
  a token requirement as an access control, and MUST NOT make an access decision
  from a claim.

  Knowing that a caller is `clinician-4417` says nothing about which records
  they may open. That needs the care relationship, the care team, the consent
  directives, and the break-glass rules — a model this repository does not have
  (`S1.5`). A service that checked a `roles` claim and considered the matter
  settled would be enforcing a policy nobody wrote, which is worse than
  enforcing none, because it looks like it works.

- **PR12.19** A verified subject MUST NOT silently replace a committer the
  request supplies. Where a write endpoint accepts an `AUDIT_DETAILS` naming a
  committer that disagrees with the verified subject, the request MUST be
  refused rather than either value being preferred.

  Preferring the token would let a caller's stated intent be overwritten
  without trace; preferring the body would let a verified caller commit under
  someone else's name, which is precisely the forgery `PR12.15` is built to
  prevent, arriving through the front door instead.

  *(2026-08-02 — **implemented**.)* `openehr-loco` refuses both, and refuses
  them differently, because they are different mistakes. A committer naming
  somebody else is `403`: authentication succeeded and the claim about who did
  the work did not. A committer carrying **no identifier** is `422`: the caller
  has not tried to impersonate anyone, they have sent valid openEHR this
  service cannot attribute, and the response says what would fix it.

  Comparison is on `external_ref.id` or on any `DV_IDENTIFIER.id`, and **never
  on the name**. A name is a display string — two clinicians share one, one
  clinician changes theirs — and an audit trail keyed on it stops being able to
  answer who acted on the day somebody marries.

  This rule may live at the HTTP edge without breaching `S1.19`, which forbids
  a service enforcing *clinical* behaviour on the grounds that such a rule stops
  applying when the store is used directly. This is a rule about the
  relationship between a **caller** and a record. There is no caller when the
  store is used directly, and nothing below this layer holds a token.

- **PR12.21** The principal MUST come from the verified token and from nothing
  else. A service MUST NOT read identity, provenance, or authority from any
  request header — `X-Principal`, `X-Forwarded-User`, `X-On-Behalf-Of`,
  `Remote-User`, `X-Provenance` and their kin — and MUST NOT let one override,
  supplement, or stand in for the verified subject. There is no
  allow-listed-peer mode and no trusted-proxy mode.

  **PASETO replaces the header.** The two mechanisms differ in where the check
  lives. A trusted header is believed because of where it arrived from, which
  puts the check in the network diagram — and network diagrams are edited by
  people who are not reading this specification. A header that is safe behind
  one ingress is attacker-controlled the day a second route to the service
  exists, and nothing in the service changes to mark the day it happened. A
  signature is checked by the recipient, on every request, and does not depend
  on any statement about topology still being true.

  This is why `PR12.1` and `PR12.2` are not restored now that a service exists.
  A service MUST have a test demonstrating that such headers do not
  authenticate and do not alter the subject; the prohibition is otherwise
  satisfied by nobody having written the feature yet, which is not the same
  thing.

- **PR12.20** Requiring a token does **not** create a read audit trail, and
  documentation MUST NOT imply that it does.

  This needs saying because verification makes the gap look closed. The service
  now establishes who is reading, on every request, and then discards it
  (`PR12.5`). The information exists and is not recorded — which is a stronger
  statement than the original "this layer records no reads", and a worse one.

## Attribution lives in the model

- **PR12.4** *(amended)* Every committed version MUST record its
  `AUDIT_DETAILS` — the committing system, the change type, the committer, and
  the commit time (`M3.15`).

  This is not a storage convention layered over openEHR; `AUDIT_DETAILS` is a
  Reference Model class, and a version without one is not a valid version. The
  store therefore cannot write an unattributed row even if a caller wanted it to.

- **PR12.3a** *(amended)* Where a committer is a `PARTY_SELF` with no name, the
  stored `audit_committer_name` MUST be `NULL` (`M3.34`).

  `NULL` here means *deliberately anonymous*, not *unknown*. An anonymous subject
  is a legitimate representation in openEHR (`lib:M5.16`), and a store writing
  `"unknown"` would convert a privacy decision into a data-quality problem that
  someone would later try to clean up.

- **PR12.10** A `CONTRIBUTION` MUST carry its own audit, distinct from the audits
  of the versions it contains (`H5.14`). The change set and the individual
  changes are attributable separately, and collapsing them loses the fact that
  one act produced several versions.

## Read access is not recorded

- **PR12.5** *(amended — **not implemented**)* A complete audit of access to
  clinical data requires recording **reads**, not only writes. This layer records
  no reads.

  A store that records only mutations cannot answer "who looked at this patient",
  which is the question an access investigation actually asks. The gap is stated
  here rather than left to be discovered: a deployment needing read auditing MUST
  provide it above this layer, and MUST NOT assume the version history serves the
  purpose.

- **PR12.6** *(amended — **not implemented**)* Were read auditing added, it would
  need a stated durability mode — whether the access record commits before the
  data is returned, or after. Recorded as a design constraint for that work, not
  as a present capability.

## Tamper evidence

- **PR12.11** Append-only enforcement (`M3.17`) is **not** tamper evidence. It
  stops the database's own `UPDATE` and `DELETE` paths; it says nothing about a
  row altered by someone with file access, a restored backup, or a privileged
  session that dropped the trigger first.

  Documentation MUST NOT describe append-only as making history tamper-proof. The
  two properties are often conflated, and the conflation is comfortable, which is
  why it needs a requirement rather than a footnote.

- **PR12.12** *(amended twice on 2026-08-02 — **implemented for `openehr-sqlite`
  only**)* This said the hash chain of `M3.16` did not exist. It does, and the
  detector now does too.

  The requirement set two conditions: the columns exist, **and** a test alters a
  row and shows the store rejects it. Both are now met, for one engine.

  Meeting the second turned up why the first was not enough. The conformance
  suite checked that the chain was *well formed* — genesis zero, each link
  matching, no digest empty — which is a claim about the writer. A chain entry
  holds the digest of the content and never the content, so a row whose document
  was edited and whose five chain columns were left alone passed every one of
  those assertions. `M3.16d` is the check that closes it, and
  `openehr-sqlite/tests/tamper.rs` demonstrates it by editing a stored document
  through a second connection with the append-only triggers dropped — which is
  the threat, since a trigger is a row in `sqlite_master` and not a law of
  physics (`PR12.11`).

  **The prohibition still binds every other engine.** No crate may describe its
  storage as tamper-evident until a test alters a row *in that engine* and shows
  the store rejects it. The judgement is shared
  (`openehr_store::integrity::verify_versions`); only the corruption is
  engine-specific, because getting past append-only enforcement is spelled
  differently by each. Five engines have no such test, and four have no store to
  write one against.

  **Two limits are demonstrated rather than argued.** Truncation is not detected
  — deleting the newest version leaves a shorter history that verifies perfectly
  — and a test asserts that, so the gap cannot quietly close or quietly widen
  without someone noticing. And an unsigned chain reports `Unkeyed`, never
  `Verified`: it catches an editor who cannot recompute a digest, which is not
  an attacker holding the database, since the algorithm is public.

## Withdrawn

Withdrawn 2026-08-01. Numbers are retained and MUST NOT be reused (`C0.5`).
All were marked **[service]** and describe a component this repository does not
build (`S1.7`).

| Id | Was |
| --- | --- |
| `PR12.1` | accept a principal from a configured trusted header |
| `PR12.2` | trust the header only from an allow-listed peer |
| `PR12.3` | a require-principal mode rejecting unattributable writes |
| `PR12.7` | accept a standard `X-Provenance` header and store the resource |

A service now exists and does establish a principal, so it is worth saying why
these are not simply restored. `PR12.1` and `PR12.2` describe **trusted header**
schemes — a header believed because of where it arrived from. PASETO replaces
that header outright, and `PR12.21` forbids reinstating it in any form.
`PR12.3` is closest to live — a require-principal mode — and its ground is now
held by `PR12.16` under a new number, because numbers are never reused (`C0.5`,
`C0.19`).

`PR12.7` is additionally withdrawn on model grounds: `Provenance` and
`AuditEvent` are FHIR resources. openEHR expresses the same obligations through
`AUDIT_DETAILS`, `CONTRIBUTION`, and `ATTESTATION`, which are part of the record
rather than documents beside it (`PR12.4`).

---

Part of the [openEHR persistence specification](index.md).
