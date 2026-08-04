# Auditing this repository

Not normative. `W0.3`–`W0.4` in [`spec/index.md`](../spec/index.md) are.

## The two rules

1. **Documentation must not claim more than is verified** (`W0.3`).
2. **A gap that is not written down reads as a pass** (`W0.4`).

Every finding in [`spec/audit.md`](../spec/audit.md) is a violation of the first,
surviving because of the second.

## Method

**Run things. Do not read them.** Reading finds inconsistencies between two
documents; running finds the ones where both documents agree and both are wrong —
which is the class that survives.

**This applies to a check you just wrote, too.** A CI check added to verify
`spec/audit.md`'s summary counts itself matched a hand count of 35 findings
against a table of 35 — until it was run against the file it was meant to
check and reported 18. `\s*` in the row-matching regex matched the newline
between two table rows, so a pattern anchored on one row's opening `|` walked
`\s*` straight into the *next* row and silently absorbed it. A check that
passes on the first file you hand it has been run once, which is not evidence
it is correct — feed it a case you know should fail before you trust a case
where it says pass.

A worked pass, in the order that has actually found things here:

```sh
# 1. Does it build and test clean?
for d in openehr openehr-store openehr-sqlite openehr-postgresql \
         openehr-mysql openehr-mariadb openehr-mssql openehr-oracle \
         openehr-loco openehr-assets; do
  (cd "$d" && cargo test --quiet && cargo clippy --all-targets --quiet) \
    || echo "FAIL $d"
done

# 2. Does every cited file exist?
grep -rn 'spec/[a-z0-9-]*\.md' --include='*.rs' --include='*.md' . | grep -v target/
ls .github/workflows            # claimed by a spec; does not exist

# 3. Does every cited command run?
sh openehr-store/scripts/verify-schema.sh mariadb   # once answered "unknown engine"

# 4. Do artefacts that should differ actually differ?
for e in postgresql sqlite mysql mariadb mssql oracle; do
  printf '%-12s ' "$e"
  cargo run -q --manifest-path "openehr-$e/Cargo.toml" --example ddl | md5
done                            # two matching hashes is a copied dialect

# 5. Do the guards cover everything they claim to?
grep -A12 'fn all()' openehr-sqlite/tests/dialects.rs   # count the dialects

# 6. Is the published metadata what the repository says?
curl -s https://crates.io/api/v1/crates/openehr \
  -H 'User-Agent: your-name (your@email)' | python3 -m json.tool | head -20
```

Step 4 is the one that found **W-01**: two engine crates hashing identically.
Step 5 is why it had survived — the guard listed five of six dialects.

## What "verified" means

| Not evidence | Evidence |
| --- | --- |
| "the same code path works elsewhere" | a test that exercises *this* path |
| a golden test asserting emitted SQL | an engine parsing that SQL |
| a test that self-skips without its database | a job that fails without it |
| a level claimed on inherited code | a transcript against this engine |
| "it was verified once" | a check that runs now |

A test that cannot fail verifies nothing. Before citing a test as evidence,
invert the behaviour it checks and confirm it goes red.

## Writing a finding

One heading per finding, in [`spec/audit.md`](../spec/audit.md):

```markdown
## W-NN — one-line summary — **Severity, state**

**Claimed.** Quote the documentation verbatim.

**Found.** What is actually true, with the command that shows it.

**Fixed.** / **Disposition.** What changed, or why it is still open.

**Residual.** What is still not true after the fix.
```

Rules:

- **Quote the claim verbatim.** Paraphrasing loses the thing that was wrong.
- **Give a checkable command or a hash.** "The dialects are identical" is an
  assertion; `40f32f64e5015f8640830a67aecb9c72` twice is evidence.
- **Severity is about the reader's exposure.** A false published claim is High.
  An unverifiable claim is Medium. A wrong URL is Low.
- **Never delete a finding** because the text that stated it was rewritten. Mark
  it fixed and keep the record. Deleting is the failure the register exists to
  prevent.
- **Record the residual.** Most fixes leave something — W-01 is fixed but its
  MariaDB verification is a local run, so W-02 still applies to it.

## Which register

| Register | For |
| --- | --- |
| [`spec/audit.md`](../spec/audit.md) | Findings spanning crates, or above either domain: `W-xx`. |
| [`spec/databases/audit.md`](../spec/databases/audit.md) | Persistence-local: `F-xx`. Currently imported and untrustworthy. |
| [`openehr/spec/audit.md`](../openehr/spec/audit.md) | Library-local: `A-xx`. |

## State the scope you did not cover

An audit that does not say what it skipped reads as one that covered everything.
`spec/audit.md` ends with an explicit list — SQL Server and Oracle DDL unparsed,
no concurrency testing, no fuzzing, the library's own findings not re-verified.

Keep that section honest and current. It is the part a later reader most needs
and the part most likely to rot.

## Mutation testing

```sh
cargo install cargo-mutants --locked
cd openehr && cargo mutants --file src/<module>.rs -j 4
```

