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

use axa::{AppContext, routes::Routes};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::get,
};
use openehr::base::ObjectVersionId;
use openehr::rm::common::{AuditDetails, PartyProxy, RevisionHistory, RevisionHistoryItem};
use openehr::rm::data_types::DvDateTime;
use openehr::rm::data_types::Text;
use openehr_store::Store as _;
use openehr_store::record::VersionRow;
use serde::Deserialize;

use crate::auth::Principal;
use crate::authz::{Action, authorize};
use crate::controllers::composition::{lock, parse_id, resource, resource_headers};
use crate::controllers::{Refusal, recorded, status_for, store};

type Reply<T> = Result<(StatusCode, HeaderMap, Json<T>), Refusal>;

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
) -> Reply<serde_json::Value> {
    let handle = store(&ctx)?;
    let guard = lock(&handle)?;
    // Every refusal below is recorded, a missing parent `EHR` included: it used
    // to return before the record was written (`db:D-29`).
    let reply = (|| -> Reply<serde_json::Value> {
        // `db:PR12.18`: the token's grants, before the store is asked anything.
        authorize(&principal, &ehr_id, Action::Read)?;
        let id = parse_id(&ehr_id)?;
        let ehr = guard.get_ehr(&id).map_err(|e| status_for(&e))?;
        // The container the record *names* (`openehr_store::ehr_status_container`);
        // whether it owns it is the scoped read's question below (`db:H5.20`).
        let Some(container) = openehr_store::ehr_status_container(&ehr) else {
            return Err(Refusal::from((
                StatusCode::NOT_FOUND,
                format!("EHR {ehr_id} names no EHR_STATUS container"),
            )));
        };
        let row = if let Some(text) = &at.version_at_time {
            let time = DvDateTime::new(text).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "malformed version_at_time".to_owned(),
                )
            })?;
            guard.version_at_time_in(&id, &container, "EHR_STATUS", &time)
        } else {
            // Scoped, never `latest_version`: the reference was submitted with
            // the EHR, so an unscoped read served another chart's status to
            // anyone whose record named its container (`db:D-23`).
            guard.latest_version_in(&id, &container, "EHR_STATUS")
        }
        .map_err(|e| status_for(&e))?;
        // The EHR_STATUS itself, the version in headers (`db:S1.23`).
        let location = format!("/openehr/v1/ehr/{ehr_id}/ehr_status/{}", row.uid);
        Ok((
            StatusCode::OK,
            resource_headers(&row.uid, &location),
            Json(resource(&row)?),
        ))
    })();
    // The target is the status the path asks for. The container id is not
    // used: a refused read must not record a container the caller could not
    // see.
    recorded(
        &ctx,
        &principal,
        "read_ehr_status",
        &ehr_id,
        "ehr_status",
        reply,
    )
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
) -> Reply<serde_json::Value> {
    let handle = store(&ctx)?;
    let guard = lock(&handle)?;
    let reply = (|| -> Reply<serde_json::Value> {
        // `db:PR12.18`: the token's grants, before the store is asked anything.
        authorize(&principal, &ehr_id, Action::Read)?;
        let id: ObjectVersionId = version_uid
            .parse()
            .map_err(|_| (StatusCode::BAD_REQUEST, "malformed version uid".to_owned()))?;
        // Scoped to this EHR and to `EHR_STATUS` (`db:H5.18`, `db:D-19`): it used
        // to return *any* version, composition included, under any EHR id.
        let row = guard
            .get_version_in(&parse_id(&ehr_id)?, &id, "EHR_STATUS")
            .map_err(|e| status_for(&e))?;
        // The EHR_STATUS itself, the version in headers (`db:S1.23`).
        let location = format!("/openehr/v1/ehr/{ehr_id}/ehr_status/{}", row.uid);
        Ok((
            StatusCode::OK,
            resource_headers(&row.uid, &location),
            Json(resource(&row)?),
        ))
    })();
    recorded(
        &ctx,
        &principal,
        "vread_ehr_status",
        &ehr_id,
        &version_uid,
        reply,
    )
}

