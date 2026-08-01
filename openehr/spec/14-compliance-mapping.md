# 14. Compliance mapping

Non-normative. This section carries no requirements; it maps external controls
onto the requirements that bear on them, so that an auditor asking "how do you
satisfy X" gets a number rather than a conversation.

**Read this first.** The crate is a library. It cannot satisfy a control on its
own — most of these rows describe a control the *deployment* satisfies, to which
the crate contributes a part. The third column says which part, and the fourth
says what the deployment must still do. A row with an empty fourth column would
be a claim this crate is not entitled to make.

## HIPAA Security Rule (45 CFR §164.3xx)

| Control | Requires | The crate contributes | The deployment must still |
| --- | --- | --- | --- |
| §164.312(a)(1) Access control | technical policy limiting access | `X11.3`–`X11.5a`: a place to record policy, a default-deny decision, a reference scheme | authenticate, resolve group membership, enforce the decision |
| §164.312(b) Audit controls | record and examine activity | `V8.13`, `V8.14`, `X11.2`: who committed what, when, why, on every version | record *reads*, retain and review the logs |
| §164.312(c)(1) Integrity | protect from improper alteration | `V8.1`–`V8.5`, `X11.9`–`X11.15`: append-only versioning plus a tamper-evident chain | store the head digest outside the database, hold the key outside it |
| §164.312(e)(1) Transmission security | protect in transit | — | provide TLS; the crate does not transmit (`S1.11`) |
| §164.514 De-identification | remove identifiers | `X11.20`–`X11.25`: masking that preserves the fact of withholding | choose the identifiers; element rules are not de-identification (`X11.25`) |

## GDPR

| Article | Requires | The crate contributes | The deployment must still |
| --- | --- | --- | --- |
| Art. 5(1)(d) Accuracy | correction is recorded | `V8.9`–`V8.10`: correction is a new version, deletion is a version | surface the history to the data subject |
| Art. 15 Right of access | a copy of the data | `J9.1`: lossless export | assemble the record, authenticate the subject |
| Art. 17 Right to erasure | erasure | `V8.10` gives logical deletion **only** | decide and implement physical erasure; the crate has no store |
| Art. 25 Data protection by design | technical measures | `X11.4` default-deny, `X11.6`–`X11.8` no PHI in output, `G7.1` demographic separation | the rest of the design |
| Art. 32 Security of processing | integrity and confidentiality | `X11.9`–`X11.19` | encryption at rest and in transit (`S1.11`) |

## IEC 62304 / ISO 13485

| Clause | Requires | The crate contributes |
| --- | --- | --- |
| 5.1 Development planning | a defined process | this specification directory, and `C0.15`–`C0.18` for amendment |
| 5.2 Requirements analysis | recorded, traceable requirements | permanent identifiers (`C0.4`, `C0.5`) cited from code and tests |
| 5.5 Unit implementation and verification | verification per unit | `T13.1`–`T13.3`, and the [matrix](conformance-matrix.md) |
| 5.7 System testing | integration-level verification | `T13.4`–`T13.8` |
| 7.1 Risk analysis | hazards identified | the "why the obvious thing is wrong" rationale attached to each requirement |
| 9.x Problem resolution | known problems recorded | [`audit.md`](audit.md) |

## ISO 13606 and openEHR conformance

The crate implements the openEHR Reference Model; it does not claim conformance
to an openEHR **conformance profile**, because those cover a service and this is
a library (`S1.7`). What it claims, and only what it claims, is in the
[conformance matrix](conformance-matrix.md).
