//! The shared conformance suite, run against a real `PostgreSQL` server.
//!
//! `#[ignore]`d, because — unlike `SqliteStore`, embedded and needing
//! nothing else — this needs a real server to connect to. Provisioning one
//! is `openehr-postgresql/scripts/verify-store.sh`, which stands up
//! `postgres:18-alpine` in a container with its port published to the host
//! (unlike `openehr-store/scripts/verify-schema.sh`'s own container, reached
//! only via `podman exec` from inside the container's own namespace — a real
//! client, not a shell, needs a real address), sets `OPENEHR_POSTGRESQL_URL`,
//! and tears the container down after:
//!
//! ```sh
//! sh openehr-postgresql/scripts/verify-store.sh
//! # or, against a server already running:
//! OPENEHR_POSTGRESQL_URL="host=127.0.0.1 port=5432 user=postgres password=... dbname=openehr" \
//!   cargo test -- --ignored
//! ```

use openehr_postgresql::PostgresqlStore;
use openehr_store::{Store as _, conformance};

fn connect() -> PostgresqlStore {
    let params = std::env::var("OPENEHR_POSTGRESQL_URL").expect(
        "set OPENEHR_POSTGRESQL_URL to a postgres connection string, or run \
         sh openehr-postgresql/scripts/verify-store.sh, which does both",
    );
    PostgresqlStore::connect(&params).expect("connect")
}

/// Drops every table this crate's own DDL creates, so each test starts from
/// the schema being absent rather than from whatever the previous test left
/// — the same "fresh database" precondition `SqliteStore`'s own tests get
/// for free from `SqliteStore::in_memory()`, since nothing here can open a
/// throwaway database per test the way an in-memory `SQLite` file can.
fn reset(store: &mut PostgresqlStore) {
    store
        .client()
        .batch_execute(
            "DROP TABLE IF EXISTS openehr_composition_index, openehr_version, \
             openehr_versioned_object, openehr_contribution, openehr_ehr, \
             openehr_schema_version CASCADE",
        )
        .expect("reset");
}

#[test]
#[ignore = "needs a real PostgreSQL server, see this file's own module doc"]
fn the_shared_suite_passes_against_a_real_database() {
    let mut store = connect();
    reset(&mut store);
    conformance::run(&mut store).expect("conformance suite");
}

#[test]
#[ignore = "needs a real PostgreSQL server, see this file's own module doc"]
fn the_ehr_status_suite_passes_against_a_real_database() {
    let mut store = connect();
    reset(&mut store);
    conformance::run_ehr_status(&mut store).expect("EHR_STATUS conformance suite");
}

#[test]
#[ignore = "needs a real PostgreSQL server, see this file's own module doc"]
fn the_is_modifiable_gate_suite_passes_against_a_real_database() {
    let mut store = connect();
    reset(&mut store);
    conformance::run_is_modifiable_gate(&mut store).expect("is_modifiable gate conformance suite");
}

/// Installing twice must be safe: a deployment runs migrations on every
/// boot. `SqliteStore`'s own suite checks this inline (`conformance::run`);
/// checked again here because `install`'s idempotence rests on
/// `CREATE TABLE IF NOT EXISTS`/`ON CONFLICT DO NOTHING`, both engine-
/// specific SQL this crate writes itself rather than inherits.
#[test]
#[ignore = "needs a real PostgreSQL server, see this file's own module doc"]
fn installing_twice_is_a_no_op() {
    let mut store = connect();
    reset(&mut store);
    store.install().expect("first install");
    store.install().expect("second install must not fail");
}

