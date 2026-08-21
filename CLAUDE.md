# CLAUDE.md

Guidance for Claude Code working in this repository.

Read [`AGENTS.md`](AGENTS.md) for the full operational guide and
[`agents/`](agents/index.md) for topic guides. This file is the short version
plus the things that specifically trip up automated work here.

**Not normative.** The specifications decide what must be true (`W0.2`):
[`spec/index.md`](spec/index.md), [`spec/databases/`](spec/databases/index.md),
[`openehr/spec/`](openehr/spec/index.md).

## Layout

Eighteen crates, **each its own Cargo workspace**. There is no root workspace —
run cargo from inside a crate directory. Eight are published at 0.4.0; the other
ten are `publish = false`.

**Local matches published: 0.4.0, out 2026-08-21.** That release was breaking —
`PartialOrd` was removed from every `DV_ORDERED` and from `DataValue`
(`lib:A-35`), the MSRV moved to N−3, and AQL string literals stopped being
mangled (`lib:A-37`) — which is why it was not 0.3.1. Read
[`agents/publishing.md`](agents/publishing.md) before touching a version
number; it is the only file that tracks this, and four others state the version
without tracking it (`spec/audit.md` **W-10**).

| Crate | Role | Level |
| --- | --- | --- |
| `openehr` | RM types, validation, paths, AQL, security | library |
| `openehr-store` | schema, projection, commit rules, conformance suite | library |
| `openehr-sqlite` | SQLite dialect **and a store** | **Verified** |
| `openehr-postgresql` | PostgreSQL 18 dialect | **Schema** |
| `openehr-mysql` | MySQL 8.4 dialect | **Schema** |
| `openehr-mariadb` | MariaDB 11.4 dialect | **Schema** |
| `openehr-mssql` | SQL Server dialect | **Dialect** |
| `openehr-oracle` | Oracle dialect | **Dialect** |
| `openehr-loco` | HTTP service: Axum, Loco, PASETO verification | not published |
| `openehr-assets` | regenerates `assets/`; fails on a stale one | not published |
| `openehr-fuzz` | fuzz harness for the RM parsers | not published |
| `openehr-<engine>-fuzz` × 6 | dialect fuzz harnesses | not published |

`openehr-loco` is outside the conformance ladder: every rung there is defined by
DDL, a `Store`, or a database server (`W0.32`). It states evidence instead.

## Commands

```sh
cd <crate> && cargo test
# `RUSTFLAGS="-D warnings"` is what CI sets. Without it a lint that fires only
# under `-D warnings` passes locally and fails in CI -- which is how
# `openehr-loco` went red on `clippy::unused_async_trait_impl` after a local
# run reported clean.
cd <crate> && RUSTFLAGS="-D warnings" cargo clippy --all-targets

# everything
for d in openehr openehr-store openehr-sqlite openehr-postgresql \
         openehr-mysql openehr-mariadb openehr-mssql openehr-oracle \
         openehr-loco openehr-assets; do
  (cd "$d" && cargo test --quiet \
     && RUSTFLAGS="-D warnings" cargo clippy --all-targets --quiet) \
    || echo "FAIL $d"
done

# regenerate the committed assets, or fail if what is committed is stale
(cd openehr-assets && cargo run -- write)   # or: -- check

# the documentation's counts, versions, levels, and shared blocks
python3 scripts/check-docs.py               # or: --fix

# benchmarks (criterion). `--test` is the one-iteration form CI runs.
(cd openehr && cargo bench)
(cd openehr-store && cargo bench -- --test)

# verify a dialect against a real engine (needs podman or docker)
sh openehr-store/scripts/verify-schema.sh postgresql|mysql|mariadb
```

Lints are `deny` (`missing_docs`, `missing_errors_doc`, `missing_panics_doc`),
`unsafe_code` is `forbid`, clippy runs at `pedantic`. **The tree is at zero
warnings — keep it there.**

## The five rules

1. **Never claim more than is verified** (`W0.3`). "The same code path works
   elsewhere" is not evidence. Every finding in
   [`spec/audit.md`](spec/audit.md) is a version of this.
2. **A gap not written down reads as a pass** (`W0.4`). Found something wrong and
   can't fix it now? Add it to the audit register.
3. **A guard is only as wide as its input list.** The cross-dialect check
   compared five of six dialects, and the sixth was a copy of another.
4. **Never create a crate by copying a sibling.** That is exactly how
   `openehr-mariadb` became `openehr-mysql` under another name.
5. **Specification first** (`W0.19`). Write the requirement down before the
   commit lands.

