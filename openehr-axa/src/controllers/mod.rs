//! HTTP controllers.
//!
//! # Status codes are this crate's actual work
//!
//! Everything else here forwards to the store. What a controller decides is
//! what a caller is told, and the decision that matters is the one below.

pub mod composition;
pub mod contribution;
pub mod ehr;
pub mod ehr_status;
pub mod metadata;
pub mod query;

use axa::AppContext;
use axum::http::{HeaderMap, StatusCode};
use openehr::rm::common::{PartyProxy, Version};
use openehr::rm::ehr::Composition;
use openehr_store::StoreError;

use crate::access::{SharedAccessLog, access};
use crate::app::SharedOpenehrStore;
use crate::auth::Principal;

/// A refusal: the status a caller is told and the sentence that says what to do
/// about it.
///
/// Sent as an RFC 9457 problem, with the sentence as `detail`. Axa's standard
/// middleware never sends a handler's plain-text body — it replaces one with a
/// generic problem so that an error cannot quote an internal string by
/// accident — so a message the caller needs (where a deleted record's history
/// is, what to send instead of `DELETE`) has to travel as the problem's
/// `detail`, which is the one field it keeps.
#[derive(Debug)]
pub struct Refusal(pub StatusCode, pub String);

impl From<(StatusCode, String)> for Refusal {
    fn from((status, message): (StatusCode, String)) -> Self {
        Self(status, message)
    }
}

impl axum::response::IntoResponse for Refusal {
    fn into_response(self) -> axum::response::Response {
        let Self(status, message) = self;
        axa::Error::with_detail(status, message).into_response()
    }
}

/// Borrows the store from the shared state.
///
/// A `503` when it is absent, because absent means [`crate::app::App::prepare`]
/// did not run or did not finish — the service is not ready, which is a
/// different thing from the request being wrong.
///
/// # Errors
///
/// Returns `503` if the store was never installed into the shared state.
pub fn store(ctx: &AppContext) -> Result<SharedOpenehrStore, (StatusCode, String)> {
    ctx.shared_store.get::<SharedOpenehrStore>().map_or_else(
        || {
            Err((
                StatusCode::SERVICE_UNAVAILABLE,
                "store is not initialised".to_owned(),
            ))
        },
        |s| Ok(s.clone()),
    )
}

