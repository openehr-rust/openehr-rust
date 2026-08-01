# 3. Storage model

**Rewritten 2026-08-01** to describe this repository. The previous text specified
a **shredded** schema — a base table per resource type, a child table per
repeating element, an `ords` ordinal path, `Reference` columns, `contained`
resources, extension leaf rows — which is a FHIR storage model and is the
opposite of what these crates do, for the reason in `S1.5`. See
[`spec/audit.md`](../audit.md) **W-04**.

Withdrawn requirements keep their numbers and are listed at the end (`C0.5`).
New requirements take the next unused ordinal rather than reusing a vacated one
(`C0.19`), so this section restarts at `M3.19`.

Requirement prefix: `M3`.

## The model

- **M3.19** The canonical JSON of a version's content **is** the record. It MUST
  be stored whole, in one column, and MUST be the authority for every attribute
  the relational columns also carry.
- **M3.20** The relational part MUST be an **index** over attributes the
  Reference Model itself fixes, and MUST NOT attempt to represent archetyped
  content as columns.

  The two together are the whole design. A `COMPOSITION` contains whatever its
  archetype says; archetypes are authored by clinicians and published long after
  the software ships. Columns can only be generated for what the model fixes at
  specification time, and for openEHR that is the envelope — who committed, when,
  which archetype, which category, which setting — not the clinical content.

  Indexing the envelope is not a compromise. Those are exactly the attributes an
  AQL `FROM` clause filters on before it reaches into content.

- **M3.21** Five tables, and no more without an amendment:

  | Table | Holds | Append-only |
  | --- | --- | --- |
  | `openehr_ehr` | one row per health record | no |
  | `openehr_versioned_object` | one row per version container | no |
  | `openehr_version` | one row per committed version | **yes** |
  | `openehr_contribution` | one row per change set | **yes** |
  | `openehr_composition_index` | the RM-level projection of a composition | no |

  The set is small deliberately. Every table here corresponds to a Reference
  Model class that exists independently of any archetype; a sixth table would
  have to name a class that does not.

- **M3.22** The tables MUST be declared once, as data, in one place
  (`openehr_store::schema`), and every dialect MUST derive its DDL from that
  declaration. A dialect MUST NOT define a table, a column, or an index.
- **M3.23** `TABLES` MUST be ordered so that every foreign key points at a table
  earlier in the list. Emitting in list order then needs no deferred constraints,
  which Oracle and SQL Server make awkward. A test enforces this.

## Every instant occupies two columns

This is the requirement most likely to be "simplified" by someone who has not
read the reason, so the reason is here rather than in a commit message.

- **M3.24** An openEHR instant MUST be stored as **two** columns: `<name>_text`,
  holding the exact lexical form, and `<name>_utc`, holding a derived instant for
  ordering.
- **M3.25** `<name>_text` is **authoritative**. Every read that reconstructs an
  openEHR object MUST take the value from it, never from the derived column.
- **M3.26** `<name>_utc` MUST be nullable, and MUST be `NULL` whenever the
  instant is not established — a local time with no offset, or a date with no
  time.

  openEHR times are ISO 8601 **strings** with deliberate partial precision.
  `2024-05` is a date known to the month — a birth date on a refugee's record, a
  diagnosis recalled as "sometime in May" — and it is not `2024-05-01`. A native
  timestamp column silently completes it, which fabricates a clinical fact, and
  normalises the lexical form, which breaks round-tripping (`lib:D3.9`,
  `lib:D3.10`).

  The derived column is `NULL` rather than guessed because that is the same
  answer `DateTime::diff_seconds` gives in Rust (`lib:D3.14`). A column that
  guessed would make SQL and the library disagree about one record, and only one
  of them would be right.

- **M3.27** A `<name>_text` column MUST NOT exist without its `<name>_utc`
  partner, and the partner MUST be typed as a derived instant and be nullable. A
  test asserts this over the whole schema, because the pairing is the kind of
  invariant that survives review and dies in a migration.
- **M3.28** Ordering and range scans MUST use the derived column. A version whose
  commit time is not an established instant is therefore **skipped** by
  time-ordered reads, exactly as `VersionedObject::version_at_time` skips it
  (`lib:V8.6`). Ordering on the lexical form would sort
  `2026-07-31T09:00:00+02:00` after `2026-07-31T08:30:00Z`, which is the wrong
  way round.

## Column types

- **M3.6** *(amended)* Logical column types are declared as `ColTy`, and each
  dialect maps them to its engine's spelling. The set is deliberately small:
  every variant is a type whose SQL spelling **differs** across the six engines.
  Anything that spells the same everywhere would not earn a variant.

  | `ColTy` | Holds |
  | --- | --- |
  | `Id(n)` | a short identifier — a UUID, a version id, an archetype id |
  | `Text(n)` | bounded free text — a name, a system id |
  | `LongText` | unbounded text |
  | `Json` | a canonical-JSON document |
  | `Instant` | an ISO 8601 instant in its exact lexical form — always text |
  | `InstantUtc` | a derived instant for ordering — nullable by construction |
  | `Int` | a whole number |
  | `Bool` | a truth value |

