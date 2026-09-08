//! The Loco application.

use async_trait::async_trait;
use axum::Router as AxumRouter;
use loco_rs::{
    Result,
    app::{AppContext, Hooks},
    bgworker::Queue,
    boot::{BootResult, StartMode, create_app},
    config::Config,
    controller::AppRoutes,
    environment::Environment,
    task::Tasks,
};
use openehr_sqlite::SqliteStore;
use openehr_store::Store as _;
use std::sync::{Arc, Mutex};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder, key_extractor::GlobalKeyExtractor};

use crate::access::{AccessLog, SharedAccessLog};
use crate::auth::{PasetoVerifier, SharedVerifier};

/// A burst of this many requests is let through before the limiter starts
/// answering `429`.
const RATE_LIMIT_BURST: u32 = 50;

/// One request-worth of quota is replenished every this many seconds after a
/// burst is spent — so, at steady state, `1.0 / RATE_LIMIT_PERIOD_SECS as f64`
/// requests per second.
const RATE_LIMIT_PERIOD_SECS: u64 = 1;

/// The store, shared across requests.
///
/// A `Mutex` because [`openehr_store::Store`] takes `&mut self` for anything
/// that writes, and `rusqlite::Connection` is `Send` and not `Sync`. Serialising
/// requests through one connection is the honest arrangement for the embedded
/// engine: `SQLite` serialises writers anyway, and pretending otherwise with a
/// pool would add contention somewhere less visible.
pub type SharedOpenehrStore = Arc<Mutex<SqliteStore>>;

/// Opens and installs the store.
///
/// Shared by [`Hooks::before_run`] and by [`crate::tasks`], because **a task
/// does not get `before_run`** — `cli::main` builds the context and calls
/// `run_task` directly, so a task reading the store out of `shared_store`
/// finds nothing. One opener rather than two means the path and the install
/// cannot drift between the server and the tools that inspect what it wrote.
///
/// # Errors
///
/// Returns [`loco_rs::Error::Message`] if the file cannot be opened or the
/// schema cannot be installed — including when the database was built under a
/// different schema version, which is refused rather than half-served
/// (`db:O10.15`).
pub fn open_store() -> Result<SqliteStore> {
    open_store_at(std::path::Path::new(
        &std::env::var("OPENEHR_SQLITE_PATH").unwrap_or_else(|_| "openehr.sqlite3".to_owned()),
    ))
}

/// Opens and installs the store at an explicit path.
///
/// Separate from [`open_store`] so that a caller can name the database instead
/// of setting a process-wide variable. That is what a task needs to verify a
/// **restored backup** — the copy an operator most wants checked, and the one
/// that is never at the path the running service uses (`db:O10.19`).
///
/// # Errors
///
/// As [`open_store`].
pub fn open_store_at(path: &std::path::Path) -> Result<SqliteStore> {
    let mut store = SqliteStore::open(path).map_err(|e| loco_rs::Error::Message(e.to_string()))?;
    store
        .install()
        .map_err(|e| loco_rs::Error::Message(e.to_string()))?;
    Ok(store)
}

/// The application.
pub struct App;

