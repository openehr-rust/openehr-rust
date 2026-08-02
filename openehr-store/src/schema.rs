//! The relational storage model for openEHR.
//!
//! # Why this is not a shredded schema
//!
//! The sibling FHIR libraries in this repository shred resources into typed
//! columns and child tables, generated from the FHIR specification. That works
//! because FHIR fixes the shape of a `Patient` at specification time.
//!
//! openEHR does not. A `COMPOSITION` contains whatever its **archetype** says,
//! archetypes are authored after the software ships, and this crate does not
//! implement them (`S1.4`). A schema shredded from the Reference Model alone
//! would have one column per RM attribute and a generic key/value table for
//! everything clinically interesting — which is a document store with extra
//! joins.
//!
//! So: the canonical JSON **is** the record, and the relational part is an
//! *index* over the attributes the Reference Model does fix — who committed,
//! when, which archetype, which category, which setting. Those are exactly the
//! attributes a population query filters on before it reaches into content.
//!
//! # Two columns for every time, and this is the important one
//!
//! openEHR times are ISO 8601 **strings** with deliberate partial precision:
//! `2024-05` is a date known to the month and is not the same as `2024-05-01`
//! (`D3.9`). Storing them in a native `TIMESTAMP` column silently completes
//! them, which fabricates a clinical fact, and normalises the lexical form,
//! which breaks round-tripping (`D3.10`).
//!
//! Every time is therefore stored twice:
//!
//! | Column | Type | Role |
//! | --- | --- | --- |
//! | `…_text` | text | **authoritative** — the exact lexical form |
//! | `…_utc` | native timestamp, nullable | derived, for ordering and range scans |
//!
//! The derived column is `NULL` whenever the instant is not established — a
//! local time with no offset, or a date with no time — because that is the same
//! answer `DateTime::diff_seconds` gives, and a column that guessed would make
//! SQL disagree with the library about the same record.

use serde::Serialize;

/// A column's logical type, mapped to a concrete SQL type by a
/// [`crate::Dialect`].
///
/// Deliberately small. Every entry here is a type whose SQL spelling differs
/// across the six engines; anything that spells the same everywhere would not
/// earn a variant.
// Serialize only: the schema is compile-time data that a tool may want to
// dump for inspection, and nothing ever reads one back — a schema read from
// JSON would be a second source of truth.
//
// Deliberately **not** `#[non_exhaustive]`, which is the opposite of the usual
// advice. `non_exhaustive` would force every dialect to carry a `_` arm, and a
// `_` arm is exactly how a newly added logical type silently acquires some
// other type's SQL — which is the shape of the sibling FHIR monorepo's **F-08**
// (an Oracle emitter producing MySQL types). Adding a variant here *should*
// break all six dialects, loudly, at compile time, so that each one decides
// what its engine spells it as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum ColTy {
    /// A short identifier — a UUID, a version id, an archetype id.
    ///
    /// Carries its maximum length because `MySQL` cannot index an unbounded
    /// `VARCHAR` and Oracle has no unbounded `VARCHAR2` at all.
    Id(u16),
    /// Free text of bounded length: a name, a system id.
    Text(u16),
    /// Unbounded text.
    LongText,
    /// A canonical-JSON document.
    Json,
    /// An ISO 8601 instant in its **exact lexical form** — always text.
    Instant,
    /// A derived UTC instant for ordering. Nullable by construction.
    InstantUtc,
    /// A whole number.
    Int,
    /// A truth value.
    Bool,
    /// A SHA-256 digest: **32 raw bytes in a binary column** (`M3.39`-`M3.42`).
    ///
    /// Never hexadecimal text. Hex reintroduces string identity — a 64-character
    /// value has a case and a collation, so equality would depend on which
    /// collation the column happens to have — and doubles the width of a column
    /// whose whole purpose is to be compared.
    Digest,
}

/// One column.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Column {
    /// The column name.
    pub name: &'static str,
    /// Its logical type.
    pub ty: ColTy,
    /// Whether `NULL` is permitted.
    pub nullable: bool,
    /// Why the column exists, emitted as a SQL comment where the dialect has
    /// them. A schema nobody can read is a schema somebody will guess at.
    pub note: &'static str,
}