## Things that will trip you up

- **`spec/databases/` was rewritten on 2026-08-01** from an imported FHIR
  specification. Withdrawn requirements keep their numbers in a table at the foot
  of each section — a citation to `M3.4` resolves to "withdrawn: shredding", not
  to nothing. New requirements start after the highest previously used ordinal,
  so §3 begins at `M3.19`. Do not renumber.
- **Two spec trees allocate the same identifiers.** `lib:S1.4` (no Archetype
  Model) and `db:S1.4` (declare an engine floor) are different requirements.
  Qualify citations with `lib:` or `db:` (`W0.5`).
- **A published version is immutable, and one here is permanently wrong.**
  `openehr` 0.1.0 went out with a `repository` pointing at an unrelated project.
  0.1.1 and 0.2.0 fixed it; 0.1.0 still says it. Read
  [`agents/publishing.md`](agents/publishing.md) before any publish.
- **Read the last CI run before believing any claim about CI.** `gh run list`.
  This file said "CI is green" while the `fuzz / openehr` job had been red on
  `main` for seventeen days, and the bug it had found was a `WHERE` clause
  silently matching nobody (`lib:A-37`). A red job on the default branch is as
  visible as a signal gets, and it was still missed.
- **`openehr-sqlite` is at Verified.** Every other crate is at
  Schema or Dialect and must not be promoted without evidence. Do not write text
  implying more continuity than a job actually provides; a specification here
  once claimed a workflow file that never existed.
- **`ColTy` is deliberately not `#[non_exhaustive]`.** Adding a variant *should*
  break all six dialects at compile time. Do not add a `_` arm to silence it —
  that is how one engine silently acquires another's types.
- **`Cargo.lock` is committed** in every crate, unusually for libraries. Leave it.
- **Fuzz properties live in `openehr_store::conformance`,** not in the fuzz
  crates. A target is a thin call. Do not inline an assertion into six crates.
- **Each engine crate has a dialect annex** at `spec/14-<engine>-dialect.md`.
  A departure from a core requirement goes there as a numbered `M14.x`, not in a
  code comment — `C0.16` calls an undeclared departure a defect.
- **Rustdoc examples are compiled and run.** Do not add `no_run` or `ignore` to
  make one pass; that turns a checked claim into an unchecked one.
- **Times are two columns.** `…_text` is authoritative and exact; `…_utc` is
  derived and nullable. Never collapse them, and never make the derived column
  non-nullable — `2024-05` is a date known to the month, not `2024-05-01`.
- **Canonical JSON must be stored in a byte-preserving type** (`db:M3.43`). Not
  `jsonb`, not MySQL's `JSON`. Both reorder keys, and MySQL rewrote a magnitude
  of `1.10` as `1.1` — a clinical precision loss, independent of any digest.
  `db:D-08` measured it. `conformance::check_dialect` refuses those two
  spellings, and `verify-schema.sh` round-trips the bytes through a real server.
- **A constructor validates and `Deserialize` does not.** `OriginalVersion::new`
  and `Ehr::new` check invariants; the derived `Deserialize` writes fields
  straight in. Anything arriving as JSON must be run through `validate()`, and
  the store does (`lib:A-23`). Adding a rule to a constructor alone leaves the
  path an HTTP service takes unchecked.

  **So no accessor may rely on a constructor's guarantee.** `DvUri::scheme()`
  read `.expect("constructor guarantees a scheme")`, with rustdoc saying
  "# Panics — Never". `{"value":"nocolon"}` deserialized cleanly and panicked
  (`lib:A-36`, `lib:D3.30a`). If a type derives `Deserialize`, every method on it
  must be total for *any* field values, and `validate()` is what reports the bad
  ones. Check the `_ => {}` arm in `Validate for DataValue` when you add a
  variant: that arm is how a whole class of values reached no check at all.
- **Nothing may index a column an engine cannot search.** A schema test refuses
  an index over `LongText` or `Json`; searching one needs the adjuncts of
  [`spec/databases/search-adjuncts.md`](spec/databases/search-adjuncts.md), and
  none is emitted anywhere (`db:P6.18`).
- **`openehr` and `openehr-store` depend inward only.** A CI job reads the
  manifests, dev-dependencies included — a probe once added `openehr-store` as a
  dev-dependency of `openehr`, a cycle, and everything still built and passed.
- **The Gregorian leap rule lives in exactly one place.** `iso8601::days_in_month`
  is `pub(crate)` for that reason. It was copied into `rm::data_structures` once
  already, and the copy — identical but for its fallback arm — was never run by
  any test (`lib:A-33`). A calendar rule fixed in one of two copies is a rule
  that disagrees with itself, which is `W-01` one level down.
