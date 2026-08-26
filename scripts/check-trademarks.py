#!/usr/bin/env python3
"""Check that every in-scope page using the openEHR mark carries the notice.

Run from the repository root:

    python3 scripts/check-trademarks.py

Exit status 0 when every file in scope that uses the mark in prose carries the
notice verbatim, 1 otherwise. The rule is `spec/professionalization/index.md`
rule 5; the notice below is the verbatim text that rule binds, as the owner
specified it on 2026-08-26.

# What this exists to prevent

This project's organisation, repository, and crate names all use a mark it
does not own — openEHR, the registered trademark of the openEHR Foundation
(U.S. Reg. No. 4,272,380, EUIPO Reg. 002994853, IP Australia Reg. 939279).
A reader who meets the name with no notice may reasonably conclude the
project is official. The notice is the floor of honest behaviour here, and a
notice that exists only where someone remembered to paste it is the same
defect class as a count nobody re-checks (`W0.39`).

# Scope, decided rather than accumulated

- Root `*.md`, except `AGENTS.md` and `CLAUDE.md` — those are operational
  guides for contributors already inside the repository, non-normative
  (`W0.2`), under the agent-file byte budget that `check-docs.py` enforces,
  and reached only from surfaces that carry the notice.
- `help/**/*.md`.
- `src/lib.rs` of every **published** crate (derived from the manifests, the
  way `check-docs.py` derives it): the rustdoc a crates.io or docs.rs reader
  lands on. Doc comments only — string literals and code are not prose.

`openehr/spec/**`, `spec/**`, and `agents/**` are deliberately out: the
specification trees use the mark in nearly every file, a notice per section
would drown the text it annotates, and every route into them starts at a
noticed surface. That decision is recorded in the professionalization spec's
Status section.

# How it decides "uses the mark in prose"

Prose only: fenced code blocks, inline code spans, link targets, autolinks,
and HTML comments are blanked before matching (offsets preserved, so reported
line numbers are real). In Rust files only `//!`/`///` doc comments are read,
with their code fences removed. The match is case-insensitive on the word
`openehr`, so a prose mention of a crate name counts as using the mark —
which it is.
"""

from __future__ import annotations

import pathlib
import re
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parent.parent

# The verbatim notice of professionalization rule 5. Line wrapping, comment
# markers, and blockquote prefixes may differ per file; the words may not.
NOTICE = (
    "openEHR® is the registered trademark of the openEHR Foundation. Use of "
    "the trademark does not constitute endorsement of this product by "
    "openEHR International or openEHR Foundation."
)

# Root documents exempt from the rule, each for a stated reason (see the
# module docstring). An exemption with no reason would be a divergence that
# survived review.
EXEMPT_ROOT = {"AGENTS.md", "CLAUDE.md"}

MARK = re.compile(r"\bopenehr\b", re.IGNORECASE)


def blanked(text: str, patterns: list[str]) -> str:
    """Replace each match with NULs, so line numbers are preserved."""
    for pattern in patterns:
        text = re.sub(pattern, lambda m: "\0" * len(m.group()), text)
    return text


def prose_markdown(text: str) -> str:
    return blanked(
        text,
        [
            r"(?ms)^([ \t]*)(`{3,}|~{3,}).*?^\1\2[ \t]*$",  # fenced code
            r"`[^`\n]+`",  # inline code
            r"\]\([^)]*\)",  # link targets
            r"<https?://[^ >\n]+>",  # autolinks
            r"(?s)<!--.*?-->",  # comments
        ],
    )


def prose_rust_docs(text: str) -> str:
    """Only doc comments, with their code fences removed."""
    kept: list[str] = []
    fence = False
    for line in text.split("\n"):
        stripped = line.lstrip()
        if not (stripped.startswith("//!") or stripped.startswith("///")):
            kept.append("")
            continue
        body = re.sub(r"^\s*//[!/]", "", line)
        if re.match(r"\s*```", body):
            fence = not fence
            kept.append("")
            continue
        kept.append("" if fence else body)
    return blanked("\n".join(kept), [r"`[^`\n]+`"])


def has_notice(text: str) -> bool:
    """The notice, allowing any wrapping, comment prefix, or blockquote."""
    flat = re.sub(r"(?m)^\s*(//[!/]|>)\s?", "", text)
    flat = re.sub(r"\s+", " ", flat)
    return NOTICE in flat


def published_lib_roots() -> list[pathlib.Path]:
    out = []
    for manifest in sorted(ROOT.glob("*/Cargo.toml")):
        package = tomllib.loads(manifest.read_text()).get("package", {})
        if package.get("publish") is False:
            continue
        lib = manifest.parent / "src/lib.rs"
        if lib.is_file():
            out.append(lib)
    return out


def main() -> int:
    targets: list[tuple[pathlib.Path, str]] = []
    for path in sorted(ROOT.glob("*.md")):
        if path.name not in EXEMPT_ROOT:
            targets.append((path, "md"))
    for path in sorted((ROOT / "help").rglob("*.md")):
        if not path.is_symlink():
            targets.append((path, "md"))
    for path in published_lib_roots():
        targets.append((path, "rs"))

    bad = 0
    uses = 0
    for path, kind in targets:
        text = path.read_text(encoding="utf-8")
        prose = prose_markdown(text) if kind == "md" else prose_rust_docs(text)
        first = MARK.search(prose)
        if first is None:
            continue
        uses += 1
        if not has_notice(prose):
            rel = path.relative_to(ROOT)
            line = prose[: first.start()].count("\n") + 1
            print(
                f"::error file={rel},line={line}::uses the openEHR mark in "
                "prose but does not carry the trademark notice verbatim "
                "(spec/professionalization/index.md rule 5)"
            )
            bad = 1

    if bad:
        print(
            "::error::the notice, verbatim, wrapped and comment-prefixed "
            f"as the file requires: {NOTICE}"
        )
        return 1

    print(
        f"{len(targets)} files in scope, {uses} use the openEHR mark in "
        "prose, and every one of those carries the notice"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