impl Column {
    /// A non-nullable column.
    #[must_use]
    pub const fn required(name: &'static str, ty: ColTy, note: &'static str) -> Self {
        Self {
            name,
            ty,
            nullable: false,
            note,
        }
    }

    /// A nullable column.
    #[must_use]
    pub const fn optional(name: &'static str, ty: ColTy, note: &'static str) -> Self {
        Self {
            name,
            ty,
            nullable: true,
            note,
        }
    }
}

/// A foreign key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ForeignKey {
    /// The referring column.
    pub column: &'static str,
    /// The referenced table.
    pub table: &'static str,
    /// The referenced column.
    pub references: &'static str,
}

/// An index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Index {
    /// The index name, unique across the schema.
    pub name: &'static str,
    /// The indexed columns, in order.
    pub columns: &'static [&'static str],
    /// Whether the index enforces uniqueness.
    pub unique: bool,
    /// Which query the index exists for. An index whose query nobody recorded
    /// is an index nobody dares drop.
    pub note: &'static str,
}

/// One table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Table {
    /// The table name.
    pub name: &'static str,
    /// What the table holds.
    pub note: &'static str,
    /// Its columns, in declaration order.
    pub columns: &'static [Column],
    /// The primary key columns.
    pub primary_key: &'static [&'static str],
    /// Foreign keys.
    pub foreign_keys: &'static [ForeignKey],
    /// Indexes.
    pub indexes: &'static [Index],
    /// Whether rows may ever be updated or deleted.
    ///
    /// `version` is append-only: openEHR's whole change-control model rests on
    /// it (`V8.10`), and a store that permitted an `UPDATE` would let a
    /// correction erase what it corrected.
    pub append_only: bool,
}

/// The `ehr` table: one row per health record.
pub const EHR: Table = Table {
    name: "openehr_ehr",
    note: "One row per EHR. Holds only what the EHR class itself fixes; \
           everything else is a reference, as in the model (E6.1).",
    columns: &[
        Column::required("ehr_id", ColTy::Id(255), "HIER_OBJECT_ID of the record"),
        Column::required("system_id", ColTy::Id(255), "the system managing it"),
        Column::required(
            "time_created_text",
            ColTy::Instant,
            "authoritative: the exact ISO 8601 form",
        ),
        Column::optional(
            "time_created_utc",
            ColTy::InstantUtc,
            "derived for ordering; NULL when the instant is not established",
        ),
        Column::required(
            "ehr_status_uid",
            ColTy::Id(255),
            "versioned object holding EHR_STATUS",
        ),
        Column::required(
            "ehr_access_uid",
            ColTy::Id(255),
            "versioned object holding EHR_ACCESS",
        ),
    ],
    primary_key: &["ehr_id"],
    foreign_keys: &[],
    indexes: &[],
    append_only: false,
};

/// The `versioned_object` table: one row per version container.
pub const VERSIONED_OBJECT: Table = Table {
    name: "openehr_versioned_object",
    note: "One row per VERSIONED_OBJECT. `rm_type` says what the versions \
           contain — COMPOSITION, EHR_STATUS, FOLDER — because openEHR versions \
           all of them the same way.",
    columns: &[
        Column::required("uid", ColTy::Id(255), "HIER_OBJECT_ID of the container"),
        Column::required("ehr_id", ColTy::Id(255), "owning record"),
        Column::required(
            "rm_type",
            ColTy::Id(64),
            "COMPOSITION | EHR_STATUS | EHR_ACCESS | FOLDER",
        ),
        Column::required(
            "time_created_text",
            ColTy::Instant,
            "authoritative lexical form",
        ),
        Column::optional("time_created_utc", ColTy::InstantUtc, "derived"),
    ],
    primary_key: &["uid"],
    foreign_keys: &[ForeignKey {
        column: "ehr_id",
        table: "openehr_ehr",
        references: "ehr_id",
    }],
    indexes: &[Index {
        name: "ix_versioned_object_ehr",
        columns: &["ehr_id", "rm_type"],
        unique: false,
        note: "list a record's compositions without scanning every version",
    }],
    append_only: false,
};