#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self>(mode, environment, config).await
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::empty()
            .add_route(controllers_routes::metadata())
            .add_route(controllers_routes::ehr())
            .add_route(controllers_routes::contribution())
            .add_route(controllers_routes::composition())
    }

    /// Opens the store and builds the verifier, **here and not in
    /// [`Hooks::boot`]**.
    ///
    /// `boot` is not on the path `start` takes. Initialising the store there
    /// left every request answering `503` while the health check stayed green —
    /// the worst combination available, because a load balancer keeps a
    /// wholly broken instance in rotation and reports it healthy.
    ///
    /// The verifier is built **first**, and its failure is fatal. A service
    /// that could not read its verification key and started anyway would serve
    /// an entire EHR to anyone who asked, with a green health check and no
    /// symptom (`db:PR12.16`). Ordering it before the store means the
    /// unconfigured case cannot reach a state where it holds an open database.
    async fn before_run(ctx: &AppContext) -> Result<()> {
        let verifier =
            PasetoVerifier::from_env().map_err(|e| loco_rs::Error::Message(e.to_string()))?;

        // Before the store, for the same reason as the verifier: a service
        // configured to audit reads and unable to write the log must not reach
        // a state where it holds an open database (`db:PR12.6`).
        let access_log = AccessLog::from_env().map_err(loco_rs::Error::Message)?;

        install(ctx, verifier, access_log, open_store()?);
        Ok(())
    }

    /// Layers a request-rate limit over the whole router — the perimeter
    /// protection `PHI.md`'s deployment statement names, and the only one
    /// this crate can offer for itself: TLS, and defence against a genuinely
    /// distributed flood, are the reverse proxy's job, stated as such rather
    /// than half-built here.
    ///
    /// The key is [`GlobalKeyExtractor`], not the crate's own default
    /// (peer IP), deliberately: behind the reverse proxy every deployment of
    /// this service is meant to sit behind (`PHI.md`), the peer IP this
    /// process sees is the proxy's, not the caller's, so a per-IP limiter
    /// would silently become a per-proxy one — no different from a global
    /// limit, except that it looks like more protection than it is. The
    /// honest alternative, trusting `X-Forwarded-For` to recover the real
    /// caller, is the same shape of mistake `auth.rs`'s own module doc
    /// refuses for identity — a header believed because of where it is
    /// expected to arrive from — and rate limiting is not worth reopening it
    /// for. One consequence follows from the choice: this limits the
    /// service's *total* load, not any one caller's, which is what
    /// [`SharedOpenehrStore`]'s single serialised connection can actually be
    /// overwhelmed by (`AGENTS.md`'s own reasoning for that `Mutex`, echoed
    /// here). Per-caller throttling would need identity — the verified
    /// PASETO subject — and this layer runs on every request, including the
    /// unauthenticated ones a flood is made of, so it cannot be keyed on a
    /// claim this early.
    ///
    /// # Errors
    ///
    /// Only if `RATE_LIMIT_BURST` or `RATE_LIMIT_PERIOD_SECS` were ever
    /// changed to zero — [`GovernorConfigBuilder::finish`]'s one failure
    /// mode, unreachable with the constants above but checked because a
    /// silently-absent rate limiter is worse than a refusal to start
    /// (`db:PR12.16`'s same reasoning, applied here).
    async fn after_routes(router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        let mut builder = GovernorConfigBuilder::default().key_extractor(GlobalKeyExtractor);
        builder
            .per_second(RATE_LIMIT_PERIOD_SECS)
            .burst_size(RATE_LIMIT_BURST);
        let governor_config = builder.finish().ok_or_else(|| {
            loco_rs::Error::Message(
                "rate limiter: RATE_LIMIT_BURST and RATE_LIMIT_PERIOD_SECS must be non-zero"
                    .to_owned(),
            )
        })?;
        Ok(router.layer(GovernorLayer::new(governor_config)))
    }

    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        // No background work. openEHR commits are synchronous by design: a
        // caller told 201 must be able to read the version back immediately.
        Ok(())
    }

    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(crate::tasks::Checkpoint);
        tasks.register(crate::tasks::Verify);
    }
}

/// Route tables, kept out of [`Hooks::routes`] so the controller modules own
/// their own paths.
mod controllers_routes {
    use loco_rs::controller::Routes;

    pub fn metadata() -> Routes {
        crate::controllers::metadata::routes()
    }
    pub fn ehr() -> Routes {
        crate::controllers::ehr::routes()
    }
    pub fn composition() -> Routes {
        crate::controllers::composition::routes()
    }
    pub fn contribution() -> Routes {
        crate::controllers::contribution::routes()
    }
}

/// Puts an already-built verifier, access log, and store into the shared
/// state `before_run` installs before any request is served.
///
/// Split out from [`Hooks::before_run`] so the installation itself — the part
/// with a consequence if it silently did nothing — can be tested without
/// going through the three environment variables `before_run` reads first.
/// This crate forbids `unsafe_code`, and `std::env::set_var` has required
/// `unsafe` since Rust's 2024 edition, so a test cannot drive `before_run`
/// through its real entry point at all; it can drive this.
fn install(ctx: &AppContext, verifier: PasetoVerifier, access_log: AccessLog, store: SqliteStore) {
    ctx.shared_store
        .insert::<SharedVerifier>(Arc::new(verifier));
    ctx.shared_store
        .insert::<SharedAccessLog>(Arc::new(access_log));
    ctx.shared_store
        .insert::<SharedOpenehrStore>(Arc::new(Mutex::new(store)));
}