- **M3.29** `Id` and `Text` MUST carry a maximum length. MySQL and MariaDB cannot
  index an unbounded `VARCHAR`, and Oracle has no unbounded `VARCHAR2` at all;
  every `Id` column in this schema is a key or part of one.
- **M3.30** `ColTy` MUST NOT be `#[non_exhaustive]`, and a dialect MUST NOT carry
  a wildcard match arm.

  This inverts the usual advice deliberately. `non_exhaustive` forces every
  dialect to write `_ => …`, and a wildcard arm is exactly how a newly added
  logical type silently acquires some other type's SQL — which is the shape of
  the sibling monorepo's **F-08**. Adding a variant *should* break all six
  dialects, loudly, at compile time, so each one decides what its engine spells
  it as.

- **M3.31** `Instant` and `InstantUtc` MUST NOT map to the same SQL type in any
  dialect. They are the authoritative and the derived halves of `M3.24`, and a
  dialect that collapses them has discarded the distinction the schema exists to
  preserve. A cross-dialect test enforces this for all six.

## The composition index

- **M3.32** `openehr_composition_index` MUST carry only attributes the Reference
  Model fixes: the archetype id, the template id, the category, the composer, the
  language, the territory, the setting, and the context start and end.
- **M3.33** Projecting a composition onto its index row MUST fail if the
  composition is not an archetype root, because `archetype_id` is the column AQL
  filters on and there would be nothing to put in it. Every other absent
  attribute becomes `NULL`, which is a fact; an invented archetype id would not
  be.
- **M3.34** `composer_name` MUST be `NULL` for an anonymous `PARTY_SELF`
  committer. That is **not** missing data: an anonymous subject is a legitimate
  and deliberate representation (`lib:M5.16`), and a store writing `"unknown"`
  would turn a privacy decision into a data-quality problem.
- **M3.35** The projection MUST be one function, shared by every engine. An
  engine computing its own would eventually index a different `category`, and the
  difference would surface as a query returning different rows on different
  engines.

## Audit and immutability

- **M3.15** *(amended)* Every `openehr_version` row MUST carry the committing
  system, the change type, the committer's name where the party has one, and the
  commit time as an instant pair (`M3.24`). Every `openehr_contribution` row MUST
  carry the same audit attributes for the change set.
- **M3.17** *(amended)* `openehr_version` and `openehr_contribution` MUST be
  append-only **in the database**, not merely by convention in application code.
  Each dialect MUST emit statements that cause the engine itself to refuse
  `UPDATE` and `DELETE` on those tables.

  A guarantee enforced only in application code ends the first time somebody
  opens a SQL console. openEHR's whole change-control model rests on this
  (`lib:V8.10`): a correction is a new version, and a store permitting an
  `UPDATE` would let a correction erase what it corrected.

- **M3.36** A dialect MUST NOT leave an append-only table unenforced. The empty
  default on `append_only_sql` exists only so a half-written dialect compiles; it
  is not a permissible resting state, and `conformance::check_dialect` fails a
  dialect that inherits it.

  Three dialects inherited that default silently for as long as they existed
  (`A-15`) while the shared documentation described append-only as a property of
  the design. The check exists because a sentence did not.

- **M3.37** Where an engine offers a form of trigger replacement that does not
  drop first, a dialect SHOULD prefer it. MySQL must `DROP TRIGGER` before
  recreating, leaving an interval in which the table would accept an `UPDATE`;
  MariaDB's `CREATE OR REPLACE TRIGGER` does not. The window is confined to
  `install()` and is tolerable there, but it is a real difference and the crate
  documentation MUST say which form it uses.

- **M3.16** *(amended — **not implemented**)* The previous text required a
  tamper-evident hash chain over history rows, under two algorithms, with
  checkpoints, key retirement, and key generation.

  **No such chain exists in this repository.** `openehr_version` has no
  `prev_hash` column and no tag column; `openehr-store` does not reference the
  chain code at all. The `openehr` crate *does* implement the primitives —
  `security::audit_chain` provides `Chain`, `ChainEntry`, `ChainKey`, SHA-256
  digests, HMAC tags with constant-time comparison, and key material zeroized on
  drop — and `security::to_canonical_string` provides the canonical form a chain
  would commit to, which the store already uses for `data_json`.

  Were it wired up, the chain's digests would be governed by `M3.39`–`M3.42`:
  SHA-256, stored as 32 raw bytes.

  So the capability is built and unused. This requirement is recorded as
  **unimplemented** rather than deleted, because deleting it would erase a
  designed-for property the library is already carrying the cost of. A store MUST
  NOT claim tamper evidence until the columns exist and a test demonstrates that
  mutation is detected. Append-only enforcement (`M3.17`) is a different and
  weaker property: it stops the database's own `UPDATE` path, and proves nothing
  about a row rewritten by someone with file access.