/// The `version` table: one row per committed version. **Append-only.**
pub const VERSION: Table = Table {
    name: "openehr_version",
    note: "One row per VERSION. Append-only: a correction is a new row, and the \
           row it corrects stays (V8.10). The version identity is stored \
           decomposed because the commit rules are checked on its parts (V8.1).",
    columns: &[
        Column::required(
            "uid",
            ColTy::Id(255),
            "full OBJECT_VERSION_ID, object::system::tree",
        ),
        Column::required("versioned_object_uid", ColTy::Id(255), "container"),
        Column::required(
            "creating_system_id",
            ColTy::Id(255),
            "the middle part of the version id — what keeps two offline systems' \
             version 2 distinct",
        ),
        Column::required("trunk_version", ColTy::Int, "version tree trunk number"),
        Column::optional("branch_number", ColTy::Int, "NULL on the trunk"),
        Column::optional("branch_version", ColTy::Int, "NULL on the trunk"),
        Column::optional(
            "preceding_version_uid",
            ColTy::Id(255),
            "NULL only for the first version (V8.3)",
        ),
        Column::required(
            "lifecycle_state_code",
            ColTy::Id(16),
            "openEHR version_lifecycle_state code",
        ),
        Column::required(
            "is_deleted",
            ColTy::Bool,
            "derived from lifecycle_state; indexed so 'current content' does not \
             need a code comparison",
        ),
        Column::required("contribution_uid", ColTy::Id(255), "the change set"),
        Column::required(
            "audit_system_id",
            ColTy::Text(255),
            "AUDIT_DETAILS.system_id",
        ),
        Column::required(
            "audit_change_type_code",
            ColTy::Id(16),
            "openEHR audit_change_type code",
        ),
        Column::optional(
            "audit_committer_name",
            ColTy::Text(255),
            "NULL for an anonymous PARTY_SELF committer — which is legitimate \
             (M5.16), not missing data",
        ),
        Column::required(
            "audit_time_committed_text",
            ColTy::Instant,
            "authoritative lexical form",
        ),
        Column::optional(
            "audit_time_committed_utc",
            ColTy::InstantUtc,
            "derived; NULL when the commit time carries no UTC offset",
        ),
        Column::optional(
            "data_json",
            ColTy::Json,
            "canonical JSON of the version's content; NULL only when the version \
             is a logical deletion (V8.9)",
        ),
        // D-07: openEHR gives VERSION and AUDIT_DETAILS four optional
        // attributes that had no column. The store accepted them and dropped
        // them silently, which for an attestation means losing the part of the
        // record that made it evidence.
        Column::optional(
            "audit_description",
            ColTy::LongText,
            "AUDIT_DETAILS.description — the free-text reason for a change, often \
             the only record of why a correction exists",
        ),
        Column::optional(
            "signature",
            ColTy::LongText,
            "VERSION.signature — carried, never verified (S1.11)",
        ),
        Column::optional(
            "attestations_json",
            ColTy::Json,
            "ORIGINAL_VERSION.attestations, canonical JSON; NULL when there are \
             none. A clinician's assertion that content is what they signed off",
        ),
        Column::optional(
            "other_input_version_uids_json",
            ColTy::Json,
            "ORIGINAL_VERSION.other_input_version_uids, canonical JSON; NULL when \
             this version is not a merge",
        ),
        // D-03: the tamper-evidence chain (M3.16). Chained per container, in
        // version-tree order — see the module header for what that does and
        // does not detect.
        Column::required(
            "chain_previous",
            ColTy::Digest,
            "digest of the preceding version's chain entry, or the genesis digest",
        ),
        Column::required(
            "chain_content",
            ColTy::Digest,
            "SHA-256 over the canonical form of this version's content",
        ),
        Column::required(
            "chain_digest",
            ColTy::Digest,
            "this entry's own digest, over previous || content || uid",
        ),
        Column::optional(
            "chain_tag_key_id",
            ColTy::Text(255),
            "which key produced the tag; NULL when the chain is unkeyed",
        ),
        Column::optional(
            "chain_tag_mac",
            ColTy::Digest,
            "HMAC-SHA-256 over the same pre-image; NULL when unkeyed. An unkeyed \
             digest over a published pre-image is reproducible by anyone who can \
             write the rows it covers",
        ),
    ],
    primary_key: &["uid"],
    foreign_keys: &[ForeignKey {
        column: "versioned_object_uid",
        table: "openehr_versioned_object",
        references: "uid",
    }],
    indexes: &[
        Index {
            name: "ix_version_container_trunk",
            columns: &[
                "versioned_object_uid",
                "trunk_version",
                "branch_number",
                "branch_version",
            ],
            unique: true,
            note: "one row per position in a version tree; also the uniqueness \
                   that makes a duplicate commit fail in the database and not \
                   only in the library (V8.2)",
        },
        Index {
            name: "ix_version_time",
            columns: &["versioned_object_uid", "audit_time_committed_utc"],
            unique: false,
            note: "version_at_time without scanning a container's whole history \
                   (V8.6)",
        },
        Index {
            name: "ix_version_preceding",
            columns: &["preceding_version_uid"],
            unique: false,
            note: "walk a version tree forwards",
        },
    ],
    append_only: true,
};

