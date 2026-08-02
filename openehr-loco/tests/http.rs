//! The claims this crate makes, served over HTTP.
//!
//! # Why these exist
//!
//! Until now every statement in this crate's README was true of code that had
//! never answered a request. The `410`-versus-`404` distinction is the reason
//! the crate exists, and it was asserted in three places and demonstrated in
//! none — which is the failure mode `W0.3` names and that this repository's
//! audit register is almost entirely made of.
//!
//! # How the router is built
//!
//! Through [`AppRoutes::to_router`], the same call [`App::boot`] makes, rather
//! than a hand-assembled router. Loco normalises a route's prefix and path when
//! it collects them — `"/composition"` plus `"/"` becomes `/composition`, not
//! `/composition/` — so a test that concatenated them itself would exercise
//! paths the server never serves and pass while production `404`s.

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{HeaderMap, HeaderName, HeaderValue, Request, StatusCode, header},
};
use loco_rs::{
    app::{AppContext, Hooks as _},
    environment::Environment,
};
use openehr::{
    base::{HierObjectId, ObjectId, ObjectRef, ObjectVersionId},
    rm::{
        common::{AuditDetails, OriginalVersion, PartyIdentified, Version},
        data_types::DvDateTime,
        ehr::Composition,
    },
    terminology::{audit_change_type, version_lifecycle_state},
};
use openehr_loco::{
    app::{App, SharedOpenehrStore},
    auth::{PasetoVerifier, SharedVerifier},
};
use openehr_sqlite::SqliteStore;
use openehr_store::{Store as _, conformance};
use pasetors::{
    claims::Claims,
    keys::{AsymmetricKeyPair, Generate as _},
    paserk::FormatAsPaserk as _,
    public,
    version4::V4,
};
use std::sync::{Arc, Mutex};
use tower::ServiceExt as _;

/// A composition that is still current.
const LIVE: &str = "87284370-2D4B-4E3D-A3F3-F303D2F4F34B";
/// A composition whose latest version deleted it.
const GONE: &str = "1B4E28BA-2FA1-11D2-883F-0016D3CCA427";
/// A composition that was never committed at all.
const ABSENT: &str = "6BA7B810-9DAD-11D1-80B4-00C04FD430C8";
const SYSTEM: &str = "ehr1.example.org";
const CONTRIBUTION: &str = "9E107D9D-3722-4EA4-A8DB-0F79A9B4E5D2";

/// Builds one version of a composition in a named container.
fn version(container: &str, n: u32, preceding: Option<u32>, deleted: bool) -> Version<Composition> {
    let id = |v: u32| -> ObjectVersionId {
        format!("{container}::{SYSTEM}::{v}")
            .parse()
            .expect("literal")
    };
    let owner = ObjectRef::new(
        "local",
        "EHR",
        ObjectId::HierObjectId(HierObjectId::from_uid_str(container).expect("literal")),
    )
    .expect("literal");
    let audit = AuditDetails::new(
        SYSTEM,
        DvDateTime::new(&format!("2026-08-01T09:{n:02}:00Z")).expect("literal"),
        if deleted {
            audit_change_type::DELETED
        } else if preceding.is_none() {
            audit_change_type::CREATION
        } else {
            audit_change_type::AMENDMENT
        },
        PartyIdentified::named("Dr A Nurse")
            .expect("literal")
            .into(),
    )
    .expect("literal");
    OriginalVersion::new(
        id(n),
        preceding.map(id),
        if deleted {
            version_lifecycle_state::DELETED
        } else {
            version_lifecycle_state::COMPLETE
        },
        // A deleted version carries no content. That is the whole point: the
        // record of the deletion survives and the clinical data does not.
        (!deleted).then(|| conformance::sample_composition(&format!("Encounter {n}"))),
        audit,
        owner,
    )
    .expect("literal")
    .into()
}

/// A service with one live composition, one deleted one, and nothing else.
struct Served {
    router: Router,
    authorization: String,
    ehr_id: String,
}

