# Rust minimum supported version: N−3

**Normative.** Requirement prefix: `RV`. RFC 2119 keywords, per
[`index.md`](index.md).

**The minimum supported Rust version (MSRV) of this repository is N−3**, where N
is the current stable release. N is **1.98**, so the MSRV is **1.95**.

This is its own document rather than a section of [`index.md`](index.md) for the
reason [`databases/search-adjuncts.md`](databases/search-adjuncts.md) is: it is
one decision that several places depend on and no section owns — eighteen
manifests, nine READMEs, one `agents/` guide, and a CI job. It is also the only
requirement in this tree whose **correct value changes on a schedule nobody
here controls**, which is a property worth writing down next to the rule rather
than discovering later.

## Why a formula rather than a number

A number is what every other repository writes, and it is what this one wrote:
`rust-version = "1.90"`, in seventeen manifests, dated by nothing. Two things
were wrong with it, and they are the two failure modes this whole specification
tree is about.

**It was not checked, and it was false.** No job ever built this repository on
1.90. Run by hand — `cargo +1.90 check --all-targets --all-features`, per crate,
which is the whole method — six crates passed and **`openehr-loco` failed**: its
own framework, `loco-rs` 1.0.1, requires 1.94. "Requires Rust 1.90+" had been
wrong for that crate since the day it was written, and nothing in the repository
could tell. That is `W0.3` exactly, and the six that did build were luck rather
than verification. See [`audit.md`](audit.md) **W-09**.

**It had no rule behind it, so it could not be wrong.** 1.90 was committed on
2026-08-01 (`9a7ecf6`), when stable was 1.97 — the number was seven releases old
on the day it arrived, and eight by the time anyone looked at it. Nothing could
report that, because there was nothing to report it against: a bare number is
consistent with itself no matter what it says. A formula has the opposite
property. It is re-derived from the outside world on every CI run, so it is
either right or loudly wrong, and never quietly either.

## The policy

- **RV1** The MSRV of every crate in this repository is **N−3**, where **N** is
  the current stable release of Rust and N−3 is three *minor* releases earlier.

  Rust ships stable every six weeks, so N−3 is an eighteen-week window: a user
  who upgrades their toolchain at least once a quarter is never locked out, and
  this repository is never more than four months behind a language feature it
  wants.

- **RV2** Every crate MUST declare that version as `rust-version` in its
  `Cargo.toml`, and all eighteen MUST declare the **same** value. A per-crate
  MSRV would mean the repository has no MSRV: a user building `openehr-sqlite`
  gets `openehr` and `openehr-store` too, so the effective floor is the highest
  of the three whatever the manifests say. One number, or none.

- **RV3** The declared MSRV MUST be **built and tested on that exact toolchain in
  CI**, not merely declared. An MSRV nothing compiles against is a claim, and a
  claim about a toolchain is worth precisely what it is worth about a database
  server (`W0.11`): a crate is at Verified only once CI has run green, and an
  MSRV is true only once a compiler of that version has agreed.

  The `msrv` job in [`.github/workflows/ci.yml`](../.github/workflows/ci.yml)
  derives N from the stable toolchain it just installed, computes N−3, installs
  *that*, and runs `cargo test` for every crate under it.

- **RV4** Every prose statement of the MSRV MUST name the same number as the
  manifests. The grep-able form is:

  ```
  Requires Rust <version>+ (edition 2024).
  ```

  and the same job checks the manifests and the prose together. Nine READMEs and
  [`agents/adding-an-engine.md`](../agents/adding-an-engine.md) carry it. A
  README that names a floor the manifest does not is the `W0.2` failure — a
  descriptive file disagreeing with the thing it describes — and here it is
  mechanically preventable, so it MUST be prevented mechanically.

- **RV5** **The number is expected to go stale, and CI is expected to say so.**
  Within six weeks of every Rust release this repository will fail its `msrv`
  job on a commit that changed nothing about Rust versions.

  That is the cost, it is deliberate, and the alternative is worse. A floor that
  only moves when somebody remembers is the 1.90 situation again, and the whole
  point of `RV1` is that nobody has to remember. The failure is cheap: one
  number, in eighteen manifests and ten documents, changed by the command the
  job's own error message prints.

  A red `msrv` job MUST NOT be worked around by pinning the toolchain, by
  loosening the check to a lower bound, or by marking the job
  `continue-on-error`. Each of those converts a check into a decoration, which
  is the defect class in [`audit.md`](audit.md) that this repository has
  committed most often.

- **RV6** Raising the floor is a **breaking change for a user below it**, so a
  release whose MSRV moved MUST say so in its changelog entry, and MUST NOT be
  published as a patch bump. Cargo will refuse the build with a clear message
  rather than miscompile, so the damage is bounded — but "your dependency
  silently stopped supporting your toolchain" is still a thing a user is
  entitled to read before it happens (`agents/publishing.md`).

- **RV7** A dependency MUST NOT be added whose own MSRV is newer than N−3.
  Its floor becomes this repository's floor regardless of what these manifests
  declare, and the `msrv` job is what discovers it — at which point the choice
  is to drop the dependency or to raise `RV1`, and `RV1` is not raised for the
  convenience of one dependency.

- **RV8** The edition is **2024** and is independent of `RV1`. Edition 2024
  requires 1.85; N−3 has been above that since 1.88 and this policy keeps it
  above. Should Rust ever ship an edition this repository wants before N−3
  supports it, the edition waits — `RV1` governs.

## What this does not cover

Stated so that "not examined" and "examined and sound" stay distinguishable
(`W0.3`):

- **The eight fuzz crates are not built by the `msrv` job.** `cargo fuzz`
  requires a nightly toolchain, so their `rust-version` is declared for
  consistency with `RV2` and is not evidence of anything. Their real floor is
  whatever nightly `cargo-fuzz` needs, which no job here pins.

- **N−3 is verified; nothing between N−3 and N is.** The job builds exactly two
  toolchains, stable and N−3. A break that appears only on 1.96 or 1.97 would
  reach a user before it reached CI. This is judged an acceptable gap — the
  compiler is not that discontinuous — rather than an unnoticed one.

- **The MSRV is verified by compiling, not by the resolver.** `cargo test`
  under N−3 uses the committed `Cargo.lock`, so it proves this dependency graph
  builds on N−3, not that a fresh `cargo update` would. `RV7` is the rule;
  `-Z minimal-versions` and `cargo-msrv` are the tools that would check it, and
  neither is run here.
