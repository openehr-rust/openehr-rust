//! Composition endpoints: create, read, delete, `_history`, and vread.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    routing::{delete, get},
};
use loco_rs::{app::AppContext, controller::Routes};
use openehr::base::HierObjectId;
use openehr_store::{Store as _, record::VersionRow};
use serde::Deserialize;

use crate::controllers::{status_for, store};
use crate::views::{Page, VersionView};

type Reply<T> = Result<(StatusCode, HeaderMap, Json<T>), (StatusCode, String)>;

/// `_count` and `_offset`, with a cap.
///
/// The cap is not politeness. An uncapped `_count` lets one request ask for a
/// record's entire history, and a store that answers it has handed a caller the
/// ability to exhaust its own memory (`db:P6.7`).
#[derive(Debug, Deserialize)]
pub struct Paging {
    #[serde(rename = "_count")]
    count: Option<usize>,
    #[serde(rename = "_offset")]
    offset: Option<usize>,
}

impl Paging {
    const MAX: usize = 100;

    fn resolve(&self) -> (usize, usize) {
        (
            self.count.unwrap_or(20).min(Self::MAX),
            self.offset.unwrap_or(0),
        )
    }
}

/// A weak `ETag` over a version identity.
///
/// **Weak**, `W/`, and deliberately. A strong `ETag` asserts byte-for-byte
/// equality of the representation; this asserts the version is the same version.
/// Two responses for one version can differ in whitespace and remain the same
/// clinical fact, and claiming strong equality would be a claim this service
/// cannot keep.
fn etag(uid: &str) -> String {
    format!("W/\"{uid}\"")
}

fn headers(uid: &str) -> HeaderMap {
    let mut map = HeaderMap::new();
    if let Ok(value) = etag(uid).parse() {
        map.insert(header::ETAG, value);
    }
    map
}

/// Lower-case hex, written with `fmt::Write` rather than a `format!` per byte.
fn hex(bytes: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(64);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
}

fn view(row: &VersionRow) -> VersionView {
    VersionView {
        uid: row.uid.clone(),
        versioned_object_uid: row.versioned_object_uid.clone(),
        lifecycle_state_code: row.lifecycle_state_code.clone(),
        is_deleted: row.is_deleted,
        // The exact lexical form, never the derived instant (`db:M3.25`).
        time_committed: row.audit_time_committed.text.clone(),
        chain_digest: hex(&row.chain.digest),
        data: row
            .data_json
            .as_deref()
            .and_then(|j| serde_json::from_str(j).ok()),
    }
}

fn parse_id(raw: &str) -> Result<HierObjectId, (StatusCode, String)> {
    raw.parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "malformed identifier".to_owned()))
}

fn lock<T>(
    handle: &std::sync::Mutex<T>,
) -> Result<std::sync::MutexGuard<'_, T>, (StatusCode, String)> {
    handle.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "store lock poisoned".to_owned(),
        )
    })
}

/// `GET /openehr/v1/ehr/{ehr_id}/composition/{uid}`
///
/// **This is the endpoint the crate exists to get right.**
///
/// A deleted composition answers `410 Gone`, not `404`. openEHR deletion is a
/// new version with a deleted lifecycle state, so the record demonstrably
/// existed and its history is still readable through `_history`. Answering
/// `404` would tell a caller it never was.
async fn read(
    State(ctx): State<AppContext>,
    Path((_ehr_id, uid)): Path<(String, String)>,
) -> Reply<VersionView> {
    let handle = store(&ctx)?;
    let guard = lock(&handle)?;
    let container = parse_id(&uid)?;
    let row = guard.latest_version(&container).map_err(|e| status_for(&e))?;

    if row.is_deleted {
        return Err((
            StatusCode::GONE,
            format!(
                "composition {} was deleted by version {}; its history remains at _history",
                container, row.uid
            ),
        ));
    }
    let head = headers(&row.uid);
    Ok((StatusCode::OK, head, Json(view(&row))))
}

