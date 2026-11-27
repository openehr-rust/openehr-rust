//! EHR endpoints.

use axa::{AppContext, routes::Routes};
use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    routing::{get, post},
};
use openehr::base::{HierObjectId, ObjectId, ObjectRef};
use openehr::rm::common::{
    Archetyped, AuditDetails, Contribution, LocatableAttrs, OriginalVersion, PartyIdentified,
    PartyProxy, PartySelf, Version,
};
use openehr::rm::data_types::{DvDateTime, DvIdentifier};
use openehr::rm::ehr::{Ehr, EhrStatus};
use openehr::terminology::{audit_change_type, version_lifecycle_state};
use openehr::validation::Validate as _;
use openehr_store::Store as _;
use uuid::Uuid;

use crate::auth::Principal;
use crate::authz::{Action, authorize};
use crate::controllers::composition::lock;
use crate::controllers::{Refusal, outcome_of, record_read, status_for, store};

type Reply<T> = Result<(StatusCode, Json<T>), Refusal>;

/// The service's own `system_id` (`db:S1.22`): the system that creates
/// records and commits their first status. Read once at startup, by
/// [`crate::app::prepare`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemId(pub String);

/// The setting naming it.
pub const SYSTEM_ID_VAR: &str = "OPENEHR_SYSTEM_ID";

/// Used when [`SYSTEM_ID_VAR`] is unset, and by any context [`prepare`]
/// never ran on (a test).
///
/// [`prepare`]: crate::app::prepare
pub const DEFAULT_SYSTEM_ID: &str = "openehr-axa.local";

/// Reads [`SYSTEM_ID_VAR`] (`db:PR12.24`'s reader: absent or empty is the
/// default) and checks it can name a creating system.
///
/// # Errors
///
/// When it is not Unicode, or cannot be both a `HIER_OBJECT_ID` (`EHR.system_id`)
/// and the creating-system part of an `OBJECT_VERSION_ID`.
pub fn system_id_from(
    lookup: &impl Fn(&str) -> Option<std::ffi::OsString>,
) -> Result<SystemId, String> {
    let id = crate::settings::claim(SYSTEM_ID_VAR, lookup)?
        .unwrap_or_else(|| DEFAULT_SYSTEM_ID.to_owned());
    let as_container = id.parse::<HierObjectId>().is_ok();
    let as_creator = format!("{}::{id}::1", Uuid::nil())
        .parse::<openehr::base::ObjectVersionId>()
        .is_ok();
    if as_container && as_creator {
        Ok(SystemId(id))
    } else {
        Err(format!(
            "{SYSTEM_ID_VAR} is not a usable system id; refusing to start"
        ))
    }
}

fn system_id(ctx: &AppContext) -> String {
    ctx.shared_store
        .get::<SystemId>()
        .map_or_else(|| DEFAULT_SYSTEM_ID.to_owned(), |s| s.0.clone())
}

type Created = Result<(StatusCode, HeaderMap, Json<Ehr>), Refusal>;

/// `POST /openehr/v1/ehr`: the server mints the `ehr_id` (`db:S1.22`).
///
/// Needs a `write` grant on `"*"`: no per-record grant can name an id that
/// does not exist yet (`db:PR12.18`). A caller with a per-record grant uses
/// `PUT /ehr/{ehr_id}`.
async fn create(State(ctx): State<AppContext>, principal: Principal, body: Bytes) -> Created {
    if !principal.grants.allows_any(Action::Write) {
        return Err(Refusal::from((
            StatusCode::FORBIDDEN,
            "POST /ehr mints a new record id, so it needs a write grant on \"*\"; \
             with a per-record grant, PUT /ehr/{ehr_id} instead (S1.22, PR12.18)"
                .to_owned(),
        )));
    }
    create_record(&ctx, &principal, &mint()?, &body)
}

/// `PUT /openehr/v1/ehr/{ehr_id}`: creates the record the caller names, or
/// answers `409` if it exists (`db:S1.22`). Needs `write` on that record.
async fn create_named(
    State(ctx): State<AppContext>,
    principal: Principal,
    Path(ehr_id): Path<String>,
    body: Bytes,
) -> Created {
    authorize(&principal, &ehr_id, Action::Write)?;
    let id: HierObjectId = ehr_id
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "malformed ehr_id".to_owned()))?;
    create_record(&ctx, &principal, &id, &body)
}

/// A fresh `HIER_OBJECT_ID` from a random UUID.
fn mint() -> Result<HierObjectId, (StatusCode, String)> {
    HierObjectId::from_uid_str(&Uuid::new_v4().to_string()).map_err(internal)
}

