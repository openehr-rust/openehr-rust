//! `EHR_ACCESS` and access-control settings.
//!
//! # openEHR deliberately does not specify a scheme
//!
//! `EHR_ACCESS` has exactly two things in it: the name of an access-control
//! scheme, and an `ACCESS_CONTROL_SETTINGS` object which is **abstract**.
//! openEHR's own words are that it takes "a completely flexible approach",
//! because access control in health is jurisdictional — a Dutch opt-out
//! register, an English legitimate-relationship model, and a US
//! break-the-glass workflow are not the same policy and cannot be one class.
//!
//! That leaves an implementation two honest options and one dishonest one:
//!
//! | Option | This crate |
//! | --- | --- |
//! | carry unknown schemes losslessly | [`AccessControlSettings::Opaque`] |
//! | offer a documented reference scheme | [`GroupSettings`] |
//! | invent a scheme and imply it is openEHR's | **no** |
//!
//! # Everything defaults to deny
//!
//! [`Decision`] has no "unknown" variant that a caller might treat as
//! permissive, and an [`AccessControlSettings::Opaque`] whose scheme this
//! process does not implement denies with
//! [`DenyReason::SchemeNotImplemented`]. A policy engine that cannot evaluate a
//! policy has not established that access is permitted, and in a record system
//! the failure mode of the other default is a disclosure.
//!
//! # This is not the whole trust boundary
//!
//! | This module provides | The deployment must provide |
//! | --- | --- |
//! | a place to record the policy | authentication |
//! | evaluation of the reference scheme | the principal's group membership |
//! | lossless carriage of other schemes | consent capture and withdrawal |
//! | a default-deny decision | audit of the decision |
//!
//! Recording *who acted* is [`crate::rm::common::AuditDetails`]; deciding
//! whether they may is this module; proving who they are is neither.

use crate::base::AccessGroupRef;
use crate::rm::common::{LocatableAttrs, impl_locatable};
use serde::{Deserialize, Serialize};

/// What a principal is asking to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Operation {
    /// Read existing content.
    Read,
    /// Commit new or amended content.
    Write,
    /// Commit a version that logically deletes content.
    Delete,
    /// Read the audit trail without reading clinical content.
    ///
    /// Separated from [`Operation::Read`] on purpose: an information governance
    /// officer investigating an access complaint needs the audit trail and does
    /// **not** need the clinical record, and a scheme that cannot express that
    /// forces the investigation to over-collect.
    Audit,
}

/// Who is asking, and what groups they hold.
///
/// Group membership is supplied by the caller rather than looked up, because
/// resolving a principal to groups is a directory operation and this crate has
/// no directory. Passing it in makes the dependency visible instead of
/// pretending it does not exist.
#[derive(Debug, Clone, PartialEq)]
pub struct AccessRequest<'a> {
    /// The operation requested.
    pub operation: Operation,
    /// The access-group identifiers the principal holds.
    pub groups: &'a [String],
    /// Whether the principal is the subject of the record.
    ///
    /// Kept separate from group membership because "the patient themselves" is
    /// a relationship, not a role, and every jurisdiction's rules about it
    /// differ from its rules about staff.
    pub is_subject: bool,
}

/// The outcome of evaluating a policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Access is permitted.
    Permit,
    /// Access is refused, and why.
    Deny(DenyReason),
}

impl Decision {
    /// Whether access was permitted.
    #[must_use]
    pub fn is_permit(&self) -> bool {
        matches!(self, Self::Permit)
    }
}

/// Why access was refused.
///
/// The reasons are distinguished because they call for different responses: a
/// missing group is a provisioning problem, an unimplemented scheme is a
/// deployment problem, and no-settings is a data problem. Collapsing them to
/// "denied" leaves an operator with nothing to act on.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DenyReason {
    /// The principal holds none of the groups the policy requires.
    NotInPermittedGroup,
    /// The `EHR_ACCESS` carries no settings, so no policy has been recorded.
    NoSettingsRecorded,
    /// The settings name a scheme this process cannot evaluate.
    SchemeNotImplemented {
        /// The scheme named in the settings.
        scheme: String,
    },
    /// The record is not modifiable and the operation would write to it.
    RecordNotModifiable,
}