- **M3.18** *(amended — **not implemented**)* Erasure under GDPR Art. 17 is the
  one sanctioned exception to append-only. No erasure operation exists here. A
  deployment needing one MUST NOT achieve it by disabling the append-only
  triggers, because that removes the guarantee for every other row at the same
  time.

## Digests

Wherever this project computes a digest — the tamper-evidence chain (`M3.16`), a
checksum adjunct for equality over a column an engine cannot compare (`U4`), or
anything added later — the algorithm and the stored representation are fixed
here rather than per use.

- **M3.39** A digest MUST be **SHA-256**.

  openEHR's own terminology names two integrity-check algorithms, SHA-1 and
  SHA-256. SHA-1 has practical collision attacks and MUST NOT be emitted; the
  `openehr` crate already refuses to. Naming one algorithm here, rather than
  leaving it to each use, is what makes a digest written by one part of the
  system verifiable by another.

- **M3.40** A stored digest MUST be **32 raw bytes in a binary column**. It MUST
  NOT be stored as hexadecimal text, base64, or any other textual encoding.

  Four reasons, in order of how much trouble each causes:

  1. **Hex text reintroduces string identity.** A 64-character hex string has a
     case (`ff` versus `FF`) and a collation, so equality depends on which
     collation the column happens to have — and this project already spends a
     whole document (`P6.6`) on the cost of two definitions of "the same
     string". A binary column compares as bytes, and bytes have no collation,
     no case, and no locale.
  2. **It doubles the column and the index.** 64 bytes against 32, on a column
     whose entire purpose is to be indexed and compared.
  3. **It invites a partial comparison.** Hex is human-readable, so it gets
     truncated in logs, compared with `LIKE`, and prefix-matched. A digest
     compared on a prefix is not a digest.
  4. **It is not the form the algorithm produces.** Encoding on write and
     decoding on read is two conversions that can disagree; the raw output has
     no such step.

  Hex remains correct for *display* — an error message, a CLI, a log line that
  has already established the value is not PHI. The requirement is about
  storage.

- **M3.41** A digest column MUST be fixed-width at exactly 32 bytes where the
  engine can express that, so a wrong-length value is rejected by the column
  rather than discovered during comparison.

- **M3.42** Comparing digests MUST be a byte-equality test on the full 32 bytes.
  A digest match alone MUST NOT be treated as proof the underlying values are
  equal: it is one collision away from returning another patient's record, and
  the comparison MUST be confirmed against the source value (`U6`).

### Engine bindings

Non-normative; the annexes govern (`X15.6`).

| Engine | 32-byte binary column |
| --- | --- |
| PostgreSQL | `bytea` |
| SQLite | `BLOB` |
| MySQL, MariaDB | `BINARY(32)` |
| SQL Server | `binary(32)` |
| Oracle | `RAW(32)` |

**Not implemented.** No digest is stored anywhere in this schema today: the
chain of `M3.16` does not exist, and the checksum adjunct belongs to a
text-search feature this layer does not have. Implementing either requires a new
`ColTy` variant, which by design will fail to compile in all six dialects until
each one states its own spelling (`M3.30`) — that break is the mechanism, not an
obstacle.

## Errors

- **M3.38** A store error MUST NOT echo stored content. It may name identifiers,
  tables, and rules; it MUST NOT name a value from a record.

  This is `lib:X11.7` inherited, and it matters more here than in the library: a
  store error is the one that reaches a connection-pool log, an APM trace, and a
  paging alert at once. Identifiers are design-time or system-minted rather than
  clinical, so naming one is safe and is the only way a caller can act
  (`lib:X11.7a`).

## Withdrawn

Withdrawn 2026-08-01. Numbers are retained and MUST NOT be reused (`C0.5`).

| Id | Was | Why withdrawn |
| --- | --- | --- |
| `M3.1` | a base table per resource type | no resource types; the record is one JSON document (`M3.19`) |
| `M3.2` | base-table system columns | superseded by `M3.21` |
| `M3.3` | a column per scalar element | shredding (`S1.5`) |
| `M3.4`, `M3.4a`, `M3.4b` | a child table per repeating element; the `ords` ordinal path | shredding |
| `M3.5` | splitting wide flattened expansions | shredding |
| `M3.6a`–`M3.6c` | `Numeric`, `TextC`, `Jsonb` binding rules | those `ColTy` variants do not exist; see `M3.6` as amended |
| `M3.7` | `CHECK (col IN …)` for required value sets | terminology is carried opaquely (`S1.9`) |
| `M3.8` | `value[x]` choice columns | a FHIR construct |
| `M3.9`, `M3.10` | `Reference` columns and cross-resource integrity | a FHIR construct; openEHR uses `OBJECT_REF` inside content |
| `M3.11`, `M3.12` | extension and primitive-extension leaf rows | a FHIR construct |
| `M3.13` | `contained` resources | a FHIR construct |
| `M3.14` | the datatype cycle | a FHIR type-graph concern |

---

Part of the [openEHR persistence specification](index.md).
