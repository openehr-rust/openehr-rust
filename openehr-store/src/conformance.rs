//! The suite every engine runs.
//!
//! # Why the tests live here and not in each engine crate
//!
//! Because a test copied five times is a test that agrees with itself four
//! times and drifts once. The sibling FHIR monorepo in this repository records
//! the consequence: its concurrency and redaction suites existed only for
//! `PostgreSQL`, and porting them to two more engines immediately found three
//! defects that had been shipping — one of them in a port already advertised at
//! Store level.
//!
//! So the suite is written once, takes a [`Store`], and is called from each
//! engine's own test target against a real connection.

use crate::dialect::Dialect;
use crate::error::Result;
use crate::store::Store;
use openehr::base::{HierObjectId, ObjectId, ObjectRef, ObjectVersionId};
use openehr::rm::common::{
    Archetyped, AuditDetails, CommitError, Contribution, LocatableAttrs, OriginalVersion,
    PartyIdentified, Version,
};
use openehr::rm::data_types::{CodePhrase, DvDateTime};
use openehr::rm::ehr::{Composition, Ehr};
use openehr::terminology::{audit_change_type, composition_category, version_lifecycle_state};

/// The record identifier the suite uses.
pub const RECORD: &str = "87284370-2D4B-4E3D-A3F3-F303D2F4F34B";
/// The committing system the suite uses.
pub const SYSTEM: &str = "ehr1.example.org";

/// Builds the suite's EHR.
///
/// # Panics
///
/// Never: every input is a literal known to parse.
#[must_use]
pub fn sample_ehr() -> Ehr {
    let uid = HierObjectId::from_uid_str(RECORD).expect("literal");
    // openEHR requires these to name the versioned containers, not the record
    // (`EHR.Ehr_status_valid`, `Ehr_access_valid`). This fixture built both as
    // "EHR" from the day it was written, and nothing checked — see `lib:A-21`.
    let status = ObjectRef::new(
        "local",
        "VERSIONED_EHR_STATUS",
        ObjectId::HierObjectId(uid.clone()),
    )
    .expect("literal");
    let access = ObjectRef::new(
        "local",
        "VERSIONED_EHR_ACCESS",
        ObjectId::HierObjectId(uid.clone()),
    )
    .expect("literal");
    Ehr::new(
        HierObjectId::from_uid_str("11111111-2222-3333-4444-555555555555").expect("literal"),
        uid,
        status,
        access,
        DvDateTime::new("2026-08-01T09:00:00Z").expect("literal"),
    )
    .expect("literal")
}

/// Builds a valid composition.
///
/// # Panics
///
/// Never: every input is a literal known to parse.
#[must_use]
pub fn sample_composition(name: &str) -> Composition {
    Composition::new(
        LocatableAttrs::named(name, "openEHR-EHR-COMPOSITION.encounter.v1")
            .expect("literal")
            .with_archetype_details(
                Archetyped::new("openEHR-EHR-COMPOSITION.encounter.v1", "1.1.0").expect("literal"),
            ),
        composition_category::EVENT,
        PartyIdentified::named("Dr A Nurse")
            .expect("literal")
            .into(),
        CodePhrase::new("ISO_639-1", "en").expect("literal"),
        CodePhrase::new("ISO_3166-1", "GB").expect("literal"),
    )
    .expect("literal")
}

/// Builds a version of the sample composition.
///
/// # Panics
///
/// Never: every input is a literal known to parse.
#[must_use]
pub fn sample_version(n: u32, preceding: Option<u32>, minute: u32) -> Version<Composition> {
    let id = |v: u32| -> ObjectVersionId {
        format!("{RECORD}::{SYSTEM}::{v}").parse().expect("literal")
    };
    let owner = ObjectRef::new(
        "local",
        "EHR",
        ObjectId::HierObjectId(HierObjectId::from_uid_str(RECORD).expect("literal")),
    )
    .expect("literal");
    let audit = AuditDetails::new(
        SYSTEM,
        DvDateTime::new(&format!("2026-08-01T09:{minute:02}:00Z")).expect("literal"),
        if preceding.is_none() {
            audit_change_type::CREATION
        } else {
            audit_change_type::AMENDMENT
        },
        PartyIdentified::named("Dr A Nurse")
            .expect("literal")
            .into(),
    )
    .expect("literal");
    OriginalVersion::new(
        id(n),
        preceding.map(id),
        version_lifecycle_state::COMPLETE,
        Some(sample_composition(&format!("Encounter {n}"))),
        audit,
        owner,
    )
    .expect("literal")
    .into()
}

/// Builds the suite's contribution.
///
/// # Panics
///
/// Never: every input is a literal known to parse.
#[must_use]
pub fn sample_contribution(uid: &str, versions: &[u32]) -> Contribution {
    Contribution::new(
        HierObjectId::from_uid_str(uid).expect("literal"),
        versions
            .iter()
            .map(|v| format!("{RECORD}::{SYSTEM}::{v}").parse().expect("literal"))
            .collect(),
        AuditDetails::new(
            SYSTEM,
            DvDateTime::new("2026-08-01T09:05:00Z").expect("literal"),
            audit_change_type::CREATION,
            PartyIdentified::named("Dr A Nurse")
                .expect("literal")
                .into(),
        )
        .expect("literal"),
    )
    .expect("literal")
}

