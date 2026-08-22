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

- **SJ2** This repository MUST NOT enable **`arbitrary_precision`**, and the
  reason is measured rather than assumed.

  It is **incompatible with this crate's serde layout**. `DATA_VALUE` is
  `#[serde(tag = "_type")]` and `LocatableAttrs` is `#[serde(flatten)]` into
  every clinical class; both buffer through an intermediate representation, and
  under `arbitrary_precision` a number arrives there as a magic map. Every
  round-trip test fails with:

  ```text
  Error("invalid type: map, expected f64", line: 0, column: 0)
  ```

  Four of the crate's round-trip tests, across `DATA_VALUE`, the data
  structures, paths, and validation.

  **And its benefit does not reach this crate's values.** What
  `arbitrary_precision` preserves is the literal text of a
  `serde_json::Number` — so `1.50` stops collapsing to `1.5`, which
  `security::canonical`'s own test documents as the limit of the guarantee. But
  the Reference Model stores magnitudes as **`f64` fields**, not as `Number`, so
  a value that passes through `DV_QUANTITY.magnitude` is an `f64` either way and
  the preservation never applies to it.

  So the feature costs the crate its serialization and buys it nothing on the
  path that matters. If preserving a submitted `1.50` ever becomes a
  requirement, the change is to the **Reference Model's number representation**
  and not to a cargo feature — `db:M3.43` already keeps the *stored* bytes
  intact, which is what a content digest is taken over.