/// The `contribution` table.
pub const CONTRIBUTION: Table = Table {
    name: "openehr_contribution",
    note: "One row per CONTRIBUTION — the unit a user recognises as 'I saved \
           the consultation', which is one change set over several versions.",
    columns: &[
        Column::required("uid", ColTy::Id(255), "HIER_OBJECT_ID"),
        Column::required("ehr_id", ColTy::Id(255), "owning record"),
        Column::required(
            "audit_change_type_code",
            ColTy::Id(16),
            "restricted to creation | amendment | deleted (V8.15)",
        ),
        Column::required("audit_system_id", ColTy::Text(255), ""),
        Column::optional("audit_committer_name", ColTy::Text(255), ""),
        Column::required("audit_time_committed_text", ColTy::Instant, "authoritative"),
        Column::optional("audit_time_committed_utc", ColTy::InstantUtc, "derived"),
    ],
    primary_key: &["uid"],
    foreign_keys: &[ForeignKey {
        column: "ehr_id",
        table: "openehr_ehr",
        references: "ehr_id",
    }],
    indexes: &[Index {
        name: "ix_contribution_ehr_time",
        columns: &["ehr_id", "audit_time_committed_utc"],
        unique: false,
        note: "a record's change history in commit order",
    }],
    append_only: true,
};

/// The `composition_index` table: the queryable projection of a composition.
pub const COMPOSITION_INDEX: Table = Table {
    name: "openehr_composition_index",
    note: "The RM-level projection of a COMPOSITION version. Every column here \
           is an attribute the Reference Model fixes, so it can be indexed \
           without an archetype (see the module header). Anything archetype-\
           defined stays in the JSON.",
    columns: &[
        Column::required("version_uid", ColTy::Id(255), "the version indexed"),
        Column::required("ehr_id", ColTy::Id(255), "owning record"),
        Column::required(
            "archetype_id",
            ColTy::Id(255),
            "COMPOSITION.archetype_details.archetype_id — the commonest AQL \
             predicate there is",
        ),
        Column::optional("template_id", ColTy::Id(255), "if a template was used"),
        Column::required(
            "category_code",
            ColTy::Id(16),
            "persistent | event | episodic | report",
        ),
        Column::optional("composer_name", ColTy::Text(255), "NULL when anonymous"),
        Column::required("language_code", ColTy::Id(32), "ISO 639-1"),
        Column::required("territory_code", ColTy::Id(32), "ISO 3166-1"),
        Column::optional("setting_code", ColTy::Id(16), "EVENT_CONTEXT.setting"),
        Column::optional(
            "context_start_text",
            ColTy::Instant,
            "authoritative lexical form",
        ),
        Column::optional("context_start_utc", ColTy::InstantUtc, "derived"),
        Column::optional("context_end_text", ColTy::Instant, "authoritative"),
        Column::optional("context_end_utc", ColTy::InstantUtc, "derived"),
    ],
    primary_key: &["version_uid"],
    foreign_keys: &[ForeignKey {
        column: "version_uid",
        table: "openehr_version",
        references: "uid",
    }],
    indexes: &[
        Index {
            name: "ix_composition_archetype",
            columns: &["ehr_id", "archetype_id"],
            unique: false,
            note: "AQL's `CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.x.v1]`",
        },
        Index {
            name: "ix_composition_context_start",
            columns: &["ehr_id", "context_start_utc"],
            unique: false,
            note: "encounters in a date range — the second commonest AQL filter",
        },
    ],
    append_only: false,
};