- **A path that resolves to nothing is not an error.** `Node::children` answers
  an attribute a class does not have with an empty vector, deliberately, so a
  wrong attribute is `NoMatch`. The consequence is that **deleting a match arm
  from the navigation table is silent** — the path stops resolving and an AQL
  query returns no rows, which reads as "no such record". Fifty such arms had no
  test (`lib:A-28`). Add an attribute to `path.rs`, add a row to
  `every_navigable_attribute_of_a_*_node_reaches_its_value`.
- **AQL here cannot parse a negative number.** `WHERE o/value/magnitude > -2.5`
  is refused at the lexer, because `-` also separates the parts of an archetype
  id. Declared as `Q12.9b`, open as `lib:A-27`. Do not "fix" it by adding a sign
  to the number scanner without deciding what `[openEHR-EHR-…]` means after an
  operator.
- **The documentation's counts and versions are checked** (`W0.39`).
  `python3 scripts/check-docs.py` derives the crate count, the published
  version, the fuzz-target count, the tutorial count, the CI job list, and every
  crate's conformance level from the tree, and fails when a document disagrees.
  Run it after anything that changes those. `--fix` rewrites the marked shared
  blocks from their owner.
- **A duplicated passage is either marked or forbidden** (`W0.38`). The
  conformance ladder appears in four documents; one is the owner and three carry
  `<!-- shared: conformance-ladder (copy) -->`, checked byte for byte. Do not
  add a fifth unmarked copy — two of the original four had already drifted
  (`W-16`).
- **A conformance level has one owner:**
  [`spec/databases/conformance-matrix.md`](spec/databases/conformance-matrix.md)
  (`W0.40`). Promote a crate there first; every README, rustdoc header, and
  index table is checked against it.
- **Benchmarks are run, never gated on** (`W0.35`, `W0.36`). `cargo bench` in
  `openehr` and `openehr-store`; CI runs them with `--test`, one iteration, and
  asserts nothing about wall-clock. Do not add a timing threshold — on a shared
  runner it fails for unrelated reasons and gets silenced.
- **The MSRV is a formula, not a number** (`RV1`): N−3, currently 1.95. The
  `msrv` job re-derives it from the stable toolchain, so it goes red within six
  weeks of every Rust release. That is deliberate; fix the number, do not pin
  the toolchain ([`spec/rust-msrv-n-minus-3.md`](spec/rust-msrv-n-minus-3.md)).
- **No `DV_ORDERED` implements `PartialOrd`, and neither does `DataValue`**
  (`lib:D3.18b`). `a < b` and `a.partial_cmp(&b)` do not compile on `DvQuantity`,
  `DvDateTime`, `DvCount`, … — call `DvOrdered::semantic_cmp`, which needs the
  trait in scope. `INTERVAL<T>` is bounded on `openehr::base::SemanticOrd`
  (`lib:D3.18c`), which has **no blanket impl**: a new type reaching
  `Interval<T>` needs an explicit one, deliberately.

  The reason is that these types derive `PartialEq` over every field — including
  the `OrderedAttrs` all of them carry — while comparing only the magnitude, so
  `5 mg precision 1` is `!=` to `5 mg precision 2` and orders `Equal`. Do not
  "fix" that by making `==` semantic: a canonicaliser that rewrote `1.10` as
  `1.1` would then pass its own round-trip test, which is `db:D-08` again
  (`lib:A-35`).
- **Every RM invariant is accounted for.** `openehr-assets` fails the build if
  one is neither cited by the crate nor dispositioned with a reason, and also if
  a disposition outlives the rule it explains (`lib:A-24`). Cite an invariant as
  `("CLASS", "Invariant_name")` **literals** at the call site, or the scanner
  cannot see it (`lib:A-25`).

## Before saying something works

Run it. Every finding in this repository's audit register was found by running
something; none by reading. In particular:

```sh
# two dialects hashing the same means one is a copy of the other
for e in postgresql sqlite mysql mariadb mssql oracle; do
  printf '%-12s ' "$e"
  cargo run -q --manifest-path "openehr-$e/Cargo.toml" --example ddl | md5
done
```

## Style

Match the surrounding code, which is unusually heavily commented and deliberately
so: comments explain *why* a decision was made, often citing a requirement id or
naming the defect the decision prevents. When you change such code, update the
reasoning rather than deleting it. A requirement whose reason is unrecorded is one
that will be removed by someone who does not know what it was protecting.
