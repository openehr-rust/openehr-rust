#!/usr/bin/env python3
"""Report every `spec/databases/` requirement with **zero** mentions in the
conformance matrix.

Run from the repository root:

    python3 scripts/check-databases-matrix-coverage.py

# What this checks, and what it deliberately does not

The library's `openehr/spec/conformance-matrix.md` is one linear walk through
every requirement, so CI can check it covers each **exactly once**
(`.github/workflows/ci.yml`, "Every requirement has exactly one row in the
library matrix"). `spec/databases/conformance-matrix.md` is not that shape: it
is five topic tables — per-engine, store-level, service, cross-cutting, and
"not implemented in the store" — and a requirement can legitimately appear in
more than one. `PR12.5` is marked satisfied in the service table (`openehr-loco`
audits reads) and listed absent in "not implemented in the store" (a program
embedding `openehr-store` directly gets none of that), and both statements are
true at once. An "exactly once" checker would flag that as a defect; it is the
document doing its job.

So this script checks the **floor** an "exactly once" checker cannot honestly
skip past for this file: a requirement mentioned **nowhere** at all. That is
unambiguous regardless of how many tables a requirement may legitimately
belong to, and it is the failure mode `spec/databases/audit.md` **D-11** found
at scale — 144 of 221 requirements, including `M3.19`, the requirement that
canonical JSON **is** the record.

**What a clean run does not prove.** A requirement that is mentioned once is
not thereby verified — `db:C0.20` already says a mark is a claim about
evidence, not about presence. This script cannot read a row and judge whether
the mark it carries is deserved; see `spec/databases/audit.md` **D-09**'s own
residual note, which says the same thing about the self-contradiction check.

# Why this is not wired into CI yet

144 missing rows is not a regression this script could have caught early; it is
the accumulated state of a matrix assessed once, by hand, on 2026-08-02, while
the specification it summarises kept growing. Wiring this into CI today would
fail the build on every push until each of the 144 is individually assessed
against six engines and a decision recorded — real engineering and domain
judgement, not something a script can manufacture by inventing rows. Filing
that as a blocking gate before the assessment exists would force exactly the
choice this repository's whole audit apparatus is built to prevent: rush 144
guesses to turn the build green, or leave it red indefinitely and have everyone
stop trusting the gate. See `spec/databases/audit.md` **D-11** for the finding,
and `plan.md`'s "Open decisions" for the recommended path to closing it.
"""

from __future__ import annotations

import glob
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent

REQUIREMENT_BULLET = re.compile(r"^- \*\*([A-Z]+[0-9]*\.[0-9]+[a-z]?)\*\*", re.MULTILINE)
INLINE_ID = re.compile(r"`([A-Z]+[0-9]*\.[0-9]+[a-z]?)`")
# The one idiom this file uses for a range: two backtick-quoted ids joined by an
# en dash or hyphen, as prose rather than a table's Id column — `M3.39`–`M3.42`.
# Only one instance exists today; if a second range shape appears, extend this
# rather than adding a second regex that quietly disagrees with it.
INLINE_RANGE = re.compile(
    r"`([A-Z]+[0-9]*)\.(\d+)`[–-]`(?:([A-Z]+[0-9]*)\.)?(\d+)`"
)

# Files that state requirements, per `spec/databases/index.md`'s Contents list.
# `conformance-matrix.md`, `audit.md`, and `index.md` are status/framework, not
# requirement sources. The three id-scheme outliers (`locale-accent-folding.md`,
# `search-adjuncts.md`, `unbounded-string-search-must-have-bounded-adjunct-and-
# checksum-adjunct.md`) use a different, non-sectioned grammar (`L1`, `AD1`) and
# are excluded automatically because `REQUIREMENT_BULLET` does not match it —
# they are referenced *by* the numbered requirements rather than being numbered
# sections themselves.
EXCLUDED = {"conformance-matrix.md", "audit.md", "index.md"}


def defined_requirements() -> set[str]:
    ids: set[str] = set()
    for path in sorted(glob.glob(str(ROOT / "spec/databases/*.md"))):
        if pathlib.Path(path).name in EXCLUDED:
            continue
        ids |= set(REQUIREMENT_BULLET.findall(pathlib.Path(path).read_text()))
    return ids


def mentioned_requirements(matrix_text: str) -> set[str]:
    ids = set(INLINE_ID.findall(matrix_text))
    for prefix, lo, prefix2, hi in INLINE_RANGE.findall(matrix_text):
        if prefix2 and prefix2 != prefix:
            continue
        ids |= {f"{prefix}.{n}" for n in range(int(lo), int(hi) + 1)}
    return ids


def main() -> int:
    matrix_path = ROOT / "spec/databases/conformance-matrix.md"
    defined = defined_requirements()
    mentioned = mentioned_requirements(matrix_path.read_text())
    missing = sorted(defined - mentioned)

    if missing:
        print(
            f"{len(missing)} of {len(defined)} spec/databases/ requirements have "
            f"zero mentions in {matrix_path.relative_to(ROOT)}:"
        )
        for ident in missing:
            print(f"  {ident}")
        print(
            f"\n{len(defined) - len(missing)} of {len(defined)} are mentioned at "
            "least once (not thereby verified — a mention is not a mark)."
        )
        return 1

    print(f"all {len(defined)} spec/databases/ requirements are mentioned at least once")
    return 0


if __name__ == "__main__":
    sys.exit(main())
