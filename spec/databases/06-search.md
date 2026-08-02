# 6. Search

**Rewritten 2026-08-01.** The previous text specified FHIR search: compiled
`SearchParameter` definitions, `:exact`/`:contains` modifiers, `_count` and
opaque paging cursors, `_include`/`_revinclude` expansion, and case- and
accent-insensitive string matching over a Unicode fold. None of it applies —
there is no text search here, and there are no search parameters to compile. See
[`spec/audit.md`](../audit.md) **W-04**.

Withdrawn requirements keep their numbers (`C0.5`); new ones start at `P6.10`
(`C0.19`).

Requirement prefix: `P6`.

## What "search" means here

- **P6.10** The queryable surface MUST be the index over Reference-Model
  attributes (`M3.20`, `M3.32`), and nothing else. Content inside a stored
  document is **not** queryable by this layer.

  This follows directly from the storage model. Archetyped content has no fixed
  shape, so there is nothing stable to index and no parameter set to compile.
  What the index offers is the envelope an AQL `FROM` clause filters on before it
  reaches into content — which is the useful part, and the part that can be made
  fast without knowing any archetype.

- **P6.11** A store MUST offer at least: a version by identifier, the latest
  version of a container, the version current at a time, every version of a
  container (`H5.3`), and compositions in a record matching an archetype id.
- **P6.12** Archetype-id lookup is the query the index principally exists for —
  AQL's `CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1]` — and MUST
  be served by an index rather than a scan.

## Indexes

- **P6.4** *(amended)* The schema MUST declare, and every dialect MUST emit,
  these indexes:

  | Index | Serves |
  | --- | --- |
  | `ix_versioned_object_ehr` | list a record's containers by type without scanning every version |
  | `ix_version_container_trunk` (**unique**) | one row per position in a version tree; also makes a duplicate commit fail in the database (`H5.10`) |
  | `ix_version_time` | `version_at_time` without scanning a container's whole history |
  | `ix_version_preceding` | walk a version tree forwards |
  | `ix_contribution_ehr_time` | a record's change history in commit order |
  | `ix_composition_archetype` | the archetype query of `P6.12` |
  | `ix_composition_context_start` | encounters in a date range |

- **P6.13** Every index MUST record **which query it exists for**. An index whose
  query nobody wrote down is an index nobody dares drop.
- **P6.14** Time-ranged queries MUST use the derived UTC column, never the
  lexical one (`M3.28`), and MUST therefore skip versions whose instant is not
  established. Ordering on the lexical form sorts
  `2026-07-31T09:00:00+02:00` after `2026-07-31T08:30:00Z`.

## Search targets and adjuncts

Added 2026-08-02. `D-08` changed a premise this section was rewritten under.

- **P6.16** Every column a query filters, orders, joins, or groups on is a
  **search target**, and MUST declare the kind of search it serves — identity,
  prefix, range, membership, or containment. See
  [`search-adjuncts.md`](search-adjuncts.md) `AD1`–`AD2`.

  `P6.13` requires an index to record the query it exists for. This is the same
  obligation one level down, and it exists because the remedy for a column an
  engine cannot search depends entirely on *which* search: the two adjuncts
  answer different questions and neither answers both.

- **P6.17** Where an engine cannot serve a search target's declared kind on the
  column's own type, that dialect MUST emit the adjuncts
  [`search-adjuncts.md`](search-adjuncts.md) specifies, and MUST NOT answer the
  search from an adjunct alone (`AD11`, `AD12`).

- **P6.18** **No search target requires an adjunct today.** Every indexed column
  in the schema is `Id(n)`, `Int`, or `InstantUtc`, and all six engines index
  and compare those. `P6.16`–`P6.17` bind the first target that is not, and are
  written in advance because one is now foreseeable rather than hypothetical.

- **P6.19** The canonical JSON column MUST NOT be given a structural or
  path index (`AD18`). Content inside a stored document is not queryable here
  (`P6.10`), and a path index requires the engine to parse and reinterpret the
  column — which `M3.43` forbids, because the bytes it then returns are not the
  bytes the content digest was taken over (`M3.16d`).

  This is the requirement most likely to be proposed as an optimisation by
  somebody who has read `P6.10` and concluded that indexing content is merely
  unimplemented rather than excluded.

## Query construction

- **P6.8** Every predicate MUST bind user-supplied values as **parameters**,
  using the dialect's placeholder style. String interpolation of a value into SQL
  is forbidden without exception.

  This is the one requirement in this section with a security consequence, and it
  is why `Placeholder` is part of the `Dialect` trait rather than left to each
  store: an archetype id arriving from a caller is untrusted input, and it is
  interpolated into a `WHERE` clause on the commonest query in the system.

- **P6.7** *(amended)* A query MUST have a bounded cost. A store MUST NOT offer
  an operation whose result set grows without limit in the size of a record
  unless the caller has asked for exactly that — `all_versions` is deliberately
  such an operation, and is named so the caller knows.
- **P6.15** A store MUST NOT silently truncate a result set. If a bound is
  applied, the caller MUST be able to tell that it was.

  A truncated result that looks complete is a clinical safety problem, not a
  performance trade: "this patient has no further encounters" and "I stopped
  looking" are different answers.

## Withdrawn

Withdrawn 2026-08-01. Numbers are retained and MUST NOT be reused (`C0.5`).

| Id | Was | Why withdrawn |
| --- | --- | --- |
| `P6.1` | compile all standard `SearchParameter`s of each version | a FHIR construct; there are no search parameters (`P6.10`) |
| `P6.2` | case-insensitive prefix match with `:exact`/`:contains` | no text search |
| `P6.3` | `_count`, opaque paging cursors, `_include`/`_revinclude` | FHIR REST result parameters; §7 is retired (`C0.6`) |
| `P6.4a` | fallback where an engine cannot index a bound text column | no unbounded searchable text columns existed; the obligation returns as `P6.17` — see the note below |
| `P6.5` | unsupported search parameters return a warning and are ignored | no search parameters; an unsupported operation refuses (`S1.11`) |
| `P6.6`, `P6.6a` | Unicode case/accent fold; prefix search as a range predicate | no text search and no fold |
| `P6.9` | unbounded text columns need a bounded adjunct and a checksum adjunct | same; superseded by `P6.17` and `AD1`–`AD20` — see the note below |

The two cross-cutting documents that supported these —
[`locale-accent-folding.md`](locale-accent-folding.md) and
[`unbounded-string-search-must-have-bounded-adjunct-and-checksum-adjunct.md`](unbounded-string-search-must-have-bounded-adjunct-and-checksum-adjunct.md)
— stay withdrawn. They specify machinery for case- and accent-insensitive text
search, which this layer still does not have.

**One half of that reasoning stopped being true.** These were withdrawn partly
because no unbounded searchable column existed. `D-08` moved canonical JSON off
`jsonb` and MySQL's `JSON` onto a byte-preserving text type (`M3.43`), and on
Oracle that is a `CLOB` — a column that engine can neither index nor `=`
compare. The largest column in the schema is now the one fewest engines can
search.

So the adjunct obligation returns, under new numbers, in
[`search-adjuncts.md`](search-adjuncts.md). The **fold** does not: `AD8`
requires a binary collation precisely so that no second definition of string
identity is introduced. Numbers are not reused (`C0.5`) — `P6.4a`, `P6.6`,
`P6.6a`, `P6.9` and `U1`–`U10` remain withdrawn, and the successors supersede
rather than revive them.

---

Part of the [openEHR persistence specification](index.md).