/// A database installed under a different schema version is refused, not
/// half-upgraded — `SqliteStore`'s own three-state check
/// (`check_schema_version`), ported rather than assumed to behave the same
/// against a real server.
#[test]
#[ignore = "needs a real PostgreSQL server, see this file's own module doc"]
fn a_database_from_another_schema_version_is_refused() {
    use openehr_store::StoreError;

    let mut store = connect();
    reset(&mut store);
    store.install().expect("fresh install");

    store
        .client()
        .execute("UPDATE openehr_schema_version SET version = 99", &[])
        .expect("rewrite version");

    match store.install() {
        Err(StoreError::SchemaVersionMismatch { found, expected }) => {
            assert_eq!(found, 99);
            assert_eq!(expected, openehr_store::SCHEMA_VERSION);
        }
        other => panic!("expected a version mismatch, got {other:?}"),
    }
}

/// The append-only triggers `PostgresqlDialect::append_only_sql` generates
/// actually fire against a real server — `openehr-store/scripts/
/// verify-schema.sh` already proves this at the DDL level (`RAISE
/// EXCEPTION` on `UPDATE`/`DELETE`); this proves it again through the
/// `Store`'s own write path, the shape `db:D-06`'s own concurrency test
/// cares about: a refusal a caller can tell apart from an ordinary error.
#[test]
#[ignore = "needs a real PostgreSQL server, see this file's own module doc"]
fn the_database_itself_refuses_to_mutate_a_version() {
    let mut store = connect();
    reset(&mut store);
    store.install().expect("install");
    let ehr = conformance::sample_ehr();
    store.create_ehr(&ehr).expect("create ehr");
    store
        .create_contribution(
            ehr.ehr_id(),
            &conformance::sample_contribution("22222222-3333-4444-5555-666666666666", &[1]),
        )
        .expect("contribution");
    store
        .commit_composition(
            ehr.ehr_id(),
            &conformance::sample_version(1, None, 5),
            "22222222-3333-4444-5555-666666666666",
        )
        .expect("commit");

    let update = store.client().execute(
        "UPDATE openehr_version SET lifecycle_state_code = '523'",
        &[],
    );
    assert!(update.is_err(), "an UPDATE on openehr_version succeeded");
    let delete = store.client().execute("DELETE FROM openehr_version", &[]);
    assert!(delete.is_err(), "a DELETE on openehr_version succeeded");

    let row = store
        .latest_version(&conformance::RECORD.parse().expect("literal"))
        .expect("still readable");
    assert_eq!(row.lifecycle_state_code, "532");
}

/// Fails if an invalid composition can reach storage. A store that accepted
/// one would make every later reader's `validate()` fail on data it cannot
/// fix. Ported from `openehr-sqlite`'s own test of the same name.
#[test]
#[ignore = "needs a real PostgreSQL server, see this file's own module doc"]
fn an_invalid_composition_is_refused_before_it_is_written() {
    use openehr::rm::common::{AuditDetails, LocatableAttrs, OriginalVersion, PartyIdentified};
    use openehr::rm::data_types::{CodePhrase, DvDateTime};
    use openehr::rm::ehr::Composition;
    use openehr::terminology::{audit_change_type, composition_category, version_lifecycle_state};

    let mut store = connect();
    reset(&mut store);
    store.install().expect("install");
    let ehr = conformance::sample_ehr();
    store.create_ehr(&ehr).expect("create ehr");
    store
        .create_contribution(
            ehr.ehr_id(),
            &conformance::sample_contribution("22222222-3333-4444-5555-666666666666", &[1]),
        )
        .expect("contribution");

    // No `archetype_details`, so not an archetype root (`E6.6a`).
    let rootless = Composition::new(
        LocatableAttrs::named("Encounter", "openEHR-EHR-COMPOSITION.encounter.v1").unwrap(),
        composition_category::EVENT,
        PartyIdentified::named("Dr A Nurse").unwrap().into(),
        CodePhrase::new("ISO_639-1", "en").unwrap(),
        CodePhrase::new("ISO_3166-1", "GB").unwrap(),
    )
    .unwrap();
    let owner = openehr::base::ObjectRef::new(
        "local",
        "EHR",
        openehr::base::ObjectId::HierObjectId(ehr.ehr_id().clone()),
    )
    .unwrap();
    let version = OriginalVersion::new(
        format!("{}::{}::1", conformance::RECORD, conformance::SYSTEM)
            .parse()
            .unwrap(),
        None,
        version_lifecycle_state::COMPLETE,
        Some(rootless),
        AuditDetails::new(
            conformance::SYSTEM,
            DvDateTime::new("2026-08-01T09:05:00Z").unwrap(),
            audit_change_type::CREATION,
            PartyIdentified::named("Dr A Nurse").unwrap().into(),
        )
        .unwrap(),
        owner,
    )
    .unwrap();

    let result = store.commit_composition(
        ehr.ehr_id(),
        &version.into(),
        "22222222-3333-4444-5555-666666666666",
    );
    assert!(
        matches!(result, Err(openehr_store::StoreError::Invalid(_))),
        "an invalid composition was accepted"
    );

    // And nothing was written — the refusal is before the transaction, not
    // inside a rolled-back one that left a container behind.
    let count: i64 = store
        .client()
        .query_one("SELECT COUNT(*) FROM openehr_versioned_object", &[])
        .expect("query")
        .get(0);
    assert_eq!(count, 0, "a refused commit left a container behind");
}

