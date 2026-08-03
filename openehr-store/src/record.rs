//! Row types, and the projection from openEHR objects onto them.
//!
//! # The projection is one function, not five
//!
//! Deriving a composition's index columns is pure logic over the Reference
//! Model — no SQL in it. It lives here so that all six engines index the same
//! attributes from the same rules. An engine that computed its own projection
//! would eventually index a different `category`, and the difference would show
//! up as a query returning different rows on different engines.

use crate::error::Result;
use openehr::base::iso8601;
use openehr::rm::common::{Locatable as _, PartyProxy, Version};
use openehr::rm::ehr::Composition;
use openehr::security::audit_chain::{Chain, ChainKey};
use serde::{Deserialize, Serialize};

/// An ISO 8601 instant as stored: the exact text, plus a derived UTC value.
///
/// See [`crate::schema`] for why both are kept. The derived half is `None`
/// whenever the instant is not established, which is the same answer the
/// library gives (`D3.14`) — so SQL and Rust cannot disagree about one record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredInstant {
    /// The authoritative lexical form.
    pub text: String,
    /// Seconds from the Unix epoch, or `None` when not established.
    pub utc_seconds: Option<i64>,
}

impl StoredInstant {
    /// Projects a date-time onto its two stored halves.
    ///
    /// # Panics
    ///
    /// Never: the epoch it differences against is a literal known to parse.
    #[must_use]
    pub fn from_date_time(value: &iso8601::DateTime) -> Self {
        // The epoch is expressed as a value rather than a constant so the
        // difference goes through exactly the same partial-order rules as every
        // other comparison in the library.
        let epoch: iso8601::DateTime = "1970-01-01T00:00:00Z".parse().expect("literal");
        Self {
            text: value.as_str().to_owned(),
            utc_seconds: value.diff_seconds(&epoch),
        }
    }
}

/// One row of the `version` table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionRow {
    /// Full `OBJECT_VERSION_ID`.
    pub uid: String,
    /// Containing versioned object.
    pub versioned_object_uid: String,
    /// The version id's middle part.
    pub creating_system_id: String,
    /// Trunk number.
    pub trunk_version: i64,
    /// Branch number, or `None` on the trunk.
    pub branch_number: Option<i64>,
    /// Branch version, or `None` on the trunk.
    pub branch_version: Option<i64>,
    /// The version this one succeeds.
    pub preceding_version_uid: Option<String>,
    /// openEHR lifecycle-state code.
    pub lifecycle_state_code: String,
    /// Whether this version marks a logical deletion.
    pub is_deleted: bool,
    /// The change set.
    pub contribution_uid: String,
    /// `AUDIT_DETAILS.system_id`.
    pub audit_system_id: String,
    /// openEHR change-type code.
    pub audit_change_type_code: String,
    /// The committer's name, where the party has one.
    pub audit_committer_name: Option<String>,
    /// When it was committed.
    pub audit_time_committed: StoredInstant,
    /// Canonical JSON of the content, or `None` for a deletion.
    pub data_json: Option<String>,
    /// `AUDIT_DETAILS.description`.
    pub audit_description: Option<String>,
    /// `VERSION.signature`, carried and never verified.
    pub signature: Option<String>,
    /// `ORIGINAL_VERSION.attestations` as canonical JSON, `None` when empty.
    pub attestations_json: Option<String>,
    /// `ORIGINAL_VERSION.other_input_version_uids` as canonical JSON, `None`
    /// when this version is not a merge.
    pub other_input_version_uids_json: Option<String>,
    /// The version's link in the tamper-evidence chain (`M3.16`).
    pub chain: ChainColumns,
}

/// The chain columns of one version row.
///
/// Grouped rather than flattened into [`VersionRow`] because they are one
/// object — a `ChainEntry` — and a caller that has three of the five has
/// nothing. Keeping them together makes that hard to get wrong.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainColumns {
    /// Digest of the preceding entry, or the genesis digest.
    pub previous: [u8; 32],
    /// SHA-256 over the canonical form of this version's content.
    pub content: [u8; 32],
    /// This entry's own digest, over `previous || content || uid`.
    pub digest: [u8; 32],
    /// Which key produced [`ChainColumns::tag_mac`], if the chain is keyed.
    pub tag_key_id: Option<String>,
    /// HMAC-SHA-256 over the same pre-image, if the chain is keyed.
    pub tag_mac: Option<[u8; 32]>,
}

