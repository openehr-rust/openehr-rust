//! `EHR_STATUS` endpoints: read only.
//!
//! # Why there is no `PUT` here yet
//!
//! ITS-REST's own `EHR` API says an `EHR_STATUS` "needs to be always created
//! and committed" alongside the `EHR` itself — `POST /ehr`'s request body is
//! an *optional* `EHR_STATUS`, with the server minting the `EHR` around it.
//! This service's `POST /ehr` ([`crate::controllers::ehr`]) instead takes a
//! whole, caller-built [`Ehr`], because [`Ehr::new`] requires its
//! `ehr_status`/`ehr_access` references as parameters rather than minting
//! them — a real, already-tracked request-shape divergence
//! (`tasks.md`'s ITS-REST item, found 2026-09-08). The practical
//! consequence for this module: **no `EHR` created through this service has
//! ever had an `EHR_STATUS` committed for it**, so there is no established
//! "current version" a `PUT` could require `If-Match` against, and no
//! settled answer for what a *first* `PUT` should do — create outright, the
//! way `POST /composition` does, or refuse for want of a precondition, the
//! way `PUT /composition/{uid}` does. Deciding that belongs with fixing
//! `POST /ehr`'s own shape, not as a guess bolted onto this module.
//!
//! What is real without that decision: reading whatever a caller of the
//! [`Store`] trait directly — a test, a migration, a future admin path —
//! has already committed via `commit_ehr_status`. That is what these two
//! endpoints do, and they need no change once `PUT` is added: `GET` here is
//! already correct for the world where an `EHR_STATUS` does exist.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::get,
};
use loco_rs::{app::AppContext, controller::Routes};
use openehr::base::{HierObjectId, ObjectId, ObjectVersionId};
use openehr::rm::data_types::DvDateTime;
use openehr::rm::ehr::Ehr;
use openehr_store::Store as _;
use serde::Deserialize;

use crate::auth::Principal;
use crate::controllers::composition::{headers, lock, parse_id, view};
use crate::controllers::{outcome_of, record_read, status_for, store};
use crate::views::VersionView;

type Reply<T> = Result<(StatusCode, HeaderMap, Json<T>), (StatusCode, String)>;

/// The `EHR_STATUS` container `ehr.ehr_status()` names, when it names one at
/// all.
///
/// `None` when the reference does not hold a [`HierObjectId`] — [`Ehr::new`]
/// checks the reference's declared *type* (`VERSIONED_EHR_STATUS`) but not
/// the identifier *kind* inside it (`lib:A-21`'s own residual). Mirrors
/// `SqliteStore::ehr_is_modifiable`'s identical extraction (`db:H5.17`);
/// duplicated rather than shared because that one is private to
/// `openehr-sqlite` and this service reaches the store only through the
/// [`Store`](openehr_store::Store) trait, which has no method for it.
fn ehr_status_container(ehr: &Ehr) -> Option<HierObjectId> {
    match ehr.ehr_status().id() {
        ObjectId::HierObjectId(uid) => Some(uid.clone()),
        _ => None,
    }
}

/// `?version_at_time=<ISO 8601>`, the one query parameter ITS-REST defines
/// for this resource.
#[derive(Debug, Deserialize)]
pub struct AtTime {
    version_at_time: Option<String>,
}

/// `GET /openehr/v1/ehr/{ehr_id}/ehr_status`
///
/// `?version_at_time=…` reads the version current at that instant
/// (`db:H5.13` skips any commit whose time is not an established instant);
/// without it, the latest.
///
/// `404` when the `EHR` itself does not exist, and `404` again — never a
/// synthesised default — when it does but no `EHR_STATUS` has ever been
/// committed for it. The second case is the ordinary one today (see the
/// module doc): this service does not yet mint one at `POST /ehr`, and
/// answering with a fabricated `is_modifiable: true` object here would
/// assert a committed fact that does not exist.
async fn read(
    State(ctx): State<AppContext>,
    principal: Principal,
    Path(ehr_id): Path<String>,
    Query(at): Query<AtTime>,
) -> Reply<VersionView> {
    let handle = store(&ctx)?;
    let guard = lock(&handle)?;
    let id = parse_id(&ehr_id)?;
    let ehr = guard.get_ehr(&id).map_err(|e| status_for(&e))?;
    let Some(container) = ehr_status_container(&ehr) else {
        return Err((
            StatusCode::NOT_FOUND,
            format!("EHR {ehr_id} names no EHR_STATUS container"),
        ));
    };

    let found = if let Some(text) = &at.version_at_time {
        let time = DvDateTime::new(text).map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "malformed version_at_time".to_owned(),
            )
        })?;
        guard.version_at_time(&container, &time)
    } else {
        guard.latest_version(&container)
    };
    record_read(
        &ctx,
        &principal,
        "read_ehr_status",
        &ehr_id,
        &container.to_string(),
        outcome_of(&found),
    )?;
    let row = found.map_err(|e| status_for(&e))?;
    let head = headers(&row.uid);
    Ok((StatusCode::OK, head, Json(view(&row))))
}

/// `GET /openehr/v1/ehr/{ehr_id}/ehr_status/{version_uid}` — vread.
///
/// Path shape matches ITS-REST exactly: `ehr_status/{version_uid}`, not
/// `ehr_status/version/{version_uid}` the way composition's own vread is
/// `composition/{uid}/version/{version_uid}` — the two resources' own
/// specifications disagree on this, and this crate follows each one rather
/// than forcing a shared shape neither actually has.
async fn vread(
    State(ctx): State<AppContext>,
    principal: Principal,
    Path((ehr_id, version_uid)): Path<(String, String)>,
) -> Reply<VersionView> {
    let handle = store(&ctx)?;
    let guard = lock(&handle)?;
    let id: ObjectVersionId = version_uid
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "malformed version uid".to_owned()))?;
    let found = guard.get_version(&id);
    record_read(
        &ctx,
        &principal,
        "vread_ehr_status",
        &ehr_id,
        &version_uid,
        outcome_of(&found),
    )?;
    let row = found.map_err(|e| status_for(&e))?;
    let head = headers(&row.uid);
    Ok((StatusCode::OK, head, Json(view(&row))))
}

/// The routes this controller owns.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/openehr/v1/ehr/{ehr_id}/ehr_status")
        .add("/", get(read))
        .add("/{version_uid}", get(vread))
}
