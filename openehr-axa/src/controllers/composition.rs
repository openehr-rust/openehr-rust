//! Composition endpoints: create, read, delete, `_history`, and vread.

use axa::{AppContext, routes::Routes};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    routing::{delete, get, post, put},
};
use openehr::base::HierObjectId;
use openehr::rm::common::Version;
use openehr::rm::ehr::Composition;
use openehr_store::{Store as _, record::VersionRow};
use serde::Deserialize;

use crate::auth::Principal;
use crate::authz::{Action, authorize};
use crate::controllers::{Refusal, check_attribution, recorded, status_for, store};
use crate::views::{Page, VersionView};

type Reply<T> = Result<(StatusCode, HeaderMap, Json<T>), Refusal>;

/// `_count` and `_offset`, with a bound.
///
/// The bound is not politeness. An unbounded `_count` lets one request ask for a
/// record's entire history, and a store that answers it has handed a caller the
/// ability to exhaust its own memory (`db:P6.7`). A `_count` outside the bound
/// is refused with 400 naming it, never clamped to the bound (RCA-IN-2).
#[derive(Debug, Deserialize)]
pub struct Paging {
    #[serde(rename = "_count")]
    count: Option<usize>,
    #[serde(rename = "_offset")]
    offset: Option<usize>,
}

impl Paging {
    const MAX: usize = 100;
    const DEFAULT_COUNT: usize = 20;

    /// # Errors
    ///
    /// 400 naming `_count` and `1 ..= 100` when `_count` is supplied outside that range.
    fn resolve(&self) -> Result<(usize, usize), (StatusCode, String)> {
        // RCA-IN-3: absent only. `db:P6.7` bounds a page; an absent `_count` is
        // `DEFAULT_COUNT` (20) rows and an absent `_offset` is `0`.
        let count = match self.count {
            None => Self::DEFAULT_COUNT,
            Some(n) if (1..=Self::MAX).contains(&n) => n,
            Some(n) => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("_count {n} is out of range: it must be 1 ..= {}", Self::MAX),
                ));
            }
        };
        Ok((count, self.offset.unwrap_or(0))) // RCA-IN-3: absent only, `db:P6.7`
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

pub(crate) fn headers(uid: &str) -> HeaderMap {
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

/// The HTTP view of a stored version.
///
/// Fallible (`db:D-28`). A stored, content-bearing row whose document does not
/// decode used to become `data: null` through `.ok()`, the same shape a
/// deletion takes. The response said "this version has no content" when the
/// truth was "this service cannot read it". Now it is a `500` naming the
/// version and quoting none of it, recorded as `error` (`db:PR12.23`). A row
/// with no document (a tombstone) is still `data: null`.
pub(crate) fn view(row: &VersionRow) -> Result<VersionView, (StatusCode, String)> {
    let data = match row.data_json.as_deref() {
        None => None,
        Some(json) => Some(serde_json::from_str(json).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!(
                    "the stored content of version {} could not be decoded",
                    row.uid
                ),
            )
        })?),
    };
    // Decoded like the content: a stored committer that does not decode is a
    // `500`, never a silent `null` (`db:D-28`).
    let committer = serde_json::from_str(&row.audit_committer_json).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "the stored committer of version {} could not be decoded",
                row.uid
            ),
        )
    })?;
    Ok(VersionView {
        uid: row.uid.clone(),
        versioned_object_uid: row.versioned_object_uid.clone(),
        lifecycle_state_code: row.lifecycle_state_code.clone(),
        is_deleted: row.is_deleted,
        // The exact lexical form, never the derived instant (`db:M3.25`).
        time_committed: row.audit_time_committed.text.clone(),
        chain_digest: hex(&row.chain.digest),
        data,
        committer,
    })
}

/// The resource a version holds, as ITS-REST answers with it (`db:S1.23`).
///
/// A version that records a deletion holds none, and is `410` (`db:S1.20`). A
/// stored document that does not decode is `500`, never `null` (`db:D-28`).
pub(crate) fn resource(row: &VersionRow) -> Result<serde_json::Value, (StatusCode, String)> {
    let Some(json) = row.data_json.as_deref() else {
        return Err((
            StatusCode::GONE,
            format!(
                "version {} records a deletion and carries no content",
                row.uid
            ),
        ));
    };
    serde_json::from_str(json).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "the stored content of version {} could not be decoded",
                row.uid
            ),
        )
    })
}

