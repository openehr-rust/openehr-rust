//! Authorization from the token's own grants (`db:PR12.18`, amended
//! 2026-11-26).
//!
//! The issuer decides who may open which record: it holds the care
//! relationships, and this service does not. The service enforces exactly
//! what the issuer wrote into the verified token, as `openehr_grants`, and
//! refuses everything else. There is no default grant, nothing is inferred
//! from a role or a name, and a grant is checked before the store is asked
//! anything, so a refusal reveals nothing about whether a record exists.

use axum::http::StatusCode;
use openehr::base::HierObjectId;
use serde::Deserialize;

use crate::auth::Principal;

/// The claim name.
pub const GRANTS_CLAIM: &str = "openehr_grants";

/// What a request does to a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    /// Any `GET` under an `EHR`, and an AQL query resolved to one.
    Read,
    /// Creating the `EHR`, declaring a contribution, committing a version.
    Write,
}

impl Action {
    fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
        }
    }
}

/// Which records a grant covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// Every record. Minted deliberately by the issuer, never assumed.
    All,
    /// One record, by its parsed id.
    Ehr(String),
}

/// One grant: a scope and the actions it allows there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grant {
    /// Which records.
    pub scope: Scope,
    /// What may be done to them.
    pub actions: Vec<Action>,
}

/// Every grant a token carries. Empty means no clinical access at all.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Grants(Vec<Grant>);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGrant {
    ehr: String,
    actions: Vec<Action>,
}

impl Grants {
    /// No grants.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// Parses the `openehr_grants` claim. `None` (no claim) is no grants.
    ///
    /// # Errors
    ///
    /// A message when the claim is present and is not a list of
    /// `{"ehr", "actions"}` objects naming `"*"` or a valid `EHR` id and the
    /// actions `read`/`write`. The caller treats the whole token as invalid.
    /// A grant the service cannot read is not one it may guess at.
    pub fn from_claim(claim: Option<&serde_json::Value>) -> Result<Self, String> {
        let Some(value) = claim else {
            return Ok(Self::none());
        };
        let raw: Vec<RawGrant> = serde_json::from_value(value.clone())
            .map_err(|_| format!("the {GRANTS_CLAIM} claim is malformed"))?;
        raw.into_iter()
            .map(|g| {
                let scope = if g.ehr == "*" {
                    Scope::All
                } else {
                    Scope::Ehr(canonical(&g.ehr).ok_or_else(|| {
                        format!("the {GRANTS_CLAIM} claim names an EHR id that does not parse")
                    })?)
                };
                Ok(Grant {
                    scope,
                    actions: g.actions,
                })
            })
            .collect::<Result<Vec<_>, String>>()
            .map(Self)
    }

    /// Whether some grant covers `action` on the record `ehr_id`.
    ///
    /// An id that does not parse is covered by no record grant, only by
    /// `"*"`. The route refuses it as malformed afterwards.
    #[must_use]
    pub fn allows(&self, ehr_id: &str, action: Action) -> bool {
        let id = canonical(ehr_id);
        self.0.iter().any(|grant| {
            grant.actions.contains(&action)
                && match &grant.scope {
                    Scope::All => true,
                    Scope::Ehr(granted) => id.as_deref() == Some(granted.as_str()),
                }
        })
    }
}

impl Grants {
    /// Whether a `"*"` grant covers `action`, for a request that names no
    /// record yet: `POST /ehr`, which mints one (`db:S1.22`).
    #[must_use]
    pub fn allows_any(&self, action: Action) -> bool {
        self.0
            .iter()
            .any(|grant| grant.scope == Scope::All && grant.actions.contains(&action))
    }
}

/// An `EHR` id in the one spelling grants and paths are compared in.
fn canonical(raw: &str) -> Option<String> {
    raw.parse::<HierObjectId>().ok().map(|id| id.to_string())
}

/// Refuses a request the caller's grants do not cover (`db:PR12.18`).
///
/// Call it before the store is asked anything, so the answer is the same for
/// a record that exists and one that does not.
///
/// # Errors
///
/// `403`, naming the action and the record the caller themselves named.
pub fn authorize(
    principal: &Principal,
    ehr_id: &str,
    action: Action,
) -> Result<(), (StatusCode, String)> {
    if principal.grants.allows(ehr_id, action) {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            format!(
                "the token grants no {} on EHR {ehr_id} (PR12.18: access is what the \
                 issuer granted in {GRANTS_CLAIM}, and nothing else)",
                action.as_str()
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, Grants};
    use serde_json::json;

    const EHR: &str = "7d44b88c-4199-4bad-97dc-d78268e01398";
    const OTHER: &str = "8e55c99d-5200-4cbe-a8ed-e89379f12409";

    #[test]
    fn no_claim_grants_nothing() {
        let grants = Grants::from_claim(None).expect("absent is valid");
        assert!(!grants.allows(EHR, Action::Read));
        assert!(!grants.allows(EHR, Action::Write));
    }

    #[test]
    fn a_grant_covers_its_record_and_its_actions_only() {
        let grants =
            Grants::from_claim(Some(&json!([{"ehr": EHR, "actions": ["read"]}]))).expect("valid");
        assert!(grants.allows(EHR, Action::Read));
        assert!(!grants.allows(EHR, Action::Write));
        assert!(!grants.allows(OTHER, Action::Read));
        assert!(!grants.allows("not-an-id", Action::Read));
        // Compared in the spelling the store itself compares in. Another spelling
        // is another record there, and here it is not covered: fail closed.
        assert!(!grants.allows(&EHR.to_uppercase(), Action::Read));
    }

    #[test]
    fn a_wildcard_covers_every_record_for_its_actions_only() {
        let grants =
            Grants::from_claim(Some(&json!([{"ehr": "*", "actions": ["write"]}]))).expect("valid");
        assert!(grants.allows(EHR, Action::Write));
        assert!(grants.allows(OTHER, Action::Write));
        assert!(!grants.allows(EHR, Action::Read));
    }

    #[test]
    fn a_malformed_claim_is_refused_whole() {
        for bad in [
            json!("read"),
            json!({"ehr": EHR, "actions": ["read"]}),
            json!([{"ehr": EHR}]),
            json!([{"ehr": EHR, "actions": ["admin"]}]),
            json!([{"ehr": "", "actions": ["read"]}]),
            json!([{"ehr": "two words", "actions": ["read"]}]),
            json!([{"ehr": EHR, "actions": ["read"], "role": "doctor"}]),
        ] {
            assert!(Grants::from_claim(Some(&bad)).is_err(), "{bad}");
        }
    }
}