/// Fails if a stored instant loses its lexical form. The whole two-column
/// design exists for this (`D3.10`). Ported from `openehr-sqlite`'s own test
/// of the same name.
#[test]
#[ignore = "needs a real PostgreSQL server, see this file's own module doc"]
fn a_partial_or_offset_instant_survives_storage_verbatim() {
    let mut store = connect();
    reset(&mut store);
    store.install().expect("install");
    let ehr = conformance::sample_ehr();
    store.create_ehr(&ehr).expect("create");

    let read = store.get_ehr(ehr.ehr_id()).expect("read");
    assert_eq!(
        read.time_created().as_str(),
        ehr.time_created().as_str(),
        "the authoritative lexical form was altered by a round trip"
    );
}

/// The **derived** UTC value round-trips to the exact instant committed, not
/// merely to *some* value that happens to sort correctly.
///
/// Found worth adding by `cargo mutants`: every other test here orders or
/// filters on `audit_time_committed_utc` through SQL — `version_at_time`,
/// `all_versions`'s own ordering — which the database computes over the
/// *stored* column, unaffected by how `PostgresqlStore::
/// timestamptz_to_utc_seconds` converts it back into a `VersionRow` for a
/// caller. A mutant that made that conversion always return `Some(0)`
/// survived every other test in this file and was `MISSED` until this one
/// existed. `openehr_store::record::StoredInstant::from_date_time` — not
/// this crate's own code, so not what is under test — supplies the
/// independently-computed expectation to compare the round trip against.
#[test]
#[ignore = "needs a real PostgreSQL server, see this file's own module doc"]
fn the_derived_utc_instant_round_trips_to_the_exact_value_committed() {
    use openehr_store::record::StoredInstant;

    let mut store = connect();
    reset(&mut store);
    store.install().expect("install");
    let ehr = conformance::sample_ehr();
    store.create_ehr(&ehr).expect("ehr");
    let contribution = "22222222-3333-4444-5555-666666666666";
    store
        .create_contribution(
            ehr.ehr_id(),
            &conformance::sample_contribution(contribution, &[1]),
        )
        .expect("contribution");
    // `sample_version(1, None, 5)`'s audit time is `2026-08-01T09:05:00Z`.
    let expected = StoredInstant::from_date_time(&"2026-08-01T09:05:00Z".parse().expect("literal"))
        .utc_seconds;
    store
        .commit_composition(
            ehr.ehr_id(),
            &conformance::sample_version(1, None, 5),
            contribution,
        )
        .expect("commit");

    let row = store
        .latest_version(&conformance::RECORD.parse().expect("literal"))
        .expect("read back");
    assert_eq!(
        row.audit_time_committed.utc_seconds, expected,
        "the derived UTC instant did not round-trip to the value committed"
    );
}