/// A server-side failure to build something from parts it chose itself.
fn internal(error: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

/// The `EHR_STATUS` committed when the request carries none (`db:S1.22`).
fn default_status() -> Result<EhrStatus, (StatusCode, String)> {
    Ok(EhrStatus::new(
        LocatableAttrs::named("EHR Status", "openEHR-EHR-EHR_STATUS.generic.v1")
            .map_err(internal)?
            .with_archetype_details(
                Archetyped::new("openEHR-EHR-EHR_STATUS.generic.v1", "1.1.0").map_err(internal)?,
            ),
        PartySelf::anonymous(),
        true,
        true,
    ))
}

/// Builds the record, its first status version and the contribution that
/// declares it; validates all three; then writes them (`db:S1.22`).
fn create_record(
    ctx: &AppContext,
    principal: &Principal,
    ehr_id: &HierObjectId,
    body: &[u8],
) -> Created {
    let status: EhrStatus = if body.iter().all(u8::is_ascii_whitespace) {
        default_status()?
    } else {
        serde_json::from_slice(body).map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "the body is not an EHR_STATUS; send one, or no body for the default".to_owned(),
            )
        })?
    };
    let system = system_id(ctx);
    let now = DvDateTime::new(&crate::access::now_rfc3339()).map_err(internal)?;
    let status_container = mint()?;
    let ehr = Ehr::new(
        system.parse().map_err(internal)?,
        ehr_id.clone(),
        ObjectRef::new(
            "local",
            "VERSIONED_EHR_STATUS",
            ObjectId::HierObjectId(status_container.clone()),
        )
        .map_err(internal)?,
        ObjectRef::new(
            "local",
            "VERSIONED_EHR_ACCESS",
            ObjectId::HierObjectId(mint()?),
        )
        .map_err(internal)?,
        now.clone(),
    )
    .map_err(internal)?;

    // The verified caller commits it: its identifier is the token subject, so
    // the stored committer is the one `PR12.19` would have demanded anyway.
    let committer: PartyProxy = PartyIdentified::new(
        principal.name.clone(),
        vec![DvIdentifier::new(&principal.subject).map_err(internal)?],
        None,
    )
    .map_err(internal)?
    .into();
    let audit = AuditDetails::new(&system, now, audit_change_type::CREATION, committer)
        .map_err(internal)?;
    let version_uid: openehr::base::ObjectVersionId = format!("{status_container}::{system}::1")
        .parse()
        .map_err(internal)?;
    let version: Version<EhrStatus> = OriginalVersion::new(
        version_uid.clone(),
        None,
        version_lifecycle_state::COMPLETE,
        Some(status),
        audit.clone(),
        ObjectRef::new("local", "EHR", ObjectId::HierObjectId(ehr_id.clone())).map_err(internal)?,
    )
    .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()))?
    .into();
    let contribution = Contribution::new(mint()?, vec![version_uid], audit).map_err(internal)?;

    // Everything checked before the first write: a status that is not valid
    // RM is refused with nothing created (`db:S1.22`).
    for report in [ehr.validate(), version.validate()] {
        report
            .into_result()
            .map_err(|r| status_for(&openehr_store::StoreError::Invalid(r)))?;
    }

    // One change: the record, its contribution and its first status, or
    // nothing (`db:H5.23`). Three calls left a statusless record behind when
    // the second or third failed, and a retry could only answer `409`
    // (`db:D-33`).
    let handle = store(ctx)?;
    let mut guard = lock(&handle)?;
    guard
        .create_ehr_with_status(&ehr, &contribution, &version)
        .map_err(|e| status_for(&e))?;

    let mut headers = HeaderMap::new();
    if let Ok(location) = HeaderValue::from_str(&format!("/openehr/v1/ehr/{ehr_id}")) {
        headers.insert(header::LOCATION, location);
    }
    if let Ok(tag) = HeaderValue::from_str(&format!("W/\"{ehr_id}\"")) {
        headers.insert(header::ETAG, tag);
    }
    Ok((StatusCode::CREATED, headers, Json(ehr)))
}

/// `GET /openehr/v1/ehr/{ehr_id}`
///
/// Verified, then discarded — this layer records no reads (`db:PR12.5`).
async fn read(
    State(ctx): State<AppContext>,
    principal: Principal,
    Path(ehr_id): Path<String>,
) -> Reply<Ehr> {
    let handle = store(&ctx)?;
    let guard = lock(&handle)?;
    let id = ehr_id
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "malformed ehr_id".to_owned()))?;
    // `db:PR12.18`: before the lookup, and recorded as `denied`.
    if let Err(denied) = authorize(&principal, &ehr_id, Action::Read) {
        record_read(&ctx, &principal, "read_ehr", &ehr_id, &ehr_id, "denied")?;
        return Err(denied.into());
    }
    let found = guard.get_ehr(&id);
    record_read(
        &ctx,
        &principal,
        "read_ehr",
        &ehr_id,
        &ehr_id,
        outcome_of(&found),
    )?;
    let ehr = found.map_err(|e| status_for(&e))?;
    Ok((StatusCode::OK, Json(ehr)))
}

/// The routes this controller owns.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/openehr/v1/ehr")
        .add("/", post(create))
        .add("/{ehr_id}", get(read).put(create_named))
}

#[cfg(test)]
mod system_id_tests {
    use super::{DEFAULT_SYSTEM_ID, SYSTEM_ID_VAR, system_id_from};
    use std::ffi::OsString;

    fn from(value: Option<&str>) -> Result<String, String> {
        system_id_from(&|name| {
            assert_eq!(name, SYSTEM_ID_VAR);
            value.map(OsString::from)
        })
        .map(|id| id.0)
    }

    /// Unset is the documented default; a usable id is taken as given.
    #[test]
    fn the_system_id_defaults_when_unset_and_is_taken_as_given_otherwise() {
        assert_eq!(from(None).as_deref(), Ok(DEFAULT_SYSTEM_ID));
        assert_eq!(
            from(Some("ehr1.example.org")).as_deref(),
            Ok("ehr1.example.org")
        );
    }

    /// An id that could not name a container or a version's creating system
    /// refuses startup, rather than minting identifiers that do not parse back.
    #[test]
    fn an_unusable_system_id_refuses_startup() {
        for bad in ["two words", "a::b"] {
            let message = from(Some(bad)).expect_err(bad);
            assert!(message.contains(SYSTEM_ID_VAR), "{message}");
        }
    }
}
