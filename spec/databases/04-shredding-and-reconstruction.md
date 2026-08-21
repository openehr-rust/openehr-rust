# 4. Projection and reconstruction

**Rewritten 2026-08-01.** The section was titled *Shredding and reconstruction*
and specified a shredder driven by a generated relational map. There is no
shredder (`S1.5`, `M3.19`). What replaces it is a **projection**: the record is
stored whole as canonical JSON, and a small set of Reference-Model attributes is
copied out alongside it for indexing. The filename keeps its old spelling so that
existing links resolve; the section number and every surviving identifier are
unchanged. See [`spec/audit.md`](../audit.md) **W-04**.

Withdrawn requirements keep their numbers (`C0.5`); new ones start at `R4.8`
(`C0.19`).

Requirement prefix: `R4`.

## Projection

- **R4.8** Storing a version MUST write the **whole** canonical JSON of its
  content to one column, and MUST additionally write the projected index columns
  (`M3.32`). The JSON is the record; the columns are derived from it.
- **R4.9** The projection MUST be a pure function of the openEHR object — no SQL
  in it — and MUST be shared by every engine (`M3.35`).
- **R4.10** A projected column MUST NOT be the only home of any fact. Every value
  in an index column MUST also be recoverable from the JSON, so that a column
  added, removed, or recomputed later cannot lose data.

  This is what makes the index safe to change. A schema whose columns are the
  sole record of something can only be migrated by rewriting rows; this one can
  be reprojected from the documents it already holds.

## Round-trip fidelity

- **R4.2** *(amended)* Round-trip MUST be **lossless**: an openEHR object written
  and read back MUST equal the original, including the exact lexical form of
  every date and time (`M3.25`), the authored order of any archetyped
  passthrough, and every value the crate carries opaquely.
- **R4.11** The canonical JSON written to the column MUST be produced by the
  shared canonicalizer (`openehr::security::to_canonical_string`), never by an
  engine's JSON functions.

  A document canonicalized by whatever one engine's JSON type happened to
  produce — reordered keys, rewritten number spellings — is reproducible by that
  engine alone. Two engines would then disagree about the bytes of the same
  record, and any future digest over those bytes would verify on one and fail on
  the other.

- **R4.12** Reading MUST reconstruct from the stored JSON, not from the index
  columns. The columns are lossy by construction: they are bounded, they are
  `NULL` where an attribute is absent, and they do not carry archetyped content.

## Validation on the way in

- **R4.13** A store MUST validate a composition against the Reference Model
  before writing it, and MUST refuse to write one that fails (`V9`).

  A store that accepted an invalid composition would make every later reader's
  `validate()` fail on data it cannot fix. The write is the last point at which
  refusing is cheap.

- **R4.3** *(amended)* A store MUST NOT reject content because it contains
  archetyped structure the crate does not interpret. Understanding archetyped
  content is not a precondition for storing it (`S1.3`); the previous rule —
  reject any element not in the version's specification — presumed a fixed
  resource shape that openEHR does not have.
- **R4.6** *(amended)* Identifiers MUST satisfy the openEHR lexical grammars for
  `UID`, `HIER_OBJECT_ID`, and `OBJECT_VERSION_ID` (`lib:I2.x`) before they reach
  a column. A malformed identifier is refused, never normalised.

## Atomicity and isolation

- **R4.4** *(amended)* A commit MUST be a single transaction: the version row,
  the index row, and any container row created by it either all appear or none
  do. A partially applied commit leaves a version whose content is unreachable by
  the index, which reads as data loss and is not repairable without inspecting
  the JSON.
- **R4.5** *(amended)* A read that returns more than one row MUST see a single
  snapshot. A reader looping against a writer MUST NOT observe a version row
  without its index row, or a container without its head version.

  **Verified 2026-08-01** by
  `openehr-sqlite/tests/concurrency.rs::a_reader_never_observes_a_torn_commit`:
  a reader loops against a writer committing 24 versions and asserts every
  version it can see has its index row visible too. It passed first time — the
  commit transaction of `R4.4` was already doing its job — which is a result and
  not a formality, because until it ran, "reads inside a transaction" was an
  argument rather than an observation.

  The test uses a **file** database with a connection per thread. An in-memory
  SQLite database is private to its connection, so the same test against
  `in_memory()` would run N independent databases and pass without testing
  anything (`D-02`).

## Withdrawn

Withdrawn 2026-08-01. Numbers are retained and MUST NOT be reused (`C0.5`).

| Id | Was | Why withdrawn |
| --- | --- | --- |
| `R4.1` | shredding and reconstruction driven by a generated relational map | no shredder and no generated map (`M3.19`, `G2.7`); superseded by `R4.8`–`R4.10` |
| `R4.7` | audit that every row fetched during reconstruction was consumed | reconstruction reads one JSON column; there are no child rows to leave behind |

---

Part of the [openEHR persistence specification](index.md).
