//! Contribution endpoints.
//!
//! # Why a contribution is created explicitly here
//!
//! A `CONTRIBUTION` is a change set: one clinical act that produced one or more
//! versions, with its **own** `AUDIT_DETAILS` distinct from theirs
//! (`db:PR12.10`). Collapsing the two loses the fact that one act produced
//! several versions, which is exactly what an investigation reconstructs.
//!
//! A service could mint one per commit and hide it. That would mean inventing
//! the change set's audit — its committer, its time, its reason — which is
//! `db:PR12.9` one level up: the store must not synthesise a committer, and a
//! service must not synthesise the act. So the caller declares the act, then
//! commits versions into it.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    routing::post,
};
use loco_rs::{app::AppContext, controller::Routes};
use openehr::rm::common::Contribution;
use openehr_store::Store as _;

use crate::auth::Principal;
use crate::controllers::composition::{lock, parse_id};
use crate::controllers::{status_for, store};

/// `POST /openehr/v1/ehr/{ehr_id}/contribution`
///
/// The [`Principal`] gates the route and is not read: a `CONTRIBUTION` carries
/// its own committer in its own audit, and `db:PR12.19` is checked where the
/// versions are committed. Checking it twice from two places would be two
/// rules that agree until one of them is edited.
async fn create(
    State(ctx): State<AppContext>,
    _principal: Principal,
    Path(ehr_id): Path<String>,
    Json(contribution): Json<Contribution>,
) -> Result<(StatusCode, Json<Contribution>), (StatusCode, String)> {
    let handle = store(&ctx)?;
    let mut guard = lock(&handle)?;
    guard
        .create_contribution(&parse_id(&ehr_id)?, &contribution)
        .map_err(|e| status_for(&e))?;
    Ok((StatusCode::CREATED, Json(contribution)))
}

/// The routes this controller owns.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/openehr/v1/ehr/{ehr_id}/contribution")
        .add("/", post(create))
}