impl Served {
    fn new() -> Self {
        let pair = AsymmetricKeyPair::<V4>::generate().expect("keypair");
        let mut paserk = String::new();
        pair.public.fmt(&mut paserk).expect("PASERK");

        let mut claims = Claims::new().expect("claims");
        claims.subject("clinician-4417").expect("subject");
        let token = public::sign(&pair.secret, &claims, None, None).expect("signs");

        let mut store = SqliteStore::in_memory().expect("store");
        store.install().expect("install");
        let ehr = conformance::sample_ehr();
        store.create_ehr(&ehr).expect("ehr");
        let ehr_id = ehr.ehr_id().clone();
        store
            .create_contribution(
                &ehr_id,
                &conformance::sample_contribution(CONTRIBUTION, &[1, 2]),
            )
            .expect("contribution");

        store
            .commit_composition(&ehr_id, &version(LIVE, 1, None, false), CONTRIBUTION)
            .expect("live v1");
        store
            .commit_composition(&ehr_id, &version(GONE, 1, None, false), CONTRIBUTION)
            .expect("gone v1");
        // The deletion is a *commit*, not a removal. Version 1 stays readable.
        store
            .commit_composition(&ehr_id, &version(GONE, 2, Some(1), true), CONTRIBUTION)
            .expect("gone v2");

        let ctx = AppContext::builder(Environment::Test, loco_rs::tests_cfg::config::test_config())
            .build();
        ctx.shared_store.insert::<SharedVerifier>(Arc::new(
            PasetoVerifier::new(&paserk, None, None, None).expect("verifier"),
        ));
        ctx.shared_store
            .insert::<SharedOpenehrStore>(Arc::new(Mutex::new(store)));

        Self {
            router: App::routes(&ctx)
                .to_router::<App>(ctx.clone(), Router::new())
                .expect("router"),
            authorization: format!("Bearer {token}"),
            ehr_id: ehr_id.to_string(),
        }
    }

    fn ehr_id(&self) -> &str {
        &self.ehr_id
    }

    async fn send(&self, request: Request<Body>) -> (StatusCode, String, Option<String>) {
        let response = self
            .router
            .clone()
            .oneshot(request)
            .await
            .expect("infallible");
        let status = response.status();
        let etag = response
            .headers()
            .get(header::ETAG)
            .and_then(|v| v.to_str().ok())
            .map(ToOwned::to_owned);
        let body = to_bytes(response.into_body(), 1 << 20).await.expect("body");
        (status, String::from_utf8_lossy(&body).into_owned(), etag)
    }

    /// A request carrying the valid token.
    async fn get(&self, path: &str) -> (StatusCode, String, Option<String>) {
        self.send(
            Request::builder()
                .uri(path)
                .header(header::AUTHORIZATION, &self.authorization)
                .body(Body::empty())
                .expect("request"),
        )
        .await
    }

    /// A request carrying nothing.
    async fn get_anonymous(&self, path: &str) -> (StatusCode, String, Option<String>) {
        self.send(
            Request::builder()
                .uri(path)
                .body(Body::empty())
                .expect("request"),
        )
        .await
    }

    fn composition(&self, uid: &str) -> String {
        format!("/openehr/v1/ehr/{}/composition/{uid}", self.ehr_id)
    }
}

// --- the distinction the crate exists for ---------------------------------

#[tokio::test]
async fn a_deleted_composition_answers_410_and_one_that_never_existed_answers_404() {
    let served = Served::new();

    let (live, _, _) = served.get(&served.composition(LIVE)).await;
    assert_eq!(live, StatusCode::OK, "the live composition should be read");

    let (gone, body, _) = served.get(&served.composition(GONE)).await;
    assert_eq!(
        gone,
        StatusCode::GONE,
        "a deleted composition answered {gone}, not 410; a caller told 404 concludes \
         the record never existed and stops looking (S1.20)"
    );
    // The body has to say where the history is, or `410` is only a nicer 404.
    assert!(body.contains("_history"), "{body}");

    let (absent, _, _) = served.get(&served.composition(ABSENT)).await;
    assert_eq!(
        absent,
        StatusCode::NOT_FOUND,
        "a composition that was never committed answered {absent}, not 404"
    );
}

#[tokio::test]
async fn a_deleted_composition_still_has_its_history() {
    let served = Served::new();
    let (status, body, _) = served
        .get(&format!("{}/_history", served.composition(GONE)))
        .await;

    assert_eq!(status, StatusCode::OK, "history was refused: {body}");
    let page: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(page["total"], 2, "{body}");
    // Oldest first, because that is the order REVISION_HISTORY requires
    // (H5.12). Reversing it is the kind of change that looks like a display
    // preference and is not.
    assert_eq!(page["items"][0]["uid"], format!("{GONE}::{SYSTEM}::1"));
    assert_eq!(page["items"][1]["is_deleted"], true);
    // The content is gone; the record that it existed is not.
    assert!(page["items"][0]["data"].is_object(), "{body}");
    assert!(page["items"][1]["data"].is_null(), "{body}");
}