/// `M3.16` / `D-03`: the chain links successive versions in a container, and
/// a mutation to stored content is detectable. Ported from
/// `openehr-sqlite`'s own test of the same name — the one difference is the
/// trigger name to drop: `PostgresqlDialect::append_only_sql` generates one
/// combined `BEFORE UPDATE OR DELETE` trigger per table, where `SQLite`'s own
/// dialect generates two.
#[test]
#[ignore = "needs a real PostgreSQL server, see this file's own module doc"]
fn the_chain_links_versions_and_notices_a_rewrite() {
    use openehr::security::{Chain, Digest256};

    let mut store = connect();
    reset(&mut store);
    store.install().expect("install");
    let ehr = conformance::sample_ehr();
    store.create_ehr(&ehr).expect("ehr");
    store
        .create_contribution(
            ehr.ehr_id(),
            &conformance::sample_contribution("c1", &[1, 2, 3]),
        )
        .expect("contribution");

    for n in 1..=3u32 {
        let preceding = (n > 1).then(|| n - 1);
        store
            .commit_composition(
                ehr.ehr_id(),
                &conformance::sample_version(n, preceding, n),
                "c1",
            )
            .unwrap_or_else(|e| panic!("commit {n}: {e}"));
    }

    let container = ehr.ehr_id().clone();
    let all = store.all_versions(&container).expect("all_versions");
    assert_eq!(all.len(), 3);

    assert_eq!(all[0].chain.previous, [0u8; 32], "first links to genesis");
    for pair in all.windows(2) {
        assert_eq!(
            pair[1].chain.previous, pair[0].chain.digest,
            "version {} does not link to {}",
            pair[1].uid, pair[0].uid
        );
    }

    let mut recomputed = Chain::new();
    for row in &all {
        let content: serde_json::Value = row
            .data_json
            .as_deref()
            .map(|j| serde_json::from_str(j).expect("stored JSON parses"))
            .expect("a committed version has content");
        recomputed
            .append(row.uid.clone(), &Some(content), None)
            .expect("append");
    }
    assert_eq!(
        recomputed.head(),
        Digest256::from_bytes(all[2].chain.digest),
        "the chain recomputed from stored rows must reach the stored head"
    );

    // Now rewrite a row behind the store's back and show the chain notices.
    // The append-only trigger blocks UPDATE, so this goes around it the way a
    // determined operator would: drop the trigger first.
    store
        .client()
        .batch_execute("DROP TRIGGER trg_append_only_openehr_version ON openehr_version;")
        .expect("drop trigger");
    store
        .client()
        .execute(
            "UPDATE openehr_version SET data_json = replace(data_json, 'Encounter 2', 'Tampered') \
             WHERE trunk_version = 2",
            &[],
        )
        .expect("rewrite");

    let after = store.all_versions(&container).expect("all_versions");
    let mut recheck = Chain::new();
    for row in &after {
        let content: serde_json::Value =
            serde_json::from_str(row.data_json.as_deref().expect("content")).expect("parses");
        recheck
            .append(row.uid.clone(), &Some(content), None)
            .expect("append");
    }
    assert_ne!(
        recheck.head(),
        Digest256::from_bytes(after[2].chain.digest),
        "a rewritten version must not recompute to the stored head — \
         the chain would be evidence of nothing"
    );
}

/// A database predating the version table is refused too. Ported from
/// `openehr-sqlite`'s own test of the same name.
#[test]
#[ignore = "needs a real PostgreSQL server, see this file's own module doc"]
fn a_database_predating_the_version_table_is_refused() {
    use openehr_store::StoreError;

    let mut store = connect();
    reset(&mut store);
    store.install().expect("install");
    let ehr = conformance::sample_ehr();
    store.create_ehr(&ehr).expect("ehr");

    // Erase the version row, leaving data behind: exactly what an older
    // database looks like.
    store
        .client()
        .execute("DELETE FROM openehr_schema_version", &[])
        .expect("clear version");

    assert!(
        matches!(
            store.install(),
            Err(StoreError::SchemaVersionMismatch { found: 0, .. })
        ),
        "a populated database with no recorded version must be refused"
    );
}