/// `ETag` naming the version and `Location` naming its own URL (`db:S1.23`).
pub(crate) fn resource_headers(version_uid: &str, location: &str) -> HeaderMap {
    let mut map = headers(version_uid);
    if let Ok(value) = location.parse() {
        map.insert(header::LOCATION, value);
    }
    map
}

/// [`resource`] for the answer after a commit: its failure must not read as
/// a failed write (`db:D-28`).
fn committed_resource(row: &VersionRow) -> Result<serde_json::Value, (StatusCode, String)> {
    resource(row).map_err(|(status, message)| {
        (
            status,
            format!("{message}; the commit itself succeeded as {}", row.uid),
        )
    })
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
    principal: Principal,
    Path((ehr_id, uid)): Path<(String, String)>,
) -> Reply<serde_json::Value> {
    let handle = store(&ctx)?;
    let guard = lock(&handle)?;
    // Decided whole, then recorded, then released (`db:PR12.23`, `db:D-29`).
    let reply = (|| -> Reply<serde_json::Value> {
        // `db:PR12.18`: the token's grants, before the store is asked anything.
        authorize(&principal, &ehr_id, Action::Read)?;
        let owner = parse_id(&ehr_id)?;
        // ITS-REST's `uid_based_id` (`db:S1.23`): a version id reads that
        // version, and a container id the latest. A version id is tried first,
        // because one would otherwise parse as a container id with an extension.
        // Scoped to the EHR in the path either way (`db:H5.18`, `db:D-19`): a
        // container the EHR does not own, or that is not a composition, is
        // `404`, exactly as an absent one is.
        let (container, row) = if let Ok(version) = uid.parse::<openehr::base::ObjectVersionId>() {
            let row = guard
                .get_version_in(&owner, &version, "COMPOSITION")
                .map_err(|e| status_for(&e))?;
            (version.object_id().to_string(), row)
        } else {
            let container = parse_id(&uid)?;
            let row = guard
                .latest_version_in(&owner, &container, "COMPOSITION")
                .map_err(|e| status_for(&e))?;
            (container.to_string(), row)
        };
        // Deleted is its own outcome (`gone`, from the `410`). An investigation
        // asking who looked at a withdrawn record is asking a sharper question
        // than who looked at a live one, and `not_found` would not distinguish
        // the two.
        if row.is_deleted {
            return Err(Refusal::from((
                StatusCode::GONE,
                format!(
                    "composition {} was deleted by version {}; its history remains at _history",
                    container, row.uid
                ),
            )));
        }
        // The COMPOSITION itself, the version in headers (`db:S1.23`).
        let location = format!("/openehr/v1/ehr/{ehr_id}/composition/{}", row.uid);
        Ok((
            StatusCode::OK,
            resource_headers(&row.uid, &location),
            Json(resource(&row)?),
        ))
    })();
    recorded(&ctx, &principal, "read", &ehr_id, &uid, reply)
}

/// `GET /openehr/v1/ehr/{ehr_id}/composition/{uid}/version/{version_uid}` — vread.
///
/// Reads one specific version. A deleted version is returned as itself rather
/// than refused: asking for a version by identity is asking what it was, and
/// "this version recorded a deletion" is the answer.
async fn vread(
    State(ctx): State<AppContext>,
    principal: Principal,
    Path((ehr_id, uid, version_uid)): Path<(String, String, String)>,
) -> Reply<VersionView> {
    let handle = store(&ctx)?;
    let guard = lock(&handle)?;
    let reply = (|| -> Reply<VersionView> {
        // `db:PR12.18`: the token's grants, before the store is asked anything.
        authorize(&principal, &ehr_id, Action::Read)?;
        let id: openehr::base::ObjectVersionId = version_uid
            .parse()
            .map_err(|_| (StatusCode::BAD_REQUEST, "malformed version uid".to_owned()))?;
        let container = parse_id(&uid)?;
        // The version must belong to the container the path names, as well as
        // to the EHR: `/composition/A/version/<a version of B>` is not a read of A.
        let row = if id.object_id().to_string() == container.to_string() {
            guard.get_version_in(&parse_id(&ehr_id)?, &id, "COMPOSITION")
        } else {
            Err(openehr_store::StoreError::NotFound {
                kind: "version",
                id: version_uid.clone(),
            })
        }
        .map_err(|e| status_for(&e))?;
        Ok((StatusCode::OK, headers(&row.uid), Json(view(&row)?)))
    })();
    recorded(&ctx, &principal, "vread", &ehr_id, &version_uid, reply)
}

