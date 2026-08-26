# Benchmarks

**Not normative** (`W0.2`). `W0.35` and `W0.36` in [`spec/index.md`](spec/index.md)
are what govern benchmarking here, and they say two things that shape this whole
document:

> Benchmarks are **run, never gated on.**

CI runs both suites with criterion's `--test` flag — one iteration, correctness
only — and asserts nothing about wall-clock time. That is deliberate. A timing
threshold on a shared runner fails for reasons that have nothing to do with the
change under test, and a check that fails for unrelated reasons is a check that
gets silenced. So there is no performance gate, and **a number in this file
moving is not a build failure**.

## What is measured

Two criterion suites, eight benchmark groups.

| Suite | Group / benchmark | What it exercises |
| --- | --- | --- |
| [`openehr/benches/rm.rs`](openehr/benches/rm.rs) | `document/deserialize` | parsing a composition from openEHR canonical JSON |
| | `document/canonical_json` | serializing that composition back to canonical JSON |
| | `document/validate` | running the Reference Model's invariants over it |
| | `query/resolve_path` | resolving an openEHR path to a value |
| | `query/parse_aql` | parsing an AQL query |
| | `iso8601/date_time`, `iso8601/date_to_the_month`, `iso8601/duration` | the ISO 8601 parsers, including a partial date |
| [`openehr-store/benches/store.rs`](openehr-store/benches/store.rs) | `project/composition_index` | projecting a composition onto its index rows |
| | `project/version_row` | projecting a version onto its storage row |
| | `integrity/verify_versions/{1,10,100}` | re-verifying stored version digests, at three sizes |

The three `verify_versions` sizes exist to show **shape**, not speed: whether the
integrity check is linear in the number of versions is a property worth knowing,
and it is one a single measurement cannot tell you.

## How to run them

```sh
(cd openehr && cargo bench)
(cd openehr-store && cargo bench)

# the one-iteration form CI runs -- proves the benchmarks still compile and run,
# and measures nothing
(cd openehr && cargo bench -- --test)
(cd openehr-store && cargo bench -- --test)

# one group only
(cd openehr && cargo bench -- iso8601)
```

Criterion writes HTML reports and keeps a local baseline under
`target/criterion/`, so a second run reports change against your own first run.
That baseline is per machine and is not committed; the percentages criterion
prints are meaningful to you, locally, and to nobody else.

## A measurement, with its conditions attached

Recorded once, by hand, so that this file contains something rather than a
promise. **These numbers describe one laptop on one afternoon.** They are not a
performance claim, not a service level, not a comparison, and not a threshold.

| Condition | Value |
| --- | --- |
| Date | 2026-08-26 |
| Machine | Apple M4 Max, 16 cores, macOS 26.6.1 |
| Toolchain | rustc 1.98.0 (`88d9e12ae`), cargo 1.98.0, release profile |
| Commit | the working tree at 0.6.0 |
| Load | an ordinary interactive desktop; nothing was quiesced |

| Benchmark | Median | Criterion's 95% interval |
| --- | --- | --- |
| `document/deserialize` | 11.38 µs | 11.230 – 11.582 µs |
| `document/canonical_json` | 29.81 µs | 29.493 – 30.293 µs |
| `document/validate` | 419 ns | 414.43 – 426.40 ns |
| `query/resolve_path` | 638 ns | 630.54 – 648.47 ns |
| `query/parse_aql` | 581 ns | 574.54 – 590.03 ns |
| `iso8601/date_time` | 115 ns | 113.69 – 116.42 ns |
| `iso8601/date_to_the_month` | 38.6 ns | 37.938 – 39.396 ns |
| `iso8601/duration` | 86.1 ns | 83.431 – 90.029 ns |
| `project/composition_index` | 210 ns | 206.63 – 214.63 ns |
| `project/version_row` | 15.86 µs | 15.634 – 16.175 µs |
| `integrity/verify_versions/1` | 1.67 µs | 1.6474 – 1.6922 µs |
| `integrity/verify_versions/10` | 16.49 µs | 16.272 – 16.798 µs |
| `integrity/verify_versions/100` | 165.14 µs | 162.83 – 168.06 µs |

## What the shape says

- **Integrity verification is linear**, at roughly 1.65 µs per version across
  1, 10, and 100 rows. There is no hidden quadratic in re-verifying a version
  chain, which is the property that actually matters — a per-record constant can
  be paid for, and an accidental `O(n²)` cannot.
- **Serializing costs about 2.6× parsing** (29.8 µs against 11.4 µs on the same
  document). Canonical JSON is deliberately expensive: `serde_json` runs with
  `preserve_order`, `float_roundtrip`, and `arbitrary_precision`, which is what
  keeps `1.50` from becoming `1.5` (`lib:D3.18d`) and what makes the stored bytes
  byte-preserving (`db:M3.43`). This project pays that cost on purpose; `db:D-08`
  is what the alternative looks like.
- **Validation is cheap next to parsing** — 419 ns against 11.4 µs. Running
  `validate()` on everything that arrives as JSON, which `lib:A-23` requires
  because `Deserialize` does not validate, costs about 4% of what parsing the
  document cost. There is no performance argument for skipping it.
- **Projection is the cheap half of a write**; `project/version_row` at ~16 µs is
  dominated by producing canonical JSON, not by row assembly.

## What is not measured, and will not be claimed

- **No end-to-end throughput against a real database server.** Any number
  produced that way would mostly describe the server, the disk, and the
  container runtime.
- **No comparison against other openEHR implementations.** Running someone
  else's system badly and publishing the result is not a benchmark, and this
  project will not do it. If you have benchmarked this against another
  implementation on hardware you control, the tracker is the place for it, and
  a correction is as welcome as a confirmation.
- **No optimisation profile is committed.** There is no flamegraph, no
  `perf`/`samply` capture, and no allocation profile in the tree. Nothing here
  has been optimised against a profile yet; the numbers above are what the
  straightforward implementation costs.

## Trademarks

openEHR® is the registered trademark of the openEHR Foundation. Use of the
trademark does not constitute endorsement of this product by openEHR
International or openEHR Foundation.