#[tokio::test]
async fn a_deleted_version_read_by_identity_is_returned_rather_than_refused() {
    let served = Served::new();
    let (status, body, _) = served
        .get(&format!(
            "{}/version/{GONE}::{SYSTEM}::2",
            served.composition(GONE)
        ))
        .await;

    // Deliberately not 410. Asking for a version *by identity* is asking what
    // that version was, and "this one recorded a deletion" is the answer. Only
    // the container read reports 410.
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&body).expect("json")["is_deleted"],
        true
    );
}

// --- verification ---------------------------------------------------------

#[tokio::test]
async fn every_clinical_route_refuses_an_unauthenticated_request() {
    let served = Served::new();
    let ehr = served.ehr_id();

    for path in [
        format!("/openehr/v1/ehr/{ehr}"),
        served.composition(LIVE),
        served.composition(GONE),
        format!("{}/_history", served.composition(LIVE)),
        format!("{}/version/{LIVE}::{SYSTEM}::1", served.composition(LIVE)),
        format!("/openehr/v1/ehr/{ehr}/composition?archetype_id=x"),
    ] {
        let (status, _, _) = served.get_anonymous(&path).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{path} was served openly");
    }
}

#[tokio::test]
async fn a_401_never_reveals_whether_the_record_exists() {
    let served = Served::new();

    // The live, the deleted, and the never-committed must be indistinguishable
    // without a token. Otherwise an anonymous caller can enumerate which
    // patients a system holds, which is a disclosure on its own.
    let mut seen = Vec::new();
    for uid in [LIVE, GONE, ABSENT] {
        let (status, body, _) = served.get_anonymous(&served.composition(uid)).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        seen.push(body);
    }
    assert!(
        seen.windows(2).all(|pair| pair[0] == pair[1]),
        "anonymous responses differ between an existing and a missing record: {seen:?}"
    );
}

#[tokio::test]
async fn no_header_can_stand_in_for_a_token() {
    // PASETO replaces the trusted header (PR12.21). A trusted header is
    // believed because of where it arrived from, so the check lives in the
    // network diagram — and a header that is safe behind one ingress is
    // attacker-controlled the day a second route exists, with nothing in the
    // code changing to mark it.
    let served = Served::new();
    let mut headers = HeaderMap::new();
    for name in [
        "x-principal",
        "x-forwarded-user",
        "x-on-behalf-of",
        "x-provenance",
        "remote-user",
        "x-authenticated-user",
    ] {
        headers.insert(
            HeaderName::from_static(name),
            HeaderValue::from_static("chief-medical-officer"),
        );
    }

    let mut request = Request::builder()
        .uri(served.composition(LIVE))
        .body(Body::empty())
        .expect("request");
    *request.headers_mut() = headers;

    let (status, _, _) = served.send(request).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "an identity header was accepted in place of a signed token"
    );
}

#[tokio::test]
async fn metadata_is_the_one_route_served_without_a_token() {
    let served = Served::new();
    let (status, body, _) = served.get_anonymous("/openehr/v1/metadata").await;

    assert_eq!(status, StatusCode::OK);
    let metadata: serde_json::Value = serde_json::from_str(&body).expect("json");
    // A caller must be able to learn the scheme by reading rather than by
    // collecting a 401.
    assert!(
        metadata["token_scheme"]
            .as_str()
            .expect("token_scheme")
            .contains("v4.public"),
        "{body}"
    );
    // And the absences must be named. Requiring a token is routinely read as
    // covering authorization too, so both are listed.
    let absences = serde_json::to_string(&metadata["not_implemented"]).expect("json");
    for expected in ["authorization", "read auditing", "GDPR"] {
        assert!(
            absences.contains(expected),
            "{expected} unnamed in {absences}"
        );
    }
}

// --- what the service tells a caller about a response ---------------------

#[tokio::test]
async fn a_read_carries_a_weak_etag_naming_the_version() {
    let served = Served::new();
    let (status, _, etag) = served.get(&served.composition(LIVE)).await;

    assert_eq!(status, StatusCode::OK);
    // Weak, and deliberately: this asserts the version is the same version,
    // not that the bytes are identical. Two serialisations of one version are
    // the same clinical fact.
    assert_eq!(
        etag.as_deref(),
        Some(format!("W/\"{LIVE}::{SYSTEM}::1\"").as_str())
    );
}

