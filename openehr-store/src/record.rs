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
}

impl VersionRow {
    /// Projects a version onto a row.
    ///
    /// # Errors
    ///
    /// Returns [`crate::StoreError::Json`] if the content cannot be
    /// canonicalised.
    pub fn project<T: Serialize>(version: &Version<T>, contribution_uid: &str) -> Result<Self> {
        let uid = version.uid();
        let audit = version.commit_audit();
        let data_json = version
            .data()
            .map(openehr::security::to_canonical_string)
            .transpose()?;
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
        })
    }
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
