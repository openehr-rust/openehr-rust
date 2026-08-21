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
| `uri` | `DvUri` and `DvEhrUri`, through **both** gates | **The gates agree**: a value the constructor accepts validates clean, and one it rejects is reported. |
| `data_value` | all 22 `DATA_VALUE` variants, deserialized and validated | Canonical form is a **fixed point** — not merely round-tripping — because a digest is taken over those bytes. |

### `iso8601` — the one with a clinical consequence

openEHR times carry **deliberate partial precision**. `2024-05` is a date known
to the month, and it is not `2024-05-01`. A parser that normalises has destroyed
a clinical distinction before storage ever sees it, so the property is not "it
parsed" but "it came back unchanged".

### `uri` — written before the fix, not after

`lib:A-36`: `DvUri::scheme()` `expect`ed a colon that "the constructor
guarantees", and `Deserialize` is derived and calls no constructor.
`{"value":"nocolon"}` deserialized cleanly and panicked on the next line. Worse
and quieter, a `DV_EHR_URI` — the type that exists so that a `LINK` cannot point
out of the record without saying so — deserialized happily from
`{"value":"https://example.org/x"}` and reported scheme `https`.

This target was written against the broken code and found the panic **from an
empty corpus**, in one run: the minimised crash input is the empty string. It is
kept pointed at the property rather than the panic, which is why it asserts that
the two gates *agree* rather than that neither crashes — a crash is one way for
them to disagree and not the interesting one.

### `data_value` — the `_ => {}` arm

`impl Validate for DataValue` is a match over 22 variants ending in `_ => {}`. A
variant with nothing to check and a variant nobody wrote a check for are the
same program text. That arm is how `A-36` survived, and it is the same shape as
the navigation-table hazard in `CLAUDE.md`, where deleting a match arm makes a
path silently resolve to nothing.

A fuzzer cannot tell a deliberate `_` from a forgotten one — worth saying,
because this target is easy to mistake for a check that it can. What it drives
is the properties that hold whatever the arm does: totality, that no violation
carries content (`lib:L10.5`), and that canonical form is a **fixed point**.

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
identifier grammar, four AQL shapes, a full valid composition, all 22
`DATA_VALUE` variants, and URIs on both sides of the constructor's answer.

**The corpus is checked** (`W0.30`). `openehr/tests/fuzz_seeds.rs` asserts that
each structured target's seeds still span both answers — at least one instance
the deserializer accepts, so the target gets past the lexer, and at least one it
refuses, so the error paths are reached by something other than mutation. A seed
that quietly stopped parsing is a file contributing nothing, and no fuzz run's
output would say so.

## Results

All seven targets run in CI on every push, and **two of them have found
defects** — which is the outcome a fuzz target exists for, and is worth
recording as prominently as a green run.

- `uri` crashed against the code as it stood when the target was written:
  `lib:A-36`, a panic reachable from any JSON document.
- `aql` crashed on 2026-08-04, **in CI, on `main`**, and the job stayed red for
  seventeen days while this file said "no crashes". It had found two ways an AQL
  query silently changes meaning — a lexer that mangled UTF-8 in string
  literals, and a renderer that dropped the parentheses its own grammar needs.
  `lib:A-37`.

The second is the one to learn from. The target worked; the gap was between the
job failing and anyone reading it.

## What this does not cover

- **Archetype and template parsing** — not implemented anywhere (`lib:S1.4`).
- **AQL execution** — parsed and statically checked, never executed
  (`lib:S1.5`), so there is no evaluator to fuzz.
- **Unit conversion, terminology resolution** — deliberately absent
  (`lib:S1.9`, `lib:S1.10`).

## Licence

Any of these, at your option — MIT, Apache-2.0, BSD-3-Clause, GPL-2.0-only, or
GPL-3.0-only. See [`LICENSE.md`](../LICENSE.md).