/// An **empty** database predating the version table installs; a populated
/// one is refused. Ported from `openehr-sqlite`'s own test of the same name.
#[test]
#[ignore = "needs a real PostgreSQL server, see this file's own module doc"]
fn an_empty_database_predating_the_version_table_still_installs() {
    let mut store = connect();
    reset(&mut store);
    store.install().expect("install");
    // Remove the version record, leaving the tables and no rows: a database
    // installed by a build older than versioning, never written to.
    store
        .client()
        .execute("DELETE FROM openehr_schema_version", &[])
        .expect("delete");

    store
        .install()
        .expect("an empty database has nothing to lose and is treated as fresh");

    // With a record in it, the same absence means something else entirely.
    store
        .client()
        .execute("DELETE FROM openehr_schema_version", &[])
        .expect("delete");
    store.create_ehr(&conformance::sample_ehr()).expect("ehr");
    assert!(
        matches!(
            store.install(),
            Err(openehr_store::StoreError::SchemaVersionMismatch { found: 0, .. })
        ),
        "a populated database with no version was treated as fresh"
    );
}

/// `M3.16c` / `T11.8`: a **truncated** chain still verifies clean, and only
/// the checkpoint reveals it. Ported from `openehr-sqlite`'s own test of the
/// same name.
#[test]
#[ignore = "needs a real PostgreSQL server, see this file's own module doc"]
fn a_truncated_chain_verifies_clean_and_only_the_checkpoint_notices() {
    use openehr::security::{Chain, Digest256};

    let mut store = connect();
    reset(&mut store);
    store.install().expect("install");
    let ehr = conformance::sample_ehr();
    store.create_ehr(&ehr).expect("ehr");
    store
        .create_contribution(
            ehr.ehr_id(),
            &conformance::sample_contribution("c1", &[1, 2, 3]),
        )
        .expect("contribution");
    for n in 1..=3u32 {
        store
            .commit_composition(
                ehr.ehr_id(),
                &conformance::sample_version(n, (n > 1).then(|| n - 1), n),
                "c1",
            )
            .expect("commit");
    }

    let container = ehr.ehr_id().clone();
    let before = store.chain_checkpoint(&container).expect("checkpoint");
    assert!(before.starts_with("entries=3 "), "{before}");
    assert!(
        !before.contains("Encounter"),
        "a checkpoint must carry no clinical content: {before}"
    );

    // Truncate: remove the newest version, as an operator with write access
    // would. The append-only trigger blocks DELETE, so it goes first.
    store
        .client()
        .batch_execute("DROP TRIGGER trg_append_only_openehr_version ON openehr_version;")
        .expect("drop trigger");
    // The index row references the version, so a foreign key blocks deleting
    // the version alone.
    store
        .client()
        .execute(
            "DELETE FROM openehr_composition_index WHERE version_uid IN \
             (SELECT uid FROM openehr_version WHERE trunk_version = 3)",
            &[],
        )
        .expect("remove index row");
    store
        .client()
        .execute("DELETE FROM openehr_version WHERE trunk_version = 3", &[])
        .expect("truncate");

    // The remaining chain is *perfectly consistent*. Every link still holds.
    let after = store.all_versions(&container).expect("all_versions");
    assert_eq!(after.len(), 2);
    let mut rebuilt = Chain::new();
    for row in &after {
        let content: serde_json::Value =
            serde_json::from_str(row.data_json.as_deref().expect("content")).expect("parses");
        rebuilt
            .append(row.uid.clone(), &Some(content), None)
            .expect("append");
    }
    assert_eq!(
        rebuilt.head(),
        Digest256::from_bytes(after[1].chain.digest),
        "the truncated history verifies against itself — which is exactly the \
         problem, and why a checkpoint is not optional"
    );

    // Only the checkpoint, held elsewhere, shows the loss.
    let now = store.chain_checkpoint(&container).expect("checkpoint");
    assert!(now.starts_with("entries=2 "), "{now}");
    assert_ne!(
        before, now,
        "a checkpoint published before the truncation must not match after it"
    );
}

