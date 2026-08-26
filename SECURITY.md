# Security policy

## Reporting a vulnerability

**Email joel@joelparkerhenderson.com.** Put `SECURITY` in the subject.

Or use GitHub's private vulnerability reporting — enabled on this repository
2026-08-26 — to open a draft advisory at
<https://github.com/openehr-rust/openehr-rust/security/advisories/new>. Both
channels reach the same one maintainer; use whichever you prefer. (An earlier
revision of this file correctly said private reporting was not enabled and
email was the only private channel; that gap is closed.)

**Do not open a public issue for a vulnerability** until it is fixed or until
the window below has passed.

### Never send patient data

This is a clinical-record library, and the reflex to attach "the document that
broke it" is the wrong one here. **A reproduction must be synthetic.** If a real
record triggered the bug, reconstruct it with invented values — the structure is
what matters, and the structure is what this project can act on.

A report containing real patient data will be deleted rather than filed, and you
will be asked for a synthetic reproduction. That is not pedantry: this project
has no lawful basis to hold your patients' data, no agreement with you covering
it, and no environment approved to store it.

### What to include

The version (`openehr 0.7.3`, a commit, or both), what you did, what you
expected, what happened, and why you think it is a security problem rather than
a defect. A failing test is the strongest possible form of this.

## What to expect

One person maintains this ([`MAINTAINERS.md`](MAINTAINERS.md)), so these are
commitments a single human can keep rather than a corporate SLA:

| Stage | Commitment |
| --- | --- |
| Acknowledgement | within **5 working days** |
| An assessment — is it a vulnerability, and how bad | within **14 days** of acknowledgement |
| Fix, or a stated reason there will not be one | agreed with you, based on severity |
| **If you get no acknowledgement within 14 days** | consider yourself released from coordinated disclosure, and publish |

That last row is deliberate. A disclosure policy that asks for silence without
promising a reply is a way of burying reports, and a one-person project is
exactly where that happens. If this project goes quiet on you, the fault is
here, and publishing is your call.

## Supported versions

**The newest published version only.** There are no backports and no long-term
support branch; a fix ships as a new release of all eight crates, which are
versioned in lockstep.

A published crate version is **immutable**. It cannot be edited, and yanking it
does not change its metadata or its documentation — a yank removes it from
resolution and leaves everything it claimed readable. So a vulnerable version
stays on crates.io, and pinning to it stays possible. `Cargo.lock` is committed
in every crate here, which makes an audit of what a given commit actually built
straightforward.

## Scope

**In scope** — the eight published crates, the DDL they emit, and
`openehr-loco`:

- Anything that makes this crate **disclose protected health information** it
  was supposed to withhold. `X11.7` forbids node content in errors, no `Display`
  renders an identifier or a media blob, and `security::redact` masks rather
  than deletes and counts rather than names. A path around any of those is a
  vulnerability, not a defect.
- **Access control that fails open.** `X11.24` requires an unevaluatable access
  scheme to deny. An input that turns a denial into a permit is the highest
  severity this project has.
- **Integrity**: a way to alter stored content while `verify_versions` still
  passes, to break the audit chain, or to make canonical JSON round-trip to
  different bytes than were stored (`db:M3.43`).
- **Append-only enforcement**: DDL that permits `UPDATE` or `DELETE` on a
  version row where the schema is supposed to refuse it.
- **Injection**: any input that reaches emitted SQL as syntax rather than as
  data — identifiers, table names, and DDL parameters included.
- **Memory-safety or panic-based denial of service** from a document, a path, or
  an AQL query that a service would plausibly accept. `unsafe_code` is `forbid`,
  so this means panics and unbounded resource use rather than corruption.
- `openehr-loco`'s **PASETO verification** and its authorisation checks.

**Out of scope** — real, but not this project's to fix:

- Your deployment: transport security, key management, authentication, network
  exposure. `S1.11` and `S1.14` state that the crate neither encrypts nor
  authenticates; those are deliberately the caller's.
- The database engines themselves. Report those upstream — and tell this project
  too if the schema depends on the behaviour.
- The openEHR specifications. Report those to openEHR International; if this
  project implemented one wrongly, that is in scope.
- Dependency advisories: report upstream first. Dependabot alerts and
  automated security fixes are **enabled** on this repository (2026-08-26),
  so an advisory affecting these crates should reach the maintainer through
  automation — but every crate here pins a committed `Cargo.lock`, so tell
  this project too if the advisory affects a pinned version.

## Documented boundaries that are not vulnerabilities

Each of these is a stated design decision with a requirement id. Reporting one
gets a citation back rather than a fix — **unless you can show it reaches
further than the boundary says**, in which case it is a real report and a
valuable one.

| Behaviour | Where it is stated |
| --- | --- |
| `Deserialize` does not validate; a document read from JSON has passed no gate until `validate()` runs | `L10.1a`, `lib:A-23` |
| Deserialization depth is not bounded by this crate; a caller reading untrusted documents must bound it | `S1.15` |
| No encryption, and `ATTESTATION.proof`'s OpenPGP signature is not verified | `S1.11` |
| No authentication; the crate records who acted and does not establish who they are | `S1.14` |
| No archetype validation, so a valid `COMPOSITION` may violate its archetype | `lib:A-40`, `K15.30` |
| External terminology codes are carried opaquely and never resolved | `S1.10` |
| AQL is parsed and never executed | `S1.5` |
| An `EHR_ACCESS` may carry no policy at all, and a deployment must not read that as permission | `S1.20`, `X11.24` |

## What happens after a report

1. A fix lands with the requirement written first (`W0.19`) and a test that
   fails without it (`C0.10`).
2. A finding goes in the audit register with evidence, because a gap that is not
   written down reads as a pass (`W0.4`).
3. A new version of all eight crates is published, and
   [`CHANGELOG.md`](CHANGELOG.md) says what it was — plainly, not euphemistically.
4. An advisory goes to the **RustSec advisory database** and GitHub's advisory
   database, so `cargo audit` finds it whether or not anyone reads this file.
5. **You are credited** by the name or handle you choose, unless you ask not to
   be.

There is **no bug bounty** and no payment. This project has no funding vehicle
at all ([`CONTRIBUTING.md`](CONTRIBUTING.md) says so plainly); nobody is being
paid here, including the maintainer.

## Known gaps in this project's own security posture

Stated because a security policy that hides its own weaknesses is worse than
none (`W0.3`):

- **Commits and tags are not signed** (`git log --format=%G?` reports `N`).
  Authorship is attested by GitHub's account controls and nothing stronger.
- ~~**GitHub private vulnerability reporting, Dependabot, and secret scanning
  are all disabled** on the repository, checked 2026-08-26.~~ Closed
  2026-08-26, later the same day: all four settings — private vulnerability
  reporting, Dependabot alerts, automated security fixes, secret scanning —
  are enabled, each verified with a `GET` after the change. Secret scanning
  **push protection** remains off, so a pushed secret is reported rather
  than blocked.
- **No SBOM is published**, and no release artefacts are attested.
- **The bus factor is one**, and every publishing identity terminates at one
  account ([`MAINTAINERS.md`](MAINTAINERS.md)). A report arriving while the
  maintainer is unavailable will sit until they return.
- **No third party has audited this code**, and the machine gates prove what
  they test rather than correctness ([`AI_STATEMENT.md`](AI_STATEMENT.md) §9).

## Trademarks

openEHR® is the registered trademark of the openEHR Foundation. Use of the
trademark does not constitute endorsement of this product by openEHR
International or openEHR Foundation.