/// The reference scheme: allow-lists of access groups per operation.
///
/// Deliberately simple, and deliberately **not** presented as openEHR's scheme
/// — it is one this crate defines, named
/// [`GroupSettings::SCHEME`], so that a deployment which needs nothing more has
/// something that works and a deployment which needs more knows it is
/// replacing something local rather than something standard.
///
/// ```
/// use openehr::security::access::{AccessRequest, Decision, GroupSettings, Operation};
///
/// let settings = GroupSettings::new()
///     .permit(Operation::Read, "care-team")
///     .permit(Operation::Write, "care-team")
///     .permit(Operation::Audit, "information-governance");
///
/// let request = AccessRequest {
///     operation: Operation::Read,
///     groups: &["care-team".to_string()],
///     is_subject: false,
/// };
/// assert_eq!(settings.decide(&request), Decision::Permit);
///
/// // An auditor may read the trail and not the record.
/// let ig = ["information-governance".to_string()];
/// assert!(settings.decide(&AccessRequest { operation: Operation::Audit, groups: &ig, is_subject: false }).is_permit());
/// assert!(!settings.decide(&AccessRequest { operation: Operation::Read, groups: &ig, is_subject: false }).is_permit());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GroupSettings {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    read_groups: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    write_groups: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    delete_groups: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    audit_groups: Vec<String>,
    #[serde(default)]
    subject_may_read: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    group_refs: Vec<AccessGroupRef>,
}

impl GroupSettings {
    /// The scheme name these settings declare.
    pub const SCHEME: &'static str = "openehr-rs.group-list.v1";

    /// Empty settings, which permit nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Permits an operation to a group.
    #[must_use]
    pub fn permit(mut self, operation: Operation, group: impl Into<String>) -> Self {
        let group = group.into();
        match operation {
            Operation::Read => self.read_groups.push(group),
            Operation::Write => self.write_groups.push(group),
            Operation::Delete => self.delete_groups.push(group),
            Operation::Audit => self.audit_groups.push(group),
        }
        self
    }

    /// Permits the record's subject to read it.
    #[must_use]
    pub fn permit_subject_read(mut self) -> Self {
        self.subject_may_read = true;
        self
    }

    /// Attaches a resolvable reference to a group definition.
    #[must_use]
    pub fn with_group_ref(mut self, group_ref: AccessGroupRef) -> Self {
        self.group_refs.push(group_ref);
        self
    }

    /// The groups permitted an operation.
    #[must_use]
    pub fn groups_for(&self, operation: Operation) -> &[String] {
        match operation {
            Operation::Read => &self.read_groups,
            Operation::Write => &self.write_groups,
            Operation::Delete => &self.delete_groups,
            Operation::Audit => &self.audit_groups,
        }
    }

    /// Evaluates a request.
    #[must_use]
    pub fn decide(&self, request: &AccessRequest<'_>) -> Decision {
        if request.is_subject && self.subject_may_read && request.operation == Operation::Read {
            return Decision::Permit;
        }
        let permitted = self.groups_for(request.operation);
        if request.groups.iter().any(|g| permitted.contains(g)) {
            Decision::Permit
        } else {
            Decision::Deny(DenyReason::NotInPermittedGroup)
        }
    }
}

/// Settings for a scheme this crate does not implement, carried unchanged.
///
/// The point is round-trip fidelity: a Dutch or Norwegian deployment's policy
/// object passes through this crate byte-for-byte, and a process that reads and
/// rewrites a record does not silently drop the policy on it. What it will not
/// do is *evaluate* it — [`AccessControlSettings::decide`] denies with
/// [`DenyReason::SchemeNotImplemented`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpaqueSettings {
    scheme: String,
    #[serde(flatten)]
    settings: serde_json::Map<String, serde_json::Value>,
}

impl OpaqueSettings {
    /// The scheme's name.
    #[must_use]
    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    /// The settings as they arrived.
    #[must_use]
    pub fn settings(&self) -> &serde_json::Map<String, serde_json::Value> {
        &self.settings
    }
}

/// The policy attached to an [`EhrAccess`].
///
/// # Dispatch is by `scheme`, and never by shape
///
/// [`GroupSettings`] has no required field, so a shape-based (`untagged`)
/// reader would match *any* settings object as the reference scheme and
/// silently discard a foreign policy's attributes — turning an
/// un-evaluatable-but-intact policy into an empty one that denies everything
/// and looks deliberate. The `scheme` key is therefore authoritative:
/// [`GroupSettings::SCHEME`] or nothing means the reference scheme, anything
/// else means [`AccessControlSettings::Opaque`].
#[derive(Debug, Clone, PartialEq)]
pub enum AccessControlSettings {
    /// The reference scheme.
    Groups(GroupSettings),
    /// Some other scheme, carried but not evaluated.
    Opaque(OpaqueSettings),
}

impl Serialize for AccessControlSettings {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Groups(g) => {
                #[derive(Serialize)]
                struct Tagged<'a> {
                    scheme: &'static str,
                    #[serde(flatten)]
                    inner: &'a GroupSettings,
                }
                Tagged {
                    scheme: GroupSettings::SCHEME,
                    inner: g,
                }
                .serialize(s)
            }
            Self::Opaque(o) => o.serialize(s),
        }
    }
}

