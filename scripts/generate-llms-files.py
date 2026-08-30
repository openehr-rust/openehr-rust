#!/usr/bin/env python3
"""Generate (or check) `llms.txt` and `llms.json` at the repository root.

Run from the repository root:

    python3 scripts/generate-llms-files.py           # check: fails if stale
    python3 scripts/generate-llms-files.py --write    # regenerate both files

Exit status 0 when both files match what this script would produce and are
each under the 40 kB budget `spec/llms-json-and-llms-txt/index.md` sets,
1 otherwise.

# What this is

`llms.txt` (see [llmstxt.org](https://llmstxt.org/)) and `llms.json` are a
curated **map** of this repository's most important content, for an LLM tool
to read instead of crawling everything. "Curated" is the operative word: this
is not every file in the repository, the way `index.md` is closer to being —
it is the subset worth a tool's limited context.

# Why generated, not hand-written

Two files stating the same curated list, by hand, is exactly the shape that
has drifted before in this repository (`spec/audit.md` **A-33**, **A-41**,
`db:D-08`, `db:D-09` — a fact stated twice that only one edit remembered to
update). `openehr-assets` exists for the same reason, one layer down: this
script is that pattern applied to two files instead of a build artifact.

**One data structure, `SECTIONS` below, is the only place the curated list is
written.** Everything else — which crates are published, what each one's
short description is — is read from the tree (`Cargo.toml`), the same
guard-as-wide-as-its-input-list discipline `check-docs.py` uses for crate
counts, so a ninth published crate cannot appear in `README.md`'s table and
stay silently absent here.

# Why no conformance levels

An earlier draft of this script put each crate's conformance level (Dialect /
Schema / Store / Verified) inline. `spec/databases/conformance-matrix.md` is
that fact's **one owner** (`db:W0.40`), restated in nine or ten places and
checked against the owner by `check-docs.py`'s `check_levels` — which globs
`*.md` and does not see a `.txt` file. Restating the level here would be an
eleventh, silently unchecked place for it to drift, for a fact this script's
own purpose (a map to *more detail*, not a restatement of it) does not need
inline. Both files link to the conformance matrices instead.
"""

from __future__ import annotations

import json
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SIZE_BUDGET = 40 * 1024  # spec/llms-json-and-llms-txt/index.md

REPO_URL = "https://github.com/openehr-rust/openehr-rust"
BLOB = f"{REPO_URL}/blob/main"

# The verbatim notice `scripts/check-trademarks.py` enforces
# (`spec/professionalization/index.md` rule 5) — kept as one literal here
# rather than imported, because the two scripts check different file sets for
# the same reason `ParseError::new` and `ParseError::invariant` are two
# functions rather than one with a flag: importing across `scripts/` files
# for one shared constant is more coupling than a notice this stable is worth.
NOTICE = (
    "openEHR® is the registered trademark of the openEHR Foundation and is "
    "used with the permission of openEHR International. Use of the "
    "trademark does not constitute endorsement of this product by openEHR "
    "International or openEHR Foundation."
)
INDEPENDENCE = (
    "This project is not affiliated with, endorsed by, or certified by "
    "openEHR International or the openEHR Foundation."
)

SUMMARY = (
    "openEHR® in Rust: the Reference Model, and persistence for six SQL "
    "engines, specification-driven and with an audited claims register."
)


def published_crates() -> list[dict[str, str]]:
    """Every publishable crate, name and short description, from its own
    `Cargo.toml` — not hand-copied, so it cannot go stale independently of
    `README.md`'s own crate table (`check-docs.py`'s `crates=`/`published=`
    counts check the same source of truth)."""
    out = []
    for manifest in sorted(ROOT.glob("openehr*/Cargo.toml")):
        data = tomllib.loads(manifest.read_text())
        package = data.get("package", {})
        if package.get("publish") is False:
            continue
        name = package["name"]
        description = package.get("description", "")
        # The short description is everything before the trademark notice —
        # see `check-trademarks.py`'s `check_descriptions` for the shape this
        # relies on: `<short description>. <notice> <trailer>`.
        short = description.split(f". {NOTICE}", 1)[0].strip()
        if not short or short == description:
            sys.exit(
                f"::error file={manifest.relative_to(ROOT)}::description does "
                "not have the `<short>. <notice>` shape generate-llms-files.py "
                "expects — has check-trademarks.py's shape changed?"
            )
        out.append({
            "title": name,
            "url": f"{BLOB}/{name}",
            "description": short,
        })
    return out


def link(title: str, path: str, description: str) -> dict[str, str]:
    return {"title": title, "url": f"{BLOB}/{path}", "description": description}


def external(title: str, url: str, description: str) -> dict[str, str]:
    return {"title": title, "url": url, "description": description}