/// `GET /openehr/v1/ehr/{ehr_id}/composition/{uid}/_history`
///
/// Oldest first, because that is the order `REVISION_HISTORY` requires
/// (`db:H5.12`).
async fn history(
    State(ctx): State<AppContext>,
    principal: Principal,
    Path((ehr_id, uid)): Path<(String, String)>,
    Query(paging): Query<Paging>,
) -> Reply<Page<VersionView>> {
    let handle = store(&ctx)?;
    let guard = lock(&handle)?;
    // The page is resolved, and every row on it decoded, before the read is
    // recorded: it used to be recorded `ok` and then refused for a bad
    // `_count` (`db:D-29`). One unreadable row fails the whole page, rather
    // than appearing as a version with no content (`db:D-28`). The page is
    // cut by the store, not here: this read the whole history to show twenty
    // versions of it (`db:P6.25`).
    let reply = (|| -> Reply<Page<VersionView>> {
        // `db:PR12.18`: the token's grants, before the store is asked anything.
        authorize(&principal, &ehr_id, Action::Read)?;
        let container = parse_id(&uid)?;
        let (count, offset) = paging.resolve()?;
        let page = guard
            .versions_page_in(
                &parse_id(&ehr_id)?,
                &container,
                "COMPOSITION",
                offset,
                count,
            )
            .map_err(|e| status_for(&e))?;
        let items = page.rows.iter().map(view).collect::<Result<Vec<_>, _>>()?;
        Ok((
            StatusCode::OK,
            HeaderMap::new(),
            Json(Page {
                items,
                total: page.total,
                offset,
                count,
            }),
        ))
    })();
    recorded(&ctx, &principal, "history", &ehr_id, &uid, reply)
}

/// `GET /openehr/v1/ehr/{ehr_id}/composition?archetype_id=…` — search.
///
/// The one query the composition index exists for (`db:P6.12`). Not AQL: this
/// service executes no AQL and does not pretend to (`db:S1.6`).
async fn search(
    State(ctx): State<AppContext>,
    principal: Principal,
    Path(ehr_id): Path<String>,
    Query(paging): Query<Paging>,
    Query(filter): Query<Search>,
) -> Reply<Page<serde_json::Value>> {
    let handle = store(&ctx)?;
    let guard = lock(&handle)?;
    let reply = (|| -> Reply<Page<serde_json::Value>> {
        // `db:PR12.18`: the token's grants, before the store is asked anything.
        authorize(&principal, &ehr_id, Action::Read)?;
        let id = parse_id(&ehr_id)?;
        // Before the record, as for `history` (`db:D-29`), and before the store,
        // which cuts the page itself (`db:P6.25`).
        let (count, offset) = paging.resolve()?;
        let page = guard
            .find_compositions_by_archetype_page(&id, &filter.archetype_id, offset, count)
            .map_err(|e| status_for(&e))?;
        let items = page
            .rows
            .iter()
            .map(serde_json::to_value)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("a stored row does not serialize: {e}"),
                )
            })?;
        Ok((
            StatusCode::OK,
            HeaderMap::new(),
            Json(Page {
                items,
                total: page.total,
                offset,
                count,
            }),
        ))
    })();
    // The archetype filter is the target: it is design-time metadata, not
    // content, so recording it says which *kind* of record was sought without
    // quoting any of them (`db:M3.38`).
    recorded(
        &ctx,
        &principal,
        "search",
        &ehr_id,
        &filter.archetype_id,
        reply,
    )
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
) -> Reply<serde_json::Value> {
    authorize(&principal, &ehr_id, Action::Write)?;
    check_attribution(&version, &principal)?;
    if version.preceding_version_uid().is_some() {
        return Err(Refusal::from((
            StatusCode::BAD_REQUEST,
            "this version names a predecessor, so it is an update: PUT it at its \
             container's address with If-Match"
                .to_owned(),
        )));
    }

    let handle = store(&ctx)?;
    let mut guard = lock(&handle)?;
    let outcome = guard
        .commit_composition(&parse_id(&ehr_id)?, &version, &commit.contribution)
        .map_err(|e| status_for(&e))?;
    let row = guard
        .get_version(&outcome.version_uid)
        .map_err(|e| status_for(&e))?;
    let location = format!("/openehr/v1/ehr/{ehr_id}/composition/{}", row.uid);
    Ok((
        StatusCode::CREATED,
        resource_headers(&row.uid, &location),
        Json(committed_resource(&row)?),
    ))
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
) -> Reply<serde_json::Value> {
    authorize(&principal, &ehr_id, Action::Write)?;
    check_attribution(&version, &principal)?;
    let expected = if_match(&request_headers)?;
    let container = parse_id(&uid)?;
    bind_update(&version, &container)?;

    let handle = store(&ctx)?;
    let mut guard = lock(&handle)?;
    // Scoped, as in `read`: the precondition is compared against the container
    // *this EHR* owns, so a caller cannot probe another chart's current version
    // through a `412` that names it (`db:D-19`).
    let current = guard
        .latest_version_in(&parse_id(&ehr_id)?, &container, "COMPOSITION")
        .map_err(|e| status_for(&e))?;

    if current.uid != expected {
        // 412, not 409. The distinction is worth keeping: `412` says the
        // precondition you stated is false, so re-read and decide; `409` says
        // the store refused the commit itself. A caller that conflates them
        // retries the wrong one.
        return Err(Refusal::from((
            StatusCode::PRECONDITION_FAILED,
            format!(
                "If-Match named {expected}, and the current version is {}",
                current.uid
            ),
        )));
    }

    bind_predecessor(&version, &expected)?;

    let outcome = guard
        .commit_composition(&parse_id(&ehr_id)?, &version, &commit.contribution)
        .map_err(|e| status_for(&e))?;
    let row = guard
        .get_version(&outcome.version_uid)
        .map_err(|e| status_for(&e))?;
    let location = format!("/openehr/v1/ehr/{ehr_id}/composition/{}", row.uid);
    Ok((
        StatusCode::OK,
        resource_headers(&row.uid, &location),
        Json(committed_resource(&row)?),
    ))
}

