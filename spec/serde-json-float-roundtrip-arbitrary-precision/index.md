# serde_json float_roundtrip arbitrary_precision

When using Rust crate serde_json, use serde_json crate features:

- "float_roundtrip" makes f64 -> JSON -> f64 produce output identical to the input.
- "arbitrary_precision" makes JSON -> serde_json::Number -> JSON produce output identical to the input.

Example in file `Cargo.toml` dependencies:

```toml
serde_json = { version = "…", features = ["float_roundtrip", "arbitrary_precision"] }
```

## What this repository does, and why only half

Requirement prefix: `SJ`.

- **SJ1** Every crate here that depends on `serde_json` MUST enable
  **`float_roundtrip`**. Thirteen manifests; the `msrv` job's sibling check in
  [`../../scripts/check-docs.py`](../../scripts/check-docs.py) is the pattern to
  copy if this ever needs enforcing mechanically.

  It closes [`openehr/spec/audit.md`](../../openehr/spec/audit.md) **A-38**
  outright. Without it, `serde_json` parses `1.5777777777770001` one ULP below
  `core::str::parse` — its parser is not the inverse of its own serializer — and
  a `DV_QUANTITY` magnitude **drifts** across repeated canonical round trips:
  `4.4444444444444444e-7 → …4454e-7 → …446e-7`. With it, the two agree bitwise
  and the canonical form is a fixed point from the first application.

  That finding was reported upstream as
  [serde-rs/json#1336](https://github.com/serde-rs/json/issues/1336) and
  recorded here as open and upstream. It was neither. The feature already
  existed and this repository had not enabled it.

- **SJ2** *(amended 2026-08-22 — was "MUST NOT enable")* Every crate here that
  depends on `serde_json` MUST enable **`arbitrary_precision`**, and the
  Reference Model MUST hold its real numbers as
  [`base::Real`](../../openehr/src/base/real.rs) rather than `f64` (`lib:D3.18d`,
  `lib:D3.18e`).

  The two halves are one change. Enabling the feature alone **breaks the crate**:
  `DATA_VALUE` is `#[serde(tag = "_type")]` and `LocatableAttrs` is
  `#[serde(flatten)]` into every clinical class, both buffer through an
  intermediate representation, and under `arbitrary_precision` a number arrives
  there as a magic map —

  ```text
  Error("invalid type: map, expected f64", line: 0, column: 0)
  ```

  — failing four round-trip tests across data types, data structures, paths and
  validation. A field typed `Real` deserializes through `serde_json::Number`,
  which reads that representation, so migrating the fields removes the breakage
  **and** delivers the benefit. Not two changes: one.

  **This requirement said the opposite for a few hours**, and the reasoning it
  gave was sound and incomplete. It observed the breakage, observed that the
  Reference Model stored magnitudes as `f64` so the preservation could not reach
  them, and concluded the feature was all cost. Both observations were true.
  The conclusion followed only if the `f64` stayed — and the `f64` was the
  defect, not a constraint. Kept here rather than rewritten away, because the
  shape recurs: `lib:A-38` was closed the same day after being filed as
  unfixable-upstream on an equally sound and equally incomplete argument.

  **What it buys.** `1.50 mg` was measured to two decimal places and `1.5 mg` to
  one. They are different records, they now hash differently, and the
  distinction survives from the wire to the digest. `db:D-08` is the same loss
  one layer out — MySQL rewrote a stored `1.10` as `1.1`, changing bytes a
  content digest had been taken over, and `db:M3.43` moved canonical JSON onto a
  byte-preserving column for it. The crate had been discarding the digits before
  storage ever saw them, and `security::canonical`'s own test recorded that as
  the limit of the guarantee. It no longer is.

  **What it does not buy.** `f64` accessors are unchanged — `magnitude()` still
  returns what the number denotes, because a reference range and a comparison
  need that. The text is reached by `magnitude_real()` and its siblings. A
  caller who does not care is unaffected.
