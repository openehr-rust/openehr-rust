#!/usr/bin/env python3
"""Check that the documentation's countable claims match the tree.

Run from the repository root:

    python3 scripts/check-docs.py

Exit status 0 when every claim agrees with what is on disk, 1 otherwise. CI
runs this same script rather than a re-implementation in YAML, for the reason
`.github/workflows/ci.yml` gives about `verify-schema.sh`: two ways of doing one
check drift, and the one that drifts is the one nobody runs.

# What this exists to prevent

`spec/audit.md` **W-10** and **W-11**: the published version was stated in five
files and updated in one, and `db:W16.1` said the repository held fourteen
crates while five other documents said seventeen. Both were counts — the
cheapest possible claim to check, and the easiest to leave behind.

The residual recorded against **W-11** was exactly this script's absence: the
CI jobs derive the *crate list* from the tree, so a crate cannot hide from a
guard, but the *documents* were still hand-counted and the next stale count
would have gone stale silently.

# How it works, and what it deliberately does not do

Every check is a **fixed-form phrase with the number punched out**. The script
finds `<number> crates, **each its own Cargo workspace**` wherever it occurs and
asserts the number is the one on disk. It does not try to understand prose, and
it does not scan for bare numerals: "six dialect crates", "the eight published
crates", and "the ten buildable crates" are all correct and all different, and a
checker that could not tell them apart would be noise.

The cost is that a *new* phrasing is invisible to this script until someone adds
it below. That is stated rather than hidden: this catches a number that went
stale in a sentence somebody already wrote, which is the failure that has
actually happened here, twice.
"""

from __future__ import annotations

import pathlib
import re
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parent.parent

WORDS = {
    1: "one", 2: "two", 3: "three", 4: "four", 5: "five", 6: "six",
    7: "seven", 8: "eight", 9: "nine", 10: "ten", 11: "eleven",
    12: "twelve", 13: "thirteen", 14: "fourteen", 15: "fifteen",
    16: "sixteen", 17: "seventeen", 18: "eighteen", 19: "nineteen",
    20: "twenty", 21: "twenty-one", 22: "twenty-two",
}
# A number word or a numeral, so a document may spell it either way. Longest
# first, so that `twenty-one` is not matched as `one` -- and every pattern is
# applied case-insensitively, because a count at the start of a sentence is
# capitalised and that is not a different claim.
NUMBER = r"(?:\d+|" + "|".join(sorted(WORDS.values(), key=len, reverse=True)) + r")"

# Registers of history. A finding that quotes the count a file *used* to carry
# is doing its job, and rewriting it would destroy the finding (`W0.6`).
# The three audit registers and the changelog. `spec/rust-msrv-n-minus-3.md` is
# deliberately NOT here: its historical passage quotes a *version*, not a count,
# so its counts are live claims and are checked like anyone else's.
HISTORICAL = {
    "spec/audit.md",
    "spec/databases/audit.md",
    "openehr/spec/audit.md",
    "CHANGELOG.md",
}


def facts() -> dict[str, int]:
    """Everything countable, counted from the tree rather than from prose."""
    manifests = sorted(ROOT.glob("*/Cargo.toml"))
    crates = [m.parent.name for m in manifests]
    published = [
        m.parent.name
        for m in manifests
        if tomllib.loads(m.read_text()).get("package", {}).get("publish") is not False
    ]
    fuzz_crates = [c for c in crates if c.endswith("-fuzz")]
    return {
        "crates": len(crates),
        "published": len(published),
        "unpublished": len(crates) - len(published),
        "dialects": len([c for c in crates if c.startswith("openehr-") and (ROOT / c / "examples/ddl.rs").is_file()]),
        "fuzz_crates": len(fuzz_crates),
        "fuzz_targets": sum(len(list((ROOT / c / "fuzz_targets").glob("*.rs"))) for c in fuzz_crates),
        "tutorials": len(list((ROOT / "openehr/examples").glob("[0-9]*.rs"))),
        "bench_crates": len([c for c in crates if (ROOT / c / "benches").is_dir()]),
    }


