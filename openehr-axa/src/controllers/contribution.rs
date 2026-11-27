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

use axa::{AppContext, routes::Routes};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    routing::post,
};
use openehr::rm::common::Contribution;
use openehr_store::Store as _;

use crate::auth::Principal;
use crate::authz::{Action, authorize};
use crate::controllers::composition::{lock, parse_id};
use crate::controllers::{Refusal, check_committer, status_for, store};

/// `POST /openehr/v1/ehr/{ehr_id}/contribution`
///
/// The contribution's own committer is checked against the verified caller
/// before anything is written (`db:PR12.19`, `db:D-25`): `403` for someone
/// else, `422` for no identifier, by the same rule a version commit uses
/// (`controllers::check_committer`). This route used to discard the
/// principal. Checking only each version's committer later could not
/// establish who declared the change set, which has its own audit
/// (`db:PR12.10`). That committer is stored whole (`db:PR12.25`).
async fn create(
    State(ctx): State<AppContext>,
    principal: Principal,
    Path(ehr_id): Path<String>,
    Json(contribution): Json<Contribution>,
) -> Result<(StatusCode, Json<Contribution>), Refusal> {
    authorize(&principal, &ehr_id, Action::Write)?;
    check_committer(contribution.audit().committer(), &principal)?;
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
