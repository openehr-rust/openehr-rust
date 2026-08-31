#!/usr/bin/env python3
"""Publish openehr-rust.github.io to its sibling repo, per spec/monorepo-github-pages/.

The site lives *in* this monorepo, at openehr-rust.github.io/, and that copy
is what every edit and review targets -- see the "Maintenance" section of
spec/monorepo-github-pages/index.md. But GitHub Pages for an organization
site must be served from a repository literally named <org>.github.io, and
GitHub Actions only discovers .github/workflows/ at a repository's own root
-- a workflow nested under a monorepo subdirectory never runs. So the actual
publish target is a *separate* repository, openehr-rust/openehr-rust.github.io,
holding a rewritten export of just that subdirectory's history. This script
is that export: `git subtree split` extracts openehr-rust.github.io/'s
history as if it were its own repository, and (with --push) force-pushes the
result as the sibling repo's main.

That sibling is derived, not authored: never commit there directly, or the
next run of this script silently discards the commit. Re-run this after
every change worth publishing.

Usage:
    scripts/publish-pages-subtree.py            # split + build-verify, no push
    scripts/publish-pages-subtree.py --push      # split + build-verify + push
    scripts/publish-pages-subtree.py --ref BRANCH --push
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
SITE_PREFIX = "openehr-rust.github.io"
SIBLING_URL = "git@github.com:openehr-rust/openehr-rust.github.io.git"
SIBLING_BRANCH = "main"


def run(cmd: list[str], **kwargs) -> subprocess.CompletedProcess:
    print(f"$ {' '.join(cmd)}")
    return subprocess.run(cmd, check=True, cwd=REPO_ROOT, **kwargs)


def split(ref: str) -> str:
    """git subtree split the site prefix off `ref`; return the new commit sha."""
    tmp_branch = "pages-export-tmp"
    subprocess.run(["git", "branch", "-D", tmp_branch], cwd=REPO_ROOT, capture_output=True)
    run(["git", "subtree", "split", f"--prefix={SITE_PREFIX}", ref, "-b", tmp_branch])
    sha = subprocess.run(
        ["git", "rev-parse", tmp_branch], cwd=REPO_ROOT, capture_output=True, text=True, check=True
    ).stdout.strip()
    print(f"Split commit: {sha}")
    return tmp_branch, sha


def build_verify(tmp_branch: str) -> None:
    """Check the split tree actually builds, in a throwaway worktree."""
    with tempfile.TemporaryDirectory(prefix="pages-export-verify-") as worktree:
        run(["git", "worktree", "add", "--detach", worktree, tmp_branch])
        try:
            print(f"Verifying build in {worktree} ...")
            subprocess.run(["pnpm", "install", "--frozen-lockfile"], cwd=worktree, check=True)
            subprocess.run(["pnpm", "run", "build"], cwd=worktree, check=True)
            print("Build verify: OK")
        finally:
            run(["git", "worktree", "remove", "--force", worktree])


def push(tmp_branch: str) -> None:
    print(f"Force-pushing {tmp_branch} -> {SIBLING_URL}#{SIBLING_BRANCH}")
    run(["git", "push", SIBLING_URL, f"{tmp_branch}:{SIBLING_BRANCH}", "--force"])


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--ref", default="main", help="monorepo ref to export from (default: main)")
    parser.add_argument("--push", action="store_true", help="force-push the export to the sibling repo")
    parser.add_argument("--skip-build-verify", action="store_true", help="skip the throwaway-worktree build check")
    args = parser.parse_args()

    if shutil.which("pnpm") is None and not args.skip_build_verify:
        print("pnpm not on PATH -- pass --skip-build-verify to split without verifying the build.", file=sys.stderr)
        return 1

    tmp_branch, sha = split(args.ref)
    try:
        if not args.skip_build_verify:
            build_verify(tmp_branch)
        if args.push:
            push(tmp_branch)
        else:
            print(
                f"\nDry run only. To publish: "
                f"git push {SIBLING_URL} {tmp_branch}:{SIBLING_BRANCH} --force"
            )
    finally:
        subprocess.run(["git", "branch", "-D", tmp_branch], cwd=REPO_ROOT, capture_output=True)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
