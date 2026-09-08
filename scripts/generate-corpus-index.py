#!/usr/bin/env python3
"""Generate (or check) `openehr/spec/corpus-index.md`.

Run from the repository root:

    python3 scripts/generate-corpus-index.py           # check: fails if stale
    python3 scripts/generate-corpus-index.py --write    # regenerate the file

Exit status 0 when the file matches what this script would produce, 1
otherwise.

# What this is

`tasks.md`'s "Run an external corpus, and cite the spec per test" asks for
"a `spec_refs` line to every conformance case naming the section it tests,
and generate an index from it." Read literally that is a large, separate
undertaking — annotating every `#[test]` in the tree with a new, checked
citation tag is its own design question (a new scanner, a decision about
what counts as "a conformance case," a decision about whether to formalise
citations that already exist informally), scoped out of this item on
2026-09-06 and left for its own entry.

This is the **narrow** reading, which the same scoping note found already
achievable: [`corpus.md`](../openehr/spec/corpus.md) and
[`json_corpus.md`](../openehr/spec/json_corpus.md) — the two external-corpus
run reports — already cite a requirement or finding id in most of their
disposition-table rows (`K15.6`, `A-71`, `D3.13a`, …). What was missing was
a way to ask the question the other way round: *what corpus evidence exists
for this id?* This script answers that by scanning both files for
backtick-quoted ids and emitting a reverse index, mechanically — it adds no
new citation, and the two run reports remain the one place the evidence
itself is described.

# Why generated, not hand-written

The alternative is a third document a maintainer updates by hand every time
a new corpus run adds a citation — the exact shape that has drifted before
in this repository (`spec/audit.md` **W-16**, **A-33**, **A-41**: a fact
stated twice that only one edit remembered to update). Regenerating from the
two source files instead means the index cannot know something the sources
do not say, and cannot go stale silently: `--write` with no diff to commit
*is* the proof it was already current.

# What counts as an id

Two shapes, matching this repository's own conventions
(`scripts/check-databases-matrix-coverage.py`'s `INLINE_ID`, and the same
pattern inline in `.github/workflows/ci.yml`'s "conformance matrix does not
contradict itself" step): a **finding**, one capital letter, a hyphen, and
digits (`A-71`, `D-08`, `W-16`); a **requirement**, one or more capital
letters, optional digits, a dot, digits, and an optional lower-case letter
(`K15.6`, `D3.13a`, `S1.4`). Both must appear inside a single pair of
backticks with nothing else between them, which is what keeps this from
matching a crate name (`` `openehr-store` ``, no digits after the hyphen) or
an ellipsised fragment (`` `CIMI-CORE-CLUSTER.…` ``, no digits after the
dot). Fenced code blocks are skipped entirely, so a shell command or an
example path in the "How to run it" sections cannot be mistaken for a
citation.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "openehr" / "spec" / "corpus-index.md"

# One pattern, both id shapes, anchored to a single pair of backticks — see
# the module doc's "What counts as an id".
ID_PATTERN = re.compile(r"`([A-Z]+-[0-9]+|[A-Z]+[0-9]*\.[0-9]+[a-z]?)`")

SOURCES = [
    ROOT / "openehr" / "spec" / "corpus.md",
    ROOT / "openehr" / "spec" / "json_corpus.md",
]


def sort_key(cid: str) -> tuple[int, str, int, str]:
    """Findings (`A-71`) before requirements (`K15.6`), each numerically
    within its own prefix rather than lexically — lexical order would put
    `A-71` before `A-9`."""
    finding = re.fullmatch(r"([A-Z]+)-([0-9]+)", cid)
    if finding:
        return (0, finding.group(1), int(finding.group(2)), "")
    req = re.fullmatch(r"([A-Z]+[0-9]*)\.([0-9]+)([a-z]?)", cid)
    assert req, cid  # ID_PATTERN admits nothing else
    return (1, req.group(1), int(req.group(2)), req.group(3))


def scan(path: Path) -> dict[str, list[tuple[str, str]]]:
    """Maps each id found in `path` to a list of `(section, context)` pairs,
    one per line it was cited on — `section` is the nearest preceding `##`
    heading, `context` the cited line itself, trimmed and truncated."""
    hits: dict[str, list[tuple[str, str]]] = {}
    section = "(introduction)"
    in_fence = False
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("```"):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        if line.startswith("## "):
            section = line[3:].strip()
            continue
        for match in ID_PATTERN.finditer(line):
            cid = match.group(1)
            context = line.strip().strip("|").strip()
            if len(context) > 100:
                context = context[:97] + "..."
            # A source row is often itself a markdown table row, so its
            # context can carry unescaped `|` — without escaping, those
            # would silently split this script's own output table into the
            # wrong number of columns.
            context = context.replace("|", "\\|")
            hits.setdefault(cid, []).append((section, context))
    return hits


def render(index: dict[str, dict[str, list[tuple[str, str]]]]) -> str:
    all_ids = sorted({cid for per_file in index.values() for cid in per_file}, key=sort_key)
    lines = [
        "# Corpus run index",
        "",
        "<!-- Generated by `scripts/generate-corpus-index.py --write`. Do not",
        "     hand-edit — see that script's own module doc for why. -->",
        "",
        "A reverse index from a requirement or finding id to where"
        " [`corpus.md`](corpus.md) and [`json_corpus.md`](json_corpus.md)"
        " — the two external-corpus run reports — cite it. The narrow"
        " reading of `tasks.md`'s \"generate an index from it\": this adds"
        " no new citation, it only makes the ones already there searchable"
        " in the other direction.",
        "",
        "| Id | File | Section | Context |",
        "| --- | --- | --- | --- |",
    ]
    for cid in all_ids:
        for filename, hits in index.items():
            for section, context in hits.get(cid, []):
                lines.append(f"| `{cid}` | `{filename}` | {section} | {context} |")
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    write = "--write" in sys.argv
    index = {path.name: scan(path) for path in SOURCES}
    content = render(index)

    if write:
        OUTPUT.write_text(content, encoding="utf-8")
        citations = sum(
            len(hits) for per_file in index.values() for hits in per_file.values()
        )
        ids = len({cid for per_file in index.values() for cid in per_file})
        print(f"wrote {OUTPUT.relative_to(ROOT)} ({ids} ids, {citations} citations)")
        return 0

    if not OUTPUT.exists():
        print(
            f"::error file={OUTPUT.relative_to(ROOT)}::does not exist; run "
            "`python3 scripts/generate-corpus-index.py --write`"
        )
        return 1
    if OUTPUT.read_text(encoding="utf-8") != content:
        print(
            f"::error file={OUTPUT.relative_to(ROOT)}::stale; run "
            "`python3 scripts/generate-corpus-index.py --write`"
        )
        return 1
    print(f"{OUTPUT.relative_to(ROOT)} is current")
    return 0


if __name__ == "__main__":
    sys.exit(main())