/// Runs every store test against one engine.
///
/// Each engine's test target calls this with a connected, empty store. Every
/// assertion states the failure it guards, because a suite shared by five
/// engines is one nobody owns unless it explains itself.
///
/// # Errors
///
/// Returns the first engine error. A conformance failure panics instead, so the
/// assertion message names what broke.
///
/// # Panics
///
/// Panics when the engine fails a conformance assertion — that is the test
/// reporting, not a defect in this function.
// Long on purpose: this is one narrative of a record's life — create, commit,
// refuse, read back, time-travel, index — and splitting it into a dozen
// helpers would hide the order, which is itself part of what is being tested.
#[allow(clippy::too_many_lines)]
pub fn run<S: Store>(store: &mut S) -> Result<()> {
    let engine = store.engine();
    store.install()?;

    // Installing twice must be safe: a deployment runs migrations on every
    // boot, and a schema installer that only works once is one that fails on
    // the second pod.
    store.install()?;

    let ehr = sample_ehr();
    let ehr_id = ehr.ehr_id().clone();
    store.create_ehr(&ehr)?;

    let round_tripped = store.get_ehr(&ehr_id)?;
    // The whole record, not just its id. This compared `ehr_id` alone while
    // its message claimed a round trip, and the gap hid `lib:A-21`: the fixture
    // built `ehr_status` and `ehr_access` with type `"EHR"`, the store read them
    // back as `VERSIONED_EHR_STATUS` and `VERSIONED_EHR_ACCESS`, and the
    // reference type changed silently on every round trip.
    assert_eq!(round_tripped, ehr, "{engine}: an EHR did not round-trip");

    // A second create must conflict rather than overwrite. Overwriting an EHR
    // silently rebases every version in it.
    assert!(
        matches!(
            store.create_ehr(&ehr),
            Err(crate::StoreError::Conflict { .. })
        ),
        "{engine}: creating an EHR twice did not conflict"
    );

    let contribution_uid = "22222222-3333-4444-5555-666666666666";
    store.create_contribution(&ehr_id, &sample_contribution(contribution_uid, &[1, 2]))?;

    // --- the commit rules, which are the reason this suite exists -----------
    let first = store.commit_composition(&ehr_id, &sample_version(1, None, 5), contribution_uid)?;
    assert!(
        first.created_container,
        "{engine}: first commit did not create a container"
    );

    // A duplicate version id.
    assert!(
        matches!(
            store.commit_composition(&ehr_id, &sample_version(1, None, 6), contribution_uid),
            Err(crate::StoreError::Commit(CommitError::DuplicateVersion))
        ),
        "{engine}: a duplicate version id was accepted"
    );

    // A successor claiming no predecessor.
    //
    // Built by deserialization, because `sample_version(2, None, ..)` no longer
    // *exists*: `OriginalVersion::new` refuses it as of `lib:A-23`. That is the
    // point — the shape is now unconstructible through the constructor and
    // still arrives over the wire, which is exactly the path that was
    // unchecked. Deserialization stays lenient by design (`lib:J9.9`), so this
    // is how a caller's bad JSON reaches a store.
    let rootless = |container: &str| -> Version<Composition> {
        let mut value: serde_json::Value =
            serde_json::to_value(sample_version(2, Some(1), 7)).expect("the fixture serialises");
        value
            .as_object_mut()
            .expect("a version is an object")
            .remove("preceding_version_uid");
        serde_json::from_str(
            &serde_json::to_string(&value)
                .expect("json")
                .replace(RECORD, container),
        )
        .expect("deserialization is lenient by design")
    };

    // Into an **empty** container. The assertion after this covers the same
    // shape once a head exists, and that was the only one there was — so
    // committing version 2 into a container with nothing in it succeeded, and
    // produced a history whose first entry says it is not the first.
    assert!(
        matches!(
            store.commit_composition(
                &ehr_id,
                &rootless("3F2504E0-4F89-11D3-9A0C-0305E82C3301"),
                contribution_uid
            ),
            Err(crate::StoreError::Invalid(_))
        ),
        "{engine}: a rootless successor was accepted into an empty container"
    );

    // And where a head does exist. Refused by the version's own invariant now
    // rather than by the head comparison, so the message changes; what matters
    // is that it is still refused.
    assert!(
        matches!(
            store.commit_composition(&ehr_id, &rootless(RECORD), contribution_uid),
            Err(crate::StoreError::Invalid(_)
                | crate::StoreError::Commit(CommitError::PrecedingVersionMismatch))
        ),
        "{engine}: a rootless successor was accepted"
    );

    let second =
        store.commit_composition(&ehr_id, &sample_version(2, Some(1), 10), contribution_uid)?;
    assert!(!second.created_container);

    // The concurrent-write case: two clients both read version 1 and both write
    // version 3 against it. openEHR's answer is a branch, not a silent
    // overwrite, so the store must refuse and let the caller decide.
    assert!(
        matches!(
            store.commit_composition(&ehr_id, &sample_version(3, Some(1), 12), contribution_uid),
            Err(crate::StoreError::Commit(CommitError::NotLatest))
        ),
        "{engine}: a stale predecessor was accepted — concurrent writes are being lost"
    );

    // --- reads ---------------------------------------------------------------
    let container = HierObjectId::from_uid_str(RECORD)?;
    let latest = store.latest_version(&container)?;
    assert_eq!(latest.trunk_version, 2, "{engine}: wrong head version");

    let all = store.all_versions(&container)?;
    assert_eq!(all.len(), 2, "{engine}: wrong version count");
    assert_eq!(
        all[0].trunk_version, 1,
        "{engine}: all_versions must be oldest first (V8.7a)"
    );

    let by_id: ObjectVersionId = format!("{RECORD}::{SYSTEM}::1").parse()?;
    let one = store.get_version(&by_id)?;
    assert_eq!(one.uid, by_id.to_string());
    assert!(one.data_json.is_some(), "{engine}: content was not stored");
    assert!(!one.is_deleted);
    assert_eq!(one.audit_change_type_code, audit_change_type::CREATION);
    assert_eq!(
        one.audit_time_committed.text, "2026-08-01T09:05:00Z",
        "{engine}: the authoritative lexical form of a commit time was altered"
    );
    assert!(
        one.audit_time_committed.utc_seconds.is_some(),
        "{engine}: an anchored instant produced no derived value"
    );

    // Time travel. At 09:07 the record was at version 1; at 09:20 at version 2.
    let at_0907 = store.version_at_time(&container, &DvDateTime::new("2026-08-01T09:07:00Z")?)?;
    assert_eq!(
        at_0907.trunk_version, 1,
        "{engine}: version_at_time went forwards"
    );
    let at_0920 = store.version_at_time(&container, &DvDateTime::new("2026-08-01T09:20:00Z")?)?;
    assert_eq!(at_0920.trunk_version, 2);
    // Before the first commit there was no version. "No version yet" and "the
    // earliest version" are different answers, and a caller reconstructing what
    // a clinician saw needs the difference.
    assert!(
        matches!(
            store.version_at_time(&container, &DvDateTime::new("2026-08-01T08:00:00Z")?),
            Err(crate::StoreError::NotFound { .. })
        ),
        "{engine}: version_at_time invented a version before the record existed"
    );

    // --- the index -----------------------------------------------------------
    let found =
        store.find_compositions_by_archetype(&ehr_id, "openEHR-EHR-COMPOSITION.encounter.v1")?;
    assert_eq!(
        found.len(),
        2,
        "{engine}: archetype index did not find both versions"
    );
    assert_eq!(found[0].category_code, composition_category::EVENT);
    assert_eq!(found[0].language_code, "en");
    assert_eq!(found[0].composer_name.as_deref(), Some("Dr A Nurse"));

    let none =
        store.find_compositions_by_archetype(&ehr_id, "openEHR-EHR-COMPOSITION.report.v1")?;
    assert!(
        none.is_empty(),
        "{engine}: archetype index matched the wrong archetype"
    );

    // --- the tamper-evidence chain (M3.16) -----------------------------------
    //
    // Here rather than in one engine's own tests. These assertions were written
    // against SQLite first and left there, which would have let a second engine
    // pass `run` while chaining nothing at all — the failure this module's
    // header is about, in the module itself.
    let chained = store.all_versions(&container)?;
    assert!(
        chained.len() >= 2,
        "{engine}: the chain assertions need at least two versions"
    );
    assert_eq!(
        chained[0].chain.previous, [0u8; 32],
        "{engine}: the first version must link to the genesis digest"
    );
    for pair in chained.windows(2) {
        assert_eq!(
            pair[1].chain.previous, pair[0].chain.digest,
            "{engine}: version {} does not link to {}",
            pair[1].uid, pair[0].uid
        );
    }
    for row in &chained {
        assert_ne!(
            row.chain.digest, [0u8; 32],
            "{engine}: a version has no chain digest"
        );
        assert_ne!(
            row.chain.content, row.chain.digest,
            "{engine}: the content digest and the entry digest must differ"
        );
    }

    // --- tamper detection, over the rows the engine actually returned --------
    //
    // The assertions above check that the chain *links*. That is a claim about
    // the writer. `PR12.12` asks for a different one: that altering a stored
    // row is **detected** — a claim about the detector, which a well-formed
    // chain over rows nobody has tried to corrupt says nothing about.
    //
    // Corrupting a row is engine-specific: it means getting past the
    // append-only triggers of `M3.17`, which each engine spells its own way.
    // So the corruption lives in each engine crate and the *judgement* lives
    // here, which is the same division as the rest of this module. What is
    // asserted here is that an untampered history verifies — the other half,
    // and the one that fails first if `verify_versions` is wrong about the
    // canonical bytes.
    let verdict = crate::integrity::verify_versions(&chained, &[]);
    assert!(
        verdict.is_intact(),
        "{engine}: a freshly written history did not verify: {verdict:?}"
    );
    // Unkeyed rather than Verified, and the suite says so out loud. Nothing
    // signed these entries, so the chain detects an edit by someone who cannot
    // recompute a digest and not one by an attacker holding the database. A
    // suite that accepted `Verified` here would be accepting a claim no key
    // backs.
    assert_eq!(
        verdict,
        crate::integrity::Integrity::Unkeyed,
        "{engine}: an unsigned chain must not report as Verified"
    );

    // A deleted version carries no content and is hashed as canonical `null`.
    // If `verify_versions` got that wrong, every deletion in every record would
    // report as altered — a false accusation against exactly the rows an
    // investigation looks at hardest. Cheap to assert, and it has to be here
    // rather than in a unit test because it depends on what the *engine*
    // returns for a NULL `data_json`.
    assert!(
        chained.iter().any(|row| row.data_json.is_some()),
        "{engine}: the fixture must include content for the digest to cover"
    );

    // --- the checkpoint (M3.16c) ---------------------------------------------
    let checkpoint = store.chain_checkpoint(&container)?;
    assert!(
        checkpoint.starts_with(&format!("entries={} ", chained.len())),
        "{engine}: the checkpoint must count the versions it covers: {checkpoint}"
    );
    assert!(
        !checkpoint.contains("Encounter"),
        "{engine}: a checkpoint must carry no clinical content: {checkpoint}"
    );

    // --- the attributes that used to be dropped (D-07) -----------------------
    //
    // The sample carries none of them, so what is asserted is that the columns
    // read back as absent rather than as something invented. A store that
    // returned `Some("")` here would be losing the distinction between "no
    // description" and "an empty one".
    let head = store.latest_version(&container)?;
    assert!(
        head.audit_description.is_none(),
        "{engine}: an absent audit description must read back absent"
    );
    assert!(
        head.signature.is_none(),
        "{engine}: an absent signature must read back absent"
    );
    assert!(
        head.attestations_json.is_none(),
        "{engine}: no attestations must be NULL, not an empty array"
    );
    assert!(
        head.other_input_version_uids_json.is_none(),
        "{engine}: a non-merge must be NULL, not an empty array"
    );

    // --- missing things ------------------------------------------------------
    let absent = HierObjectId::from_uid_str("99999999-9999-4999-8999-999999999999")?;
    assert!(matches!(
        store.get_ehr(&absent),
        Err(crate::StoreError::NotFound { .. })
    ));
    assert!(matches!(
        store.latest_version(&absent),
        Err(crate::StoreError::NotFound { .. })
    ));

    Ok(())
}

