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
| `P6.4a` | fallback where an engine cannot index a bound text column | no unbounded searchable text columns exist |
| `P6.5` | unsupported search parameters return a warning and are ignored | no search parameters; an unsupported operation refuses (`S1.11`) |
| `P6.6`, `P6.6a` | Unicode case/accent fold; prefix search as a range predicate | no text search and no fold |
| `P6.9` | unbounded text columns need a bounded adjunct and a checksum adjunct | same; the cross-cutting file specifying this is withdrawn with it |

The two cross-cutting documents that supported these — `locale-accent-folding.md`
and `unbounded-string-search-must-have-bounded-adjunct-and-checksum-adjunct.md` —
are withdrawn for the same reason. They specify machinery for a text-search
feature this layer does not have.

---

Part of the [openEHR persistence specification](index.md).
