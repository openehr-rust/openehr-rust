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

A worked pass, in the order that has actually found things here:

```sh
# 1. Does it build and test clean?
for d in openehr openehr-store openehr-sqlite openehr-postgresql \
         openehr-mysql openehr-mariadb openehr-mssql openehr-oracle; do
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