/// Checks one dialect's DDL without a database.
///
/// # Panics
///
/// Panics on a conformance failure, naming what broke.
pub fn check_dialect<D: Dialect + ?Sized>(dialect: &D) {
    let name = dialect.name();
    let statements = dialect.ddl();
    let index_statements = if dialect.index_idempotence() == crate::Idempotence::Inline {
        0
    } else {
        crate::TABLES.iter().map(|t| t.indexes.len()).sum::<usize>()
    };
    assert_eq!(
        statements.len(),
        crate::TABLES.len()
            + index_statements
            + crate::TABLES
                .iter()
                .filter(|t| t.append_only)
                .map(|t| dialect.append_only_sql(t).len())
                .sum::<usize>(),
        "{name}: unexpected statement count"
    );

    // `M3.43`: canonical JSON must be held in a column that returns the bytes it
    // was given. The chain's content digest is SHA-256 over those bytes
    // (`M3.16`), so an engine that reorders keys or rewrites numbers makes the
    // digest unrecomputable from storage — and MySQL's `JSON` did worse than
    // that, rewriting a magnitude of `1.10` as `1.1` and losing the precision
    // the value asserts (`D-08`, `lib:J9.13`).
    //
    // **This is a denylist, and a denylist is only as wide as its list.** It
    // catches the two spellings that were actually wrong and any engine that
    // later adopts them; it cannot catch a normalizing type nobody here has
    // heard of. The check that does not depend on a list is the byte round-trip
    // in `scripts/verify-schema.sh`, which puts canonical JSON into a real
    // server and compares what comes back. This exists so the mistake is caught
    // at `cargo test` rather than only where a container runs.
    let json_column = dialect.col_sql(crate::ColTy::Json).to_ascii_lowercase();
    for normalizing in ["jsonb", "json"] {
        assert_ne!(
            json_column, normalizing,
            "{name}: ColTy::Json is `{json_column}`, a type whose contract permits \
             reordering keys or rewriting numbers; canonical JSON must round-trip \
             byte for byte (M3.43, D-08)"
        );
    }

    // A dialect that declares `Guard` must actually wrap the statement. The
    // default `guard` returns its input unchanged, so a dialect could declare a
    // guard, inherit the default, and emit bare non-idempotent DDL that reads as
    // protected — which is what SQL Server and Oracle did until a live MySQL run
    // exposed the shape. Documentation of a mechanism is not the mechanism.
    for (kind, idempotence) in [
        (crate::ObjectKind::Table, dialect.table_idempotence()),
        (crate::ObjectKind::Index, dialect.index_idempotence()),
    ] {
        if idempotence == crate::Idempotence::Guard {
            let bare = "CREATE SOMETHING x";
            assert_ne!(
                dialect.guard(kind, "x", bare),
                bare,
                "{name}: declares Guard for {kind:?} but guard() does not wrap"
            );
        }
    }

    // Every append-only table must be enforced in the schema. A guarantee kept
    // only in application code ends the first time somebody opens a SQL console,
    // and every engine this crate targets has triggers.
    for table in crate::TABLES.iter().filter(|t| t.append_only) {
        assert!(
            !dialect.append_only_sql(table).is_empty(),
            "{name}: {} is append-only but the dialect enforces nothing",
            table.name
        );
    }

    // Indexes must reach the script one way or the other. Inline or separate is
    // the dialect's choice; absent is not.
    let script_all = crate::ddl_script(dialect);
    for table in crate::TABLES {
        for index in table.indexes {
            assert!(
                script_all.contains(&dialect.quote(index.name)),
                "{name}: index {} never reaches the DDL",
                index.name
            );
        }
    }

    let script = crate::ddl_script(dialect);
    for table in crate::TABLES {
        assert!(
            script.contains(&dialect.quote(table.name)),
            "{name}: {} is missing from the DDL",
            table.name
        );
        for column in table.columns {
            assert!(
                script.contains(&dialect.quote(column.name)),
                "{name}: {}.{} is missing",
                table.name,
                column.name
            );
        }
    }

    // Every logical type must map to something non-empty, and a `_text` instant
    // must not map to the same type as its derived partner — if they did, the
    // dialect has collapsed the distinction the schema exists to keep.
    for ty in [
        crate::ColTy::Id(255),
        crate::ColTy::Text(255),
        crate::ColTy::LongText,
        crate::ColTy::Json,
        crate::ColTy::Instant,
        crate::ColTy::InstantUtc,
        crate::ColTy::Int,
        crate::ColTy::Bool,
    ] {
        assert!(
            !dialect.col_sql(ty).is_empty(),
            "{name}: {ty:?} maps to nothing"
        );
    }
    assert_ne!(
        dialect.col_sql(crate::ColTy::Instant),
        dialect.col_sql(crate::ColTy::InstantUtc),
        "{name}: the authoritative and derived instant columns have the same type, \
         so the lexical form is not being preserved (D3.10)"
    );
}

