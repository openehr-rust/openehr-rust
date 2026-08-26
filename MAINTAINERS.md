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

**Commits and tags in this repository are not cryptographically signed**
(`git log --format=%G?` reports `N`). Do not treat authorship in the history as
attested by anything stronger than GitHub's account controls. If that matters to
your adoption, say so on the tracker — it is a solvable gap, and it is listed
here rather than left for you to discover.

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
this file gains a row, [`.github/CODEOWNERS`](.github/CODEOWNERS) gains their
address on the areas they own, and the identity table above gains a second
holder wherever the identity permits one — `cargo owner --add` on the crates,
and a second organisation owner on GitHub. Those three edits are the whole
mechanism.

Contributions are held to what this repository already requires of itself:
a specification before the commit (`W0.19`), a test that would have caught the
defect, no claim beyond what has been run (`W0.3`), and a zero-warning tree.
[`AGENTS.md`](AGENTS.md) is the operational guide and applies to human and
machine contributors alike.

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
