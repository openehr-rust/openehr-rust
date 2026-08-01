# openehr-fuzz

Fuzz targets for the [`openehr`](../openehr) Reference Model parsers.

Not published. This is a test harness.

## Why this crate is the important one

Six `openehr-<engine>-fuzz` crates already fuzz the dialects. A dialect owns
four small functions and only one of them — identifier quoting — takes untrusted
input. **These are the parsers that read documents from outside the process**,
and they are where a malformed input could end a process holding clinical data.

That gap was tracked as `lib:A-09` and was the larger half of `db:T11.9`.

## Targets

| Target | Drives | Property beyond "does not panic" |
| --- | --- | --- |
| `iso8601` | `Date`, `Time`, `DateTime`, `Duration` | **Lexical fidelity** (`lib:D3.10`): a parsed value's `as_str()` is the input, byte for byte. |
| `object_id` | 10 identifier grammars | Round-trip through `Display`. |
| `aql` | `AqlQuery` parse, accessors, `check()` | **Normalisation is idempotent** — rendering and reparsing yields the same text. |
| `path` | `item_at_path` against a real composition | Totality over caller-supplied paths. |
| `canonical_json` | `Composition` deserialize → `validate()` → canonical re-serialize | **Round-trip** (`db:R4.2`), and that validation is total on values that never saw a constructor. |

### `iso8601` — the one with a clinical consequence

openEHR times carry **deliberate partial precision**. `2024-05` is a date known
to the month, and it is not `2024-05-01`. A parser that normalises has destroyed
a clinical distinction before storage ever sees it, so the property is not "it
parsed" but "it came back unchanged".

### `canonical_json` — the widest untrusted surface

A `COMPOSITION` arriving as JSON has been checked by **nothing** until
`validate()` runs: serde writes fields directly and never calls a constructor.
That is the "two gates, not one" rule (`db:V9.8`), and this target drives the
second gate with values no constructor would ever have produced.

Seeded with a real composition emitted by
`openehr/examples/01_build_composition`, because random bytes are not valid
JSON for a `COMPOSITION` and an unseeded target would test the JSON lexer and
nothing else. With the seed it reaches **~4,800 covered edges** against ~650 for
`iso8601` — that difference is the evidence the seed is doing its job.

## On recursion depth

`lib:S1.15` says the crate MUST NOT bound recursion on deserialization, and that
a caller reading untrusted documents has to. That is a **documented design
decision, not a defect**, so these targets do not treat deep nesting as a
finding. `serde_json` applies its own default recursion limit to the input,
which keeps `canonical_json` testing parser logic rather than rediscovering a
stated limitation.

A fuzzer pointed at that would produce a "finding" in seconds and it would be
wrong. Worth saying, because it is exactly the sort of result that looks
impressive and means nothing.

## Running

```sh
cargo install cargo-fuzz

cd ../openehr
cargo +nightly fuzz run --fuzz-dir ../openehr-fuzz canonical_json -- -max_total_time=60
```

Reproduce a crash:

```sh
cargo +nightly fuzz run --fuzz-dir ../openehr-fuzz canonical_json artifacts/canonical_json/<file>
```

## Seed corpus

`corpus/*/seed-*` is committed (`db:T11.9`); inputs the fuzzer discovers are
git-ignored. The seeds are the cases that matter and that random mutation is
slow to reach: partial-precision dates, a leap-day that is not one, every
identifier grammar, four AQL shapes, and a full valid composition.

## Results

No crashes. All five targets run in CI on every push.

## What this does not cover

- **Archetype and template parsing** — not implemented anywhere (`lib:S1.4`).
- **AQL execution** — parsed and statically checked, never executed
  (`lib:S1.5`), so there is no evaluator to fuzz.
- **Unit conversion, terminology resolution** — deliberately absent
  (`lib:S1.9`, `lib:S1.10`).

## Licence

Any of these, at your option — MIT, Apache-2.0, BSD-3-Clause, GPL-2.0-only, or
GPL-3.0-only. See [`LICENSE.md`](../LICENSE.md).