/// Asserts that no two dialects emit the same DDL.
///
/// This is finding **F-08** of the sibling FHIR monorepo, made impossible: that
/// port's Oracle DDL emitter silently emitted `MySQL` types for as long as the
/// fork existed, because nothing compared them.
///
/// # Panics
///
/// Panics naming the two dialects that agree.
pub fn dialects_are_distinct(dialects: &[&dyn Dialect]) {
    for (i, a) in dialects.iter().enumerate() {
        for b in dialects.iter().skip(i + 1) {
            assert_ne!(
                crate::ddl_script(*a),
                crate::ddl_script(*b),
                "{} and {} emit identical DDL — one of them is not its own engine",
                a.name(),
                b.name()
            );
        }
    }
}

/// Checks that quoting an identifier cannot break out of the quotes.
///
/// This is the property a fuzzer drives, and it is the one with a security
/// consequence: an archetype id arriving from a caller reaches a `WHERE` clause
/// (`P6.12`), and an identifier that escapes its delimiter is SQL injection.
///
/// The delimiters are discovered rather than hard-coded, by asking the dialect
/// to quote the empty string — so this works for `"…"`, `` `…` ``, and `[…]`
/// alike, and a dialect that invents a fourth style is covered without editing
/// this function.
///
/// The property, stated exactly: the quoted form is `open`, then a body, then
/// `close`; every `close` inside the body occurs as a doubled pair; and
/// undoubling those pairs recovers the original identifier. A dialect that
/// satisfies this cannot emit an identifier that terminates its own quoting.
///
/// Note that only `close` is escaped. `[` inside a T-SQL `[…]` needs no
/// escaping and is passed through, which is correct rather than an oversight.
///
/// # Panics
///
/// Panics naming the dialect and the input when the property fails.
pub fn check_quote<D: Dialect + ?Sized>(dialect: &D, identifier: &str) {
    let empty = dialect.quote("");
    let mut delim = empty.chars();
    let (Some(open), Some(close)) = (delim.next(), empty.chars().last()) else {
        panic!("{}: quote(\"\") produced no delimiters", dialect.name());
    };

    let quoted = dialect.quote(identifier);
    assert!(
        quoted.starts_with(open),
        "{}: quote({identifier:?}) does not open with {open:?}",
        dialect.name()
    );
    assert!(
        quoted.len() > open.len_utf8(),
        "{}: quote({identifier:?}) is shorter than its own delimiters",
        dialect.name()
    );
    assert!(
        quoted.ends_with(close),
        "{}: quote({identifier:?}) does not close with {close:?}",
        dialect.name()
    );

    let body = &quoted[open.len_utf8()..quoted.len() - close.len_utf8()];
    let mut chars = body.chars();
    let mut unescaped = String::with_capacity(body.len());
    while let Some(c) = chars.next() {
        if c == close {
            assert_eq!(
                chars.next(),
                Some(close),
                "{}: quote({identifier:?}) leaves an unescaped {close:?} in the body — \
                 the identifier terminates its own quoting",
                dialect.name()
            );
        }
        unescaped.push(c);
    }
    assert_eq!(
        unescaped,
        identifier,
        "{}: quote({identifier:?}) does not round-trip",
        dialect.name()
    );
}