#[cfg(test)]
mod tests {
    use super::{App, SharedAccessLog, SharedOpenehrStore, SharedVerifier, install};
    use crate::access::AccessLog;
    use crate::auth::PasetoVerifier;
    use loco_rs::{
        app::{AppContext, Hooks as _},
        environment::Environment,
    };
    use openehr_sqlite::SqliteStore;
    use openehr_store::Store as _;
    use pasetors::{
        keys::{AsymmetricKeyPair, Generate as _},
        paserk::FormatAsPaserk as _,
        version4::V4,
    };

    /// `install` — the part of `before_run` with a consequence if it silently
    /// did nothing — actually puts the verifier, the access log, and the
    /// store into `ctx.shared_store`.
    ///
    /// This is the fail-closed startup path the module doc calls out by name:
    /// a service that started without a working verifier would serve an
    /// entire EHR to anyone who asked, with a green health check and no
    /// symptom (`db:PR12.16`). Nothing previously exercised this — `tests/
    /// http.rs` builds its router against a `ctx` it populates by hand,
    /// bypassing `before_run` (and now `install`) entirely — so a version
    /// that did nothing at all would fail no test in this crate (`lib:A-09`).
    #[test]
    fn install_puts_the_verifier_access_log_and_store_into_shared_state() {
        let pair = AsymmetricKeyPair::<V4>::generate().expect("keypair");
        let mut paserk = String::new();
        pair.public.fmt(&mut paserk).expect("PASERK");
        let verifier = PasetoVerifier::new(&paserk, None, None, None).expect("verifier");
        let access_log = AccessLog::off();
        let mut store = SqliteStore::in_memory().expect("in-memory store");
        store.install().expect("install");

        let ctx = AppContext::builder(Environment::Test, loco_rs::tests_cfg::config::test_config())
            .build();
        assert!(ctx.shared_store.get::<SharedVerifier>().is_none());
        assert!(ctx.shared_store.get::<SharedAccessLog>().is_none());
        assert!(ctx.shared_store.get::<SharedOpenehrStore>().is_none());

        install(&ctx, verifier, access_log, store);

        assert!(
            ctx.shared_store.get::<SharedVerifier>().is_some(),
            "install did not install a verifier"
        );
        assert!(
            ctx.shared_store.get::<SharedAccessLog>().is_some(),
            "install did not install an access log"
        );
        assert!(
            ctx.shared_store.get::<SharedOpenehrStore>().is_some(),
            "install did not open the store"
        );
    }

    /// The name and version strings are what they claim to be, not a
    /// constant left over from a mutation.
    #[test]
    fn the_app_names_and_versions_itself() {
        assert_eq!(App::app_name(), "openehr_loco", "CARGO_CRATE_NAME changed?");
        assert!(App::app_version().contains(env!("CARGO_PKG_VERSION")));
    }

    /// The rate limiter answers `429` once `RATE_LIMIT_BURST` requests have
    /// arrived faster than the quota replenishes, and not before — checked
    /// directly against `after_routes`, since `tests/http.rs`'s own fixture
    /// builds its router through `AppRoutes::to_router` alone and never calls
    /// this hook (the same gap `install`'s own test above exists to close
    /// for `before_run`; a limiter wired up wrong, or not at all, would fail
    /// no other test in this crate).
    #[tokio::test]
    async fn the_rate_limiter_answers_429_only_after_the_burst_is_spent() {
        use axum::{
            Router,
            body::Body,
            http::{Request, StatusCode},
            routing::get,
        };
        use tower::ServiceExt as _;

        let ctx = AppContext::builder(Environment::Test, loco_rs::tests_cfg::config::test_config())
            .build();
        let base = Router::new().route("/x", get(|| async { "ok" }));
        let router = App::after_routes(base, &ctx).await.expect("after_routes");

        for n in 0..super::RATE_LIMIT_BURST {
            let response = router
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/x")
                        .body(Body::empty())
                        .expect("request"),
                )
                .await
                .expect("infallible");
            assert_eq!(
                response.status(),
                StatusCode::OK,
                "request {n} inside the burst was limited early"
            );
        }

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/x")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("infallible");
        assert_eq!(
            response.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "the request past the burst was not limited"
        );
    }
}