# (fact, regex with exactly one capturing group around the number, description)
#
# Each pattern must match the number and enough surrounding words that it cannot
# match a different, correct count elsewhere.
PATTERNS: list[tuple[str, str, str]] = [
    ("crates", rf"({NUMBER}) crates, \*\*each its own Cargo workspace\*\*", "the crate count"),
    ("crates", rf"({NUMBER}) crates, each its own Cargo workspace", "the crate count"),
    ("crates", rf"The repository holds \*\*({NUMBER})\*\* crates", "the crate count"),
    ("crates", rf"({NUMBER}) crates implementing openEHR in Rust", "the crate count"),
    ("crates", rf"declared identically by all ({NUMBER}) crates", "the crate count"),
    ("crates", rf"all ({NUMBER}) MUST declare the \*\*same\*\* value", "the crate count"),
    ("crates", rf"\*\*all ({NUMBER})\*\* crates declare the same five licences", "the crate count"),
    ("unpublished", rf"the other ({NUMBER}) are `publish = false`", "the unpublished count"),
    ("unpublished", rf"The other ({NUMBER}) MUST declare `publish = false`", "the unpublished count"),
    ("unpublished", rf"the other ({NUMBER}) — `openehr-loco`", "the unpublished count"),
    ("fuzz_crates", rf"({NUMBER}) fuzz harnesses", "the fuzz-crate count"),
    ("fuzz_crates", rf"({NUMBER}) fuzz crates are `publish = false`", "the fuzz-crate count"),
    ("fuzz_crates", rf"({NUMBER}) fuzz crates are not built by the `msrv` job", "the fuzz-crate count"),
    ("fuzz_targets", rf"({NUMBER}) targets in total", "the fuzz-target count"),
    ("fuzz_targets", rf"({NUMBER}) targets across", "the fuzz-target count"),
    ("fuzz_targets", rf"({NUMBER}) targets in all, committed seed corpora", "the fuzz-target count"),
    ("tutorials", rf"the ({NUMBER}) runnable tutorials", "the tutorial count"),
]


def documents(include_historical: bool = False) -> list[tuple[str, str]]:
    """Every Markdown file that is part of this repository, read once.

    `target/` is pruned during the walk rather than filtered after it: a
    `cargo doc` tree holds thousands of dependency READMEs, and walking them
    took this script from under a second to over two minutes.
    """
    out = []
    for path in sorted(ROOT.rglob("*.md")):
        rel = path.relative_to(ROOT)
        if "target" in rel.parts or ".git" in rel.parts:
            continue
        posix = rel.as_posix()
        if not include_historical and posix in HISTORICAL:
            continue
        out.append((posix, path.read_text(errors="replace")))
    return out


def versions() -> tuple[str, str]:
    """`(live, local)` — what is on crates.io, and what the manifests say.

    Both are owned by `agents/publishing.md`. A published version is immutable
    and is **not** derivable from the tree: `Cargo.toml` says what is here, not
    what is on crates.io.

    The two are equal most of the time and deliberately differ between a version
    bump and a release. This function reads both because the first draft of this
    check assumed they were always equal, and reported eight errors the moment
    0.4.0 was staged — a checker that cannot express "prepared but not yet
    published" forces you to either lie in the manifests or switch it off, and
    both are worse than the state it was complaining about.
    """
    text = (ROOT / "agents/publishing.md").read_text()
    m = re.search(r"are live at \*\*([0-9.]+)\*\* on crates\.io", text)
    if not m:
        sys.exit(
            "::error file=agents/publishing.md::the fixed-form 'are live at "
            "**X.Y.Z** on crates.io' sentence is gone; it is what every other "
            "document's version is checked against. Restore it, or update this "
            "script and that sentence together."
        )
    live = m.group(1)
    staged = re.search(r"Local is ([0-9.]+) and NOT yet published", text)
    return live, (staged.group(1) if staged else live)