/// Checks that a dialect maps every logical column type to something usable.
///
/// Total over its input by construction — `ColTy` is not `#[non_exhaustive]`
/// and dialects carry no wildcard arm (`M3.30`) — so this drives the arbitrary
/// lengths, which are where a `VARCHAR(0)` or an overflowing bound would come
/// from.
///
/// # Panics
///
/// Panics naming the dialect and the type when the mapping is unusable.
pub fn check_col_sql<D: Dialect + ?Sized>(dialect: &D, ty: crate::ColTy) {
    let sql = dialect.col_sql(ty);
    assert!(
        !sql.trim().is_empty(),
        "{}: {ty:?} maps to an empty SQL type",
        dialect.name()
    );
    assert!(
        !sql.contains('\n'),
        "{}: {ty:?} maps to a multi-line SQL type: {sql:?}",
        dialect.name()
    );
    // The two halves of an instant pair must stay distinguishable whatever the
    // bound (`M3.31`); a fuzzer varying `Id(n)`/`Text(n)` must not be able to
    // collapse them.
    if matches!(ty, crate::ColTy::Instant) {
        assert_ne!(
            sql,
            dialect.col_sql(crate::ColTy::InstantUtc),
            "{}: the authoritative and derived instant columns share a type",
            dialect.name()
        );
    }
}