/// `GET /openehr/v1/ehr/{ehr_id}/composition/{uid}/version/{version_uid}` — vread.
///
/// Reads one specific version. A deleted version is returned as itself rather
/// than refused: asking for a version by identity is asking what it was, and
/// "this version recorded a deletion" is the answer.
async fn vread(
    State(ctx): State<AppContext>,
    Path((_ehr_id, _uid, version_uid)): Path<(String, String, String)>,
) -> Reply<VersionView> {
    let handle = store(&ctx)?;
    let guard = lock(&handle)?;
    let id = version_uid
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "malformed version uid".to_owned()))?;
    let row = guard.get_version(&id).map_err(|e| status_for(&e))?;
    let head = headers(&row.uid);
    Ok((StatusCode::OK, head, Json(view(&row))))
}

/// `GET /openehr/v1/ehr/{ehr_id}/composition/{uid}/_history`
///
/// Oldest first, because that is the order `REVISION_HISTORY` requires
/// (`db:H5.12`).
async fn history(
    State(ctx): State<AppContext>,
    Path((_ehr_id, uid)): Path<(String, String)>,
    Query(paging): Query<Paging>,
) -> Reply<Page<VersionView>> {
    let handle = store(&ctx)?;
    let guard = lock(&handle)?;
    let container = parse_id(&uid)?;
    let all = guard.all_versions(&container).map_err(|e| status_for(&e))?;
    let (count, offset) = paging.resolve();
    let items = all.iter().skip(offset).take(count).map(view).collect();
    Ok((
        StatusCode::OK,
        HeaderMap::new(),
        Json(Page {
            items,
            total: all.len(),
            offset,
            count,
        }),
    ))
}

/// `GET /openehr/v1/ehr/{ehr_id}/composition?archetype_id=…` — search.
///
/// The one query the composition index exists for (`db:P6.12`). Not AQL: this
/// service executes no AQL and does not pretend to (`db:S1.6`).
async fn search(
    State(ctx): State<AppContext>,
    Path(ehr_id): Path<String>,
    Query(paging): Query<Paging>,
    Query(filter): Query<Search>,
) -> Reply<Page<serde_json::Value>> {
    let handle = store(&ctx)?;
    let guard = lock(&handle)?;
    let id = parse_id(&ehr_id)?;
    let rows = guard
        .find_compositions_by_archetype(&id, &filter.archetype_id)
        .map_err(|e| status_for(&e))?;
    let (count, offset) = paging.resolve();
    let items = rows
        .iter()
        .skip(offset)
        .take(count)
        .map(|r| serde_json::to_value(r).unwrap_or(serde_json::Value::Null))
        .collect();
    Ok((
        StatusCode::OK,
        HeaderMap::new(),
        Json(Page {
            items,
            total: rows.len(),
            offset,
            count,
        }),
    ))
}

/// The search filter.
#[derive(Debug, Deserialize)]
pub struct Search {
    archetype_id: String,
}

/// `DELETE /openehr/v1/ehr/{ehr_id}/composition/{uid}`
///
/// Not implemented, and it returns `501` rather than pretending.
///
/// Deleting in openEHR means committing a version whose lifecycle state is
/// `deleted` (`db:H5.2`), which needs an `AUDIT_DETAILS` naming who did it and
/// why. That cannot be synthesised from a bare `DELETE` with no body: this
/// service does not authenticate (`db:S1.8`), so it has no committer to record,
/// and inventing one would put a false name in an audit trail.
#[allow(clippy::unused_async)]
async fn remove(Path((_ehr_id, _uid)): Path<(String, String)>) -> (StatusCode, String) {
    (
        StatusCode::NOT_IMPLEMENTED,
        "deletion is a commit carrying AUDIT_DETAILS; POST a deleted version instead \
         (openEHR H5.2). This service has no committer to record (S1.8)."
            .to_owned(),
    )
}

/// The routes this controller owns.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/openehr/v1/ehr/{ehr_id}/composition")
        .add("/", get(search))
        .add("/{uid}", get(read))
        .add("/{uid}", delete(remove))
        .add("/{uid}/_history", get(history))
        .add("/{uid}/version/{version_uid}", get(vread))
}