def check_versions() -> int:
    """Every restatement of the published version matches the one file that owns it."""
    want, local = versions()
    bad = 0
    for rel, text in documents():
        if rel == "agents/publishing.md":
            continue
        for pattern in (
            r"live on crates\.io at \*\*([0-9.]+)\*\*",
            r"on crates\.io at ([0-9.]+)",
            r"[Aa]ll eight are on crates\.io at \*\*([0-9.]+)\*\*",
            r"[Ee]ight are published at ([0-9.]+)",
        ):
            for m in re.finditer(pattern, text):
                if m.group(1) != want:
                    line = text[: m.start()].count("\n") + 1
                    print(
                        f"::error file={rel},line={line}::the published version is "
                        f"{want} (agents/publishing.md), this says {m.group(1)}"
                    )
                    bad = 1
    # And the tree itself: every publishable manifest carries the SAME version,
    # and it is the one the release table calls local. A mismatch between two
    # siblings is the one `agents/publishing.md` warns about -- a dependency
    # pinned to a version that is not the one beside it resolves to a published
    # crate rather than the local path, so the workspace silently tests
    # something other than what it ships.
    for manifest in sorted(ROOT.glob("*/Cargo.toml")):
        raw = tomllib.loads(manifest.read_text())
        pkg = raw.get("package", {})
        if pkg.get("publish") is False:
            continue
        rel = manifest.relative_to(ROOT)
        if pkg.get("version") != local:
            print(
                f"::error file={rel}::version is {pkg.get('version')}, "
                f"agents/publishing.md says local is {local}"
            )
            bad = 1
        for section in ("dependencies", "dev-dependencies", "build-dependencies"):
            for name, spec in (raw.get(section) or {}).items():
                if not name.startswith("openehr") or not isinstance(spec, dict):
                    continue
                if "version" in spec and spec["version"] != local:
                    print(
                        f"::error file={rel}::depends on {name} "
                        f"{spec['version']}, but local is {local}"
                    )
                    bad = 1
    if not bad:
        state = (
            f"live {want} everywhere"
            if want == local
            else f"live {want}, local {local} staged for release"
        )
        print(f"versions agree: {state}, manifests and inter-crate pins included")
    return bad


def check_ci_jobs() -> int:
    """The documented job list is the job list.

    `spec/audit.md` **W-02** is the finding that this repository once described
    a CI workflow that did not exist. The inverse -- a job that exists and is
    described nowhere -- is quieter and just as misleading to whoever is
    deciding whether a claim is covered.
    """
    workflow = (ROOT / ".github/workflows/ci.yml").read_text()
    # Top-level keys under `jobs:` -- two-space indent, no deeper.
    body = workflow.split("\njobs:\n", 1)[1]
    jobs = set(re.findall(r"^  ([a-z][a-z0-9_-]*):$", body, re.MULTILINE))
    bad = 0
    for doc in ("AGENTS.md", "spec/audit.md"):
        text = (ROOT / doc).read_text()
        # Only the table headed `| Job | Covers |`. Both files carry other
        # tables whose first column is a backticked name -- crates, in AGENTS.md
        # -- and a scan that could not tell them apart would report every crate
        # as a job that does not exist.
        tables = re.findall(
            r"^\| Job \| Covers \|\n(?:\|[^\n]*\n)+", text, re.MULTILINE
        )
        if not tables:
            print(f"::error file={doc}::no `| Job | Covers |` table found")
            bad = 1
            continue
        documented = set(
            re.findall(r"^\| `([a-z][a-z0-9_-]*)` \|", "".join(tables), re.MULTILINE)
        )
        missing = sorted(jobs - documented)
        extra = sorted(documented - jobs)
        if missing:
            print(f"::error file={doc}::CI jobs with no row: {', '.join(missing)}")
            bad = 1
        if extra:
            print(f"::error file={doc}::rows for jobs that do not exist: {', '.join(extra)}")
            bad = 1
    if not bad:
        print(f"all {len(jobs)} CI jobs have a row in AGENTS.md and spec/audit.md")
    return bad


# --------------------------------------------------------------------------
# Shared blocks: one owner, many copies, checked byte for byte
# --------------------------------------------------------------------------
#
# `W0.1` says a normative statement must exist in exactly one place, "because
# where two files state one rule, one of them is a copy, and a copy is a future
# divergence". The conformance ladder was stated in **four**: `C0.8`, which owns
# it; `spec/index.md` `W0.8`, which says the ladder is defined in `C0.8` and
# then reproduces it; `openehr-store/spec/conformance.md`; and
# `agents/conformance.md`.
#
# Deleting three copies would be the literal reading of `W0.1` and would make
# every one of those documents worse -- a reader deciding whether a crate's
# claim is honest should not have to open another file to see what the levels
# mean. So the copies stay and are made *mechanical*: one block is the owner,
# the rest are marked as copies, and this refuses to pass when they diverge.
#
#     <!-- shared: NAME (owner) -->      ... <!-- /shared: NAME -->
#     <!-- shared: NAME (copy) -->       ... <!-- /shared: NAME -->
#
# `--fix` rewrites the copies from the owner.