/// The schema version this build of the crate writes and expects.
///
/// Bumped whenever `TABLES` changes in a way an existing database cannot serve:
/// a new column, a changed type, a new constraint. Not bumped for a comment.
///
/// It exists because `install()` is `CREATE TABLE IF NOT EXISTS`. Against a
/// database built by an older version, every statement no-ops, `install()`
/// returns `Ok`, and the first commit fails on a column that is not there —
/// success followed by an unexplained failure, which is the shape this project
/// refuses everywhere else.
///
/// `4` since 2026-08-02: `ColTy::Json` moved off normalizing JSON column types
/// (`M3.43`, `D-08`). No column was added or removed, but on `PostgreSQL` and
/// `MySQL` the *type* changed, and a database built under `3` returns bytes the
/// content digest cannot be recomputed from.
pub const SCHEMA_VERSION: i64 = 4;

/// The `schema_version` table: what shape this database is in.
///
/// The sixth table, and not an openEHR class. `M3.21` fixes five tables because
/// each corresponds to a Reference Model class; this one holds no clinical data
/// and exists so a deployment can be told its database is the wrong shape
/// instead of discovering it mid-commit (`O10.14`).
pub const SCHEMA_VERSION_TABLE: Table = Table {
    name: "openehr_schema_version",
    note: "One row. The schema version this database was installed under, so a \
           mismatched binary refuses rather than half-working.",
    columns: &[
        Column::required("version", ColTy::Int, "matches SCHEMA_VERSION"),
        Column::required(
            "applied_text",
            ColTy::Instant,
            "authoritative: when this version was applied",
        ),
        Column::optional("applied_utc", ColTy::InstantUtc, "derived"),
    ],
    primary_key: &["version"],
    foreign_keys: &[],
    indexes: &[],
    append_only: false,
};