impl<'de> Deserialize<'de> for AccessControlSettings {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;

        let mut map = serde_json::Map::<String, serde_json::Value>::deserialize(d)?;
        let scheme = map
            .get("scheme")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        match scheme.as_deref() {
            None | Some(GroupSettings::SCHEME) => {
                map.remove("scheme");
                serde_json::from_value(serde_json::Value::Object(map))
                    .map(Self::Groups)
                    .map_err(D::Error::custom)
            }
            Some(_) => serde_json::from_value(serde_json::Value::Object(map))
                .map(Self::Opaque)
                .map_err(D::Error::custom),
        }
    }
}

impl AccessControlSettings {
    /// The scheme's name.
    #[must_use]
    pub fn scheme(&self) -> &str {
        match self {
            Self::Groups(_) => GroupSettings::SCHEME,
            Self::Opaque(o) => o.scheme(),
        }
    }

    /// Evaluates a request, denying by default.
    #[must_use]
    pub fn decide(&self, request: &AccessRequest<'_>) -> Decision {
        match self {
            Self::Groups(g) => g.decide(request),
            Self::Opaque(o) => Decision::Deny(DenyReason::SchemeNotImplemented {
                scheme: o.scheme().to_owned(),
            }),
        }
    }
}

/// The access-control object of an EHR.
///
/// ```
/// use openehr::rm::common::LocatableAttrs;
/// use openehr::security::access::{
///     AccessRequest, DenyReason, Decision, EhrAccess, GroupSettings, Operation,
/// };
///
/// // An EHR_ACCESS with no settings recorded denies, and says which of the
/// // several reasons for denial applies.
/// let bare = EhrAccess::new(
///     LocatableAttrs::named("EHR Access", "openEHR-EHR-EHR_ACCESS.generic.v1").unwrap(),
/// );
/// let request = AccessRequest { operation: Operation::Read, groups: &[], is_subject: false };
/// assert_eq!(
///     bare.decide(&request),
///     Decision::Deny(DenyReason::NoSettingsRecorded),
/// );
///
/// let configured = bare.with_settings(
///     GroupSettings::new().permit(Operation::Read, "care-team").into(),
/// );
/// let member = ["care-team".to_string()];
/// assert!(configured
///     .decide(&AccessRequest { operation: Operation::Read, groups: &member, is_subject: false })
///     .is_permit());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EhrAccess {
    #[serde(flatten)]
    locatable: LocatableAttrs,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    settings: Option<AccessControlSettings>,
}

impl_locatable!(EhrAccess, "EHR_ACCESS");

impl EhrAccess {
    /// Builds an `EHR_ACCESS` with no policy recorded.
    #[must_use]
    pub fn new(locatable: LocatableAttrs) -> Self {
        Self {
            locatable,
            settings: None,
        }
    }

    /// Attaches a policy.
    #[must_use]
    pub fn with_settings(mut self, settings: AccessControlSettings) -> Self {
        self.settings = Some(settings);
        self
    }

    /// The policy, if one is recorded.
    #[must_use]
    pub fn settings(&self) -> Option<&AccessControlSettings> {
        self.settings.as_ref()
    }

    /// The scheme's name, if a policy is recorded.
    #[must_use]
    pub fn scheme(&self) -> Option<&str> {
        self.settings.as_ref().map(AccessControlSettings::scheme)
    }

    /// Evaluates a request against the recorded policy.
    ///
    /// Denies when nothing is recorded. An EHR with no access settings has not
    /// been configured to permit anything, and reading "unconfigured" as
    /// "unrestricted" is how a test deployment becomes a breach.
    #[must_use]
    pub fn decide(&self, request: &AccessRequest<'_>) -> Decision {
        match &self.settings {
            None => Decision::Deny(DenyReason::NoSettingsRecorded),
            Some(settings) => settings.decide(request),
        }
    }
}

