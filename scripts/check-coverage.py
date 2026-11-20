#!/usr/bin/env python3
"""Enforce W0.41 against a cargo-llvm-cov LCOV export."""

from __future__ import annotations

import argparse
import hashlib
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
REGISTER = ROOT / "coverage-exclusions.tsv"


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


def main() -> None:
    args = arguments()
    line_hits: dict[tuple[Path, int], int] = {}
    functions_found = 0
    functions_hit = 0
    source: Path | None = None

    for raw in args.lcov.read_text().splitlines():
        if raw.startswith("SF:"):
            source = Path(raw[3:]).resolve()
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
