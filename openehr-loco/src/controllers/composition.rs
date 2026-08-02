//! Composition endpoints: create, read, delete, `_history`, and vread.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    routing::{delete, get, post, put},
};
use loco_rs::{app::AppContext, controller::Routes};
use openehr::base::HierObjectId;
use openehr::rm::common::Version;
use openehr::rm::ehr::Composition;
use openehr_store::{Store as _, record::VersionRow};
use serde::Deserialize;

use crate::auth::Principal;
use crate::controllers::{check_attribution, status_for, store};
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

pub(crate) fn parse_id(raw: &str) -> Result<HierObjectId, (StatusCode, String)> {
    raw.parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "malformed identifier".to_owned()))
}

pub(crate) fn lock<T>(
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
    _principal: Principal,
    Path((_ehr_id, uid)): Path<(String, String)>,
) -> Reply<VersionView> {
    let handle = store(&ctx)?;
    let guard = lock(&handle)?;
    let container = parse_id(&uid)?;
    let row = guard
        .latest_version(&container)
        .map_err(|e| status_for(&e))?;

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
    _principal: Principal,
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
    _principal: Principal,
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
    _principal: Principal,
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

/// Which change set a commit belongs to.
#[derive(Debug, Deserialize)]
pub struct Commit {
    contribution: String,
}

/// `POST /openehr/v1/ehr/{ehr_id}/composition?contribution=…`
///
/// The body is a whole `VERSION`, not a bare `COMPOSITION`.
///
/// The alternative would have this service mint the version identifier, the
/// commit time, and the `AUDIT_DETAILS` around a composition it was handed —
/// inventing the record of who did what, when, and why. That is clinical
/// behaviour and belongs to the caller (`db:S1.19`, `db:PR12.9`). What this
/// endpoint does is check the caller may make the commit they are describing,
/// and hand it to the store.
async fn create(
    State(ctx): State<AppContext>,
    principal: Principal,
    Path(ehr_id): Path<String>,
    Query(commit): Query<Commit>,
    Json(version): Json<Version<Composition>>,
) -> Reply<VersionView> {
    check_attribution(&version, &principal)?;
    if version.preceding_version_uid().is_some() {
        return Err((
            StatusCode::BAD_REQUEST,
            "this version names a predecessor, so it is an update: PUT it at its \
             container's address with If-Match"
                .to_owned(),
        ));
    }

    let handle = store(&ctx)?;
    let mut guard = lock(&handle)?;
    let outcome = guard
        .commit_composition(&parse_id(&ehr_id)?, &version, &commit.contribution)
        .map_err(|e| status_for(&e))?;
    let row = guard
        .get_version(&outcome.version_uid)
        .map_err(|e| status_for(&e))?;
    let head = headers(&row.uid);
    Ok((StatusCode::CREATED, head, Json(view(&row))))
}

/// `PUT /openehr/v1/ehr/{ehr_id}/composition/{uid}?contribution=…`
///
/// Requires `If-Match` naming the version being replaced.
///
/// # Why `If-Match` is required rather than optional
///
/// Without it, two clinicians who opened the same composition both commit and
/// the second silently supersedes the first. The store would refuse a stale
/// predecessor (`db:H5.9`), so nothing is lost — but a caller who never sent a
/// precondition learns that only from a `409` they did not ask for. Requiring
/// the header makes the concurrency contract explicit at the point of use.
async fn update(
    State(ctx): State<AppContext>,
    principal: Principal,
    Path((ehr_id, uid)): Path<(String, String)>,
    Query(commit): Query<Commit>,
    request_headers: HeaderMap,
    Json(version): Json<Version<Composition>>,
) -> Reply<VersionView> {
    check_attribution(&version, &principal)?;
    let expected = if_match(&request_headers)?;

    let handle = store(&ctx)?;
    let mut guard = lock(&handle)?;
    let container = parse_id(&uid)?;
    let current = guard
        .latest_version(&container)
        .map_err(|e| status_for(&e))?;

    if current.uid != expected {
        // 412, not 409. The distinction is worth keeping: `412` says the
        // precondition you stated is false, so re-read and decide; `409` says
        // the store refused the commit itself. A caller that conflates them
        // retries the wrong one.
        return Err((
            StatusCode::PRECONDITION_FAILED,
            format!(
                "If-Match named {expected}, and the current version is {}",
                current.uid
            ),
        ));
    }

    let outcome = guard
        .commit_composition(&parse_id(&ehr_id)?, &version, &commit.contribution)
        .map_err(|e| status_for(&e))?;
    let row = guard
        .get_version(&outcome.version_uid)
        .map_err(|e| status_for(&e))?;
    let head = headers(&row.uid);
    Ok((StatusCode::OK, head, Json(view(&row))))
}

/// The version named by `If-Match`.
///
/// # Two forms are accepted, deliberately
///
/// `W/"<version-uid>"` is what this service emits, and the bare
/// `<version-uid>` is what the openEHR REST API specifies. Refusing the second
/// would mean a client written against openEHR could not talk to this service.
///
/// # A declared departure from RFC 9110
///
/// RFC 9110 §13.1.1 requires **strong** comparison for `If-Match`, under which
/// a weak tag never matches anything. This service compares weakly, and
/// `db:H5.15` records why: the tag names a *version*, and "is the head still
/// version N" is precisely the question optimistic concurrency asks. A strong
/// tag would fail on two byte-different serialisations of one version, which
/// are the same clinical fact — so strong comparison here would reject correct
/// requests to protect against nothing.
fn if_match(headers: &HeaderMap) -> Result<String, (StatusCode, String)> {
    let raw = headers
        .get(header::IF_MATCH)
        .ok_or((
            StatusCode::PRECONDITION_REQUIRED,
            "If-Match is required on an update: name the version you are replacing, \
             as W/\"<version-uid>\" or as the bare version uid"
                .to_owned(),
        ))?
        .to_str()
        .map_err(|_| (StatusCode::BAD_REQUEST, "malformed If-Match".to_owned()))?
        .trim();

    if raw == "*" {
        // `*` means "any current representation". On a container that exists it
        // is satisfied by anything, which is the opposite of what an update to
        // a versioned record needs: it would let a caller overwrite a version
        // they have never seen.
        return Err((
            StatusCode::PRECONDITION_REQUIRED,
            "If-Match: * is not accepted here; name the version you are replacing".to_owned(),
        ));
    }
    Ok(raw
        .strip_prefix("W/")
        .unwrap_or(raw)
        .trim_matches('"')
        .to_owned())
}

/// `DELETE /openehr/v1/ehr/{ehr_id}/composition/{uid}`
///
/// Not implemented, and it returns `501` rather than pretending.
///
/// Deleting in openEHR means committing a version whose lifecycle state is
/// `deleted` (`db:H5.2`), carrying an `AUDIT_DETAILS` that says who did it,
/// what kind of change it was, and **why** — and a `preceding_version_uid`
/// placing it in the history.
///
/// # The reason changed when verification arrived, so it is restated
///
/// This previously said the service had no committer to record. It now has
/// one: [`Principal::subject`] is verified on every request and would map onto
/// `AUDIT_DETAILS.committer` perfectly well.
///
/// What a bare `DELETE` still cannot supply is the rest. A deletion without a
/// reason is a row in an audit trail that answers the easy question and not the
/// one an investigation asks, and defaulting the description to `"deleted via
/// HTTP DELETE"` would be synthesising the part that matters — the same
/// objection as `db:PR12.9`, one field along. So the endpoint stays refused,
/// now by choice rather than by inability, and the caller is told what to send
/// instead.
#[allow(clippy::unused_async)]
async fn remove(
    _principal: Principal,
    Path((_ehr_id, _uid)): Path<(String, String)>,
) -> (StatusCode, String) {
    (
        StatusCode::NOT_IMPLEMENTED,
        "deletion is a commit: POST a version whose lifecycle state is deleted, carrying \
         AUDIT_DETAILS with a change reason and a preceding_version_uid (openEHR H5.2). \
         A bare DELETE carries no reason, and this service will not invent one (PR12.9)."
            .to_owned(),
    )
}

/// The routes this controller owns.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/openehr/v1/ehr/{ehr_id}/composition")
        .add("/", get(search))
        .add("/", post(create))
        .add("/{uid}", get(read))
        .add("/{uid}", put(update))
        .add("/{uid}", delete(remove))
        .add("/{uid}/_history", get(history))
        .add("/{uid}/version/{version_uid}", get(vread))
}
