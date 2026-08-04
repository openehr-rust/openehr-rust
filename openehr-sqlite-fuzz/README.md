# openehr-sqlite-fuzz

Fuzz targets for the **SQLite** dialect of [`openehr-sqlite`](../openehr-sqlite).

Not published. This is a test harness; a crates.io release of it would claim to
be part of the library's surface.

## What is fuzzed

| Target | Property |
| --- | --- |
| `quote` | An identifier cannot escape its own quoting. |
| `col_sql` | Every logical column type maps to something usable. |

### `quote` — the one with a security consequence

SQLite quotes identifiers as `"openehr_version"`, escaping an embedded `"` by
doubling it to `""`.

SQLite also accepts `[...]` and backticks for compatibility, but emits and expects the SQL-standard double quote. Only the form this dialect emits is fuzzed.

This matters because an **archetype id arrives from a caller** and reaches a
`WHERE` clause — it is the query the composition index exists for (`P6.12`). An
identifier that terminates its own delimiter is SQL injection.

The property, stated exactly:

> The quoted form is `"`, then a body, then `"`; every `"` in
> the body occurs as a doubled pair; and undoubling those pairs recovers the
> original identifier.

A dialect satisfying that cannot emit an identifier which ends its own quoting.
The delimiters are **discovered**, not hard-coded — the property asks the dialect
to quote the empty string and reads the first and last characters — so it works
for `"…"`, `` `…` `` and `[…]` alike, and a seventh style would be covered
without editing it.

### `col_sql`

Drives arbitrary `ColTy` values, including arbitrary bounds for `Id(n)` and
`Text(n)`. Asserts the mapping is non-empty, single-line, and that `Instant` and
`InstantUtc` never collapse to one type (`M3.31`) — the distinction the whole
schema turns on.

## The properties are not in this crate

They live in `openehr_store::conformance`, shared by all six fuzz crates. A
target here is a thin call and nothing else.

That is deliberate. Six copies of one assertion is precisely the arrangement
that produced [**W-01**](../spec/audit.md) — `openehr-mariadb` shipped as a
name-substituted copy of `openehr-mysql` and nothing noticed. A fuzz harness
that repeated the mistake it exists to catch would be worse than none.

## Running

Needs a nightly toolchain and `cargo-fuzz`. Run from the **engine crate**, not
from here:

```sh
cargo install cargo-fuzz

cd ../openehr-sqlite
cargo +nightly fuzz run --fuzz-dir ../openehr-sqlite-fuzz quote   -- -max_total_time=60
cargo +nightly fuzz run --fuzz-dir ../openehr-sqlite-fuzz col_sql -- -max_total_time=60
```

A crash writes the offending input to `artifacts/`, which is git-ignored.
Reproduce it with:

```sh
cargo +nightly fuzz run --fuzz-dir ../openehr-sqlite-fuzz quote artifacts/quote/<file>
```

## Seed corpus

`corpus/` is committed (`T11.9`). The seeds are the adversarial cases a fuzzer
would otherwise take a long time to reach: each delimiter alone, each doubled
form, three injection attempts of the shape `a"; DROP TABLE …; --`, a NUL
byte, and an astral-plane character.

Inputs the fuzzer *discovers* are deliberately **not** committed. They are
machine-specific, they grow without bound, and a corpus nobody curated is noise
in every future diff.

## These targets are run, not merely committed

CI runs both on every push. A committed fuzz target nobody executes is a claim
rather than a check, and `T11.9` requires the difference.

They have also been shown to **fail** when they should. `openehr-store` carries
`should_panic` tests defining dialects with exactly the defects these properties
exist to catch — one that escapes nothing, one that collapses the two instant
columns, one that maps a type to nothing — because a check that cannot fail is
indistinguishable from a control that works (`T11.10`).

## Results so far

No crashes. All six dialects quote correctly across millions of executions,
including NUL, astral-plane characters, and every injection attempt in the seed
corpus.

## What is *not* fuzzed here

The real parsers. ISO 8601, `OBJECT_ID`, openEHR paths, AQL, and canonical-JSON
deserialization all accept documents from outside the process, and all live in
the [`openehr`](../openehr) crate — covered separately, by
[`openehr-fuzz`](../openehr-fuzz), not by these six. That was the larger half of
`T11.9` and is tracked as `lib:A-09`; `openehr-fuzz`'s own README has its
targets and results.

A dialect owns four small functions, and only one of them takes untrusted input.
These crates cover that one honestly; they do not make the crate safe against a
malformed document.

## Licence

Any of these, at your option — MIT, Apache-2.0, BSD-3-Clause, GPL-2.0-only, or
GPL-3.0-only. See [`LICENSE.md`](../LICENSE.md).
