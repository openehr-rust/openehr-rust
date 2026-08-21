# Changelog

Covers the eight published crates as a set: `openehr`, `openehr-store`,
`openehr-sqlite`, `openehr-postgresql`, `openehr-mysql`, `openehr-mariadb`,
`openehr-mssql`, `openehr-oracle`. They are versioned in lockstep and released
together.

## 0.4.0 — 2026-08-21

**Breaking.** Two of the three items below change an API; the third raises the
minimum toolchain. Every affected line in a dependent is a **compile error**,
never a silent change in behaviour — which is the property that made the
`PartialOrd` removal safe to do at all.


- **MSRV raised from 1.90 to 1.95, and it is now a rule rather than a number:
  N−3, three Rust releases behind stable**
  ([`spec/rust-msrv-n-minus-3.md`](spec/rust-msrv-n-minus-3.md)).

  Raising a floor is breaking for a user below it (`RV6`), so it is recorded
  here rather than left to be discovered by a build error. Cargo refuses with a
  clear message rather than miscompiling, so the damage is bounded.

  1.90 was never verified: no job had ever compiled this repository with a 1.90
  toolchain, and the claim was **false** for `openehr-loco`, whose framework
  requires 1.94. CI now derives N−3 from the stable toolchain it installs and
  builds and tests every crate on it (`spec/audit.md` **W-09**).

- **A runnable tutorial for the persistence layer**:
  `openehr-sqlite/examples/01_store_a_record.rs`, run by CI on every push. The
  five existing tutorials build and check documents in memory; this is the other
  half — install, commit, amend, read the history, resolve a point-in-time read,
  query the archetype index, watch a stale predecessor be refused, print a
  tamper-evidence checkpoint, and watch the database's own trigger refuse a raw
  `UPDATE` that went around the `Store`.

- **Criterion benchmarks** in `openehr` and `openehr-store`. A number from them
  is not a conformance claim and nothing is gated on wall-clock (`W0.34`,
  `W0.35`); CI runs them with `--test`, one iteration, so they cannot rot
  (`W0.36`).

- **`scripts/check-docs.py`**, run by the `claims` job: derives the crate count,
  the published version, the fuzz-target and tutorial counts, the CI job list,
  and every crate's conformance level from the tree, and fails when a document
  disagrees. Duplicated passages are bound to one owner with
  `<!-- shared: NAME (owner) -->` markers and compared byte for byte (`W0.38`).
  Three findings were drift of exactly this kind (**W-10**, **W-11**, **W-16**).