/// Maps a store error onto a status code.
///
/// # The distinction this crate exists to get right
///
/// **Deleted is `410 Gone`. Never existed is `404 Not Found`.**
///
/// openEHR does not remove anything: a deletion is a new version carrying a
/// deleted lifecycle state, and the history it supersedes stays (`db:H5.2`).
/// So a caller asking for a deleted record is asking about something that *was*
/// there — and `404` would say it never was. That is not a nicety. A clinician
/// or an auditor told "not found" concludes the record never existed; told
/// "gone", they know to ask for the history.
///
/// `410` is decided by the caller of this function, which has the version in
/// hand; this maps everything else.
#[must_use]
pub fn status_for(error: &StoreError) -> (StatusCode, String) {
    let code = match error {
        StoreError::NotFound { .. } => StatusCode::NOT_FOUND,
        // Three different situations, one answer. `Conflict` is "it already
        // exists"; `Commit` is "another writer took that position in the
        // version tree" (`db:H5.9`); `NotModifiable` is "the record is
        // deactivated" (`db:H5.17`). All three are the caller's to resolve —
        // by re-reading and retrying, or by reactivating the record first —
        // and `409` is what says "the request conflicts with the current
        // state of the target resource" rather than blaming the request
        // itself, which is what a `4xx` outside the 409/412 pair would imply.
        //
        // A version naming a change set it is not part of (`db:H5.22`) is the
        // same kind of refusal: the request conflicts with the declared state.
        StoreError::Conflict { .. }
        | StoreError::Commit(_)
        | StoreError::NotModifiable { .. }
        | StoreError::NotInContribution { .. } => StatusCode::CONFLICT,
        StoreError::Invalid(_) | StoreError::Parse(_) => StatusCode::UNPROCESSABLE_ENTITY,
        // The store refuses what it cannot persist rather than dropping it
        // silently (`db:D-07`). `501` rather than `400`: the request is
        // well-formed and this service cannot honour it.
        StoreError::Unsupported { .. } => StatusCode::NOT_IMPLEMENTED,
        // `400`, not `501`: an AQL query outside the bounded subset
        // (`db:P6.20`–`P6.24`) is a malformed *request* — the caller wrote a
        // query this executor does not run — not a capability this service
        // is missing across the board the way `Unsupported` names.
        //
        // A query over the executor's work budget (`db:P6.26`) is the same:
        // the caller's to resolve by narrowing it. Not `500`, since nothing
        // failed, and not truncated, since the refusal is the answer.
        StoreError::AqlUnsupported(_) | StoreError::AqlBudgetExceeded { .. } => {
            StatusCode::BAD_REQUEST
        }
        StoreError::SchemaVersionMismatch { .. } => StatusCode::SERVICE_UNAVAILABLE,
        // Everything else, including variants added after this was written.
        //
        // `StoreError` is `#[non_exhaustive]`, so a catch-all is required — the
        // opposite trade-off from `ColTy`, which is deliberately *not*
        // `non_exhaustive` so that adding a variant breaks every dialect at
        // compile time (`db:M3.30`). There a wildcard hides a defect; here the
        // enum is open by design, and `500` is the conservative answer for a
        // failure this build has never heard of. Never a 2xx, and never a 4xx,
        // which would blame the caller for something unknown.
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    // The message names identifiers, tables, and rules and never record content
    // (`db:M3.38`), so it is safe to return. That is a property of the store's
    // errors, not of this function, and it is why this can forward them at all.
    (code, error.to_string())
}

/// Whether the committer a request supplies is the caller who sent it.
///
/// # The check `db:PR12.19` asks for
///
/// A verified subject MUST NOT silently replace the committer in the body, and
/// a body naming somebody else MUST NOT be written. Preferring the token would
/// overwrite the caller's stated intent without trace; preferring the body
/// would let a verified caller commit under another clinician's name — which
/// is the forgery `db:PR12.15` is built to prevent, arriving through the front
/// door instead.
///
/// So neither wins. They must agree, or nothing is written.
///
/// # Why this rule may live at the HTTP edge
///
/// `db:S1.19` forbids a service enforcing clinical behaviour, on the grounds
/// that a rule at the edge stops applying the moment somebody uses the store
/// directly. This is not one of those: it is a rule about the relationship
/// between a **caller** and a record, and there is no caller when the store is
/// used directly. It cannot live lower down because nothing lower down has a
/// token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attribution {
    /// An identifier on the committer equals the verified subject.
    Agrees,
    /// The committer is identified, and as somebody else.
    Disagrees,
    /// The committer carries no identifier to compare.
    ///
    /// Not the same as disagreeing, and answered differently: the caller has
    /// not tried to impersonate anyone, they have sent something this service
    /// cannot attribute.
    Unidentified,
}

/// Compares a committer against the verified subject.
///
/// Matches on `external_ref.id` or on any `DV_IDENTIFIER.id`, never on the
/// **name**. A name is a display string — two clinicians share one, one
/// clinician changes theirs — and an audit trail keyed on it is one that stops
/// being able to answer who acted the day somebody marries.
#[must_use]
pub fn attribution(committer: &PartyProxy, subject: &str) -> Attribution {
    let external = committer
        .external_ref()
        .map(|reference| reference.id().to_string());
    let mut candidates = committer
        .identifiers()
        .iter()
        .map(|identifier| identifier.id().to_owned())
        .chain(external)
        .peekable();

    if candidates.peek().is_none() {
        return Attribution::Unidentified;
    }
    if candidates.any(|candidate| candidate == subject) {
        Attribution::Agrees
    } else {
        Attribution::Disagrees
    }
}

/// Refuses a commit whose committer is not the caller.
///
/// # Errors
///
/// `403` when the body names somebody else — the caller is who they say they
/// are and may not act as another; `422` when the committer carries no
/// identifier, because the request is well-formed openEHR that this service
/// cannot attribute.
pub fn check_attribution(
    version: &Version<Composition>,
    principal: &Principal,
) -> Result<(), (StatusCode, String)> {
    check_committer(version.commit_audit().committer(), principal)
}

