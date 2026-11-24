#!/usr/bin/env python3
"""Enforce W0.41 against a cargo-llvm-cov LCOV export."""

from __future__ import annotations

import argparse
import hashlib
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
REGISTER = ROOT / "coverage-exclusions.tsv"
FLOORS = ROOT / "coverage-floor.tsv"


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--workspace", required=True)
    parser.add_argument("--lcov", required=True, type=Path)
    return parser.parse_args()


def exclusions(workspace: str) -> dict[tuple[Path, int], str]:
    found: dict[tuple[Path, int], str] = {}
    for number, raw in enumerate(REGISTER.read_text().splitlines(), 1):
        if not raw or raw.startswith("#"):
            continue
        fields = raw.split("\t")
        if len(fields) != 5:
            raise SystemExit(f"{REGISTER}:{number}: expected five tab-separated fields")
        owner, source, line_text, expected_hash, reason = fields
        if owner != workspace:
            continue
        source_path = (ROOT / source).resolve()
        try:
            source_path.relative_to((ROOT / workspace / "src").resolve())
        except ValueError as error:
            raise SystemExit(f"{REGISTER}:{number}: exclusion is outside {workspace}/src") from error
        if source_path.suffix != ".rs":
            raise SystemExit(f"{REGISTER}:{number}: exclusion is not Rust source")
        line = int(line_text)
        lines = source_path.read_bytes().splitlines(keepends=True)
        if line < 1 or line > len(lines):
            raise SystemExit(f"{REGISTER}:{number}: line {line} does not exist")
        actual_hash = hashlib.sha256(lines[line - 1]).hexdigest()
        if actual_hash != expected_hash:
            raise SystemExit(
                f"{REGISTER}:{number}: {source}:{line} changed; review or remove its exclusion"
            )
        key = (source_path, line)
        if key in found:
            raise SystemExit(f"{REGISTER}:{number}: duplicate exclusion for {source}:{line}")
        found[key] = reason
    return found


def floor(workspace: str) -> tuple[int, int] | None:
    """The transitional ratchet for a workspace not yet at 100% (`W0.41a`).

    Returns `(uncovered_lines, uncovered_functions)` the workspace is allowed,
    or `None` when it owes the full 100%. A floor may only fall: a workspace
    that beats its floor fails until the floor is lowered, so a gain cannot be
    given back by the next change.
    """
    if not FLOORS.exists():
        return None
    for number, raw in enumerate(FLOORS.read_text().splitlines(), 1):
        if not raw or raw.startswith("#"):
            continue
        fields = raw.split("\t")
        if len(fields) != 4:
            raise SystemExit(f"{FLOORS}:{number}: expected four tab-separated fields")
        if fields[0] == workspace:
            return int(fields[1]), int(fields[2])
    return None


def main() -> None:
    args = arguments()
    line_hits: dict[tuple[Path, int], int] = {}
    functions_found = 0
    functions_hit = 0
    source: Path | None = None
    own_src = (ROOT / args.workspace / "src").resolve()
    in_scope = False

    # Only this workspace's own `src/` is its production source. A path
    # dependency's lines (and stray profile data from other projects sharing a
    # target directory) are another owner's evidence (`W0.41`).
    for raw in args.lcov.read_text().splitlines():
        if raw.startswith("SF:"):
            source = Path(raw[3:]).resolve()
            in_scope = source.is_relative_to(own_src)
        elif not in_scope:
            continue
        elif raw.startswith("DA:"):
            if source is None:
                raise SystemExit("DA record appeared before SF")
            line_text, hits_text, *_ = raw[3:].split(",")
            key = (source, int(line_text))
            line_hits[key] = max(line_hits.get(key, 0), int(hits_text))
        elif raw.startswith("FNF:"):
            functions_found += int(raw[4:])
        elif raw.startswith("FNH:"):
            functions_hit += int(raw[4:])

    if not line_hits or not functions_found:
        raise SystemExit("coverage export contains no production lines or functions")

    allowed = exclusions(args.workspace)
    missing = {key for key, hits in line_hits.items() if hits == 0}
    stale = set(allowed) - missing
    unregistered = missing - set(allowed)
    if stale:
        rendered = "\n".join(f"  {path}:{line}" for path, line in sorted(stale))
        raise SystemExit(f"stale coverage exclusions (remove them):\n{rendered}")
    functions_missed = functions_found - functions_hit
    allowance = floor(args.workspace)
    if allowance is not None:
        max_lines, max_functions = allowance
        if len(unregistered) > max_lines or functions_missed > max_functions:
            rendered = "\n".join(f"  {path}:{line}" for path, line in sorted(unregistered))
            raise SystemExit(
                f"{args.workspace} fell below its coverage floor: "
                f"{len(unregistered)} uncovered lines (floor {max_lines}), "
                f"{functions_missed} uncovered functions (floor {max_functions}):\n{rendered}"
            )
        if len(unregistered) < max_lines or functions_missed < max_functions:
            raise SystemExit(
                f"{args.workspace} beat its coverage floor "
                f"({len(unregistered)}/{max_lines} lines, {functions_missed}/{max_functions} "
                f"functions): lower it in coverage-floor.tsv so the gain is kept"
            )
        total = len(line_hits) - len(allowed)
        print(
            f"{args.workspace}: ratchet held at {total - len(unregistered)}/{total} lines, "
            f"{functions_hit}/{functions_found} functions (target 100%; W0.41a)"
        )
        return
    if unregistered:
        rendered = "\n".join(f"  {path}:{line}" for path, line in sorted(unregistered))
        raise SystemExit(f"uncovered production lines:\n{rendered}")
    if functions_hit != functions_found:
        raise SystemExit(
            f"function coverage is incomplete: {functions_hit}/{functions_found} functions"
        )

    executable = len(line_hits) - len(allowed)
    covered = sum(hits > 0 for hits in line_hits.values())
    if covered != executable:
        raise SystemExit(f"line coverage is incomplete: {covered}/{executable} executable lines")
    print(
        f"{args.workspace}: 100.00% executable lines ({covered}/{executable}); "
        f"100.00% functions ({functions_hit}/{functions_found}); "
        f"{len(allowed)} hash-pinned unreachable lines"
    )


if __name__ == "__main__":
    main()