impl VersionRow {
    /// Projects a version onto a row.
    ///
    /// `previous` is the chain digest of the version this one follows in the
    /// same container, or `None` for the first — see [`ChainColumns`] and
    /// `M3.16`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::StoreError::Json`] if the content cannot be
    /// canonicalised.
    ///
    /// # Panics
    ///
    /// Never: the chain entry is read back immediately after the `append` that
    /// pushed it, on a chain this function owns and nothing else can touch.
    pub fn project<T: Serialize>(
        version: &Version<T>,
        contribution_uid: &str,
        previous: Option<[u8; 32]>,
        key: Option<&ChainKey>,
    ) -> Result<Self> {
        let uid = version.uid();
        let audit = version.commit_audit();
        let data_json = version
            .data()
            .map(openehr::security::to_canonical_string)
            .transpose()?;

        // The chain covers the version's *content*, which is what a reader
        // would be shown, and is chained to the previous version in the same
        // container. See the module header for what that detects.
        let mut chain_builder = previous.map_or_else(Chain::new, |head| {
            Chain::resume_from(openehr::security::Digest256::from_bytes(head))
        });
        chain_builder.append(uid.to_string(), &version.data(), key)?;
        let entry = chain_builder
            .entries()
            .last()
            .expect("append pushed an entry");
        let chain = ChainColumns {
            previous: *entry.previous.as_bytes(),
            content: *entry.content.as_bytes(),
            digest: *entry.digest.as_bytes(),
            tag_key_id: entry.tag.as_ref().map(|t| t.key_id().to_owned()),
            tag_mac: entry
                .tag
                .as_ref()
                .and_then(|t| <[u8; 32]>::try_from(t.mac()).ok()),
        };
        Ok(Self {
            uid: uid.to_string(),
            versioned_object_uid: uid.object_id().to_string(),
            creating_system_id: uid.creating_system_id().to_string(),
            trunk_version: i64::from(uid.version_tree_id().trunk_version()),
            branch_number: uid.version_tree_id().branch_number().map(i64::from),
            branch_version: uid.version_tree_id().branch_version().map(i64::from),
            preceding_version_uid: version.preceding_version_uid().map(ToString::to_string),
            lifecycle_state_code: version.lifecycle_state_code().to_owned(),
            is_deleted: version.is_deleted(),
            contribution_uid: contribution_uid.to_owned(),
            audit_system_id: audit.system_id().to_owned(),
            audit_change_type_code: audit.change_type_code().to_owned(),
            audit_committer_name: party_name(audit.committer()),
            audit_time_committed: StoredInstant::from_date_time(audit.time_committed().value()),
            data_json,
            audit_description: audit.description().map(ToString::to_string),
            signature: version.signature().map(ToOwned::to_owned),
            attestations_json: encode_if_any(version.attestations())?,
            other_input_version_uids_json: encode_if_any(version.other_input_version_uids())?,
            chain,
        })
    }
}

/// Canonical JSON for a slice, or `None` when it is empty.
///
/// `None` rather than `[]` so that "this version is not a merge" and "this
/// version merged nothing" are the same absent value in SQL, which they are.
///
/// # Errors
///
/// Returns [`crate::StoreError::Json`] if the value cannot be canonicalised.
fn encode_if_any<T: Serialize>(items: &[T]) -> Result<Option<String>> {
    if items.is_empty() {
        return Ok(None);
    }
    Ok(Some(openehr::security::to_canonical_string(&items)?))
}

/// A committer's or composer's name, where the party form carries one.
///
/// `None` for a `PARTY_SELF`, which is **not** missing data: an anonymous
/// subject is a legitimate and deliberate representation (`M5.16`), and a store
/// that wrote `"unknown"` here would turn a privacy decision into a data
/// quality problem.
fn party_name(party: &PartyProxy) -> Option<String> {
    party.name().map(ToOwned::to_owned)
}

/// One row of the `composition_index` table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositionIndexRow {
    /// The version indexed.
    pub version_uid: String,
    /// Owning record.
    pub ehr_id: String,
    /// The composition's archetype.
    pub archetype_id: String,
    /// The template, if one was used.
    pub template_id: Option<String>,
    /// openEHR composition-category code.
    pub category_code: String,
    /// The composer's name, where they have one.
    pub composer_name: Option<String>,
    /// ISO 639-1.
    pub language_code: String,
    /// ISO 3166-1.
    pub territory_code: String,
    /// openEHR setting code, if there is a context.
    pub setting_code: Option<String>,
    /// Context start.
    pub context_start: Option<StoredInstant>,
    /// Context end.
    pub context_end: Option<StoredInstant>,
}

