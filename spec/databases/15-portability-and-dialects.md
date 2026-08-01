# 15. Portability and dialects

**Rewritten 2026-08-01.** The previous text described six ports each holding a
copy of a shared `map/`/`gen/` tree, and required those copies to stay
byte-identical. That arrangement does not exist here: the shared code lives once,
in `openehr-store`, and the engine crates depend on it. See
[`spec/audit.md`](../audit.md) **W-04**.

This section defines what makes six crates one product rather than six products
with a shared ancestor.

Withdrawn requirements keep their numbers (`C0.5`); new ones start at `X15.13`
(`C0.19`).

Requirement prefix: `X15`.

## The dialect boundary

- **X15.13** A dialect owns exactly **four** things:

  | Method | Owns |
  | --- | --- |
  | `col_sql` | how the engine spells each logical column type |
  | `quote` | how it quotes an identifier |
  | `placeholder` | how it writes a bind placeholder |
  | `append_only_sql` | how it enforces append-only |

  Plus two idempotence declarations (`G2.13`), and optionally `guard` and
  `terminator`.

- **X15.14** A dialect MUST NOT own the schema. Which tables exist, which
  columns, which indexes, what order they are emitted in, the projection onto
  rows, the commit rules, and the conformance suite all come from
  `openehr-store` (`G2.8`, `M3.22`).

  This is not a style preference. A dialect that owns only spellings **cannot**
  emit another engine's schema, because it does not have one to emit. The sibling
  FHIR monorepo gave each of six ports a full copy of the generator, and one copy
  spent that fork's entire life producing another engine's types because nothing
  compared them.

- **X15.1** *(amended)* The portable core MUST exist in exactly **one** crate,
  depended on by the engine crates. It MUST NOT be copied into them and then
  policed for drift.

  The previous formulation — six copies, required to stay identical, with CI to
  detect divergence — treats the symptom. A copy that cannot be made is a copy
  that cannot drift.

## A shared core is necessary and is not sufficient

- **X15.15** Every engine crate MUST be compared against every other, and the
  comparison MUST cover **all** of them. No two dialects may emit the same DDL.
- **X15.16** The comparison's **coverage** MUST itself be checked. A list of
  dialects to compare is hand-maintained, and a comparison that omits a dialect
  cannot find that dialect identical to another — it reports the same green as a
  complete one.

  `X15.15` existed and passed for as long as `openehr-mariadb` was a
  name-substituted copy of `openehr-mysql`, emitting byte-identical DDL and
  exporting a struct still called `MysqlDialect`. The comparison listed five
  dialects; that was the sixth (**W-01**). `X15.16` is the requirement that
  finding produced: `openehr-sqlite/tests/dialects.rs` now ties the length of its
  dialect list to the number of engine crates.

- **X15.17** A new engine crate MUST NOT be created by copying an existing one.
  It MUST be written from the `Dialect` trait, deciding each method from the
  engine's own documentation.
- **X15.18** A dialect MUST differ from its nearest neighbour in at least one
  respect that is a **real engine fact**, and that difference MUST be named in
  the crate's documentation and asserted by a test.

  If no such difference can be named, either the crate is unnecessary or the
  difference has not been found yet — and the second is the dangerous case.
  MariaDB and MySQL genuinely agree on every type spelling; they differ on
  `CREATE INDEX IF NOT EXISTS` and `CREATE OR REPLACE TRIGGER`, and those two
  facts are what make the crates distinct rather than the type table.

- **X15.19** Types that differ across engines in reality MUST differ across
  dialects in the code. Distinct DDL is necessary but not sufficient: two
  dialects could differ only in quoting while mapping booleans identically, which
  is how a copied emitter hides.

## What must be identical

- **X15.3** *(amended)* Table and index names MUST be the same on every engine
  (`G2.18`). A schema that is name-for-name comparable is what makes a
  cross-engine diff a meaningful test.