/// A contribution is actually written, and declaring one twice conflicts.
/// Ported from `openehr-sqlite`'s own test of the same name — found there by
/// mutation testing (`lib:A-09`), and this crate's own `commit_conflict`
/// needed the same evidence (`cargo mutants`' own `MISSED` on
/// `create_contribution`'s duplicate-uid path before this test existed).
#[test]
#[ignore = "needs a real PostgreSQL server, see this file's own module doc"]
fn a_contribution_is_persisted_and_cannot_be_declared_twice() {
    let mut store = connect();
    reset(&mut store);
    store.install().expect("install");
    let ehr = conformance::sample_ehr();
    let ehr_id = ehr.ehr_id().clone();
    store.create_ehr(&ehr).expect("ehr");

    let uid = "7C2E4B90-1A3D-4E58-9F6B-0D8C7A5E4B31";
    let contribution = conformance::sample_contribution(uid, &[1]);
    store
        .create_contribution(&ehr_id, &contribution)
        .expect("first declaration");

    // The row is there. Asserted directly, because the `Store` trait offers
    // no way to read a contribution back — which is why the no-op survived
    // in `SqliteStore` until this exact test was written.
    let count: i64 = store
        .client()
        .query_one(
            "SELECT count(*) FROM openehr_contribution WHERE uid = $1",
            &[&uid],
        )
        .expect("query")
        .get(0);
    assert_eq!(count, 1, "the contribution was not written");

    // And a second declaration of the same change set conflicts rather than
    // overwriting: one act, one contribution.
    assert!(
        matches!(
            store.create_contribution(&ehr_id, &contribution),
            Err(openehr_store::StoreError::Conflict { .. })
        ),
        "a duplicate contribution was accepted"
    );
}

/// The checkpoint carries a real digest, and the store names itself. Ported
/// from `openehr-sqlite`'s own test of the same name.
#[test]
#[ignore = "needs a real PostgreSQL server, see this file's own module doc"]
fn a_checkpoint_carries_a_real_digest_and_the_store_names_itself() {
    let mut store = connect();
    reset(&mut store);
    store.install().expect("install");
    assert_eq!(store.engine(), "PostgreSQL");

    let ehr = conformance::sample_ehr();
    let ehr_id = ehr.ehr_id().clone();
    store.create_ehr(&ehr).expect("ehr");
    let contribution = "22222222-3333-4444-5555-666666666666";
    store
        .create_contribution(
            &ehr_id,
            &conformance::sample_contribution(contribution, &[1]),
        )
        .expect("contribution");
    store
        .commit_composition(
            &ehr_id,
            &conformance::sample_version(1, None, 5),
            contribution,
        )
        .expect("commit");

    let container = openehr::base::HierObjectId::from_uid_str(conformance::RECORD).expect("uid");
    let checkpoint = store.chain_checkpoint(&container).expect("checkpoint");

    // The head digest, in full, and matching what the row actually holds.
    let head = store.latest_version(&container).expect("head");
    let expected = head.chain.digest.iter().fold(String::new(), |mut acc, b| {
        use std::fmt::Write as _;
        let _ = write!(acc, "{b:02x}");
        acc
    });
    assert_eq!(expected.len(), 64);
    assert!(
        checkpoint.contains(&expected),
        "the checkpoint carries no digest: {checkpoint}"
    );
    assert_ne!(expected, "0".repeat(64), "that is the genesis digest");
}
