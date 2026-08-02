# Search adjuncts

Normative rules for making a column searchable on an engine that cannot index or
compare it directly. Requirements are numbered `AD<n>` and use RFC 2119 keywords.

This is its own document for the reason [locale and accent
folding](locale-accent-folding.md) was: it is one decision several sections
depend on and no single section owns. It changes the **shared schema**, which
every dialect derives from (`M3.22`), so it has the widest blast radius of any
change here.

> **Nothing in this repository emits an adjunct today, and nothing needs to.**
> Every indexed column in the schema is bounded — `Id(n)`, `Int`, or
> `InstantUtc` — and a bounded column indexes and compares on all six engines.
> These requirements bind the **first search target that is not**, and they are
> written now because that target is closer than it was.

## Why this exists, when its predecessor was withdrawn

An earlier document specified two adjuncts under the numbers `U1`–`U10`. It was
withdrawn on 2026-08-01 with `P6.4a`, `P6.6`, `P6.6a` and `P6.9`, on the grounds
that this layer has no text search and no unbounded searchable columns.

The first half of that reasoning still holds. The second half stopped being
true. `D-08` moved canonical JSON off `jsonb` and MySQL's `JSON` onto a
byte-preserving text type (`M3.43`), because a normalizing column returns
different bytes from the ones the content digest was taken over — and on Oracle
a byte-preserving text type is a `CLOB`, which that engine can neither index nor
`=` compare (`openehr-oracle` `M14.8`). The largest column in the schema is now
the one the fewest engines can search.

Numbers are never reused (`C0.5`). `U1`–`U10` stay withdrawn; these are `AD1`
onward, and this document supersedes rather than revives.

## What a search target is

- **AD1** A **search target** is a column a query filters, orders, joins, or
  groups on. Storing a value does not make it a search target, and neither does
  returning it.

- **AD2** Every search target MUST declare the **kind** of search it serves:

  | Kind | Question it answers |
  | --- | --- |
  | identity | is this column exactly this value |
  | prefix | does it start with this |
  | range | is it between these, and in what order |
  | membership | is it one of these |
  | containment | does it contain this |

  A target whose kind is unwritten cannot be given the right adjunct, because
  the two adjuncts answer different questions and neither answers both
  (`AD6`). This is `P6.13` one level down: an index must record the query it
  exists for, and an adjunct must record the *kind* of query.

- **AD3** A column that is not a declared search target MUST NOT be given an
  adjunct. Adjuncts are write amplification and storage paid for at every
  commit; paying it speculatively is paying it forever for a query nobody
  wrote.

## Which engines need them

- **AD4** An adjunct is required only where the target engine cannot serve the
  declared search kind on the column's own type.

  | Logical type | PostgreSQL | SQLite | MySQL | MariaDB | SQL Server | Oracle |
  | --- | --- | --- | --- | --- | --- | --- |
  | `Id(n)`, `Text(n)` | index, `=` | index, `=` | index, `=` | index, `=` | index, `=` | index, `=` |
  | `LongText`, `Json` | index, `=` | index, `=` | prefix index, `=` | prefix index, `=` | **no index**, `=` | **no index, no `=`** |
  | `Digest` (32 bytes) | index, `=` | index, `=` | index, `=` | index, `=` | index, `=` | index, `=` |

  **Provenance, because a table like this is exactly what gets quoted later.**
  The `PostgreSQL`, `MySQL` and `MariaDB` behaviour for `Json` was measured
  against real servers in `D-08`. The `SQL Server` and `Oracle` rows are taken
  from those engines' documentation and from the dialect annexes `M14.16` and
  `M14.8`; **no run in this repository has demonstrated them**, and a crate MUST
  NOT cite this table as evidence for a conformance level (`C0.20`).

  Oracle is the sharp row and the reason this document exists. On SQL Server an
  unindexable column still answers `=`, so an affected search is correct and
  merely scans. On Oracle a `CLOB` answers neither, so the same design makes the
  search **fail** rather than slow down.

## The adjuncts

- **AD5** Three adjuncts are defined. A search target takes those its declared
  kinds require, and no others.

  | Adjunct | Type | Serves |
  | --- | --- | --- |
  | `<col>_idx` | bounded character, binary collation | prefix, range, ordering |
  | `<col>_h` | SHA-256 of the **whole** value, 32 raw bytes | identity, membership |
  | `<col>_len` | integer, the value's length in bytes | a cheap negative filter before an expensive comparison |

- **AD6** **No adjunct substitutes for another.** A bounded adjunct cannot
  answer identity, because two values agreeing in their first *n* bytes are
  indistinguishable in it. A checksum adjunct cannot answer a prefix or a range,
  because a digest destroys order. A length adjunct answers neither; it only
  ever rules a row *out*.

  A target given only the bounded adjunct has equality that silently returns the
  wrong rows. One given only the checksum has no prefix search at all. Emitting
  one and calling the problem solved is the failure this requirement is written
  to prevent.

- **AD7** The checksum MUST be SHA-256, stored as 32 raw bytes in a binary
  column and never as hexadecimal text (`M3.39`–`M3.42`), and MUST be computed
  in Rust over the same canonical bytes the rest of the project uses — never by
  a SQL function.

  Two implementations of "the same value", one in SQL and one in Rust, must
  agree for every byte or the system quietly loses matches. One implementation
  cannot disagree with itself. This is also why `M3.43` exists: a column that
  normalizes what it was given makes the Rust-side digest unreproducible from
  storage, and an adjunct computed over bytes the database will not return is
  worse than no adjunct.

