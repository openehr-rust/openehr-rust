# Changelog

Covers the eight published crates as a set: `openehr`, `openehr-store`,
`openehr-sqlite`, `openehr-postgresql`, `openehr-mysql`, `openehr-mariadb`,
`openehr-mssql`, `openehr-oracle`. They are versioned in lockstep and released
together.

## Unreleased

**`#![forbid(unsafe_code)]` at every crate root and every fuzz target** — 32
files, added 2026-08-26. The ten buildable crates already forbade it through
`[lints.rust]` in their manifests; the attribute states the same guarantee in
the source, where removing it is a visible edit to the file it protects rather
than a line in a manifest.

**The eight fuzz crates were not covered before this.** They carried no
`[lints]` table, so `unsafe_code` was not forbidden in any of the 21 fuzz
targets, while this repository's documentation said the tree forbids it. No
`unsafe` was present — `grep -rn '\bunsafe\b' --include='*.rs'` finds one hit,
in a comment explaining why a test cannot drive `App::before_run` — so the claim
was true of the code and false of the configuration.

Both halves are now in place in all eighteen crates: `unsafe_code = "forbid"` in
every manifest, covering files not yet written, and the attribute in every
existing root and target, surviving a manifest edit. Verified with
`cargo fuzz build` across all eight fuzz crates and `cargo test` plus
`RUSTFLAGS="-D warnings" cargo clippy --all-targets` across the ten buildable
ones.

**Scope change, specification only — no code changed.** The Archetype Model is
now in scope. `S1.4` — *the crate MUST NOT implement the Archetype Model* — was
withdrawn on 2026-08-26 under the new `C0.19`, and `S1.21` plus
[`openehr/spec/15-archetypes.md`](openehr/spec/15-archetypes.md) require AOM2,
ADL 2 parsing, ADL 1.4 ingestion, specialisation and flattening, template
expansion, operational templates, validation of Reference Model data against an
operational template, and a repository abstraction for retrieval.

**Added: `openehr::am`, the AOM2 object model** (`K15.1`–`K15.4`). `Archetype`,
`CComplexObject`, `CAttribute`, `CObject`, `CPrimitive`, `ArchetypeSlot`,
`CArchetypeRoot`, `MultiplicityInterval`, `Cardinality`,
`ArchetypeTerminology`, and `TermDefinition`, with construction-time checking of
the AOM2 validity conditions that one artefact decides on its own — `VARDT`,
`VATDF`, `VACDF`, `VATCD`, `VOKU` — and `Archetype::check` to re-run them on
anything that arrived as JSON, because `Deserialize` writes fields straight in
(`L10.1a`). `am::AM_RELEASE` names the targeted release, 2.3.0.

A primitive constraint this crate cannot model becomes
`CPrimitive::Unsupported` and survives a round trip rather than being dropped:
a dropped constraint silently widens an archetype, which is the failure the
withdrawn `S1.4` predicted.

**Twenty-eight of the thirty-two requirements remain unimplemented**, and the
practically important one is among them: **this crate still cannot tell you
whether a `COMPOSITION` conforms to its archetype.** No ADL parser, no
flattening, no template expansion, no operational template, no retrieval. The
conformance matrix marks each `spec` and `A-40` tracks them. `validate()` is
unchanged and remains Reference-Model-level.

`L10.2` is amended: validation now has two levels, and a verdict must say which
one produced it. `S1.5` is unchanged — AQL is still parsed and not executed
(`K15.29`).

## 0.6.0 — 2026-08-22

**A representation change, and the reason it is not 0.5.1.** The source API is
additive — `magnitude()` still returns `f64`, `Real` and the `_real` accessors
are new — but **serialization changes**: a document carrying `1.50` now
round-trips as `1.50` rather than `1.5`, and its canonical digest differs from
what 0.5.0 produces for the same input. Cargo treats `0.5.x` as compatible, so a
patch would reach dependents on `cargo update` and silently change their
digests.

**Stored data is unaffected.** `db:M3.43` keeps canonical JSON byte-preserving
and `verify_versions` hashes the bytes that were stored, so records written by
0.5.0 keep their bytes and still verify. What changes is the bytes produced for
new commits from input that carried digits an `f64` discards.


