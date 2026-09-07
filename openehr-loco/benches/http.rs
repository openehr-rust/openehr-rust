//! An HTTP round trip through the real router: request in, response out.
//!
//! # What a number here is, and is not
//!
//! **Not a conformance claim** (`W0.3`) and **not a CI gate**. See
//! `openehr/benches/rm.rs` for the reasoning; it applies unchanged. CI runs
//! this with `--test`, one iteration, so a benchmark cannot rot unnoticed.
//!
//! # Why one route, and why a duplicated setup
//!
//! Reading a composition is the one request every other route's cost is
//! measured relative to: authentication, routing, the store read, and
//! serialising the response, with nothing else layered on top. This is the
//! same [`SqliteStore::in_memory`] and PASETO setup `tests/http.rs`'s own
//! `Served` builds, duplicated rather than shared — a `benches/` binary and a
//! `tests/` binary are compiled separately, and Cargo has no third place
//! between them a struct like `Served` could live for both to import from
//! without becoming part of this crate's public API just to be borrowed by
//! its own harness. Kept intentionally small (one route, no read auditing) so
//! the duplication stays cheap to keep honest against its original — `W0.38`.
//!
//! No network socket opens: [`tower::ServiceExt::oneshot`] drives the
//! [`axum::Router`] in process, the same way `tests/http.rs` does, so this
//! measures the service's own cost and not a loopback socket's.

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, header},
};
use criterion::{Criterion, criterion_group, criterion_main};
use loco_rs::{
    app::{AppContext, Hooks as _},
    environment::Environment,
};
use openehr_loco::{
    access::{AccessLog, SharedAccessLog},
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
use std::hint::black_box;
use std::sync::{Arc, Mutex};
use tower::ServiceExt as _;

/// A router with one committed composition, read auditing off (the default),
/// and the token that reads it — everything `read_composition` needs.
fn served() -> (Router, String, String) {
    let pair = AsymmetricKeyPair::<V4>::generate().expect("keypair");
    let mut claims = Claims::new().expect("claims");
    claims.subject("clinician-4417").expect("subject");
    let token = public::sign(&pair.secret, &claims, None, None).expect("signs");

    let mut store = SqliteStore::in_memory().expect("store");
    store.install().expect("install");
    let ehr = conformance::sample_ehr();
    store.create_ehr(&ehr).expect("ehr");
    let ehr_id = ehr.ehr_id().clone();
    let contribution = "22222222-3333-4444-5555-666666666666";
    store
        .create_contribution(&ehr_id, &conformance::sample_contribution(contribution, &[1]))
        .expect("contribution");
    store
        .commit_composition(&ehr_id, &conformance::sample_version(1, None, 5), contribution)
        .expect("commit");

    let ctx = AppContext::builder(Environment::Test, loco_rs::tests_cfg::config::test_config()).build();
    let mut paserk = String::new();
    pair.public.fmt(&mut paserk).expect("PASERK");
    ctx.shared_store.insert::<SharedVerifier>(Arc::new(
        PasetoVerifier::new(&paserk, None, None, None).expect("verifier"),
    ));
    ctx.shared_store
        .insert::<SharedOpenehrStore>(Arc::new(Mutex::new(store)));
    ctx.shared_store
        .insert::<SharedAccessLog>(Arc::new(AccessLog::off()));

    let router = App::routes(&ctx)
        .to_router::<App>(ctx.clone(), Router::new())
        .expect("router");
    (router, format!("Bearer {token}"), ehr_id.to_string())
}

fn read_composition(c: &mut Criterion) {
    let (router, authorization, ehr_id) = served();
    let uid = conformance::RECORD;
    let path = format!("/openehr/v1/ehr/{ehr_id}/composition/{uid}");
    let runtime = tokio::runtime::Runtime::new().expect("runtime");

    c.bench_function("http/read_composition", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let request = Request::builder()
                    .uri(black_box(&path))
                    .header(header::AUTHORIZATION, &authorization)
                    .body(Body::empty())
                    .expect("request");
                let response = router.clone().oneshot(request).await.expect("infallible");
                assert!(response.status().is_success(), "{}", response.status());
                to_bytes(response.into_body(), 1 << 20).await.expect("body")
            })
        });
    });
}

criterion_group!(benches, read_composition);
criterion_main!(benches);