Run over one file at a time. Eighty mutants take two minutes; the whole crate
would be hours, which is why **only the diff** is in CI. The `mutants` job in
`.github/workflows/ci.yml` runs `--in-diff` against the pull request's base, so
lines a change touches must be covered by a test that notices them changing.
That job asserts nothing about the rest of the tree, and the `T13.2` row in each
conformance matrix says so — do not cite it as though it did (`W0.3`).

Read the result as a question, not a score. The first run over
`security/audit_chain.rs` missed 40 of 67 viable mutants, and the answer was not
"the tests are bad" — it was that **nothing had ever put a `Chain` through
serde**, so every arithmetic operation in the hex codecs was free to change. Four
tests took it to one survivor (`lib:A-09`).

Two things to know before believing a number:

- **`cargo mutants` runs the tests of the crate it mutates.** `Chain::from_stored`
  survived being replaced with `Default::default()` because its only callers are
  in `openehr-store`. A cross-crate test is not coverage of this crate.
- **Some crates need `--in-place`.** `openehr-sqlite` dev-depends on its five
  sibling engine crates so one test can compare all six dialects (`W-01`), and
  `cargo mutants` copies a crate to a temporary directory where `../openehr-mariadb`
  does not resolve. It reports "cargo build failed in an unmutated tree", which
  reads like the crate is broken. `--in-place` mutates the real tree and
  restores it, and takes no `-j`.
- **Sometimes the answer is to delete.** `AccessLog::path` survived because it
  was a public accessor with no caller and a doc comment describing a use that
  did not exist. A test would have preserved it.
- **Writing the test is what finds the bug.** The mutant on
  `ordered_attrs_of` said one arm was untested. Building a fixture for each of
  the classes it lists showed the list was wrong: the four temporal types
  implement `DvOrdered` and carry `OrderedAttrs`, and none of them was there, so
  a normal range on a `DV_DATE` was unreachable by path against `Q12.7a`
  (`lib:A-29`). The mutant pointed at a line; the fixture is what read it.
- **Test the function, not only its caller.** `days_from_civil` is private and
  its one caller differences two of its results — which cancels every constant
  in it, so `- 719_468` could become `+ 719_468` undetected. Five survivors
  outlived a nine-date table for that reason alone. A `#[cfg(test)]` module in
  the same file can call a private function directly; do that when the caller
  is lossy.
- **An ordering test does not test scale.** Comparing `09:00:00` with
  `09:01:00` cannot tell `m * 60_000` from `m + 60_000`: addition is monotonic,
  so the ordering comes out the same. Only a pair crossing a component boundary
  — `00:02:00` against `00:00:59` — pins the magnitude of a term.
- **Validating on the way in is not testing the way out.** Almost every
  survivor in `rm/common.rs` was an accessor returning a constant. The
  constructors there enforce their invariants thoroughly, which is exactly why
  nobody noticed the getters were unread: a test that builds an object and
  asserts the constructor refused bad input never reads a field back. Round-trip
  what you build.
- **Asserting `None` is half a test.** An optional accessor needs both cases:
  a constant `None` passes every absent assertion. Several fields here have no
  builder at all — `IntervalEvent::state`, `Folder::details`,
  `CareEntryAttrs::guideline_id` — and are reachable only by deserializing
  JSON, so a test that only constructs objects can never reach them.
- **Zero is not a value.** The seconds term of `subtract_seconds` survived a
  fresh table of thirteen dates because every event time in it ended `:00`:
  added to zero, `+` and `-` are indistinguishable. Give each component a
  distinct non-zero value.
- **Checking `None` twice is not two tests.** The first fix for
  `DvMultimedia::encapsulated` asserted only the absent case, which a mutant
  returning a leaked `Default::default()` also satisfies — `EncapsulatedAttrs`
  derives `Default`, and its default is all-`None`. The mutation run caught
  this immediately; trust it over a test that merely compiles and passes.
- **A survivor with no way to write a test is the strongest finding, not a
  blocker.** `EncapsulatedAttrs::charset`/`language` existed and worked; the
  problem was that nothing returned an `EncapsulatedAttrs` to call them on.
  When a mutant resists every test you try, check whether the code under test
  is actually reachable before concluding the test is hard (`lib:A-34`).
- **A boundary-only test cannot tell `<` from `>`.** `open_and_closed_ends_
  differ_at_the_boundary` checked the value equal to an excluded bound, which
  both `value < hi` and `value > hi` reject identically. A value strictly
  *inside* the range is what a strict comparison needs to prove itself against.
- **A survivor can be a proof, not a gap.** `Parser::integer`'s `v >= 0` guard
  could be replaced with `true` and nothing failed — because the lexer starts a
  number only at a digit and never emits a negative one, so the guard is
  unreachable. The finding was not a missing test; it was that **AQL cannot
  express a negative literal at all** (`lib:A-27`). Before writing a test for a
  survivor, check whether the mutant is equivalent, and if it is, ask what the
  code was defending against and whether that thing can happen.
- **A survivor in the safe direction is not always worth chasing.** `Debug for
  Mac` replaced with `Ok(())` prints nothing, which is safer than what it does.
  Pinning exact `Debug` output would freeze formatting for no benefit.
