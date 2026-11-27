//! `db:D-22`, `db:PR12.25`: who committed a change survives a restart, and two
//! clinicians who share a display name stay two clinicians.
//!
//! Before schema 6 the only stored trace of a committer was
//! `audit_committer_name`. Two parties called "Dr A Nurse" left identical rows,
//! and the identifier the service had verified was discarded.

use openehr::base::HierObjectId;
use openehr::rm::common::{AuditDetails, Contribution, PartyIdentified, PartyProxy, Version};
use openehr::rm::data_types::{DvDateTime, DvIdentifier};
use openehr::rm::ehr::Composition;
use openehr::terminology::audit_change_type;
use openehr_sqlite::SqliteStore;
use openehr_store::{Store as _, conformance};

const A: &str = "11111111-aaaa-4444-8888-000000000001";
const B: &str = "11111111-bbbb-4444-8888-000000000002";

fn clinician(id: &str) -> PartyProxy {
    PartyIdentified::new(
        Some("Dr A Nurse".to_owned()),
        vec![DvIdentifier::new(id).expect("literal")],
        None,
    )
    .expect("literal")
    .into()
}

/// `conformance::sample_version`'s version `n`, committed by `party`.
fn version_by(n: u32, preceding: Option<u32>, party: &PartyProxy) -> Version<Composition> {
    let mut json =
        serde_json::to_value(conformance::sample_version(n, preceding, n * 5)).expect("json");
    json["commit_audit"]["committer"] = serde_json::to_value(party).expect("json");
    serde_json::from_value(json).expect("deserialise")
}

fn contribution_by(uid: &str, n: u32, party: PartyProxy) -> Contribution {
    Contribution::new(
        HierObjectId::from_uid_str(uid).expect("literal"),
        vec![
            conformance::composition_version_uid(n)
                .parse()
                .expect("literal"),
        ],
        AuditDetails::new(
            conformance::SYSTEM,
            DvDateTime::new("2026-08-01T09:05:00Z").expect("literal"),
            audit_change_type::CREATION,
            party,
        )
        .expect("literal"),
    )
    .expect("literal")
}

#[test]
fn two_clinicians_with_one_name_are_told_apart_after_a_restart() {
    let path = std::env::temp_dir().join(format!(
        "openehr-attribution-{}.sqlite3",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    {
        let mut store = SqliteStore::open(&path).expect("open");
        store.install().expect("install");
        let ehr = conformance::sample_ehr();
        let ehr_id = ehr.ehr_id().clone();
        store.create_ehr(&ehr).expect("ehr");
        store
            .create_contribution(&ehr_id, &contribution_by(A, 1, clinician("clinician-0001")))
            .expect("contribution A");
        store
            .create_contribution(&ehr_id, &contribution_by(B, 2, clinician("clinician-0002")))
            .expect("contribution B");
        store
            .commit_composition(
                &ehr_id,
                &version_by(1, None, &clinician("clinician-0001")),
                A,
            )
            .expect("v1");
        store
            .commit_composition(
                &ehr_id,
                &version_by(2, Some(1), &clinician("clinician-0002")),
                B,
            )
            .expect("v2");
    }

    // A restart: a new process would open the file again. Read-only, so the
    // check cannot write the answer it reads.
    let store = SqliteStore::open_read_only(&path).expect("reopen");
    let rows = store
        .all_versions(&HierObjectId::from_uid_str(conformance::RECORD).expect("literal"))
        .expect("history");
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[0].audit_committer_name, rows[1].audit_committer_name,
        "same display name"
    );
    let committer =
        |json: &str| -> PartyProxy { serde_json::from_str(json).expect("stored committer") };
    let first = committer(&rows[0].audit_committer_json);
    let second = committer(&rows[1].audit_committer_json);
    assert_ne!(first, second, "two clinicians must not collapse into one");
    assert_eq!(first, clinician("clinician-0001"));
    assert_eq!(second, clinician("clinician-0002"));
    assert_eq!(rows[0].contribution_uid, A);
    assert_eq!(rows[1].contribution_uid, B);

    // The contributions carry their own committer, whole, too (`db:PR12.10`).
    let contribution_committer = |uid: &str| -> PartyProxy {
        let json: String = store
            .connection()
            .query_row(
                "SELECT audit_committer_json FROM openehr_contribution WHERE uid = ?1",
                [uid],
                |row| row.get(0),
            )
            .expect("contribution row");
        committer(&json)
    };
    assert_eq!(contribution_committer(A), clinician("clinician-0001"));
    assert_eq!(contribution_committer(B), clinician("clinician-0002"));
    drop(store);
    let _ = std::fs::remove_file(&path);
}