BLOCK = re.compile(
    r"^(?P<indent>[ \t]*)<!-- shared: (?P<name>[a-z0-9-]+) \((?P<role>owner|copy)\) -->\n"
    r"(?P<body>.*?)"
    r"^[ \t]*<!-- /shared: (?P=name) -->$",
    re.MULTILINE | re.DOTALL,
)


def dedent(body: str) -> str:
    lines = [ln for ln in body.splitlines() if ln.strip()]
    if not lines:
        return body
    pad = min(len(ln) - len(ln.lstrip()) for ln in lines)
    return "\n".join(ln[pad:] if ln.strip() else "" for ln in body.splitlines())


def check_shared_blocks(fix: bool = False) -> int:
    owners: dict[str, tuple[str, str]] = {}
    copies: list[tuple[str, str, re.Match[str]]] = []
    for rel, text in documents(include_historical=True):
        for m in BLOCK.finditer(text):
            name, role = m.group("name"), m.group("role")
            if role == "owner":
                if name in owners:
                    print(f"::error file={rel}::block {name!r} has two owners "
                          f"({owners[name][0]} is the other); exactly one may own it")
                    return 1
                owners[name] = (rel, dedent(m.group("body")).rstrip())
            else:
                copies.append((rel, name, m))

    bad = 0
    for name, (owner_file, _) in sorted(owners.items()):
        if not any(c[1] == name for c in copies):
            print(f"::warning::block {name!r} is owned by {owner_file} and copied nowhere")

    edits: dict[str, str] = {}
    for rel, name, m in copies:
        if name not in owners:
            print(f"::error file={rel}::block {name!r} is marked a copy, but nothing owns it")
            bad = 1
            continue
        owner_file, want = owners[name]
        got = dedent(m.group("body")).rstrip()
        if got == want:
            continue
        if fix:
            indent = m.group("indent")
            body = "\n".join(indent + ln if ln.strip() else "" for ln in want.splitlines())
            text = edits.get(rel) or dict(documents(include_historical=True))[rel]
            edits[rel] = text.replace(m.group(0), m.group(0).replace(m.group("body"), body + "\n"), 1)
            print(f"fixed {rel}: block {name!r} rewritten from {owner_file}")
        else:
            print(f"::error file={rel}::block {name!r} differs from its owner "
                  f"{owner_file}; run `python3 scripts/check-docs.py --fix`")
            bad = 1

    for rel, text in edits.items():
        (ROOT / rel).write_text(text)

    if not bad and not edits:
        print(f"{len(copies)} shared blocks match their {len(owners)} owners")
    return bad


# --------------------------------------------------------------------------
# Conformance levels: one owner, every restatement checked
# --------------------------------------------------------------------------

LEVELS_OWNER = "spec/databases/conformance-matrix.md"


def owned_levels() -> dict[str, str]:
    text = (ROOT / LEVELS_OWNER).read_text()
    section = text.split("## Conformance levels", 1)
    if len(section) < 2:
        sys.exit(f"::error file={LEVELS_OWNER}::no `## Conformance levels` section")
    rows = re.findall(
        r"^\| `(openehr-[a-z]+)` \| \*\*(Dialect|Schema|Store|Verified)\*\* \|",
        section[1], re.MULTILINE,
    )
    if not rows:
        sys.exit(f"::error file={LEVELS_OWNER}::the levels table has no readable rows")
    return dict(rows)