#[tokio::test]
async fn a_page_reports_the_total_before_paging() {
    let served = Served::new();
    let (status, body, _) = served
        .get(&format!(
            "{}/_history?_count=1&_offset=1",
            served.composition(GONE)
        ))
        .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    let page: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(page["items"].as_array().expect("items").len(), 1);
    assert_eq!(page["offset"], 1);
    // Two exist; one was returned. Without `total` a short page is
    // indistinguishable from the end of the results.
    assert_eq!(page["total"], 2, "{body}");
    assert_eq!(page["items"][0]["uid"], format!("{GONE}::{SYSTEM}::2"));
}

#[tokio::test]
async fn an_uncapped_count_cannot_be_asked_for() {
    let served = Served::new();
    let (status, body, _) = served
        .get(&format!(
            "{}/_history?_count=100000",
            served.composition(GONE)
        ))
        .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    // The cap is not politeness. An uncapped `_count` hands a caller the
    // ability to exhaust the server's memory against a long history (P6.7).
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&body).expect("json")["count"],
        100
    );
}

#[tokio::test]
async fn a_malformed_identifier_is_the_callers_fault_and_not_a_missing_record() {
    let served = Served::new();

    // 400, not 404. "That is not an identifier" and "no such record" are
    // different answers, and only one of them means the request is worth
    // sending again unchanged.
    //
    // A space is used because it is genuinely outside all three UID grammars.
    // This test first asserted 400 for `not-a-uuid` and was wrong: openEHR
    // UIDs are `UUID | ISO_OID | INTERNET_ID`, and hyphens are legal in a
    // domain label, so `not-a-uuid` is a perfectly valid HIER_OBJECT_ID that
    // simply names nothing — and 404 was the correct answer. Worth recording,
    // because "identifier" and "UUID" are used interchangeably in a lot of
    // openEHR tooling and here they are not the same thing.
    let (malformed, _, _) = served.get(&served.composition("not%20a%20uid")).await;
    assert_eq!(malformed, StatusCode::BAD_REQUEST);

    let (unused_but_valid, _, _) = served.get(&served.composition("not-a-uuid")).await;
    assert_eq!(unused_but_valid, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn deletion_by_http_verb_is_refused_and_says_what_to_send_instead() {
    let served = Served::new();
    let (status, body, _) = served
        .send(
            Request::builder()
                .method("DELETE")
                .uri(served.composition(LIVE))
                .header(header::AUTHORIZATION, &served.authorization)
                .body(Body::empty())
                .expect("request"),
        )
        .await;

    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
    // The refusal has to be actionable, or it is just a wall.
    assert!(body.contains("AUDIT_DETAILS"), "{body}");
    assert!(body.contains("preceding_version_uid"), "{body}");

    // And the composition is still there afterwards. A 501 that had already
    // half-applied would be worse than a 200.
    let (after, _, _) = served.get(&served.composition(LIVE)).await;
    assert_eq!(after, StatusCode::OK);
}

// --- readiness ------------------------------------------------------------

#[tokio::test]
async fn a_service_without_its_store_reports_503_and_not_404() {
    // `before_run` failed or has not finished. The distinction matters to a
    // load balancer: 503 is "not me, not yet", and 404 is "this will never
    // work", and only one of them gets retried.
    let ctx =
        AppContext::builder(Environment::Test, loco_rs::tests_cfg::config::test_config()).build();
    let pair = AsymmetricKeyPair::<V4>::generate().expect("keypair");
    let mut paserk = String::new();
    pair.public.fmt(&mut paserk).expect("PASERK");
    ctx.shared_store.insert::<SharedVerifier>(Arc::new(
        PasetoVerifier::new(&paserk, None, None, None).expect("verifier"),
    ));

    let router = App::routes(&ctx)
        .to_router::<App>(ctx.clone(), Router::new())
        .expect("router");
    let mut claims = Claims::new().expect("claims");
    claims.subject("clinician-4417").expect("subject");
    let token = public::sign(&pair.secret, &claims, None, None).expect("signs");

    let response = router
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/openehr/v1/ehr/{}/composition/{LIVE}",
                    conformance::sample_ehr().ehr_id()
                ))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("infallible");

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}