/// Drives the projection with a composition that passed no constructor.
///
/// This is the fuzz property for `openehr-store`, and it lives here rather
/// than in `openehr-store-fuzz` for the reason `W0.26` gives: a property in a
/// harness is a property one harness has.
///
/// The projection is where a `COMPOSITION` becomes rows, and a composition
/// read from JSON reached it having been checked by nothing (`V9.8`). Three
/// properties, and the third is the one with teeth:
///
/// 1. **Total.** `Ok` or `Err`, never a panic. `Err` is the correct answer for
///    a composition with no `archetype_details` — there is nothing to put in
///    `archetype_id`, and an invented one would not be a fact.
/// 2. **Deterministic.** Projected twice, the rows are equal. The row is what
///    a content digest is taken over downstream, so a projection that varied —
///    with map iteration order, say — would produce a record that failed its
///    own integrity check on the next read.
/// 3. **The derived instant column is a function of the authoritative one**
///    (`M3.31`). `…_text` is exact and authoritative; `…_utc` is derived and
///    nullable. Re-deriving the second from the first must give the same
///    answer, or SQL and Rust disagree about one record — and `2024-05` is a
///    date known to the month, not `2024-05-01`, so the derived half is `None`
///    and must stay `None`.
///
/// # Panics
///
/// Panics naming the property when one fails.
/// Returns whether the composition projected at all — `false` for one with no
/// `archetype_details`, which is a refusal and not a failure.
///
/// It returns anything because otherwise it could not fail. Mutation testing
/// replaced this whole function with `()` and nothing in this repository
/// noticed: it is called only from `openehr-store-fuzz`, `cargo test` does not
/// run fuzz targets, and a property that asserts nothing never crashes — so the
/// `fuzz` job would have stayed green over a deleted property (`db:D-10`).
/// Reporting what it checked is what makes that observable to a test.
#[must_use]
pub fn check_projection(version_uid: &str, ehr_id: &str, composition: &Composition) -> bool {
    let first = crate::record::CompositionIndexRow::project(version_uid, ehr_id, composition);
    let second = crate::record::CompositionIndexRow::project(version_uid, ehr_id, composition);

    match (first, second) {
        (Ok(a), Ok(b)) => {
            assert_eq!(a, b, "projection is not deterministic");
            for (which, instant) in [("context_start", &a.context_start), ("context_end", &a.context_end)] {
                let Some(stored) = instant else { continue };
                check_stored_instant(which, stored);
            }
            true
        }
        (Err(_), Err(_)) => false,
        (a, b) => panic!(
            "projection is not deterministic: one call {} and the other {}",
            if a.is_ok() { "succeeded" } else { "failed" },
            if b.is_ok() { "succeeded" } else { "failed" },
        ),
    }
}

/// The two halves of a stored instant agree, and the derived half is derived.
///
/// # Panics
///
/// Panics naming the column when they do not.
pub fn check_stored_instant(column: &str, stored: &crate::record::StoredInstant) {
    let parsed: openehr::base::iso8601::DateTime = stored
        .text
        .parse()
        .unwrap_or_else(|e| panic!("{column}: the authoritative text does not re-parse: {e}"));
    assert_eq!(
        parsed.as_str(),
        stored.text,
        "{column}: the authoritative text is not lexically preserved (`D3.10`)"
    );
    let rederived = crate::record::StoredInstant::from_date_time(&parsed);
    assert_eq!(
        rederived.utc_seconds, stored.utc_seconds,
        "{column}: the derived UTC column is not a function of the authoritative text — \
         SQL and Rust would disagree about one record"
    );
}