def check_levels() -> int:
    """No document may give a crate a level the matrix does not.

    `C0.9` puts the level in the first screenful of every README and every
    crate's rustdoc, and `spec/index.md`, `README.md`, `CLAUDE.md`, `AGENTS.md`
    and two more restate it in tables of their own. That is nine or ten
    statements of one fact, and `W0.9` -- a crate MUST NOT claim a level it has
    not earned -- is enforced against all of them or against none.
    """
    want = owned_levels()
    bad = 0
    sources: list[tuple[str, str]] = list(documents(include_historical=False))
    # Crate rustdoc carries the claim too, and is what a docs.rs reader sees.
    for lib in sorted(ROOT.glob("openehr-*/src/lib.rs")):
        sources.append((lib.relative_to(ROOT).as_posix(), lib.read_text()))

    for rel, text in sources:
        if rel == LEVELS_OWNER:
            continue
        # A table row naming a crate and a level.
        for m in re.finditer(
            r"\| \[?`(openehr-[a-z]+)`\]?[^|]*\|[^|]*?\*\*(Dialect|Schema|Store|Verified)\*\*",
            text,
        ):
            crate, said = m.group(1), m.group(2)
            if crate in want and said != want[crate]:
                line = text[: m.start()].count("\n") + 1
                print(f"::error file={rel},line={line}::{crate} is at "
                      f"{want[crate]} ({LEVELS_OWNER}), this says {said}")
                bad = 1
        # `# Conformance level: **X**` in a crate's own README or rustdoc.
        if "/" in rel:
            crate = rel.split("/", 1)[0]
            if crate in want:
                for m in re.finditer(r"Conformance level: \*{0,2}(\w+)", text):
                    if m.group(1) != want[crate]:
                        line = text[: m.start()].count("\n") + 1
                        print(f"::error file={rel},line={line}::{crate} is at "
                              f"{want[crate]} ({LEVELS_OWNER}), this says {m.group(1)}")
                        bad = 1
    if not bad:
        print("every conformance level restated in the tree matches "
              + LEVELS_OWNER.rsplit("/", 1)[-1])
    return bad


# --------------------------------------------------------------------------
# The repository audit register counts itself
# --------------------------------------------------------------------------


def check_audit_summary() -> int:
    """`spec/audit.md`'s headline paragraph agrees with its own headings.

    The library register already had this check, in CI, because its summary said
    "Seventeen findings" while the table beneath listed thirty-five (`lib:A-26`).
    The repository register had the same shape of claim and no check at all --
    its findings are `## W-NN — title — **Severity, status**` headings rather
    than table rows, which is a difference in formatting and not in how quietly
    a hand count goes wrong.
    """
    path = ROOT / "spec/audit.md"
    text = path.read_text()
    heads = re.findall(
        r"^## (W-\d+) — .*? — \*\*(High|Medium|Low), ([^*]+)\*\*$",
        text, re.MULTILINE,
    )
    if not heads:
        print("::error file=spec/audit.md::no `## W-NN — … — **Severity, status**` headings found")
        return 1
    by_sev: dict[str, list[str]] = {"High": [], "Medium": [], "Low": []}
    fixed = 0
    for ident, severity, status in heads:
        by_sev[severity].append(ident)
        # "partially fixed" deliberately does not count. `W-03` is fixed going
        # forward and never will be for the published `openehr` 0.1.0, whose
        # `repository` field is immutable and wrong; folding it in with the rest
        # would round an unfixable defect up to a fixed one.
        if status.strip() == "fixed":
            fixed += 1

    flat = re.sub(r"\s+", " ", text)
    words = {v: k for k, v in WORDS.items()}
    claimed = re.search(
        r"(\w+) findings: (\w+) High \(([^)]*)\), (\w+) Medium \(([^)]*)\), "
        r"(\w+) Low \(([^)]*)\)\.\s*\*\*(\w+) are\s*fixed\*\*",
        flat,
    )
    if not claimed:
        print(
            "::error file=spec/audit.md::the fixed-form summary sentence was not "
            "found — did its wording change? Update the sentence and this check "
            "together, rather than deleting one of them."
        )
        return 1
    total_w, high_w, high_ids, med_w, med_ids, low_w, low_ids, fixed_w = claimed.groups()

    def num(word: str) -> int | None:
        return int(word) if word.isdigit() else words.get(word.lower())

    bad = 0
    for label, said, actual in (
        ("total", num(total_w), len(heads)),
        ("High", num(high_w), len(by_sev["High"])),
        ("Medium", num(med_w), len(by_sev["Medium"])),
        ("Low", num(low_w), len(by_sev["Low"])),
        ("fixed", num(fixed_w), fixed),
    ):
        if said != actual:
            print(f"::error file=spec/audit.md::{label}: the paragraph says {said}, "
                  f"the headings say {actual}")
            bad = 1
    # The parenthesised lists must name exactly the findings at that severity.
    for severity, listed in (("High", high_ids), ("Medium", med_ids), ("Low", low_ids)):
        named = set(re.findall(r"W-\d+", listed))
        have = set(by_sev[severity])
        if named != have:
            print(f"::error file=spec/audit.md::{severity}: paragraph names "
                  f"{sorted(named)}, headings say {sorted(have)}")
            bad = 1
    if not bad:
        print(f"spec/audit.md: {len(heads)} findings, {fixed} fixed — the paragraph agrees")
    return bad


