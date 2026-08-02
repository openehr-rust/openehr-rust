//! EHR endpoints.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use loco_rs::{app::AppContext, controller::Routes};
use openehr::rm::ehr::Ehr;
use openehr_store::Store as _;

use crate::auth::Principal;
use crate::controllers::{status_for, store};

type Reply<T> = Result<(StatusCode, Json<T>), (StatusCode, String)>;

/// `POST /openehr/v1/ehr`
///
/// The [`Principal`] is extracted and not read. Creating an EHR records no
/// committer — `EHR` carries no `AUDIT_DETAILS`; its contained versions do — so
/// there is nothing here for the verified subject to attribute. It is required
/// because the route must not be open, not because this handler uses it.
async fn create(
    State(ctx): State<AppContext>,
    _principal: Principal,
    Json(ehr): Json<Ehr>,
) -> Reply<Ehr> {
    let handle = store(&ctx)?;
    let mut guard = handle.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "store lock poisoned".to_owned(),
        )
    })?;
    guard.create_ehr(&ehr).map_err(|e| status_for(&e))?;
    Ok((StatusCode::CREATED, Json(ehr)))
}

/// `GET /openehr/v1/ehr/{ehr_id}`
///
/// Verified, then discarded — this layer records no reads (`db:PR12.5`).
async fn read(
    State(ctx): State<AppContext>,
    _principal: Principal,
    Path(ehr_id): Path<String>,
) -> Reply<Ehr> {
    let handle = store(&ctx)?;
    let guard = handle.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "store lock poisoned".to_owned(),
        )
    })?;
    let id = ehr_id
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "malformed ehr_id".to_owned()))?;
    let ehr = guard.get_ehr(&id).map_err(|e| status_for(&e))?;
    Ok((StatusCode::OK, Json(ehr)))
}

/// The routes this controller owns.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/openehr/v1/ehr")
        .add("/", post(create))
        .add("/{ehr_id}", get(read))
}
