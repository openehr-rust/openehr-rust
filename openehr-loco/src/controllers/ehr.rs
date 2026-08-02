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

use crate::controllers::{status_for, store};

type Reply<T> = Result<(StatusCode, Json<T>), (StatusCode, String)>;

/// `POST /openehr/v1/ehr`
async fn create(State(ctx): State<AppContext>, Json(ehr): Json<Ehr>) -> Reply<Ehr> {
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
async fn read(State(ctx): State<AppContext>, Path(ehr_id): Path<String>) -> Reply<Ehr> {
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