/// An update commits to the container its path names (`db:H5.21`,
/// `db:D-24`); [`bind_predecessor`] is the other half.
///
/// The precondition is checked against the path's container, and the commit
/// goes wherever the *body* says, so without this the two could name
/// different compositions. Both belong to this `EHR`, so the ownership checks
/// (`db:H5.18`, `db:H5.19`) pass, and a `PUT` to one composition committed a
/// new version of another. Checked before the store is touched; a mismatch is
/// the caller's `400`.
///
/// Identifiers are compared in their parsed, rendered form, as everywhere else
/// in this service. Spelling is not normalised; a `Uuid` renders as written.
fn bind_update(
    version: &Version<Composition>,
    container: &HierObjectId,
) -> Result<(), (StatusCode, String)> {
    let target = version.uid().object_id().to_string();
    if target != container.to_string() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("the version is for container {target}, and this request names {container}"),
        ));
    }
    Ok(())
}

/// The other half of `db:H5.21`: the submitted version must replace the one
/// the precondition named. Run after the `412` check, so a stale `If-Match`
/// is still answered `412` (`db:H5.15`) and an `EHR` that does not own the
/// container still gets `404`. By then `expected` *is* the current version.
fn bind_predecessor(
    version: &Version<Composition>,
    expected: &str,
) -> Result<(), (StatusCode, String)> {
    let preceding = version
        .preceding_version_uid()
        .map_or_else(|| "absent".to_owned(), ToString::to_string);
    if preceding != expected {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "the version's preceding_version_uid is {preceding}, and If-Match names {expected}"
            ),
        ));
    }
    Ok(())
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
async fn remove(_principal: Principal, Path((_ehr_id, _uid)): Path<(String, String)>) -> Refusal {
    Refusal(
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

#[cfg(test)]
mod paging_tests {
    use super::Paging;

    /// RCA-TEST-8: absent takes the declared default; supplied out of range is
    /// refused, never clamped (RCA-IN-2).
    #[test]
    fn a_page_defaults_when_absent_and_refuses_a_count_out_of_range() {
        let page = |count, offset| Paging { count, offset }.resolve();
        assert_eq!(page(None, None).unwrap(), (20, 0));
        assert_eq!(page(Some(1), None).unwrap(), (1, 0));
        assert_eq!(page(Some(100), Some(7)).unwrap(), (100, 7));
        for bad in [0, 101, usize::MAX] {
            let (status, message) = page(Some(bad), None).unwrap_err();
            assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
            assert!(message.contains("_count"), "{message}");
        }
    }
}
