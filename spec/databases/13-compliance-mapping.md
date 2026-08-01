# 13. Compliance mapping

**Rewritten 2026-08-01.** The previous table cited requirements that are now
withdrawn, retired-section identifiers that were never restored, and evidence
("Inferno run", "live TLS smoke test") that cannot be produced against a library.
Several rows mapped a regulatory obligation to a hash chain and an erasure
operation that do not exist, which is the worst place in the tree for an
overstatement. See [`spec/audit.md`](../audit.md) **W-04**.

**Non-normative.** This section defines no requirements. It maps obligations to
the numbered requirements that exist to support them, so a reviewer can trace a
regulation to a requirement to a test.

## What these crates are, for this purpose

These are **components, not certified systems**. They cannot make a deployment
compliant, and they must not be the reason a deployment cannot be.

A library contributes evidence for a handful of obligations and is irrelevant to
most. The table below is deliberately short for that reason: a long mapping from
a small library is a sign that somebody has been generous with the word
"supports".

## The mapping

Status is as of 2026-08-01 and reflects what is **verified**, not what is
intended.

| Obligation | Supported by | Status |
| --- | --- | --- |
| HIPAA §164.312(b) — audit controls | `M3.15`, `PR12.4`, `PR12.10`, `M3.34` | **Partial.** Writes record system, change type, committer, and time, enforced by the model itself. **Reads are not recorded at all** (`PR12.5`). |
| HIPAA §164.312(c)(1) — integrity | `M3.17`, `M3.36`, `R4.4` | **Partial.** Append-only is enforced by the database on three engines and verified there. This is **not** tamper evidence (`PR12.11`), and there is no hash chain (`M3.16`). |
| HIPAA §164.312(e) — transmission security | `O10.7` | **Deployment's.** The requirement that a PHI-carrying connection be encrypted is stated; no crate here opens a network connection, so nothing implements it yet (`O10.15`). |
| HIPAA §164.502(b) — minimum necessary | `PR12.8` | **Out of scope.** Nothing here evaluates whether a principal *may* see what it asked for. The store records that it did. |
| GDPR Art. 17 — erasure | `M3.18` | **Not implemented.** No erasure operation exists, and disabling the append-only triggers to achieve one is forbidden because it removes the guarantee for every other row. |
| GDPR Art. 30 — records of processing | `PR12.4`, `PR12.10` | **Partial**, for the same reason as §164.312(b): writes only. |
| GDPR Art. 32 — security of processing | `O10.7`, `O10.10`, `M3.38` | **Partial.** Connection encryption is required, supply-chain evidence ships, and errors and logs are forbidden from carrying record content. |
| IEC 62304 §5–8 — lifecycle traceability | `C0.4`, `C0.5`, `C0.20`, `T11.10` | **Supported.** Every requirement has a stable identifier, identifiers are never reused, and a requirement with no evidence is recorded as unverified rather than assumed. |
| Software identity / SBOM accuracy | `O10.11`, `W16.15`, `W16.17` | **Supported.** Lock files are committed, published versions must match their source, and the `repository` field must name the actual repository — which it did not until 2026-08-01 (**W-03**). |

## Obligations this layer does not touch

Stated so that "not covered" and "covered elsewhere" stay distinguishable:

- **Authorization** — scopes, consent, compartments, access-label enforcement.
  Lives at the perimeter (`PR12.8`).
- **Authentication.** This layer records who a caller *says* acted (`S1.8`).
- **Terminology validation.** Out of scope (`V9.4`); external codes are carried
  opaquely.
- **Read auditing.** Not implemented (`PR12.5`), and the version history does not
  serve the purpose — it records changes, and an access investigation asks about
  reads.
- **Certification of any kind.** No crate here is certified, and none claims to
  be.

## How to use this table in an audit

1. Pick the obligation row.
2. Follow each requirement id into the core specification — it says what MUST
   hold and why.
3. Read the **Status** column, which is the part that matters. Several rows are
   *Partial* or *Not implemented*, and those words are load-bearing.
4. Check the [conformance matrix](conformance-matrix.md) for the **engine you are
   actually deploying**. A requirement verified against SQLite is not thereby
   verified against Oracle.

Step 4 is the one that gets skipped and the one that matters: only one of six
engine crates has a store at all, and two have never had their DDL parsed by the
engine they name (`C0.8`).

## A caution about this document

A compliance mapping is the document most likely to be read by someone who will
not check it, and therefore the one where an overstatement travels furthest. The
version this replaced mapped HIPAA integrity to a hash chain that was never
built, and GDPR erasure to a purge operation that does not exist.

If a row here says **Supported**, a reader should be able to follow it to a test
that runs. If it cannot, the row is a defect and belongs in
[`spec/audit.md`](../audit.md).

---

Part of the [openEHR persistence specification](index.md).
