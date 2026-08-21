//! Store a record end to end: install, commit, read, query, verify, refuse.
//!
//! ```sh
//! cargo run --example 01_store_a_record
//! ```
//!
//! `openehr`'s five tutorials build and validate documents in memory. This is
//! the other half — what happens when one reaches a database — and it is the
//! only runnable tutorial for the persistence layer, which is why it walks the
//! whole loop rather than one call.
//!
//! It runs against a real SQLite database, in process, with no setup. That is
//! the reason `openehr-sqlite` is the crate this tutorial lives in: it is the
//! only one at conformance level **Verified**, and the only one with a `Store`.
//! The five other engine crates supply a `Dialect` and no more, so there is
//! nothing to demonstrate here that would not be a claim about code that does
//! not exist (`C0.11`).

use openehr::base::{HierObjectId, ObjectVersionId};
use openehr::rm::common::Version;
use openehr::rm::data_types::DvDateTime;
use openehr::rm::ehr::Composition;
use openehr_sqlite::SqliteStore;
use openehr_store::conformance::{RECORD, sample_contribution, sample_ehr, sample_version};
use openehr_store::{CommitOutcome, Store, StoreError};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ---- 1. Install ------------------------------------------------------
    //
    // `install()` creates five tables and seven indexes and records
    // `SCHEMA_VERSION`. It is idempotent: running it twice is a no-op, which is
    // what `verify-schema.sh` checks against every engine's own server, because
    // a DDL script that only works on an empty database is one nobody can
    // redeploy.
    let mut store = SqliteStore::in_memory()?;
    store.install()?;
    println!("installed on {}", store.engine());

    // ---- 2. A record, and a change set -----------------------------------
    //
    // Every version belongs to a CONTRIBUTION -- openEHR's unit of change. Two
    // compositions committed by one clinician in one sitting share it, and that
    // is how a reader later asks "what else changed at the same time".
    let ehr = sample_ehr();
    let ehr_id = ehr.ehr_id().clone();
    store.create_ehr(&ehr)?;
    store.create_contribution(&ehr_id, &sample_contribution("ctrb-0001", &[1, 2]))?;

    // ---- 3. Commit, then amend -------------------------------------------
    //
    // The store validates before writing (`db:V9.8`). It has to: serde writes
    // fields straight in and calls no constructor, so a composition that
    // arrived as JSON has been checked by *nothing* until this point. A store
    // that wrote it anyway would leave every later reader's `validate()`
    // failing on data they cannot fix.
    let first: Version<Composition> = sample_version(1, None, 0);
    let outcome: CommitOutcome = store.commit_composition(&ehr_id, &first, "ctrb-0001")?;
    assert!(
        outcome.created_container,
        "the first version of a container creates it"
    );
    println!("committed  {}", outcome.version_uid);

    // A second version naming the first as its predecessor. Naming the *wrong*
    // predecessor is refused, below.
    let second: Version<Composition> = sample_version(2, Some(1), 5);
    store.commit_composition(&ehr_id, &second, "ctrb-0001")?;
    println!("amended    version 2");

    // ---- 4. Read it back -------------------------------------------------
    let container: HierObjectId = RECORD.parse()?;
    let latest = store.latest_version(&container)?;
    println!(
        "latest     {} ({}), committed {}",
        latest.uid, latest.audit_change_type_code, latest.audit_time_committed.text
    );

    // Two columns for every time (`db:M3.31`). `…_text` is the authoritative
    // lexical form and `…_utc` is derived and nullable, because `2024-05` is a
    // date known to the month and is not `2024-05-01`. Ordering uses the
    // derived column; display uses the exact one.
    println!(
        "           text {:?}, derived utc {:?}",
        latest.audit_time_committed.text, latest.audit_time_committed.utc_seconds
    );

    // History is oldest-first -- the order `REVISION_HISTORY` requires
    // (`db:V8.7a`). openEHR contradicts itself about this in prose; the
    // `most_recent_version` postcondition settles it.
    let history = store.all_versions(&container)?;
    println!("history    {} versions, oldest first", history.len());

    // Point-in-time read: which version was current at 09:02, between the two
    // commits?
    let at = DvDateTime::new("2026-08-01T09:02:00Z")?;
    println!("at 09:02   {}", store.version_at_time(&container, &at)?.uid);

    // ---- 5. The one query the index exists for ---------------------------
    //
    // `AQL`'s `CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1]`.
    // Archetyped content is stored as canonical JSON rather than shredded into
    // columns (`db:P6.10`), so the queryable surface is this index over
    // Reference-Model attributes -- not the document body.
    let found =
        store.find_compositions_by_archetype(&ehr_id, "openEHR-EHR-COMPOSITION.encounter.v1")?;
    println!("indexed    {} composition(s) of that archetype", found.len());

    // ---- 6. The commit rules refuse what they should ---------------------
    //
    // A version naming a predecessor that is not the head. Two clinicians
    // amending the same composition from the same starting point is the case
    // this catches, and it is a lost-update bug in any store that does not.
    let stale: Version<Composition> = sample_version(3, Some(1), 10);
    match store.commit_composition(&ehr_id, &stale, "ctrb-0001") {
        Err(StoreError::Commit(e)) => println!("refused    stale predecessor: {e}"),
        other => panic!("a stale predecessor must be refused, got {other:?}"),
    }

    // ---- 7. Tamper-evidence ----------------------------------------------
    //
    // Every version carries a digest over its content, chained to the version
    // before it, so altering or removing one in the *middle* invalidates
    // everything after it (`db:M3.16`).
    //
    // The chain cannot detect **truncation**: delete the newest version and
    // what remains is a shorter chain that verifies perfectly. The checkpoint
    // closes that gap, and only if it is published somewhere the database
    // administrator does not control (`db:M3.16c`) -- one stored beside the
    // data it attests to is worth nothing.
    //
    // It carries a count, a head digest, and an identifier, and **no clinical
    // content**, so it is safe to ship to a log or a witness that must never
    // hold patient data.
    println!("checkpoint {}", store.chain_checkpoint(&container)?);

    // ---- 8. Append-only is enforced by the database ----------------------
    //
    // Not by this crate's code: by triggers in the DDL. A store is not the only
    // thing with a connection, and a rule enforced only in Rust is a rule an
    // administrator with `psql` does not have.
    let uid: ObjectVersionId = format!("{RECORD}::ehr1.example.org::1").parse()?;
    let direct = store.connection().execute(
        "UPDATE openehr_version SET lifecycle_state_code = '523' WHERE uid = ?1",
        [uid.to_string()],
    );
    match direct {
        Err(e) => println!("refused    direct UPDATE: {e}"),
        Ok(n) => panic!("append-only must refuse an UPDATE, but it changed {n} row(s)"),
    }

    // And the row is intact -- which is the half a refusal alone does not
    // prove (`C0.12`).
    let after = store.get_version(&uid)?;
    assert_eq!(after.lifecycle_state_code, latest_state(&history));
    println!("intact     version 1 unchanged after the refused UPDATE");

    Ok(())
}

/// The lifecycle state version 1 was committed with, read from the history
/// rather than restated, so this assertion cannot drift from the fixture.
fn latest_state(history: &[openehr_store::record::VersionRow]) -> String {
    history
        .first()
        .expect("the history has at least one version")
        .lifecycle_state_code
        .clone()
}