impl From<GroupSettings> for AccessControlSettings {
    fn from(v: GroupSettings) -> Self {
        Self::Groups(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rm::common::LocatableAttrs;

    fn attrs() -> LocatableAttrs {
        LocatableAttrs::named("EHR Access", "openEHR-EHR-EHR_ACCESS.generic.v1").unwrap()
    }

    #[test]
    fn nothing_recorded_denies_rather_than_permits() {
        let access = EhrAccess::new(attrs());
        let request = AccessRequest {
            operation: Operation::Read,
            groups: &["care-team".to_string()],
            is_subject: false,
        };
        assert_eq!(
            access.decide(&request),
            Decision::Deny(DenyReason::NoSettingsRecorded)
        );
    }

    #[test]
    fn an_unimplemented_scheme_denies_and_names_itself() {
        let opaque: AccessControlSettings = serde_json::from_str(
            r#"{"scheme":"nl.nictiz.opt-out.v2","register":"national","withdrawn":false}"#,
        )
        .unwrap();
        assert_eq!(opaque.scheme(), "nl.nictiz.opt-out.v2");
        let request = AccessRequest {
            operation: Operation::Read,
            groups: &[],
            is_subject: false,
        };
        assert_eq!(
            opaque.decide(&request),
            Decision::Deny(DenyReason::SchemeNotImplemented {
                scheme: "nl.nictiz.opt-out.v2".to_owned()
            })
        );
    }

    #[test]
    fn an_unimplemented_scheme_round_trips_unchanged() {
        // The fidelity guarantee: a policy this crate cannot evaluate must
        // still survive a read-modify-write cycle intact.
        let json = r#"{"register":"national","scheme":"nl.nictiz.opt-out.v2","withdrawn":true}"#;
        let settings: AccessControlSettings = serde_json::from_str(json).unwrap();
        let back = crate::security::canonical::to_canonical_string(&settings).unwrap();
        assert_eq!(back, json);
    }

    #[test]
    fn operations_are_separately_permitted() {
        let settings = GroupSettings::new()
            .permit(Operation::Read, "care-team")
            .permit(Operation::Audit, "information-governance");
        let ig = ["information-governance".to_string()];
        let care = ["care-team".to_string()];

        let permitted = |groups: &[String], op| {
            settings
                .decide(&AccessRequest {
                    operation: op,
                    groups,
                    is_subject: false,
                })
                .is_permit()
        };
        assert!(permitted(&care, Operation::Read));
        assert!(!permitted(&care, Operation::Write));
        assert!(permitted(&ig, Operation::Audit));
        // The separation that matters: an auditor gets the trail, not the
        // clinical content.
        assert!(!permitted(&ig, Operation::Read));
    }

    #[test]
    fn the_subject_reads_only_when_the_policy_says_so() {
        let settings = GroupSettings::new();
        let request = AccessRequest {
            operation: Operation::Read,
            groups: &[],
            is_subject: true,
        };
        assert!(!settings.decide(&request).is_permit());
        assert!(settings.permit_subject_read().decide(&request).is_permit());
    }

    #[test]
    fn subject_read_permission_does_not_leak_into_write() {
        let settings = GroupSettings::new().permit_subject_read();
        let write = AccessRequest {
            operation: Operation::Write,
            groups: &[],
            is_subject: true,
        };
        assert!(!settings.decide(&write).is_permit());
    }

    /// The accessors an `EHR_ACCESS` exposes, and the group list it keeps.
    ///
    /// `scheme()` could return `None`, `Some("")` or `Some("xyzzy")` and
    /// nothing failed; so could `settings()`, and `with_group_ref` could drop
    /// every group it was given. `scheme` is the attribute openEHR's
    /// `Scheme_valid` constrains and `lib:S1.20` declares a departure from, so
    /// a reader who wants to know what policy is in force asks it — and until
    /// now it could have answered anything (`lib:A-09`).
    #[test]
    fn an_ehr_access_reports_the_policy_it_holds() {
        let attrs = |name: &str, node: &str| {
            crate::rm::common::LocatableAttrs::named(name, node).expect("literal")
        };

        // No policy recorded: both accessors say so, and `S1.20` is why that
        // state exists at all — "unset" is not "deny all", and the decision
        // below still denies.
        let bare = EhrAccess::new(attrs("access", "at0000"));
        assert!(bare.settings().is_none());
        assert!(bare.scheme().is_none());

        let group = |id: &str| {
            AccessGroupRef::new(
                "local",
                crate::base::ObjectId::HierObjectId(
                    crate::base::HierObjectId::from_uid_str(id).expect("literal"),
                ),
            )
            .expect("literal")
        };
        let groups = GroupSettings::default()
            .with_group_ref(group("6BA7B810-9DAD-11D1-80B4-00C04FD430C8"))
            .with_group_ref(group("3F2504E0-4F89-11D3-9A0C-0305E82C3301"));
        // Serialising is how a dropped group becomes visible without an
        // accessor for the list.
        let shown = serde_json::to_string(&groups).expect("json");
        assert!(shown.contains("6BA7B810"), "a group reference was dropped: {shown}");
        assert!(shown.contains("3F2504E0"), "a group reference was dropped: {shown}");

        let configured =
            EhrAccess::new(attrs("access", "at0000")).with_settings(groups.clone().into());
        assert!(configured.settings().is_some());
        let scheme = configured.scheme().expect("a configured access has a scheme");
        assert!(!scheme.is_empty(), "openEHR's Scheme_valid wants a name");
        assert_eq!(scheme, AccessControlSettings::from(groups).scheme());
    }
}
