# 12. Trust, principal, and audit

**Rewritten 2026-08-01.** Most of this section specified how a **service**
establishes a principal — a trusted header, an allow-listed proxy, a
require-principal mode, an `X-Provenance` header. No service exists (`S1.7`), and
openEHR carries attribution inside the model rather than alongside it, which
changes the shape of the answer. See [`spec/audit.md`](../audit.md) **W-04**.

Withdrawn requirements keep their numbers (`C0.5`); new ones start at `PR12.9`
(`C0.19`).

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

- **PR12.9** A store MUST NOT infer, default, or synthesize a committer. If the
  caller does not supply one, the commit is refused; it is never attributed to
  the process, the connection user, or "system".

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

- **PR12.12** *(**not implemented**)* Tamper evidence would require the hash
  chain of `M3.16`, which does not exist here. The `openehr` crate implements the
  primitives — digests, keyed tags with constant-time comparison, key material
  zeroized on drop — and the store does not use them. No crate may claim tamper
  evidence until the columns exist and a test demonstrates that mutation is
  detected.

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

`PR12.7` is additionally withdrawn on model grounds: `Provenance` and
`AuditEvent` are FHIR resources. openEHR expresses the same obligations through
`AUDIT_DETAILS`, `CONTRIBUTION`, and `ATTESTATION`, which are part of the record
rather than documents beside it (`PR12.4`).

---

Part of the [openEHR persistence specification](index.md).