/// Every table, in dependency order: a table's foreign keys always point at a
/// table earlier in this list, so emitting them in order needs no deferral.
pub const TABLES: &[Table] = &[
    SCHEMA_VERSION_TABLE,
    EHR,
    VERSIONED_OBJECT,
    VERSION,
    CONTRIBUTION,
    COMPOSITION_INDEX,
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_table_is_internally_consistent() {
        let mut names = HashSet::new();
        for table in TABLES {
            assert!(names.insert(table.name), "duplicate table {}", table.name);
            let columns: HashSet<&str> = table.columns.iter().map(|c| c.name).collect();
            assert_eq!(
                columns.len(),
                table.columns.len(),
                "duplicate column in {}",
                table.name
            );
            for key in table.primary_key {
                assert!(
                    columns.contains(key),
                    "{}: pk {key} is not a column",
                    table.name
                );
            }
            for fk in table.foreign_keys {
                assert!(
                    columns.contains(fk.column),
                    "{}: fk {} is not a column",
                    table.name,
                    fk.column
                );
            }
            for index in table.indexes {
                for column in index.columns {
                    assert!(
                        columns.contains(column),
                        "{}: index {} names {column}, which is not a column",
                        table.name,
                        index.name
                    );
                }
            }
        }
    }

    /// Whether every one of the six engines can index **and** `=` compare this
    /// type directly, with no adjunct.
    ///
    /// An exhaustive match with no wildcard, for `M3.30`'s reason: adding a
    /// `ColTy` should force a decision here too. A new type is not searchable
    /// until someone says which engines can search it, and the compiler is what
    /// makes that a decision rather than an oversight.
    //
    // The `true` arms stay separate although they agree. Each is `true` for its
    // own reason — bounded by construction, fixed width, a native scalar — and
    // merging them would delete three reasons to leave one fact, so that a type
    // whose reason later stops holding would move with the others. Same trade as
    // the dialects' `col_sql`.
    #[allow(clippy::match_same_arms)]
    const fn every_engine_can_search(ty: ColTy) -> bool {
        match ty {
            // Bounded by construction (`M3.29`), and bounded character columns
            // index and compare everywhere.
            ColTy::Id(_) | ColTy::Text(_) => true,
            // Fixed width, and never hexadecimal text (`M3.40`).
            ColTy::Digest => true,
            ColTy::Instant | ColTy::InstantUtc | ColTy::Int | ColTy::Bool => true,
            // The two that are not. On SQL Server these cannot be indexed; on
            // Oracle a CLOB can be neither indexed nor `=` compared. Searching
            // one needs the adjuncts of `db:AD5`, and nothing emits an adjunct
            // (`db:P6.18`).
            ColTy::LongText | ColTy::Json => false,
        }
    }

    /// `db:P6.18` as a check rather than a sentence.
    ///
    /// The specification says no search target requires an adjunct today,
    /// because every indexed column is a type all six engines handle. That is
    /// true, and nothing stopped it from quietly stopping being true — an index
    /// added over `data_json` would compile, pass every other test, and produce
    /// a schema that cannot be searched on two of the six engines: a scan on
    /// SQL Server and an error on Oracle.
    ///
    /// `D-08` is why this is worth a test now rather than later. Canonical JSON
    /// moved onto a byte-preserving text type, which on Oracle is a `CLOB` — so
    /// the largest column in the schema became the one fewest engines can
    /// search, and it is exactly the column somebody will reach for first.
    #[test]
    fn every_indexed_column_is_one_every_engine_can_search() {
        for table in TABLES {
            for index in table.indexes {
                for name in index.columns {
                    let column = table
                        .columns
                        .iter()
                        .find(|c| c.name == *name)
                        .expect("checked by every_table_is_internally_consistent");
                    assert!(
                        every_engine_can_search(column.ty),
                        "{}: index {} covers {name}, whose type {:?} cannot be \
                         indexed or compared on every engine. Either index a \
                         different column or specify its adjuncts \
                         (spec/databases/search-adjuncts.md AD16), and note that \
                         no adjunct is emitted anywhere yet (db:P6.18).",
                        table.name,
                        index.name,
                        column.ty
                    );
                }
            }
        }
    }

    /// `db:P6.13`: an index whose query nobody recorded is one nobody dares
    /// drop.
    #[test]
    fn every_index_records_the_query_it_exists_for() {
        for table in TABLES {
            for index in table.indexes {
                assert!(
                    !index.note.trim().is_empty(),
                    "{}: index {} records no query",
                    table.name,
                    index.name
                );
            }
        }
    }

    #[test]
    fn foreign_keys_only_point_backwards() {
        // The property that lets `ddl()` emit tables in list order without
        // deferred constraints — which Oracle and SQL Server make awkward.
        let mut seen: HashSet<&str> = HashSet::new();
        for table in TABLES {
            for fk in table.foreign_keys {
                assert!(
                    seen.contains(fk.table) || fk.table == table.name,
                    "{} references {} before it is defined",
                    table.name,
                    fk.table
                );
            }
            seen.insert(table.name);
        }
    }

    #[test]
    fn every_instant_has_a_derived_partner_and_the_partner_is_nullable() {
        // The rule from the module header, made checkable: a `_text` column is
        // authoritative and required; its `_utc` partner is derived and must be
        // nullable, because the instant is not always established (D3.14).
        for table in TABLES {
            for column in table.columns {
                if let Some(stem) = column.name.strip_suffix("_text") {
                    let partner = format!("{stem}_utc");
                    let found = table
                        .columns
                        .iter()
                        .find(|c| c.name == partner)
                        .unwrap_or_else(|| {
                            panic!("{}: {} has no {partner}", table.name, column.name)
                        });
                    assert_eq!(found.ty, ColTy::InstantUtc);
                    assert!(found.nullable, "{}: {partner} must be nullable", table.name);
                    assert_eq!(column.ty, ColTy::Instant);
                }
            }
        }
    }

    // The assertions are on constants, which clippy notices. That is the point:
    // this test exists so that flipping one of those constants fails here, in a
    // test that names the consequence, rather than silently in six engines.
    #[allow(clippy::assertions_on_constants)]
    #[test]
    fn the_version_table_is_append_only() {
        // Stated as data rather than as prose, so a dialect can emit whatever
        // its engine offers to enforce it.
        assert!(VERSION.append_only);
        assert!(CONTRIBUTION.append_only);
        assert!(!EHR.append_only, "an EHR's status references do change");
    }
}