- **X15.2** *(amended)* The **canonical form** of a stored document MUST be
  computed in Rust, by one function shared by every engine, and MUST NOT be
  delegated to the database (`R4.11`).

  A document canonicalized by whatever one engine's JSON type produced —
  reordered keys, rewritten number spellings — is reproducible by that engine
  alone. No other engine could verify it, and the format would not survive a
  port.

- **X15.20** The projection from openEHR objects onto index columns MUST be one
  shared function (`M3.35`). An engine computing its own would eventually index a
  different `category`, and the difference would surface as a query returning
  different rows on different engines.

## Dialect annexes

- **X15.6** *(amended)* Every engine crate MUST have a `spec/14-<engine>-dialect.md`
  annex addressing, explicitly and by name, each of the following. "Not
  applicable" is an acceptable answer; silence is not, because silence is
  indistinguishable from not having considered it.

  1. **Engine floor** (`S1.4`) — the minimum version, and the dialect fact that
     sets it.
  2. **`ColTy` binding** (`M3.6`) — the full table, with each choice justified.
  3. **The two instant columns** (`M3.31`) — the types bound to `Instant` and
     `InstantUtc`, and confirmation they differ.
  4. **Idempotence** (`G2.13`) — the declaration for tables and for indexes, and
     the engine fact behind each.
  5. **Append-only enforcement** (`M3.17`, `M3.37`) — the exact mechanism, and
     whether it leaves a window in which the guarantee lapses.
  6. **Placeholder syntax** — and the driver that expects it.
  7. **Identifier quoting** — including how the engine's own quote character is
     escaped.
  8. **The difference from the nearest neighbouring dialect** (`X15.18`).
  9. **Unmet core requirements** — every one, as a numbered departure (`C0.14`).

- **X15.7** A departure MUST cite the core requirement it amends by number and
  state what holds instead. Prose that merely describes the engine is not a
  departure and does not license one (`C0.17`).
- **X15.8** An annex MUST NOT restate core requirements it does not change. The
  annex is a diff, and a diff that includes unchanged lines is not one.
- **X15.9** An annex MUST carry a status — **proposed** or **ratified** — and a
  proposed annex MUST NOT be cited as evidence for a conformance level (`C0.9`).

  **None of the six crates has an annex.** All six `spec/` directories are empty.
  That is a standing violation of `X15.6`, recorded here rather than left to be
  discovered per crate, and it is why several dialect decisions are currently
  documented only in rustdoc.

## Cross-engine agreement

- **X15.10** *(amended)* Two engines at **Store** level or above MUST agree on
  the logical content of a store: the same composition committed through both
  produces the same logical rows under the same identifiers, and either engine's
  read of the other's rows yields the same openEHR object. Physical form differs
  by binding; logical content does not.

  **Untestable today.** Only `openehr-sqlite` has a store, so there is no second
  implementation to compare against.

- **X15.11** *(amended — **not implemented**)* A tamper-evidence chain written by
  one engine would have to be verifiable by another, given the same key material.
  No chain exists (`M3.16`), so this is a property the design reserves rather
  than one it has.
- **X15.12** *(amended)* A cross-engine test MUST exist for whatever *can* be
  compared without two stores. Today that is the DDL: `dialects_are_distinct`
  compares all six dialects, and companion tests assert the boolean and JSON
  spellings genuinely differ, that no dialect collapses the two instant columns,
  that placeholders match their drivers, and that every dialect enforces
  append-only.

  What remains untested across engines is everything that needs two stores —
  `X15.10` above.

## Withdrawn

Withdrawn 2026-08-01. Numbers are retained and MUST NOT be reused (`C0.5`).

| Id | Was | Why withdrawn |
| --- | --- | --- |
| `X15.4` | the accent/case fold must be byte-identical across ports | there is no fold; nothing here does text search (see §6) |
| `X15.5` | the stored image of `ords` must be the shared array literal | `ords` was a shredding construct (`M3.4`, withdrawn) |

---

Part of the [openEHR persistence specification](index.md).