- **AD8** The bounded adjunct MUST use a **binary** collation and MUST NOT fold
  case or accents. No fold is specified for this layer — the document that
  specified one is withdrawn — and an adjunct that folded differently from its
  source column would be a second definition of string identity, which is the
  cost `P6.6` was withdrawn to avoid paying.

## What adjuncts are, and are not

- **AD9** Adjuncts are **derived**. The writer MUST compute them from the source
  column; a reader reconstructing an openEHR object MUST NOT read them.

  They are not part of the record. It follows that they MUST NOT affect
  round-trip fidelity (`M3.19`), and that the hash chain's pre-image MUST NOT
  include them (`M3.16`). A chain committing to a derived column would break the
  moment the derivation changed — a migration concern the chain has no business
  carrying, and one it cannot distinguish from tampering (`M3.16d`).

- **AD10** An adjunct MUST NOT be nullable where its source is not, and MUST be
  written in the same statement as its source. An adjunct written afterwards is
  an adjunct that is absent between the two statements, and a search running
  then returns a wrong answer rather than an error.

## How a query uses them

- **AD11** A search MUST NOT return a row that only an adjunct matched. An
  adjunct is an access path, never an answer.

- **AD12** An identity predicate MUST match the checksum adjunct **and** confirm
  against the source column.

  A digest match alone is one collision away from returning another patient's
  record — the same rule as `M3.42`, applied to a search rather than to a
  comparison. The confirmation costs one row. On Oracle, where the source is a
  `CLOB` that cannot be `=` compared, the confirmation MUST use whatever that
  engine does offer (`DBMS_LOB.COMPARE`), which is precisely the case that made
  the checksum necessary.

- **AD13** A prefix or range predicate MUST use the bounded adjunct as a
  **filter** and confirm against the source column. The index narrows; the
  comparison decides.

- **AD14** A test asserting a search's results MUST be written so that it fails
  if the confirmation step is removed, and that failure MUST be demonstrated by
  mutation (`T11.10`).

  A missing confirmation is invisible until two values collide, which is to say
  invisible in every test anyone thinks to write. Nothing else establishes that
  `AD11` holds.

## Per search target

- **AD15** **Bounded string search** (`Id(n)`, `Text(n)`) takes no adjunct on
  any engine. Every one of the six indexes and compares a bounded character
  column. This is why the schema has no adjunct today: every indexed column is
  bounded, deliberately (`M3.29`).

- **AD16** **Unbounded string and CLOB search** (`LongText`, `Json`) takes the
  checksum adjunct for identity and the bounded adjunct for prefix or range, on
  SQL Server and Oracle only. The four engines that index and compare these
  types directly MUST NOT emit either (`AD3`).

- **AD17** **Binary search** takes the checksum adjunct for identity and the
  length adjunct as a pre-filter, and MUST NOT take a bounded adjunct: a prefix
  of arbitrary bytes is not a search anyone has asked for here, and offering it
  would invite prefix-matching a digest, which `M3.40` forbids for good reason.

  `Digest` columns are exempt. They are already exactly 32 bytes, indexable and
  comparable on all six engines, and an adjunct over a digest is a digest of a
  digest.

- **AD18** **The canonical JSON document is a search target for identity only**,
  and only if some future operation needs to ask whether a document has been
  stored before. It MUST NOT be given a structural or path index.

  Content inside a stored document is not queryable by this layer (`P6.10`), and
  a JSON-path index would require the engine to parse and interpret the column —
  reintroducing exactly the engine-side reinterpretation `M3.43` was written to
  remove. A store wanting content search builds it above this layer, over its
  own projection, and does not reach back into `data_json`.

## Recording them

- **AD19** A dialect that materializes an adjunct MUST record in its annex which
  columns take one, which kinds it serves, and what the bound *n* is; and MUST
  NOT claim the search requirement it supports until it has (`X15.6`, `C0.16`).

- **AD20** The shared schema MUST record that an adjunct is *available* for a
  column; the dialect decides whether to materialize it. A dialect MUST NOT
  invent an adjunct the schema does not declare, for the same reason it may not
  invent a column (`M3.22`).

## Why not the alternatives

Recorded because each looks reasonable until examined.

**Bind the column and document the limit.** Give the compared column a bounded
type and say that longer values are not exactly searchable. No schema change —
and a permanent, silent functional gap on two engines, in which a search for a
long value returns nothing and reports no error. A wrong answer that looks like
an empty result is the worst failure mode available.

**Truncate the stored value.** Loses data and breaks `M3.19`. Not worth further
discussion.

**Split the value across a bounded column and a `CLOB` tail.** Preserves
indexability and losslessness, and pushes reassembly into the projection and the
reader — the two places where a defect corrupts records rather than merely
failing to find them. Adjuncts leave the source column whole and add beside it:
the same benefit at a fraction of the risk.

**Let the engine do it.** A full-text index, or `jsonb` path operators. This is
the option `D-08` closed: the moment the engine interprets the column, the bytes
it returns are not the bytes that were stored, and the content digest cannot be
recomputed from storage.

---

Part of the [openEHR persistence specification](index.md).