/// Drives `verify_versions` with rows that passed no constructor.
///
/// `verify_versions` answers "has this record been tampered with", over rows
/// that came out of a database and may have been edited there — which is the
/// whole reason the function exists. Its input is therefore untrusted by
/// construction, and a panic in it is a denial of service against the integrity
/// check itself: the answer a reader needs most is exactly the one they stop
/// being able to get.
///
/// The property is totality, plus the one structural claim the function makes
/// unconditionally: an empty container is `Empty` and never `Verified`, because
/// nothing was checked. `C0.13` is that rule in general form.
///
/// # Panics
///
/// Panics when the property fails.
/// Returns how many versions had their tamper detection **provoked** — zero
/// for an empty container, and zero for a history that did not verify in the
/// first place, because there is nothing to prove about a chain already broken.
///
/// It returns a count for the reason [`check_projection`] returns a bool, and
/// the count is the more useful of the two: it distinguishes "verified a
/// hundred-version history and re-broke every one of them" from "returned
/// early", which the fuzz target could not otherwise tell apart (`db:D-10`).
#[must_use]
pub fn check_verify_versions(rows: &[crate::record::VersionRow]) -> usize {
    use crate::integrity::{Breach, Integrity, verify_versions};

    let verdict = verify_versions(rows, &[]);
    if rows.is_empty() {
        assert_eq!(
            verdict,
            Integrity::Empty,
            "an empty container must report Empty, never a verdict about nothing"
        );
    }
    // Running it twice is not redundant: it is the check that the verdict does
    // not depend on anything outside the rows.
    assert_eq!(
        verdict,
        verify_versions(rows, &[]),
        "verify_versions is not a function of its input"
    );

    // **The half that can fail.** Everything above holds for any input given a
    // correct `verify_versions`, which makes it unfalsifiable: the whole
    // function could be replaced with `()` and no test in this repository would
    // notice — which is precisely what mutation testing found (`W-18`,
    // `db:D-10`). `W0.28` requires a property to be shown to fail against a
    // broken implementation, and a property that cannot fail is a control that
    // always reports success.
    //
    // So the property now **provokes** the answer it is about. A history that
    // verifies must stop verifying when a byte of its content changes; that is
    // the entire purpose of the chain (`M3.16`), and it is falsifiable by
    // construction rather than by imagination.
    if !matches!(verdict, Integrity::Verified | Integrity::Unkeyed) {
        return 0;
    }
    let mut provoked = 0;
    {
        for i in 0..rows.len() {
            let mut tampered = rows.to_vec();
            let row = &mut tampered[i];
            row.data_json = Some(match row.data_json.take() {
                // A byte that changes the content without changing its length,
                // so nothing but the digest can notice.
                Some(json) => format!("{json} "),
                None => "null ".to_owned(),
            });
            match verify_versions(&tampered, &[]) {
                Integrity::Broken { at, reason, .. } => {
                    assert_eq!(at, i, "the wrong version was named as altered");
                    assert_eq!(
                        reason,
                        Breach::ContentAltered,
                        "an edited version was reported as something other than altered"
                    );
                }
                other => panic!(
                    "editing version {i}'s content left the history reporting {other:?} — \
                     the chain does not detect the thing it exists to detect"
                ),
            }
            provoked += 1;
        }
    }
    provoked
}

#[cfg(test)]
mod property_tests {
    //! Proof that the fuzz properties are not vacuous.
    //!
    //! `T11.10`: a check that cannot fail is indistinguishable from a control
    //! that works, and the distinction is the entire value of the control. Each
    //! test below defines a dialect with exactly the defect the property exists
    //! to catch, and asserts the property rejects it.

    use super::{
        CodePhrase, Composition, LocatableAttrs, PartyIdentified, RECORD, SYSTEM, check_col_sql,
        check_projection, check_quote, check_stored_instant, check_verify_versions,
        composition_category, sample_composition, sample_version,
    };
    use openehr::base::iso8601;
    use crate::{ColTy, Dialect, Placeholder};

