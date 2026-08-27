# Maintainers

This file is the roster, and the honest answer to the question any serious
adopter of health-record software asks: **who ships the fix, and what happens if
they cannot?**

Nothing here is aspirational. Every statement describes the project as it stands
on the date given, and each one was checked against the registry or the API
rather than remembered. That is the same rule the rest of this repository runs
on (`W0.3`: never claim more than is verified).

## Roster

| Person | GitHub | crates.io | Role | Since |
| --- | --- | --- | --- | --- |
| Joel Parker Henderson (joel@joelparkerhenderson.com) | [@joelparkerhenderson](https://github.com/joelparkerhenderson) | `joelparkerhenderson` | Maintainer, sole | 2026-08-01 |

**The bus factor of this project is one.** Checked 2026-08-26:
`GET /repos/openehr-rust/openehr-rust/collaborators` returns exactly one login,
and `GET /api/v1/crates/openehr/owners` returns exactly one owner. One person
can merge, one person can publish, and no second person can do either. No
company stands behind the project and no legal entity is a party to it.

Read every other statement in this repository against that sentence rather than
around it.

## Publishing identities

An inventory nobody has written down is an inventory nobody can hand over.

| Identity | What it can publish | Held by | Recovery if the holder is unavailable |
| --- | --- | --- | --- |
| The GitHub organisation `openehr-rust` and the repository under it | the source, issues, releases, and every repository setting | the maintainer's account, sole collaborator | GitHub's own account and organisation recovery, between GitHub and the account holder. There is no second organisation owner |
| A crates.io API token, used from a workstation | the eight published crates | the maintainer, on his own machine. **There is no Trusted Publishing and no publish workflow** — `.github/workflows/` holds `ci.yml` and nothing else, and `agents/publishing.md` documents publishing as a manual `cargo publish` after `cargo login` | none. A leaked token is revoked at crates.io; a lost one is reissued by the same account. Crate ownership moves only by `cargo owner`, which needs that account |
| Git tags | which commit a release refers to | the maintainer | not applicable; tags are public and reproducible from history |
| The SSH commit- and tag-signing key (`SHA256:Ah1MPQNTLGuOy0JwLcU7LbnhSa7cRVqMaDggXwllRXc`, ed25519) | the verified signature on a commit or tag, from the point it was configured | the maintainer, passphrase-protected, on his own hardware | none: the private key is not escrowed. A successor would generate a new key and re-establish trust from a signed statement on the repository; commits made before the key existed, or before an account trusts it, stay unsigned regardless |

**Commit and tag signing is configured, starting 2026-08-27, and is not
retroactive.** `git config` for this repository (not global — other
repositories on this machine are unaffected unless configured separately)
sets `gpg.format ssh`, `user.signingkey` to the key above, and
`commit.gpgsign` / `tag.gpgsign` to `true`; `~/.ssh/allowed_signers` is set
for local verification (`git log --show-signature`). Every commit and tag
before `143b4e8` is unsigned and stays that way — a history cannot be signed
retroactively without rewriting it, and rewriting published history is worse
than the gap it would close.

**GitHub and GitLab account registration is the residual step, and it needs a
human.** Adding a signing key to an account is an interactive action neither
`gh` nor this session can complete unattended: GitHub requires the
`admin:ssh_signing_key` OAuth scope, granted via
`gh auth refresh -h github.com -s admin:ssh_signing_key` (opens a browser),
then `gh ssh-key add <path> --type signing`; GitLab has no equivalent CLI
here and needs the key added by hand under **Preferences → SSH Keys**, usage
type **Signing Key**. Until both are done, `git log --show-signature` verifies
locally but GitHub/GitLab will show new commits as **Unverified** rather than
**Verified** — a real, temporary gap, not a documentation lag: check
`git log --format='%G?'` on the latest commit, or the badge on the commit
page, before trusting either.

Do not treat authorship in history before 2026-08-27, or in any commit while
account registration is pending, as attested by anything stronger than
GitHub's or GitLab's account controls. If that matters to your adoption, check
the date and the account-verification badge rather than assuming from this
paragraph alone — a description is not a certificate, which is `W0.2`'s point
applied to this file rather than to a spec.

There is no container image, no hosted service, no documentation domain, no
Zenodo deposit, and no DOI. docs.rs builds the API documentation from the
published crates and is operated by the Rust project, not by this one.

## If the maintainer is unavailable

No document can conjure a succession plan. What is true instead:

- **Nothing already published disappears.** A published crate version is
  immutable and cannot be deleted — only yanked, which itself requires the owner
  account. `agents/publishing.md` opens with the consequence of that
  immutability, learned the hard way: `openehr` 0.1.0 shipped with a
  `repository` field pointing at an unrelated project, and it says so
  permanently.
- **Nothing new ships.** No release, no fix, no advisory. A dependent stays on
  the version it pinned, indefinitely.
- **The work is not lost, and forking is the intended remedy.** The licence is a
  five-way choice (see [`LICENSE.md`](LICENSE.md)), the history is public, and —
  unusually — the reasoning is in the tree rather than in someone's head: the
  specifications in [`spec/`](spec/index.md), the operational guides in
  [`agents/`](agents/index.md), and the register of known defects in
  [`spec/audit.md`](spec/audit.md). A fork inherits all of it and is a
  legitimate continuation. Take it rather than waiting.
- **`Cargo.lock` is committed in every crate**, unusually for libraries, so a
  build from a given commit resolves to the dependency versions it was tested
  with even years later.

If you are considering this software where patient data is involved and a
one-person project is not acceptable to you — which is a reasonable position —
the mitigation is on your side of the boundary: pin a version, keep a fork you
can build, and budget for maintaining it. That answer is more useful than a
continuity plan with nobody behind it.

## Becoming a maintainer

The route is ordinary and it is open: send patches, take part in the issue
tracker, and take responsibility for an area. What changes when someone does:
this file gains a row, [`CODEOWNERS`](CODEOWNERS) gains their
address on the areas they own, and the identity table above gains a second
holder wherever the identity permits one — `cargo owner --add` on the crates,
and a second organisation owner on GitHub. Those three edits are the whole
mechanism.

Contributions are held to what this repository already requires of itself:
a specification before the commit (`W0.19`), a test that would have caught the
defect, no claim beyond what has been run (`W0.3`), and a zero-warning tree.
[`AGENTS.md`](AGENTS.md) is the operational guide and applies to human and
machine contributors alike.

## Issues, and what response to expect

**Public issues are read within a week.** That is a target one person can
usually keep, not a contract — the honest version of every commitment in this
file. A reply may be "filed, no timeline"; what it will not be is a claim of
progress nothing substantiates (`W0.3`). The templates in
`.github/ISSUE_TEMPLATE/` say what makes a report actionable, and the
contribution valued most here is the *wrong claim* report: documentation
saying something the software does not do. Vulnerabilities have their own
channel and their own stated windows — see below.

## Security reports

[`SECURITY.md`](SECURITY.md) is the policy: email
joel@joelparkerhenderson.com rather than opening a public issue, send a
**synthetic** reproduction and never patient data, and expect the handling to be
one person's, on one person's schedule — with a stated escalation path if that
person goes quiet.

---

Structure adapted from [FerroEHR's `MAINTAINERS.md`](https://github.com/rubentalstra/FerroEHR/blob/develop/MAINTAINERS.md)
(MIT), whose framing of continuity risk is better than what this project would
have written unprompted. The facts are this project's own.