def build_sections() -> list[dict[str, object]]:
    """The one curated list. Add a document here, in the section it belongs
    in — this is the only edit adding one requires."""
    return [
        {
            "title": "Docs",
            "links": [
                link("README", "README.md",
                     "what this is, the crates, install, and six runnable tutorials"),
                link("Documentation index", "index.md",
                     "every entry point in the repository, routed by what you are doing"),
                link("PHI.md", "PHI.md",
                     "what the software does with patient data, in plain language, "
                     "for a privacy or clinical-safety reader"),
                link("INSTALL.md", "INSTALL.md", "installing and building from source"),
                link("COMPARISONS.md", "COMPARISONS.md",
                     "how this differs from EHRbase, FerroEHR, Archie, and the "
                     "commercial platforms"),
                link("SECURITY.md", "SECURITY.md",
                     "reporting a vulnerability, and this project's own posture gaps"),
            ],
        },
        {
            "title": "Specification",
            "links": [
                link("Specification root", "spec/index.md",
                     "crate map, identifier namespaces, the conformance ladder"),
                link("Library specification", "openehr/spec/index.md",
                     "the Reference Model, `lib:` requirement ids, sections 0-15"),
                link("Database specification", "spec/databases/index.md",
                     "persistence, `db:` requirement ids, sections 0-16"),
                link("Conformance matrix - library", "openehr/spec/conformance-matrix.md",
                     "what the Reference Model crate actually satisfies today"),
                link("Conformance matrix - databases",
                     "spec/databases/conformance-matrix.md",
                     "per-engine status; the single owner of every crate's "
                     "conformance level"),
                link("Library audit register", "openehr/spec/audit.md",
                     "what has been found wrong in the Reference Model crate, "
                     "with evidence"),
                link("Database audit register", "spec/databases/audit.md",
                     "what has been found wrong in persistence, with evidence"),
            ],
        },
        {
            "title": "Crates",
            "links": published_crates(),
        },
        {
            "title": "Examples",
            "links": [
                link("Reference Model tutorials", "openehr/examples/",
                     "five runnable examples: build, validate, paths and AQL, "
                     "versioning, redaction"),
                link("Persistence tutorial",
                     "openehr-sqlite/examples/01_store_a_record.rs",
                     "store and read a record end to end - in `openehr-sqlite`, "
                     "the only crate with a working `Store`"),
            ],
        },
        {
            "title": "Contributing",
            "links": [
                link("AGENTS.md", "AGENTS.md", "how to work here - the operational guide"),
                link("Topic guides", "agents/index.md",
                     "engines, auditing, conformance, publishing, openEHR concepts"),
                link("openehr-skill", "openehr-skill/SKILL.md",
                     "openEHR concepts and vocabulary, as a Claude Code Skill"),
                link("openehr-rust-maintainer-skill",
                     "openehr-rust-maintainer-skill/SKILL.md",
                     "this repository's own engineering conventions, as a "
                     "Claude Code Skill"),
                link("CONTRIBUTING.md", "CONTRIBUTING.md", "ways to help, and the claims rule"),
                link("CODE_OF_CONDUCT.md", "CODE_OF_CONDUCT.md",
                     "conduct, including the claim-accuracy clause"),
            ],
        },
        {
            "title": "Optional",
            "links": [
                link("GOVERNANCE.md", "GOVERNANCE.md", "who decides, on what basis"),
                link("MAINTAINERS.md", "MAINTAINERS.md", "one maintainer, stated plainly"),
                link("CHANGELOG.md", "CHANGELOG.md",
                     "what changed in each release, and why it was that version number"),
                link("NEWS.md", "NEWS.md", "the short form of the changelog, and what is coming"),
                link("TRADEMARKS.md", "TRADEMARKS.md",
                     "the openEHR mark, what is and is not claimed"),
                link("LICENSE.md", "LICENSE.md", "five licences, your choice of any one"),
                link("BENCHMARKS.md", "BENCHMARKS.md",
                     "what is measured, and why nothing here is gated on timing"),
                link("AI_STATEMENT.md", "AI_STATEMENT.md",
                     "how this code was written, and what that does and does not prove"),
                link("RFC.md", "RFC.md",
                     "what this project does not know, and the feedback that would change it"),
                external("openehr-rust.github.io", "https://openehr-rust.github.io",
                          "the project's landing page"),
            ],
        },
    ]


def render_txt(sections: list[dict[str, object]]) -> str:
    lines = [
        "# openehr-rust",
        "",
        f"> {SUMMARY}",
        "",
        NOTICE,
        "",
        INDEPENDENCE,
    ]
    for section in sections:
        lines.append("")
        lines.append(f"## {section['title']}")
        lines.append("")
        for entry in section["links"]:
            lines.append(f"- [{entry['title']}]({entry['url']}): {entry['description']}")
    lines.append("")
    return "\n".join(lines)


def render_json(sections: list[dict[str, object]]) -> str:
    doc = {
        "name": "openehr-rust",
        "repository": f"{REPO_URL}/",
        "homepage": "https://openehr-rust.github.io",
        "summary": SUMMARY,
        "trademark_notice": NOTICE,
        "independence": INDEPENDENCE,
        "sections": sections,
    }
    return json.dumps(doc, indent=2, ensure_ascii=False) + "\n"


def main() -> int:
    write = "--write" in sys.argv
    sections = build_sections()
    outputs = {
        ROOT / "llms.txt": render_txt(sections),
        ROOT / "llms.json": render_json(sections),
    }

    bad = 0
    for path, content in outputs.items():
        size = len(content.encode("utf-8"))
        if size > SIZE_BUDGET:
            print(
                f"::error file={path.relative_to(ROOT)}::{size} bytes, over "
                f"the {SIZE_BUDGET}-byte budget "
                "(spec/llms-json-and-llms-txt/index.md)"
            )
            bad = 1
        if write:
            path.write_text(content, encoding="utf-8")
            print(f"wrote {path.relative_to(ROOT)} ({size} bytes)")
        elif not path.exists():
            print(f"::error file={path.relative_to(ROOT)}::does not exist; "
                  "run `python3 scripts/generate-llms-files.py --write`")
            bad = 1
        elif path.read_text(encoding="utf-8") != content:
            print(f"::error file={path.relative_to(ROOT)}::stale; run "
                  "`python3 scripts/generate-llms-files.py --write`")
            bad = 1
        else:
            print(f"{path.relative_to(ROOT)} is current ({size} bytes)")

    return bad


if __name__ == "__main__":
    sys.exit(main())
