# openehr

openEHR Reference Model types, validation, paths, AQL parsing, and
change-control security primitives — in Rust.

[openEHR](https://specifications.openehr.org/) specifies clinical information
as a small, stable **Reference Model** of about ninety classes, plus
**archetypes** that constrain it into clinical content. This crate implements
the Reference Model and the machinery around it, so a Rust program can read,
build, check, address, and safely disclose openEHR data without inventing its
own idea of what a health record is.

```toml
[dependencies]
openehr = "0.2"
```

## Install

```toml
[dependencies]
openehr = "0.2"
```

Requires Rust 1.95+ (edition 2024).

## What it does

```rust
use openehr::path::Pathable;
use openehr::rm::ehr::Composition;
use openehr::validation::Validate;

// Read a composition another openEHR implementation wrote.
let composition: Composition = serde_json::from_str(json)?;

// Check the Reference Model invariants. Deserialization never calls a
// constructor, so this is the only gate on data that arrived from elsewhere.
composition.validate_ok()?;

// Address a node by openEHR path.
let systolic = composition.item_at_path(
    "/content[openEHR-EHR-OBSERVATION.blood_pressure.v2]\
     /data/events[at0006]/data/items[at0004]/value/magnitude",
)?;
```

| Module | openEHR component |
| --- | --- |
| `base` | BASE: identifiers, references, intervals, ISO 8601 |
| `rm::data_types` | RM: Data Types — every `DV_*` class |
| `rm::data_structures` | RM: Data Structures — `ITEM_*`, `CLUSTER`, `ELEMENT`, `HISTORY` |
| `rm::common` | RM: Common — archetyping, parties, audit, change control |
| `rm::ehr` | RM: EHR — `COMPOSITION`, the five entry classes, `EHR_STATUS`, `FOLDER` |
| `rm::demographic` | RM: Demographic — `PERSON`, `ROLE`, `ORGANISATION`, `AGENT` |
| `terminology` | TERM: the openEHR support terminology, sixteen groups |
| `path` | openEHR path parsing and navigation |
| `aql` | QUERY: AQL lexing, parsing, and static checking |
| `validation` | Reference Model invariant checking |
| `security` | `EHR_ACCESS`, tamper-evident audit chaining, redaction |

Everything serializes to and from **openEHR canonical JSON** (ITS-JSON).

## What it does not do

Stating this plainly is part of the design. A clinical library that implies
coverage it does not have is worse than a small one.

| Not implemented | Why |
| --- | --- |
| ADL parsing, flattening, templates, and **archetype validation of data** | **specified, not built.** `S1.4` excluded the Archetype Model until 2026-08-26; [§15](spec/15-archetypes.md) now requires it. `am` is the AOM2 object model (`K15.1`–`K15.4`); the other 28 requirements have no code (`A-40`), so a composition that passes `validate()` may still violate its archetype. A partial constraint engine stays prohibited — `K15.20` refuses rather than passes |
| AQL **execution** | needs a repository; `aql` parses and checks, and returns no rows |
| Terminology lookup beyond openEHR's own | needs a terminology server; external codes are carried opaquely |
| UCUM unit conversion | a wrong conversion is a thousand-fold dosing error |
| REST service, persistence, EHR Extract | out of scope — see [`spec/01-scope.md`](spec/01-scope.md) |
| HL7 `GTS` / `PIVL` timing evaluation | returns `Unsupported` rather than a guess |
| `OpenPGP` verification, encryption | key management belongs to the deployment |

Where openEHR defines an operation this crate does not implement, the operation
returns an explicit `Unsupported` error naming the specification section that
records the exclusion. It never returns a plausible default.

## Three design commitments

**Refuse rather than guess.** Comparison is partial throughout. A
month-precision date is not ordered against a day inside that month; `5 mg` is
not comparable with `5 mL`; a path matching three elements fails rather than
returning the first. Each has a plausible wrong answer that no downstream reader
could detect.

```rust
let may: Date = "2024-05".parse()?;
let may_17: Date = "2024-05-17".parse()?;
assert_eq!(may.semantic_cmp(&may_17), None);  // May which day?

let mg = DvQuantity::new(5.0, "mg")?;
let ml = DvQuantity::new(5.0, "mL")?;
assert_eq!(mg.semantic_cmp(&ml), None);       // not the same dose of anything
```

It is `semantic_cmp` and not `partial_cmp` on purpose, and the reason is worth
one paragraph because it will otherwise look like an oversight.

Equality here is **record identity**: a `DV_QUANTITY` that records
`precision: 1` is a different stored value from one that records `2`, and
`2026-08-01T12:00:00+01:00` is a different stored value from `…T11:00:00Z` —
the text is what round-trips and what a content digest is taken over. Ordering
compares only the magnitude, so those pairs order **equal** while comparing
**unequal**. Rust's `PartialOrd` requires `a == b` exactly when `partial_cmp`
reports `Equal`, so implementing it would mean shipping `a != b` alongside
`a <= b && a >= b` — invisible inside this crate, and a wrong answer inside a
caller's `binary_search` or `dedup_by`. So no `DV_ORDERED` implements
`PartialOrd`; comparison is a named method (`D3.18b`, formerly the finding
`A-35`).

**Absence is structured.** openEHR's four null flavours are four different
clinical facts, and this crate will not let them collapse:

| Flavour | Means |
| --- | --- |
| `271｜no information｜` | nobody looked |
| `253｜unknown｜` | somebody looked and could not find out |
| `272｜masked｜` | the value exists and is withheld |
| `273｜not applicable｜` | the question does not arise |

"No allergy history recorded" is the first and "no known allergies" is the
fourth. Prescribing software that treats them alike will eventually give a
penicillin-allergic patient penicillin.

**Nothing prints protected health information.** No `Display` renders an
identifier or a media blob; no error echoes a submitted value; a validation
report names paths and invariants and never content; redaction masks rather than
deletes, and reports *how much* it withheld rather than *what*.

## Two gates, not one

Constructors enforce invariants on data the program **builds**. `validation`
enforces them on data the program **receives** — `serde` writes fields directly
and never calls a constructor. A service that deserializes and stores without
validating has no invariant checking at all, whatever its constructors do.

```rust
// No constructor in this crate produces this. A sender can still send it.
let element: Element = serde_json::from_str(
    r#"{"name":{"value":"Systolic"},"archetype_node_id":"at0004",
        "value":{"_type":"DV_COUNT","magnitude":1},
        "null_flavour":{"value":"unknown","defining_code":
          {"terminology_id":{"value":"openehr"},"code_string":"253"}}}"#,
)?;
assert_eq!(element.validate().violations()[0].invariant, "Inv_null_flavour_indicated");
```

## Security

`security` supplies what a library can supply, and says what it cannot.

- **`EHR_ACCESS`** with a default-deny decision, a documented reference scheme,
  and lossless carriage of schemes it cannot evaluate. Dispatch is by declared
  scheme name and never by object shape, so a foreign policy is never
  reinterpreted as an empty local one.
- **A tamper-evident chain** over committed versions, unkeyed or with an
  `HMAC-SHA-256` tag. The documentation states plainly what an unkeyed chain
  buys — it detects careless modification and supports an external witness, and
  it does not stop an informed attacker with write access. Only a tag mismatch
  is a tampering finding; an unheld key is reported as an unheld key.
- **Redaction** that masks as `272｜masked｜`, keeps the document valid, and
  counts rather than names what it withheld.

What the deployment must still provide: authentication, group membership,
transport security, key storage, consent capture, and log retention. See
[`spec/11-security.md`](spec/11-security.md) for the whole boundary.

## Examples

```sh
cargo run --example 01_build_composition     # build a blood pressure, emit canonical JSON
cargo run --example 02_validate_incoming     # four defects a JSON schema would not catch
cargo run --example 03_paths_and_aql         # path navigation and AQL parsing side by side
cargo run --example 04_versioning_and_audit  # commits, concurrent-write refusal, chain verification
cargo run --example 05_access_and_redaction  # default-deny decisions and consent filtering
```

## Specification-driven

[`spec/`](spec/index.md) is normative. Every requirement has a permanent
identifier cited from the code, the tests, and the documentation, so a claim
about this crate is traceable back to a decision.

| Read | For |
| --- | --- |
| [`spec/index.md`](spec/index.md) | the map, and what this spec adds to openEHR's |
| [`spec/01-scope.md`](spec/01-scope.md) | what is excluded and why |
| [`spec/conformance-matrix.md`](spec/conformance-matrix.md) | what is verified **today** |
| [`spec/audit.md`](spec/audit.md) | every known gap, with evidence |

Two numbers from those files, because they are the ones worth knowing before
depending on this crate: of 291 requirements, **237 are verified by a named
test** and **3 are implemented with no test at all**. A further 13 are marked
`type` — enforced by the compiler, where a runtime test could not fail — rather
than counted as verified, so the first number means what it says.

## Status

Version 0.1.0. First release. The Reference Model surface is complete for the
packages listed above and the open findings are in
[`spec/audit.md`](spec/audit.md) — twelve of them, seven already fixed, none a
false claim in the documentation.

Every code fragment above is compiled and run as a test
(`tests/readme.rs`). A documented example that does not compile is worse than
none, because it costs the reader the time to find out.

## Building

```sh
cargo build
cargo test                      # unit, integration, and doctests
cargo clippy --all-targets      # pedantic, with missing_docs/errors/panics denied
cargo fmt --all -- --check
```

MSRV is **N−3** — three Rust releases behind stable, currently **1.95** — and is
`rust-version` in every `Cargo.toml`. Rust edition 2024. The policy, and the CI
job that re-derives the number rather than trusting it, are in
[`spec/rust-msrv-n-minus-3/index.md`](../spec/rust-msrv-n-minus-3/index.md).

## Benchmarks

```sh
cargo bench                     # criterion; results in target/criterion/
cargo bench -- --test           # one iteration each, which is what CI runs
```

`benches/rm.rs` measures the paths a whole document travels — deserialization
(the widest untrusted surface), validation (gate two, one full traversal),
canonical JSON (taken over by a content digest, so it runs twice per round
trip), path resolution, AQL parsing, and ISO 8601 parsing, which is the hottest
because openEHR times are preserved lexically and every instant in a document is
parsed as text.

**A number here is not a conformance claim** (`W0.3`, `W0.34`). No requirement in
this repository is stated in seconds and no crate's conformance level depends on
a timing. CI runs the benchmarks with `--test` and gates on nothing: wall-clock
on a shared runner varies by more than most real regressions, and a threshold
that fails for unrelated reasons is one somebody silences.

## Licence

Any of these, at your option — MIT, Apache-2.0, BSD-3-Clause, GPL-2.0-only, or
GPL-3.0-only. See [`LICENSE.md`](LICENSE.md).

openEHR specifications are published by the openEHR Foundation under CC-BY-SA;
this crate is an independent implementation and is not endorsed by or affiliated
with the openEHR Foundation.