- **BREAKING (representation, not signature): the Reference Model's real
  numbers preserve the digits they were written with** (`lib:D3.18d`–`D3.18f`).
  `1.50 mg` and `1.5 mg` are now different records and hash differently.

  `DV_QUANTITY.magnitude` and `.accuracy`, `DV_SCALE.value`,
  `DV_PROPORTION.numerator` and `.denominator`, and `DV_COUNT.accuracy` are
  `openehr::base::Real` instead of `f64`. `serde_json`'s `arbitrary_precision`
  feature is enabled, which is what makes the literal text reachable.

  **The `f64` accessors are unchanged.** `magnitude()` still returns `f64`;
  `magnitude_real()` is new and returns the text. Same for `value`, `accuracy`,
  `numerator`, `denominator`. Code that reads magnitudes compiles untouched.

  What does change for a caller: a struct-literal construction of these types
  (there is none in the public API — all go through constructors), and any code
  matching on the field types. Serialized output changes only where the input
  carried digits an `f64` discards, which is the point.

  Every digit survives, including trailing zeros and significant digits beyond
  what an `f64` can hold. One measured exception: exponent notation normalises,
  `1e5` and `1E5` both to `1e+5`, with no digit lost and the value unchanged.

  `db:D-08` is this same loss one layer out — MySQL rewrote a stored `1.10` as
  `1.1`, changing bytes a content digest covered, and `db:M3.43` moved canonical
  JSON onto a byte-preserving column for it. The crate had been discarding the
  digits before storage ever saw them, and `security::canonical`'s own test
  recorded that as the limit of the guarantee. It no longer is.

- **`serde_json`'s `float_roundtrip` feature is enabled**, closing `lib:A-38`.
  A `DV_QUANTITY` magnitude no longer drifts across repeated canonical round
  trips: `serde_json`'s parser was one ULP off `core::str::parse` for some
  inputs, so it was not the inverse of its own serializer.

  Recorded here because the effect is visible to a dependent: a value read back
  is now bit-identical to the value written, where before it could move. The
  digest over the *stored* bytes was never affected (`db:M3.43`).

  `arbitrary_precision` is deliberately **not** enabled — it is incompatible
  with this crate's `#[serde(tag)]` and `#[serde(flatten)]` layout, and its
  benefit applies to `serde_json::Number` rather than to the `f64` fields the
  Reference Model uses. See `spec/serde-json-float-roundtrip-arbitrary-precision/`.


- **`conformance::check_projection` and `check_verify_versions` now return what
  they checked** — `bool` for whether the composition projected, `usize` for how
  many versions had their tamper detection provoked (`db:D-10`). Not breaking:
  a caller ignoring the result still compiles.

  They return anything because otherwise they could not fail. Both were
  replaceable with `()` and nothing in the repository noticed — they are called
  only from `openehr-store-fuzz`, `cargo test` does not run fuzz targets, and a
  property that asserts nothing never crashes. `check_verify_versions` also now
  **provokes** what it is about: for a history that verifies, editing each
  version's content must make the chain report `ContentAltered`.

- **No behaviour change in `openehr`**, but two matches in `DataValue` gained
  the tests that make them non-deletable (`lib:A-39`), and
  `INTERVAL<T>::contains` treating "not comparable" as *not contained* is now
  stated as a requirement rather than left implicit (`lib:D3.14a`).

## 0.5.0 — 2026-08-21

**A feature and a behaviour change, neither an API break.** 0.5.0 rather than
0.4.1 because cargo treats `0.4.x` as compatible: a dependent on `openehr =
"0.4"` picks up a patch on `cargo update`, and the rendering change below is
visible to anyone asserting on the text of a rendered query.


- **AQL accepts negative numeric literals** (`lib:Q12.9b`, closing `lib:A-27`).
  `WHERE o/value/magnitude > -2.5` — a base excess, a temperature difference, a
  scale scored below zero — parses. So does `MATCHES {-1, 0, 1}`.

  The sign is resolved by the parser at operand position, never by the number
  scanner, so an archetype id is unaffected:
  `openEHR-EHR-COMPOSITION.encounter.v1` begins with a letter and is scanned as
  a word that absorbs its own hyphens. `> -openEHR-EHR-…` is an error, not a
  guess. `LIMIT`/`OFFSET` refuse a sign deliberately and say why (`Q12.9d`).

- **A real numeric literal renders with a decimal point** (`lib:Q12.9e`).
  `Number(0.0)` rendered as `0` and reparsed as `Integer(0)` — a literal
  changing type across a round trip. Pre-existing; found by fuzzing the widened
  grammar above.

## 0.4.0 — 2026-08-21

**Breaking.** Two of the three items below change an API; the third raises the
minimum toolchain. Every affected line in a dependent is a **compile error**,
never a silent change in behaviour — which is the property that made the
`PartialOrd` removal safe to do at all.


- **MSRV raised from 1.90 to 1.95, and it is now a rule rather than a number:
  N−3, three Rust releases behind stable**
  ([`spec/rust-msrv-n-minus-3/index.md`](spec/rust-msrv-n-minus-3/index.md)).

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

- **The `agents/` directory is lowercase** (`AG1`, `spec/agents-directory-name-is-lowercase/index.md`).
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

## Trademarks

openEHR® is a registered trademark of openEHR International (the openEHR
Foundation). This project is an independent implementation: it is not
affiliated with, endorsed by, or certified by openEHR International.
