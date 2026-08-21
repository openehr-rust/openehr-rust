# openehr-store-fuzz

Fuzz targets for [`openehr-store`](../openehr-store) — the projection and the
integrity check.

Not published. This is a test harness.

## Why this crate exists

Six `openehr-<engine>-fuzz` crates fuzz the dialects, and
[`openehr-fuzz`](../openehr-fuzz) fuzzes the Reference Model parsers. Between
them sat the layer that is neither: `openehr-store`, where a `COMPOSITION`
**becomes rows**, and where `verify_versions` answers *has this record been
tampered with*.

Neither is a parser, and both take input from outside the process. The
projection's input is a composition that arrived as JSON and was therefore
checked by nothing (`db:V9.8`). `verify_versions`'s input is rows read back out
of a database — and its whole subject is a row somebody edited there, so
"untrusted" is not a hypothesis about it but its premise.

## Targets

| Target | Drives | Property beyond "does not panic" |
| --- | --- | --- |
| `project` | `CompositionIndexRow::project` | **Deterministic**, and the derived UTC column is a function of the authoritative text (`db:M3.31`). |
| `integrity` | `verify_versions` | A function of its input, and an empty container reports `Empty` — never a verdict about nothing (`C0.13`). |

### `project` — the two-column instant rule

A stored instant is **two columns**: `…_text` is authoritative and exact, and
`…_utc` is derived and nullable. `2024-05` is a date known to the month and is
*not* `2024-05-01`, so the derived half of that instant is `None` and must stay
`None`.

The property is not "both columns are populated" but that the second is
**re-derivable from the first**. If it ever is not, SQL and Rust disagree about
one record, and which answer a reader gets depends on which one they asked.

### `integrity` — a panic here is the denial of service that matters

Every other panic in this repository loses a request. A panic in the integrity
check loses *the answer a reader needs most*, at exactly the moment they need
it: the record they are asking about is the one that looks wrong.

## Properties live in `openehr-store`

`W0.26`. `conformance::check_projection`, `check_stored_instant`, and
`check_verify_versions` are the properties; the targets here are thin calls.
A property that lives in a harness is a property one harness has.

## Running

```sh
cargo install cargo-fuzz

cd ../openehr-store
cargo +nightly fuzz run --fuzz-dir ../openehr-store-fuzz project -- -max_total_time=60
```

## Seed corpus

`corpus/*/seed-*` is committed (`db:T11.9`); inputs the fuzzer discovers are
git-ignored. `project` is seeded with a real composition, and `integrity` with
one- and two-version histories emitted by `conformance::sample_version` —
because random bytes are never a valid `VERSION` row, and an unseeded target
would exercise `serde_json` and stop (`W0.30`).

## Results

No crashes. Both targets run in CI on every push.

## Licence

Any of these, at your option — MIT, Apache-2.0, BSD-3-Clause, GPL-2.0-only, or
GPL-3.0-only. See [`LICENSE.md`](LICENSE.md).