impl CompositionIndexRow {
    /// Projects a composition onto its index row.
    ///
    /// # Errors
    ///
    /// Returns [`crate::StoreError::Invalid`] if the composition is not an
    /// archetype root, because `archetype_id` is the column `AQL` filters on and
    /// there is nothing to put in it. Every other absent attribute becomes
    /// `NULL`, which is a fact; an invented archetype id would not be.
    pub fn project(version_uid: &str, ehr_id: &str, composition: &Composition) -> Result<Self> {
        let details = composition.archetype_details().ok_or_else(|| {
            let mut report = openehr::ValidationReport::new();
            report.push(openehr::Violation {
                path: String::new(),
                class: "COMPOSITION",
                invariant: "Is_archetype_root",
                detail: "cannot index a composition with no archetype_details",
            });
            crate::StoreError::Invalid(report)
        })?;
        let context = composition.context();
        Ok(Self {
            version_uid: version_uid.to_owned(),
            ehr_id: ehr_id.to_owned(),
            archetype_id: details.archetype_id().to_string(),
            template_id: details.template_id().map(ToString::to_string),
            category_code: composition.category_code().to_owned(),
            composer_name: party_name(composition.composer()),
            language_code: composition.language().code_string().to_owned(),
            territory_code: composition.territory().code_string().to_owned(),
            setting_code: context.map(|c| c.setting().defining_code().code_string().to_owned()),
            context_start: context.map(|c| StoredInstant::from_date_time(c.start_time().value())),
            context_end: context
                .and_then(|c| c.end_time())
                .map(|t| StoredInstant::from_date_time(t.value())),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::VersionRow;
    use openehr::rm::common::{AuditDetails, OriginalVersion, PartyIdentified, Version};
    use openehr::rm::data_types::{DvDateTime, Text};
    use openehr::rm::ehr::Composition;
    use openehr::terminology::{audit_change_type, version_lifecycle_state};

    fn version(
        build: impl FnOnce(OriginalVersion<Composition>) -> OriginalVersion<Composition>,
    ) -> Version<Composition> {
        let audit = AuditDetails::new(
            "ehr1.example.org",
            DvDateTime::new("2026-08-01T09:00:00Z").expect("literal"),
            audit_change_type::CREATION,
            PartyIdentified::named("Dr A Nurse")
                .expect("literal")
                .into(),
        )
        .expect("literal");
        let original = OriginalVersion::new(
            format!("{}::ehr1.example.org::1", crate::conformance::RECORD)
                .parse()
                .expect("literal"),
            None,
            version_lifecycle_state::COMPLETE,
            Some(crate::conformance::sample_composition("Encounter")),
            audit,
            crate::conformance::sample_ehr().ehr_status().clone(),
        )
        .expect("literal");
        build(original).into()
    }

    /// `D-07`: the four attributes that used to be dropped are now stored.
    ///
    /// Before this, `project` refused them; before *that*, it accepted them and
    /// returned `Ok` while losing them. Refusing was the smaller evil, not a
    /// good outcome — openEHR permits these and a store should hold them.
    #[test]
    fn the_attributes_that_used_to_be_dropped_are_persisted() {
        let audit = AuditDetails::new(
            "ehr1.example.org",
            DvDateTime::new("2026-08-01T09:00:00Z").expect("literal"),
            audit_change_type::AMENDMENT,
            PartyIdentified::named("Dr A Nurse")
                .expect("literal")
                .into(),
        )
        .expect("literal")
        .with_description(
            Text::plain("corrected after telephone call with the lab").expect("literal"),
        );
        let v: Version<Composition> = OriginalVersion::new(
            format!("{}::ehr1.example.org::2", crate::conformance::RECORD)
                .parse()
                .expect("literal"),
            Some(
                format!("{}::ehr1.example.org::1", crate::conformance::RECORD)
                    .parse()
                    .expect("literal"),
            ),
            version_lifecycle_state::COMPLETE,
            Some(crate::conformance::sample_composition("Encounter")),
            audit,
            crate::conformance::sample_ehr().ehr_status().clone(),
        )
        .expect("literal")
        .with_signature("-----BEGIN PGP SIGNATURE-----")
        .into();

        // The other two `D-07` attributes travel through `encode_if_any`, and
        // this test asserted neither until mutation testing replaced that
        // function with `Ok(None)` and nothing failed — the exact defect `D-07`
        // is about, reachable again with one edit (`lib:A-09`).
        let attested = openehr::rm::common::Attestation::new(
            AuditDetails::new(
                "ehr1.example.org",
                DvDateTime::new("2026-08-01T10:00:00Z").expect("literal"),
                audit_change_type::AMENDMENT,
                PartyIdentified::named("Dr B Consultant")
                    .expect("literal")
                    .into(),
            )
            .expect("literal"),
            Text::plain("countersigned").expect("literal"),
            false,
        );
        let merged: openehr::base::ObjectVersionId =
            format!("{}::other.example.org::1", crate::conformance::RECORD)
                .parse()
                .expect("literal");
        let v: Version<Composition> = match v {
            Version::Original(original) => original
                .with_attestation(attested)
                .with_other_input_version_uid(merged.clone())
                .into(),
            imported @ Version::Imported(_) => imported,
        };

        let row = VersionRow::project(&v, "c1", None, None).expect("projects");
        assert_eq!(
            row.audit_description.as_deref(),
            Some("corrected after telephone call with the lab"),
        );
        assert_eq!(
            row.signature.as_deref(),
            Some("-----BEGIN PGP SIGNATURE-----")
        );
        assert!(
            row.attestations_json
                .as_deref()
                .is_some_and(|j| j.contains("countersigned")),
            "attestations were not persisted: {:?}",
            row.attestations_json
        );
        assert!(
            row.other_input_version_uids_json
                .as_deref()
                .is_some_and(|j| j.contains(&merged.to_string())),
            "merged version ids were not persisted: {:?}",
            row.other_input_version_uids_json
        );
    }

    /// `M3.34`: an anonymous committer is `NULL`, and a named one is not.
    ///
    /// Both directions, because only one of them was covered anywhere and the
    /// projection could return `Some("xyzzy")` for every party without a test
    /// failing. `NULL` here means *deliberately anonymous*; a store writing
    /// `"unknown"` would turn a privacy decision into a data-quality problem
    /// somebody later tries to clean up.
    #[test]
    fn an_anonymous_committer_is_null_and_a_named_one_is_written() {
        let with_committer = |party: openehr::rm::common::PartyProxy| {
            let audit = AuditDetails::new(
                "ehr1.example.org",
                DvDateTime::new("2026-08-01T09:00:00Z").expect("literal"),
                audit_change_type::CREATION,
                party,
            )
            .expect("literal");
            let v: Version<Composition> = OriginalVersion::new(
                format!("{}::ehr1.example.org::1", crate::conformance::RECORD)
                    .parse()
                    .expect("literal"),
                None,
                version_lifecycle_state::COMPLETE,
                Some(crate::conformance::sample_composition("Encounter")),
                audit,
                crate::conformance::sample_ehr().ehr_status().clone(),
            )
            .expect("literal")
            .into();
            VersionRow::project(&v, "c1", None, None)
                .expect("projects")
                .audit_committer_name
        };

        assert_eq!(
            with_committer(
                PartyIdentified::named("Dr A Nurse")
                    .expect("literal")
                    .into()
            ),
            Some("Dr A Nurse".to_owned())
        );
        assert_eq!(
            with_committer(openehr::rm::common::PartySelf::anonymous().into()),
            None
        );
    }

    /// Absent is `NULL`, not `[]`.
    ///
    /// "This version is not a merge" and "this version merged nothing" are the
    /// same fact, and SQL has one way to say it.
    #[test]
    fn empty_collections_are_null_rather_than_an_empty_array() {
        let row = VersionRow::project(&version(|v| v), "c1", None, None).expect("projects");
        assert!(row.attestations_json.is_none());
        assert!(row.other_input_version_uids_json.is_none());
        assert!(row.audit_description.is_none());
        assert!(row.signature.is_none());
    }

    /// `M3.16`: the first version links to the genesis digest.
    #[test]
    fn the_first_version_chains_to_genesis() {
        let row = VersionRow::project(&version(|v| v), "c1", None, None).expect("projects");
        assert_eq!(row.chain.previous, [0u8; 32], "first entry follows genesis");
        assert_ne!(row.chain.digest, [0u8; 32], "the entry has its own digest");
        assert_ne!(
            row.chain.content, row.chain.digest,
            "the content digest and the entry digest are different values"
        );
    }

    /// A successor links to its predecessor, and the link changes the digest.
    ///
    /// This is the whole property: altering or removing an earlier version
    /// changes every digest after it.
    #[test]
    fn a_successor_links_to_its_predecessor() {
        let first = VersionRow::project(&version(|v| v), "c1", None, None).expect("projects");
        let second = VersionRow::project(&version(|v| v), "c1", Some(first.chain.digest), None)
            .expect("projects");

        assert_eq!(second.chain.previous, first.chain.digest);
        assert_eq!(
            second.chain.content, first.chain.content,
            "same content digests the same"
        );
        assert_ne!(
            second.chain.digest, first.chain.digest,
            "identical content at a different position must not share a digest, \
             or a version could be moved without detection"
        );
    }

    /// The digest is a function of its inputs and nothing else.
    #[test]
    fn chaining_is_deterministic() {
        let a = VersionRow::project(&version(|v| v), "c1", None, None).expect("projects");
        let b = VersionRow::project(&version(|v| v), "c1", None, None).expect("projects");
        assert_eq!(a.chain.digest, b.chain.digest);
    }

    /// Unkeyed by default: a tag is present only when a key is supplied.
    #[test]
    fn the_chain_is_unkeyed_unless_a_key_is_given() {
        let row = VersionRow::project(&version(|v| v), "c1", None, None).expect("projects");
        assert!(row.chain.tag_key_id.is_none());
        assert!(row.chain.tag_mac.is_none());
    }
}