/// `GET /openehr/v1/ehr/{ehr_id}/versioned_ehr_status/revision_history`
/// (`db:S1.24`): every status version, oldest first, each with its commit
/// `AUDIT_DETAILS`, the committer included (`db:PR12.25`).
async fn revision_history(
    State(ctx): State<AppContext>,
    principal: Principal,
    Path(ehr_id): Path<String>,
) -> Reply<serde_json::Value> {
    let handle = store(&ctx)?;
    let guard = lock(&handle)?;
    let reply = (|| -> Reply<serde_json::Value> {
        // `db:PR12.18`: the token's grants, before the store is asked anything.
        authorize(&principal, &ehr_id, Action::Read)?;
        let id = parse_id(&ehr_id)?;
        let ehr = guard.get_ehr(&id).map_err(|e| status_for(&e))?;
        let Some(container) = openehr_store::ehr_status_container(&ehr) else {
            return Err(Refusal::from((
                StatusCode::NOT_FOUND,
                format!("EHR {ehr_id} names no EHR_STATUS container"),
            )));
        };
        // Scoped as every status read is (`db:H5.20`).
        let rows = guard
            .all_versions_in(&id, &container, "EHR_STATUS")
            .map_err(|e| status_for(&e))?;
        let items = rows
            .iter()
            .map(revision_item)
            .collect::<Result<Vec<_>, _>>()?;
        let history = RevisionHistory::new(items).map_err(|_| {
            (
                StatusCode::NOT_FOUND,
                format!("EHR {ehr_id} has no EHR_STATUS version"),
            )
        })?;
        let body = serde_json::to_value(history).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("the revision history does not serialize: {e}"),
            )
        })?;
        Ok((StatusCode::OK, HeaderMap::new(), Json(body)))
    })();
    recorded(
        &ctx,
        &principal,
        "ehr_status_history",
        &ehr_id,
        "ehr_status",
        reply,
    )
}

/// One stored status version as a `REVISION_HISTORY_ITEM` (`db:S1.24`).
///
/// Every field is rebuilt from what the store holds, and one that does not
/// decode is `500`, naming the version and quoting nothing (`db:D-28`). The
/// description comes back as plain text: the store keeps a description's
/// text, not a coded description's code.
fn revision_item(row: &VersionRow) -> Result<RevisionHistoryItem, (StatusCode, String)> {
    let unreadable = |what: &str| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("the stored {what} of version {} could not be read", row.uid),
        )
    };
    let committer: PartyProxy =
        serde_json::from_str(&row.audit_committer_json).map_err(|_| unreadable("committer"))?;
    let time =
        DvDateTime::new(&row.audit_time_committed.text).map_err(|_| unreadable("commit time"))?;
    let mut audit = AuditDetails::new(
        &row.audit_system_id,
        time,
        &row.audit_change_type_code,
        committer,
    )
    .map_err(|_| unreadable("audit"))?;
    if let Some(description) = &row.audit_description {
        audit = audit.with_description(
            Text::plain(description.clone()).map_err(|_| unreadable("description"))?,
        );
    }
    let version = row.uid.parse().map_err(|_| unreadable("version id"))?;
    RevisionHistoryItem::new(version, vec![audit]).map_err(|_| unreadable("audit"))
}

/// The `versioned_ehr_status` routes (`db:S1.24`), under their own ITS-REST
/// prefix.
#[must_use]
pub fn versioned_routes() -> Routes {
    Routes::new()
        .prefix("/openehr/v1/ehr/{ehr_id}/versioned_ehr_status")
        .add("/revision_history", get(revision_history))
}

/// The routes this controller owns.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/openehr/v1/ehr/{ehr_id}/ehr_status")
        .add("/", get(read))
        .add("/{version_uid}", get(vread))
}