# --------------------------------------------------------------------------
# Agent-facing files stay small enough to be read whole
# --------------------------------------------------------------------------

AGENT_FILE_LIMIT = 40 * 1024


def check_agent_file_sizes() -> int:
    """`CLAUDE.md`, `AGENTS.md`, and `agents/*.md` stay under 40 kB each.

    These are the files an agent loads before it knows what the task is, so
    their cost is paid on every session whether or not they turn out to be
    relevant. A guide that grows past skimming length stops being read, and an
    unread guide is worse than a missing one -- it looks like coverage.

    40 kB is roughly ten thousand tokens: large enough for a topic guide with
    its reasoning intact, small enough that four of them still leave room to
    work. When one approaches the limit, the answer is to move a topic into its
    own guide under `agents/`, not to delete the reasoning -- `W0.2` makes these
    descriptive, and the description that survives trimming is usually the
    *what*, while the *why* is the half that was worth keeping.
    """
    bad = 0
    # `AGENTS.md` is a file and keeps its uppercase name; `agents/` is a
    # directory and does not (`AG1`, `AG2`). The directory is asserted to exist
    # rather than globbed hopefully: this line read `ROOT / "AGENTS"` after the
    # rename, which still resolves on a case-insensitive filesystem and finds
    # **nothing** on the Linux runner -- so the check would have quietly fallen
    # to two files and reported success. `AG4`.
    guides = ROOT / "agents"
    if not guides.is_dir():
        print(f"::error::{guides.relative_to(ROOT)}/ does not exist; this check "
              f"would silently cover only the two root files")
        return 1
    files = [ROOT / "CLAUDE.md", ROOT / "AGENTS.md", *sorted(guides.glob("*.md"))]
    if len(files) < 3:
        print(f"::error::only {len(files)} agent-facing files found; expected the "
              f"two root files plus the guides in {guides.relative_to(ROOT)}/")
        return 1
    largest = 0
    for f in files:
        size = len(f.read_bytes())
        largest = max(largest, size)
        if size > AGENT_FILE_LIMIT:
            print(
                f"::error file={f.relative_to(ROOT)}::{size} bytes, over the "
                f"{AGENT_FILE_LIMIT}-byte budget for agent-facing files; "
                f"split a topic into its own agents/ guide"
            )
            bad = 1
    if not bad:
        print(
            f"{len(files)} agent-facing files, largest {largest} bytes "
            f"({largest * 100 // AGENT_FILE_LIMIT}% of the 40 kB budget)"
        )
    return bad


def main() -> int:
    fix = "--fix" in sys.argv
    f = facts()
    bad = (
        check_shared_blocks(fix)
        | check_levels()
        | check_versions()
        | check_ci_jobs()
        | check_audit_summary()
        | check_agent_file_sizes()
    )
    checked = 0
    seen: dict[str, int] = {pattern: 0 for _, pattern, _ in PATTERNS}

    for rel, text in documents():
        for fact, pattern, what in PATTERNS:
            want = f[fact]
            for m in re.finditer(pattern, text, re.IGNORECASE):
                checked += 1
                seen[pattern] += 1
                said = m.group(m.lastindex)
                got = int(said) if said.isdigit() else {v: k for k, v in WORDS.items()}.get(said.lower())
                if got != want:
                    line = text[: m.start()].count("\n") + 1
                    print(
                        f"::error file={rel},line={line}::{what} is {want} "
                        f"({WORDS.get(want, want)}), this says {said}: {m.group(0)!r}"
                    )
                    bad = 1

    # A pattern nobody matches describes a sentence that has been rewritten. It
    # is not an error -- prose is allowed to change -- but a silently dead check
    # is how this file would stop being worth running, so it is reported.
    for _, pattern, what in PATTERNS:
        if seen[pattern] == 0:
            print(f"::warning::no document matches {pattern!r} ({what}); the phrase moved or went away")

    if bad:
        print("::error::documentation disagrees with the tree; the tree is right")
        return 1

    print(
        f"{checked} count claims agree with the tree: "
        + ", ".join(f"{k}={v}" for k, v in sorted(f.items()))
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