- **AQL string literals are no longer corrupted, and rendering no longer changes
  what a query asks** (`lib:A-37`, `lib:Q12.15`, `lib:Q12.15a`, `lib:Q12.15b`).

  The lexer copied a string literal one UTF-8 **byte** at a time, so `'Müller'`
  became `'MÃ¼ller'` and a `WHERE` against it matched nobody — the query parsed,
  checked clean, and was about a different string. Separately, the `FROM`
  renderer omitted parentheses its own grammar needs, so
  `(EHR e CONTAINS COMPOSITION c) OR EHR x` rendered as text that re-parsed to
  `EHR e CONTAINS (COMPOSITION c OR EHR x)` — a query over different records.
  Rendering also now escapes `'` and `\` in string literals.

  Not a breaking API change; a behaviour fix. Code that round-tripped a query
  through `to_string()` was getting a different query back, and now is not.

- **Known limitation, upstream: `serde_json` reads back a number it did not
  write** (`lib:A-38`). Its float parser is one ULP below `core::str::parse`
  for some inputs, so a magnitude **drifts** across repeated canonical-JSON
  round trips — three applications before it settled in the observed case, with
  no bound established. Reported upstream as
  [serde-rs/json#1336](https://github.com/serde-rs/json/issues/1336). **Stored bytes, and the
  content digest over them, are unaffected** — `db:M3.43` stores canonical JSON
  byte-preserving and the integrity check hashes the stored bytes rather than
  re-deriving them, so no false tamper alarm is reachable. Recorded rather than
  worked around; the fix is upstream.

- **The `agents/` directory is lowercase** (`AG1`, `spec/agents-directory-name-is-lowercase.md`).
  `AGENTS/` became `agents/`; the file `AGENTS.md` keeps its name, which is a
  cross-tool convention. Affects nobody depending on these crates.

- **BREAKING: no `DV_ORDERED` implements `PartialOrd` any more, and neither
  does `DATA_VALUE`** (`lib:D3.18b`, closing `lib:A-35`). Comparison is
  `DvOrdered::semantic_cmp`; `INTERVAL<T>` is bounded on the new
  `openehr::base::SemanticOrd` rather than on `PartialOrd` (`lib:D3.18c`).

  All ten types derived `PartialEq` over every field — including the
  `OrderedAttrs` each carries, and `DV_QUANTITY`'s `precision` and
  `units_display_name` — while comparing only the magnitude. So
  `5 mg precision 1` was `!=` to `5 mg precision 2` while `<=` and `>=` were
  both true of it, which is exactly what Rust's `PartialOrd` contract forbids.
  Invisible inside this crate; a wrong answer inside a caller's `binary_search`
  or `dedup_by`.

  **Migrating**: `a < b` becomes `a.semantic_cmp(&b) == Some(Ordering::Less)`,
  `a.partial_cmp(&b)` becomes `a.semantic_cmp(&b)`, and `DvOrdered` must be in
  scope. Every affected line is a compile error, never a silent change — the
  four crates in this repository that depend on `openehr` needed no edits at
  all. **No behaviour changed**: the comparison logic is the same logic, reached
  through a different name.

- **`DV_URI` and `DV_EHR_URI` are validated, and reading one no longer panics**
  (`lib:A-36`). A `DV_URI` deserialized from `{"value":"nocolon"}` panicked in
  `scheme()`; a `DV_EHR_URI` deserialized from `{"value":"https://…"}` reported
  scheme `https`, which is what that type exists to make impossible.

  `DvUri::scheme()` and `rest()` now return `""` where there is no scheme, and
  `validate()` reports `DV_URI.Value_valid`, `DV_URI.Uri_well_formed`, and
  `DV_EHR_URI.Scheme_valid` — including on `LINK.target`, on every `LOCATABLE`,
  which was validated nowhere. **Behaviour change for callers:** a document that
  previously validated clean and carried a malformed or foreign-scheme URI now
  reports violations, and code matching on `scheme()` for a value built by
  `Deserialize` gets `""` instead of a panic.

## 0.3.0

**Breaking.** No migration path exists or is planned before 1.0
(`db:O10.14`). A deployment on 0.2.0 exports its data, upgrades, recreates the
schema, and reloads.

- **`SCHEMA_VERSION` now exists, and is `4`.** A database written by 0.2.0
  records no schema version at all. `Store::install()` now refuses to open a
  *populated* database that has none, rather than guessing which schema it
  is (`db:O10.16`). A fresh, empty database installs normally.
- **`ColTy::Json` moved off `jsonb` (PostgreSQL) and `JSON` (MySQL) onto a
  byte-preserving text type** (`db:M3.43`, `db:D-08`). Both prior types
  normalise on the way in — reordering object keys, and on MySQL rewriting a
  magnitude of `1.10` as `1.1` — which changes the bytes a content digest was
  taken over. A database created under 0.2.0 has columns of the old type and
  already-normalised content; it cannot be upgraded in place.
- **Nine columns added to `openehr_version`**, carrying the tamper-evident
  hash chain (`db:D-07`). Absent from the 0.2.0 schema.
- **`ColTy::Digest` added.** `ColTy` is deliberately not `#[non_exhaustive]`,
  so any `Dialect` implementation outside this repository fails to compile
  against 0.3.0 until it handles the new variant — intentional, not an
  oversight.
- **`OriginalVersion::new` refuses input it previously accepted**
  (`lib:A-23`): a first version naming a preceding version, or a
  non-first version naming none, is now a construction error rather than a
  silently inconsistent value.
- **`Date`, `Time`, `DateTime`, and `Duration` no longer implement
  `PartialOrd`/`Ord`** (`openehr::base::iso8601`; `lib:A-32`). `Eq` on these
  types is lexical — two values are equal only when written the same way —
  while chronological (or, for `Duration`, length) order compares what the
  value denotes, and Rust requires `PartialOrd` to agree with `Eq` wherever
  it is implemented. It cannot, for either of these types, without giving up
  something load-bearing (record identity for `Eq`, or the query the ordering
  exists for), so the trait impl is gone rather than left contradicting
  itself. Callers using `<`, `<=`, `.partial_cmp()`, `.min()`, `.max()`, or
  `sort()` directly on these four types call the new inherent method
  `.semantic_cmp(&self, other: &Self) -> Option<core::cmp::Ordering>`
  instead. **This does not affect** `DvDate`, `DvTime`, `DvDateTime`, or
  `DvDuration` in `openehr::rm::data_types` — their own `PartialOrd` impls are
  unchanged and still work with `<` and friends; only the four bare ISO 8601
  types lost the trait.

**Fixed**, not breaking:

- A normal range on a `DV_DATE`, `DV_TIME`, `DV_DATE_TIME`, or `DV_DURATION`
  was silently unreachable by path and never contributed to `is_abnormal()`
  — the four temporal types were missing from the internal list of classes
  carrying `DV_ORDERED` attributes, despite implementing it (`lib:A-29`).
  They now behave as documented; no signature changed.

See [`spec/audit.md`](spec/audit.md) and [`openehr/spec/audit.md`](openehr/spec/audit.md)
for the full findings this release closes, and
[`agents/publishing.md`](agents/publishing.md) for the publishing process
itself.

## 0.2.0 and earlier

Not tracked here. See the git history and crates.io.
