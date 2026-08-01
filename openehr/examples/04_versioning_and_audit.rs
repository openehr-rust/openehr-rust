//! Commit versions to a `VERSIONED_COMPOSITION`, chain them, and watch the
//! chain catch an edit.
//!
//! ```sh
//! cargo run --example 04_versioning_and_audit
//! ```
//!
//! openEHR never updates in place. A change is a new `VERSION` in a
//! `VERSIONED_OBJECT`, carrying the `AUDIT_DETAILS` of its commit; a deletion
//! is a version whose `data` is absent and whose `lifecycle_state` is
//! `deleted`. What the model does not give you is *detection*: nothing in it
//! notices a version edited in the database after the fact. That is what
//! [`Chain`](openehr::security::Chain) adds.

use openehr::base::{HierObjectId, ObjectId, ObjectRef, ObjectVersionId};
use openehr::rm::common::{
    AuditDetails, CommitError, OriginalVersion, PartyIdentified, VersionedObject,
};
use openehr::rm::data_types::DvDateTime;
use openehr::security::{Chain, ChainKey, ChainStatus, Digest256};
use openehr::terminology::{audit_change_type, version_lifecycle_state};

const RECORD: &str = "87284370-2D4B-4E3D-A3F3-F303D2F4F34B";
const SYSTEM: &str = "ehr1.example.org";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let uid = HierObjectId::from_uid_str(RECORD)?;
    let owner = ObjectRef::new("local", "EHR", ObjectId::HierObjectId(uid.clone()))?;
    let mut versioned: VersionedObject<String> =
        VersionedObject::new(uid, owner.clone(), DvDateTime::new("2026-07-31T09:00:00Z")?);

    // The chain key lives in the process and never in the database. A key
    // stored where an attacker already has write access protects nothing.
    let key = ChainKey::new("2026-Q3", vec![0x5Au8; 32])?;
    let mut chain = Chain::new();

    // ---- three commits ---------------------------------------------------
    let commits = [
        (1, None, audit_change_type::CREATION, "Initial assessment"),
        (
            2,
            Some(1),
            audit_change_type::AMENDMENT,
            "Initial assessment, corrected laterality",
        ),
        (
            3,
            Some(2),
            audit_change_type::MODIFICATION,
            "Initial assessment, corrected laterality, plan added",
        ),
    ];
    for (n, preceding, change, content) in commits {
        let version = OriginalVersion::new(
            version_id(n)?,
            preceding.map(version_id).transpose()?,
            version_lifecycle_state::COMPLETE,
            Some(content.to_owned()),
            audit(change, &format!("2026-07-31T09:{:02}:00Z", n * 5))?,
            owner.clone(),
        )?;
        versioned.commit(version.into())?;
        chain.append(version_id(n)?.to_string(), &content, Some(&key))?;
    }
    println!("{} versions committed", versioned.version_count());
    println!("chain head: {}", chain.head());
    println!("checkpoint: {}\n", chain.checkpoint());

    // ---- a concurrent write is refused, not merged -----------------------
    // Two clients both read version 2 and both write version 4. openEHR's
    // answer is a branch, not a silent overwrite, so the refusal is the point
    // at which a caller must decide which it wants.
    let stale = OriginalVersion::new(
        version_id(4)?,
        Some(version_id(2)?),
        version_lifecycle_state::COMPLETE,
        Some("concurrent edit".to_owned()),
        audit(audit_change_type::AMENDMENT, "2026-07-31T09:20:00Z")?,
        owner.clone(),
    )?;
    assert_eq!(versioned.commit(stale.into()), Err(CommitError::NotLatest));
    println!("concurrent write refused: {}", CommitError::NotLatest);

    // ---- reading the record as it was ------------------------------------
    let at_0910 = DvDateTime::new("2026-07-31T09:12:00Z")?;
    let then = versioned
        .version_at_time(&at_0910)
        .and_then(|v| v.data().cloned())
        .unwrap_or_else(|| "<no version yet>".to_owned());
    println!("as of 09:12: {then}");

    // ---- the audit trail -------------------------------------------------
    let history = versioned.revision_history().expect("three versions");
    println!("\nrevision history, most recent first:");
    for item in history.items() {
        let audit = &item.audits()[0];
        println!(
            "  {}  {:<14} {}",
            item.version_id().version_tree_id(),
            audit.change_type().value(),
            audit.time_committed()
        );
    }

    // ---- verification ----------------------------------------------------
    println!("\nverify (holding the key): {:?}", chain.verify(&[&key]));
    assert!(chain.verify(&[&key]).is_fully_verified());

    // An edit of the kind a migration or a stray UPDATE would make.
    let mut tampered = chain.clone();
    tampered.entries_mut()[1].content = Digest256::GENESIS;
    let status = tampered.verify(&[&key]);
    println!("verify (after an edit):   {status:?}");
    assert!(status.is_finding());

    // A reader holding a *different* key is a key-distribution problem, not a
    // forgery. Reporting it as tampering would start an incident response
    // against a deployment mistake.
    let other = ChainKey::new("2025-Q4", vec![0x11u8; 32])?;
    let status = chain.verify(&[&other]);
    println!("verify (wrong key held):  {status:?}");
    assert!(!status.is_finding());
    assert!(matches!(status, ChainStatus::UnknownKey { .. }));

    Ok(())
}

fn version_id(n: u32) -> Result<ObjectVersionId, openehr::ParseError> {
    format!("{RECORD}::{SYSTEM}::{n}").parse()
}

fn audit(change_type: &str, at: &str) -> Result<AuditDetails, Box<dyn std::error::Error>> {
    Ok(AuditDetails::new(
        SYSTEM,
        DvDateTime::new(at)?,
        change_type,
        PartyIdentified::named("Dr A Nurse")?.into(),
    )?)
}
