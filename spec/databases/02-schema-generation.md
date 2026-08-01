# 2. Schema generation

**Rewritten 2026-08-01** to describe this repository. The previous text required
DDL to be *generated* from specification packages (`StructureDefinitions`,
`SearchParameters`) by a `gen` operation, committed under `assets/`, with an
identifier budget, deterministic abbreviation, collision hashing, and an install
that creates "thousands of tables — 7,355 for R5". None of that applies: there is
no generator, no `assets/`, and the schema is five tables declared as Rust
constants. See [`spec/audit.md`](../audit.md) **W-04**.

Withdrawn requirements keep their numbers (`C0.5`); new ones start at `G2.7`
(`C0.19`).

Requirement prefix: `G2`.

## The schema is declared, not generated

- **G2.7** The schema MUST be **compile-time data** in `openehr_store::schema` —
  tables, columns, primary keys, foreign keys, and indexes as Rust constants. It
  MUST NOT be generated from an external artefact at build time or install time.

  Generation is the right answer when the shape comes from a specification that
  fixes it, which is true of FHIR and false of openEHR (`S1.5`). Here the shape
  comes from the Reference Model classes that exist independently of any
  archetype, and there are five of them. A generator would be machinery for
  producing a constant.

  The consequence is worth stating positively: a build needs no network, no
  specification package, and no checksum manifest, and the schema a reader sees
  in `schema.rs` is exactly the schema that ships.

- **G2.8** DDL MUST be **derived** from that declaration by the shared
  `Dialect::ddl` default methods. A dialect MUST NOT write a `CREATE TABLE`
  statement of its own.

  This is the boundary the whole architecture rests on. A dialect that owns only
  spellings cannot emit another engine's schema, because it does not own the
  schema. See §15 and `W0.14`.

- **G2.9** Generation MUST be deterministic: the same declaration MUST produce
  byte-identical DDL on every run, so a change in emitted SQL appears as a diff
  in a golden test rather than as a migration that fails in someone's staging
  environment. Every engine crate MUST carry such golden tests.

## Emission order

- **G2.10** `ddl()` MUST emit, in this order: every `CREATE TABLE` in `TABLES`
  order, then every index, then every append-only statement.
- **G2.11** Indexes MUST come after **all** tables rather than after each table.
  A partial failure then leaves a readable schema rather than a half-indexed one.
- **G2.12** Tables MUST be emitted in `TABLES` order, which `M3.23` requires to
  be dependency order. No deferred-constraint mechanism is then needed, which
  matters because Oracle and SQL Server make deferral awkward.

## Idempotence

`install()` must be safe to run twice: an operator who repeats it, or a
deployment that retries, must not get a hard error the second time.

- **G2.13** A dialect MUST declare idempotence **separately** for tables and for
  indexes. One flag covering both gets it wrong on at least one target engine.

  This is not a hypothetical tidiness argument. The first live run against MySQL
  failed exactly here: MySQL accepts `CREATE TABLE IF NOT EXISTS` and **rejects**
  `CREATE INDEX IF NOT EXISTS`, so a single flag emitted a script that created
  every table and then died at the first index (`A-13`).

- **G2.14** The three declarable strategies are:

  | `Idempotence` | Means |
  | --- | --- |
  | `IfNotExists` | the statement accepts an inline `IF NOT EXISTS` clause |
  | `Guard` | the statement must be wrapped by `Dialect::guard` |
  | `Inline` | no separate statement — the object is declared inside its table and inherits that table's idempotence |

- **G2.15** A dialect declaring `Guard` MUST actually wrap the statement.
  `conformance::check_dialect` fails a dialect that declares `Guard` and returns
  the statement unchanged, because a guard that is documented and not emitted is
  worse than none: it reads as protection while providing none.
- **G2.16** A dialect MUST NOT emit an `IF NOT EXISTS` clause its engine does not
  accept, and MUST NOT omit one its declaration claims. A cross-dialect test
  asserts the emitted script and the declaration agree, in both directions.
- **G2.17** `Inline` is the idiomatic answer where an engine cannot say
  `CREATE INDEX IF NOT EXISTS` but can declare the index inside `CREATE TABLE`,
  where it inherits the table's idempotence. One statement, one object, one
  idempotence rule — a solution rather than a workaround.

  MySQL declares `Inline` for this reason. MariaDB does **not**: it has accepted
  `CREATE INDEX IF NOT EXISTS` since 10.0.5, and using the MySQL form there would
  be an inherited decision rather than an engine fact — which is precisely how
  `openehr-mariadb` came to be a copy of `openehr-mysql` (**W-01**).

## Identifiers

- **G2.18** Table and index names MUST be identical on every engine. A schema
  that is name-for-name comparable across engines is what makes a cross-engine
  diff meaningful; per-engine names would make every engine's schema unique and
  every such comparison impossible.
- **G2.19** Every identifier MUST fit the tightest target engine's limit. Oracle
  allowed 30 bytes before 12.2 and 128 after; the names in this schema are well
  inside every target's limit, and MUST stay so. There is no abbreviation or
  collision-hashing mechanism, and none is needed while the schema is five tables
  — but a name added without checking is how that stops being true.
- **G2.20** Identifiers MUST be quoted by the dialect on emission, never
  interpolated raw, and a dialect's `quote` MUST escape the engine's own quote
  character. Unquoted identifiers are the commonest source of a script that
  parses everywhere except its target.

## Withdrawn

Withdrawn 2026-08-01. Numbers are retained and MUST NOT be reused (`C0.5`).

| Id | Was | Why withdrawn |
| --- | --- | --- |
| `G2.1` | generate DDL from specification packages into `assets/` | no generator, no packages, no `assets/`; the schema is declared (`G2.7`) |
| `G2.2` | deterministic generation with `assets/CHECKSUMS.txt` | superseded by `G2.9`; there is no artefact to checksum |
| `G2.3` | snake_case element paths, `Patient.name.given` → `patient_name_given` | shredding (`S1.5`) |
| `G2.4` | a per-port identifier budget with deterministic abbreviation and collision hashing | five tables with fixed names; superseded by `G2.18`–`G2.19` |
| `G2.5` | `init` idempotent and "effectively atomic" over thousands of tables, with staging namespaces and chunked transactions | five tables; superseded by `G2.13`–`G2.17` |
| `G2.6` | bound generated table width below the engine's column limit | no generated tables; the widest here is 16 columns |

---

Part of the [openEHR persistence specification](index.md).