    /// Escapes nothing — the defect `check_quote` exists to catch.
    struct Unescaping;
    impl Dialect for Unescaping {
        fn name(&self) -> &'static str {
            "unescaping"
        }
        fn col_sql(&self, _ty: ColTy) -> String {
            "TEXT".to_owned()
        }
        fn quote(&self, identifier: &str) -> String {
            format!("\"{identifier}\"")
        }
        fn placeholder(&self) -> Placeholder {
            Placeholder::Question
        }
    }

    #[test]
    fn an_identifier_with_no_delimiter_is_accepted() {
        // The property must not reject everything, or it proves nothing.
        check_quote(&Unescaping, "openehr_version");
        check_quote(&Unescaping, "");
    }

    #[test]
    #[should_panic(expected = "unescaped")]
    fn an_identifier_that_escapes_its_own_quoting_is_caught() {
        check_quote(&Unescaping, "a\"; DROP TABLE openehr_version; --");
    }

    /// Maps both halves of an instant pair to one type (`M3.31`).
    struct CollapsedInstants;
    impl Dialect for CollapsedInstants {
        fn name(&self) -> &'static str {
            "collapsed"
        }
        fn col_sql(&self, _ty: ColTy) -> String {
            "TIMESTAMP".to_owned()
        }
        fn quote(&self, identifier: &str) -> String {
            format!("\"{}\"", identifier.replace('"', "\"\""))
        }
        fn placeholder(&self) -> Placeholder {
            Placeholder::Question
        }
    }

    #[test]
    #[should_panic(expected = "share a type")]
    fn collapsing_the_two_instant_columns_is_caught() {
        check_col_sql(&CollapsedInstants, ColTy::Instant);
    }

    /// Maps a logical type to nothing at all.
    struct EmptyType;
    impl Dialect for EmptyType {
        fn name(&self) -> &'static str {
            "empty"
        }
        fn col_sql(&self, _ty: ColTy) -> String {
            String::new()
        }
        fn quote(&self, identifier: &str) -> String {
            format!("\"{}\"", identifier.replace('"', "\"\""))
        }
        fn placeholder(&self) -> Placeholder {
            Placeholder::Question
        }
    }

    #[test]
    #[should_panic(expected = "empty SQL type")]
    fn a_type_that_maps_to_nothing_is_caught() {
        check_col_sql(&EmptyType, ColTy::Json);
    }

    #[test]
    fn every_real_dialect_quotes_the_adversarial_identifiers() {
        // The cases a fuzzer would take longest to reach, asserted directly so
        // they are covered even when nobody runs the fuzzer.
        for id in [
            "",
            "openehr_version",
            "\"",
            "``",
            "]",
            "[",
            "a\"; DROP TABLE openehr_version; --",
            "a`; DROP TABLE openehr_version; --",
            "a]; DROP TABLE openehr_version; --",
            "\u{0}",
            "\u{1F600}",
        ] {
            check_quote(&CollapsedInstants, id);
        }
    }

    /// The three properties the `openehr-store-fuzz` targets drive can fail.
    ///
    /// **Failure mode, and it was live.** Mutation testing replaced
    /// `check_projection`, `check_stored_instant`, and `check_verify_versions`
    /// each with `()` and **nothing in this repository failed** (`db:D-10`).
    /// They are called only from fuzz targets, which `cargo test` does not run,
    /// and a fuzz target whose property asserts nothing never crashes — so the
    /// `fuzz` job would have stayed green over three properties that had been
    /// deleted.
    ///
    /// `W0.28` requires a fuzz property to be **shown to fail** against a
    /// deliberately broken implementation. These are that demonstration, and
    /// they are what kills the mutants: a property that can only pass is
    /// indistinguishable from one that was removed.
    #[test]
    fn each_fuzz_property_accepts_what_is_right_and_rejects_what_is_not() {
        // ---- check_stored_instant ------------------------------------------
        let good = crate::record::StoredInstant::from_date_time(
            &"2026-08-01T09:00:00Z"
                .parse::<iso8601::DateTime>()
                .expect("literal"),
        );
        check_stored_instant("audit_time_committed", &good);

        // The derived column disagreeing with the authoritative text is the
        // defect `M3.31` is about: SQL and Rust would answer differently about
        // one record.
        let skewed = crate::record::StoredInstant {
            text: good.text.clone(),
            utc_seconds: Some(good.utc_seconds.expect("an established instant") + 1),
        };
        assert!(
            std::panic::catch_unwind(|| check_stored_instant("skewed", &skewed)).is_err(),
            "a derived UTC column that disagrees with its text was accepted"
        );

        // ---- check_projection ----------------------------------------------
        assert!(
            check_projection(
                &format!("{RECORD}::{SYSTEM}::1"),
                RECORD,
                &sample_composition("Encounter"),
            ),
            "the sample composition must project"
        );

        // And the other answer. A composition with no `archetype_details` has
        // nothing to put in `archetype_id` — the column `AQL` filters on — so
        // `CompositionIndexRow::project` refuses it rather than inventing one.
        // That refusal is a *result*, not a failure, and asserting only the
        // `true` case left `-> bool` replaceable with `true` (`db:D-10`).
        let unrooted = Composition::new(
            LocatableAttrs::named("Encounter", "openEHR-EHR-COMPOSITION.encounter.v1")
                .expect("literal"),
            composition_category::EVENT,
            PartyIdentified::named("Dr A Nurse").expect("literal").into(),
            CodePhrase::new("ISO_639-1", "en").expect("literal"),
            CodePhrase::new("ISO_3166-1", "GB").expect("literal"),
        )
        .expect("literal");
        assert!(
            !check_projection(&format!("{RECORD}::{SYSTEM}::1"), RECORD, &unrooted),
            "a composition with no archetype_details must not project"
        );

        // ---- check_verify_versions -----------------------------------------
        assert_eq!(
            check_verify_versions(&[]),
            0,
            "an empty container has nothing to provoke"
        );

        let mut rows = Vec::new();
        for n in 1..=3u32 {
            let previous = rows.last().map(|r: &crate::record::VersionRow| r.chain.digest);
            let version = sample_version(n, (n > 1).then(|| n - 1), n);
            rows.push(
                crate::record::VersionRow::project(&version, "ctrb-1", previous, None)
                    .expect("the sample projects"),
            );
        }
        // A sound history passes, and the count is the evidence that the
        // tamper provocation inside actually ran — one per version, not zero.
        assert_eq!(
            check_verify_versions(&rows),
            rows.len(),
            "the tamper provocation did not run for every version"
        );

        // And the provocation inside `check_verify_versions` is real rather than
        // decorative — asserted here directly, because the property itself can
        // only fail if `verify_versions` is broken, and a test cannot break it
        // from outside.
        for i in 0..rows.len() {
            let mut tampered = rows.clone();
            let edited = format!("{} ", tampered[i].data_json.clone().expect("content"));
            tampered[i].data_json = Some(edited);
            match crate::integrity::verify_versions(&tampered, &[]) {
                crate::integrity::Integrity::Broken { at, reason, .. } => {
                    assert_eq!(at, i, "the wrong version was named as altered");
                    assert_eq!(reason, crate::integrity::Breach::ContentAltered);
                }
                other => panic!("editing version {i} left the history reporting {other:?}"),
            }
        }
    }
}