/// [`check_attribution`] for any `AUDIT_DETAILS.committer`: a version's, or a
/// contribution's own (`db:PR12.19`, `db:PR12.25`, `db:D-25`). One rule for
/// both, so they cannot drift apart.
///
/// # Errors
///
/// As [`check_attribution`].
pub fn check_committer(
    committer: &openehr::rm::common::PartyProxy,
    principal: &Principal,
) -> Result<(), (StatusCode, String)> {
    match attribution(committer, &principal.subject) {
        Attribution::Agrees => Ok(()),
        Attribution::Disagrees => Err((
            StatusCode::FORBIDDEN,
            "the committer in AUDIT_DETAILS is not the verified caller; this service \
             will not commit under another party's identity, and will not silently \
             replace the one you sent (PR12.19)"
                .to_owned(),
        )),
        Attribution::Unidentified => Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            "the committer in AUDIT_DETAILS carries no identifier, so it cannot be \
             checked against the verified caller. Give the committer an external_ref \
             or a DV_IDENTIFIER whose id is the token subject (PR12.19)"
                .to_owned(),
        )),
    }
}

/// Records a read, and refuses it if the record cannot be written.
///
/// # Why the `?` on this is load-bearing
///
/// Every read handler calls this **before** returning clinical content, and
/// propagates its error. That ordering is the whole guarantee (`db:PR12.6`):
/// there is no path on which a body is served for an access that was not
/// recorded. A handler that called this afterwards, or ignored the result,
/// would leave the log looking complete while missing exactly the reads that
/// happened as it failed.
///
/// A `503` rather than a `500`: the request is fine and the service is not
/// currently able to serve it safely, which is a state a caller may retry and
/// a load balancer should route around.
///
/// # Errors
///
/// `503` when the access record could not be written.
pub fn record_read(
    ctx: &AppContext,
    principal: &Principal,
    action: &str,
    ehr: &str,
    target: &str,
    outcome: &str,
) -> Result<(), (StatusCode, String)> {
    let Some(log) = ctx.shared_store.get::<SharedAccessLog>() else {
        // Absent means `prepare` has not finished. Reads must not be served
        // ahead of the thing that records them.
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "the access log is not initialised".to_owned(),
        ));
    };
    log.record(&access(principal, action, ehr, target, outcome))
        .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, e))
}

/// The outcome an access record states, read from the status of the response
/// about to be sent (`db:PR12.23`, `db:D-29`).
///
/// From the response rather than from the store lookup. A lookup that
/// succeeded is not a read that succeeded: the page can still be refused and
/// the document can still fail to decode.
#[must_use]
pub fn outcome_of_status(status: StatusCode) -> &'static str {
    match status {
        s if s.is_success() => "ok",
        StatusCode::NOT_FOUND => "not_found",
        StatusCode::GONE => "gone",
        // The token's grants did not cover it (`db:PR12.18`). Neither a refused
        // request nor a missing record: the caller was not allowed to ask.
        StatusCode::FORBIDDEN => "denied",
        s if s.is_server_error() => "error",
        _ => "refused",
    }
}

/// Records a read whose response is already decided, then releases it.
///
/// If the record cannot be written, the response is dropped and the caller
/// gets `503`, as [`record_read`] does: no clinical content leaves ahead of
/// its record.
///
/// # Errors
///
/// The response's own refusal, or `503` if the read cannot be recorded.
pub fn recorded<T>(
    ctx: &AppContext,
    principal: &Principal,
    action: &str,
    ehr: &str,
    target: &str,
    reply: Result<(StatusCode, HeaderMap, axum::Json<T>), Refusal>,
) -> Result<(StatusCode, HeaderMap, axum::Json<T>), Refusal> {
    let status = match &reply {
        Ok((status, ..)) | Err(Refusal(status, _)) => *status,
    };
    record_read(
        ctx,
        principal,
        action,
        ehr,
        target,
        outcome_of_status(status),
    )?;
    reply
}

/// The outcome label for a store result, for [`record_read`]. Used where
/// nothing after the lookup can still refuse the response (`ehr`, `query`);
/// everywhere else, [`recorded`].
#[must_use]
pub fn outcome_of<T>(result: &Result<T, openehr_store::StoreError>) -> &'static str {
    match result {
        Ok(_) => "ok",
        Err(StoreError::NotFound { .. }) => "not_found",
        Err(_) => "refused",
    }
}
