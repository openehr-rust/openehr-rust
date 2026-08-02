//! HTTP controllers.
//!
//! # Status codes are this crate's actual work
//!
//! Everything else here forwards to the store. What a controller decides is
//! what a caller is told, and the decision that matters is the one below.

pub mod composition;
pub mod ehr;
pub mod metadata;

use axum::http::StatusCode;
use loco_rs::app::AppContext;
use openehr_store::StoreError;

use crate::app::SharedOpenehrStore;

/// Borrows the store from the shared state.
///
/// A `503` when it is absent, because absent means [`crate::app::App::before_run`]
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
        // Two different situations, one answer. `Conflict` is "it already
        // exists"; `Commit` is "another writer took that position in the
        // version tree" (`db:H5.9`). Both are the caller's to resolve by
        // re-reading and retrying, and `409` is what says so.
        StoreError::Conflict { .. } | StoreError::Commit(_) => StatusCode::CONFLICT,
        StoreError::Invalid(_) | StoreError::Parse(_) => StatusCode::UNPROCESSABLE_ENTITY,
        // The store refuses what it cannot persist rather than dropping it
        // silently (`db:D-07`). `501` rather than `400`: the request is
        // well-formed and this service cannot honour it.
        StoreError::Unsupported { .. } => StatusCode::NOT_IMPLEMENTED,
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
