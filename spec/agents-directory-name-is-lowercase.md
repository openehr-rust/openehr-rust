# The agents directory is named `agents`, lowercase

**Normative.** Requirement prefix: `AG`. RFC 2119 keywords, per
[`index.md`](index.md).

- **AG1** A directory holding guidance for AI agents MUST be named **`agents`**,
  in lowercase. It was `AGENTS/` until 2026-08-21.

- **AG2** The **file** [`AGENTS.md`](../AGENTS.md) keeps its uppercase name.
  That is not an inconsistency to be tidied away: `AGENTS.md` is a cross-tool
  convention that agents look for by exact name, so renaming it would make this
  repository's guidance invisible to the tools it is written for. `AG1` binds
  directories, which no convention constrains.

- **AG3** The rule generalises: **every directory in this repository is
  lowercase.** `spec/`, `assets/`, `scripts/`, `openehr/`, `openehr-store/`,
  and the fifteen other crates already were, and `AGENTS/` was the one
  exception. A single shouting directory in an otherwise lowercase tree is the
  kind of difference a reader stores as "there must be a reason", and there was
  none — it was inherited from the file beside it.

## Why this is written down rather than just done

A rename is invisible six months later. What is not invisible is the next
person adding `DOCS/` or `NOTES/`, reasoning by analogy from a directory that no
longer exists, or "fixing" `agents/` back to `AGENTS/` to match `AGENTS.md` —
which is the same reasoning that produced the inconsistency in the first place.

`W0.2` makes this repository's guidance files descriptive and its `spec/` files
normative, so a naming rule that only lived in `AGENTS.md` would be advice. This
one decides.

## Renaming on a case-insensitive filesystem

macOS and Windows treat `AGENTS` and `agents` as the same path, so a plain
`git mv AGENTS agents` is a no-op or an error depending on the tool. The rename
goes through an intermediate name:

```sh
git mv AGENTS agents-tmp && git mv agents-tmp agents
```

Recorded because the obvious command silently does nothing, and a rename that
appears to have worked and has not is exactly the class of thing
[`audit.md`](audit.md) is full of.

## What had to change with it

Ten files referenced `AGENTS/` by path: `README.md`, `CLAUDE.md`, `AGENTS.md`,
`CHANGELOG.md`, three files under `spec/`, one guide,
[`scripts/check-docs.py`](../scripts/check-docs.py), and
[`.github/workflows/ci.yml`](../.github/workflows/ci.yml). The last two matter
most — a checker that looks for a file at a path that no longer exists reports
nothing and passes.

- **AG4** A path rename MUST be followed by
  `python3 scripts/check-docs.py` and by the link check that fails on a
  Markdown link resolving to nothing. Neither is optional: `W-02` is the finding
  for a document naming a workflow file that had never existed, and a rename is
  the cheapest possible way to create one.
