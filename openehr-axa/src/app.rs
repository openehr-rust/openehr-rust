//! The application: its routes, its startup, and its tasks.

use axa::{
    AppContext, Result,
    cli::{Cli, Command},
    config::{Defaults, MB},
    routes::AppRoutes,
    server,
    task::Tasks,
};
use axum::Router as AxumRouter;
use openehr_sqlite::SqliteStore;
use openehr_store::Store as _;
use std::sync::{Arc, Mutex};
use tower_governor::{
    GovernorLayer, governor::GovernorConfigBuilder, key_extractor::GlobalKeyExtractor,
};

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
/// The server's opener, used by [`prepare`]. The inspection tasks do **not**
/// use it: they open read-only through [`open_store_read_only`], because an
/// inspector that creates and installs turns a mistyped backup path into a new
/// empty store (`db:O10.20`, `db:D-27`). Both read the same path setting
/// ([`configured_path`]), so the default database cannot drift between the
/// server and the tools that inspect what it wrote. A task does not get
/// `prepare` — `app::run` builds the context and runs it directly — so it
/// opens its own store either way.
///
/// # Errors
///
/// Returns [`axa::Error::Message`] if the file cannot be opened or the
/// schema cannot be installed — including when the database was built under a
/// different schema version, which is refused rather than half-served
/// (`db:O10.15`).
pub fn open_store() -> Result<SqliteStore> {
    open_store_at(&configured_path()?)
}

/// The database path the service uses: `OPENEHR_SQLITE_PATH`, or
/// `openehr.sqlite3` when it is unset.
fn configured_path() -> Result<std::path::PathBuf> {
    let path = match std::env::var("OPENEHR_SQLITE_PATH") {
        Ok(path) => path,
        // RCA-IN-3: absent only. An unset variable means the default file.
        Err(std::env::VarError::NotPresent) => "openehr.sqlite3".to_owned(),
        // RCA-IN-4: a supplied but unusable setting stops startup.
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err(axa::Error::Message(
                "OPENEHR_SQLITE_PATH is not valid Unicode".to_owned(),
            ));
        }
    };
    Ok(std::path::PathBuf::from(path))
}

/// Opens an existing store **read-only**, for the inspection tasks
/// (`db:O10.20`): `path` when given, otherwise the service's own database.
///
/// Not [`open_store_at`]: that one creates and installs, which is right for a
/// server starting on an empty volume and wrong for a tool pointed at a
/// backup. Through it, a mistyped backup path became a new six-table database
/// and `checkpoint` printed an empty witness with exit 0 (`db:D-27`).
///
/// # Errors
///
/// Returns [`axa::Error::Message`] if the file does not exist, cannot be
/// opened, or holds no installed schema of this build's version.
pub fn open_store_read_only(path: Option<&std::path::Path>) -> Result<SqliteStore> {
    let path = match path {
        Some(path) => path.to_path_buf(),
        None => configured_path()?,
    };
    SqliteStore::open_read_only(&path)
        .map_err(|e| axa::Error::Message(format!("{}: {e}", path.display())))
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
    let mut store = SqliteStore::open(path).map_err(|e| axa::Error::Message(e.to_string()))?;
    store
        .install()
        .map_err(|e| axa::Error::Message(e.to_string()))?;
    Ok(store)
}

/// The service name, as `--help` and the log report it.
#[must_use]
pub fn app_name() -> &'static str {
    env!("CARGO_CRATE_NAME")
}

/// The version, with the build it came from.
#[must_use]
pub fn app_version() -> String {
    format!(
        "{} ({})",
        env!("CARGO_PKG_VERSION"),
        option_env!("BUILD_SHA")
            .or(option_env!("GITHUB_SHA"))
            .unwrap_or("dev")
    )
}

/// The route groups, on whatever `base` already carries.
fn add_routes(base: AppRoutes) -> AppRoutes {
    base.add_route(controllers_routes::metadata())
        .add_route(controllers_routes::ehr())
        .add_route(controllers_routes::ehr_status())
        .add_route(controllers_routes::versioned_ehr_status())
        .add_route(controllers_routes::contribution())
        .add_route(controllers_routes::composition())
        .add_route(controllers_routes::query())
}

/// The clinical routes alone.
///
/// What the HTTP tests mount, so that a test of a status code is not also a
/// test of the health endpoints.
#[must_use]
pub fn routes() -> AppRoutes {
    add_routes(AppRoutes::empty())
}

/// The clinical routes and Axa's `/_health/*` endpoints: what the server runs.
#[must_use]
pub fn service_routes() -> AppRoutes {
    add_routes(AppRoutes::with_default_routes())
}

/// Opens the store and builds the verifier, **before the server listens and
/// not lazily**.
///
/// Initialising the store later left every request answering `503` while the
/// health check stayed green — the worst combination available, because a
/// load balancer keeps a wholly broken instance in rotation and reports it
/// healthy.
///
/// The verifier is built **first**, and its failure is fatal. A service
/// that could not read its verification key and started anyway would serve
/// an entire EHR to anyone who asked, with a green health check and no
/// symptom (`db:PR12.16`). Ordering it before the store means the
/// unconfigured case cannot reach a state where it holds an open database.
///
/// # Errors
///
/// [`axa::Error::Message`] if the verification key is absent or malformed, if
/// the access log is configured and cannot be opened, or if the store cannot
/// be opened (see [`open_store`]).
pub fn prepare(ctx: &AppContext) -> Result<()> {
    let verifier = PasetoVerifier::from_env().map_err(|e| axa::Error::Message(e.to_string()))?;

    // Before the store, for the same reason as the verifier: a service
    // configured to audit reads and unable to write the log must not reach
    // a state where it holds an open database (`db:PR12.6`).
    let access_log = AccessLog::from_env().map_err(axa::Error::Message)?;
    // The service's own system id (`db:S1.22`), refused at startup if unusable,
    // like every other setting (`db:PR12.24`).
    let system_id = crate::controllers::ehr::system_id_from(&|name| std::env::var_os(name))
        .map_err(axa::Error::Message)?;
    ctx.shared_store.insert(system_id);

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
pub fn with_rate_limit(router: AxumRouter) -> Result<AxumRouter> {
    let mut builder = GovernorConfigBuilder::default().key_extractor(GlobalKeyExtractor);
    builder
        .per_second(RATE_LIMIT_PERIOD_SECS)
        .burst_size(RATE_LIMIT_BURST);
    let governor_config = builder.finish().ok_or_else(|| {
        axa::Error::Message(
            "rate limiter: RATE_LIMIT_BURST and RATE_LIMIT_PERIOD_SECS must be non-zero".to_owned(),
        )
    })?;
    Ok(router.layer(GovernorLayer::new(governor_config)))
}

/// The settings Axa reads, with this service's own defaults.
///
/// The port and the 8 MB body limit are the ones the Loco configuration files this service used to carry
/// carried, so a proxy or health check pointed at the old service still finds
/// this one. The service has no database of Axa's (an empty name): its store is
/// `openehr-store`, opened by [`prepare`].
#[must_use]
pub fn defaults() -> Defaults {
    Defaults {
        database: "",
        port: 5150,
        body_limit: 8 * MB,
        request_timeout_seconds: Some(10),
        ..Defaults::default()
    }
}

/// The finished router: the clinical routes and the health endpoints, the
/// rate limit, and then Axa's standard middleware.
///
/// In that order on purpose. A layer added *after* the standard middleware sits
/// outside it, so the limiter's `429` would carry no request id and no
/// security headers, and would escape the timeout and panic protection.
///
/// # Errors
///
/// As [`with_rate_limit`], or if the middleware cannot be applied.
pub fn router(ctx: &AppContext) -> Result<AxumRouter> {
    service_routes().into_app_with(ctx.clone(), with_rate_limit)
}

/// Runs one command line: serve, run a task, check the configuration, or ask a
/// running instance whether it is healthy.
///
/// # Errors
///
/// Whatever the command fails with, so that `main` exits non-zero.
pub async fn run(cli: Cli) -> Result<()> {
    let environment = cli.environment();
    // `check` and `health` are answered before anything is opened, so they
    // work with the store missing.
    if let Some(answered) = axa::cli::answer(&cli, &environment, &defaults()).await {
        return answered;
    }
    // `Config::for_task` for a task, which does not listen and so must not be
    // refused for a listener-only setting; `Config::from_env` otherwise; the
    // command line's `--binding` and `--port` applied either way.
    let config = cli.config(&environment, &defaults())?;
    axa::logger::init(app_name(), &config.logger);
    let ctx = AppContext::new(environment, (), config);
    match cli.command {
        Command::Start { .. } => {
            prepare(&ctx)?;
            let server_config = ctx.config.server.clone();
            server::serve(router(&ctx)?, &server_config).await
        }
        // A task opens its own store: `prepare` is not on this path (see
        // [`crate::tasks`]). With no name, the task list is printed.
        Command::Task { name, params } => tasks().run_or_list(name.as_deref(), &ctx, &params).await,
        Command::Db { .. } => Err(axa::Error::Message(
            "this service has no database of Axa's: its store is openehr-store, \
             installed when it starts (db:O10.14)"
                .to_owned(),
        )),
        Command::Check | Command::Health { .. } => Ok(()), // Answered above.
    }
}

/// The operator tasks.
///
/// No background work exists beside them: openEHR commits are synchronous by
/// design, so a caller told 201 can read the version back immediately.
#[must_use]
pub fn tasks() -> Tasks {
    let mut tasks = Tasks::default();
    tasks.register(crate::tasks::Checkpoint);
    tasks.register(crate::tasks::Verify);
    tasks
}

/// Route tables, kept out of [`routes`] so the controller modules own their
/// own paths.
mod controllers_routes {
    use axa::routes::Routes;

    pub fn metadata() -> Routes {
        crate::controllers::metadata::routes()
    }
    pub fn ehr() -> Routes {
        crate::controllers::ehr::routes()
    }
    pub fn ehr_status() -> Routes {
        crate::controllers::ehr_status::routes()
    }
    pub fn versioned_ehr_status() -> Routes {
        crate::controllers::ehr_status::versioned_routes()
    }
    pub fn composition() -> Routes {
        crate::controllers::composition::routes()
    }
    pub fn contribution() -> Routes {
        crate::controllers::contribution::routes()
    }
    pub fn query() -> Routes {
        crate::controllers::query::routes()
    }
}

/// Puts an already-built verifier, access log, and store into the shared
/// state `prepare` installs before any request is served.
///
/// Split out from [`prepare`] so the installation itself — the part
/// with a consequence if it silently did nothing — can be tested without
/// going through the three environment variables `prepare` reads first.
/// This crate forbids `unsafe_code`, and `std::env::set_var` has required
/// `unsafe` since Rust's 2024 edition, so a test cannot drive `prepare`
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
    use super::{
        SharedAccessLog, SharedOpenehrStore, SharedVerifier, app_name, app_version, defaults,
        install, prepare, router, routes, service_routes, with_rate_limit,
    };
    use crate::access::AccessLog;
    use crate::auth::PasetoVerifier;
    use openehr_sqlite::SqliteStore;
    use openehr_store::Store as _;
    use pasetors::{
        keys::{AsymmetricKeyPair, Generate as _},
        paserk::FormatAsPaserk as _,
        version4::V4,
    };

    /// `install` — the part of `prepare` with a consequence if it silently
    /// did nothing — actually puts the verifier, the access log, and the
    /// store into `ctx.shared_store`.
    ///
    /// This is the fail-closed startup path the module doc calls out by name:
    /// a service that started without a working verifier would serve an
    /// entire EHR to anyone who asked, with a green health check and no
    /// symptom (`db:PR12.16`). Nothing previously exercised this — `tests/
    /// http.rs` builds its router against a `ctx` it populates by hand,
    /// bypassing `prepare` (and now `install`) entirely — so a version
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

        let ctx = axa::AppContext::for_test(()).expect("test context");
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
        assert_eq!(app_name(), "openehr_axa", "CARGO_CRATE_NAME changed?");
        assert!(app_version().contains(env!("CARGO_PKG_VERSION")));
    }

    /// The rate limiter answers `429` once `RATE_LIMIT_BURST` requests have
    /// arrived faster than the quota replenishes, and not before — checked
    /// directly against `after_routes`, since `tests/http.rs`'s own fixture
    /// builds its router through `AppRoutes::to_router` alone and never calls
    /// this hook (the same gap `install`'s own test above exists to close
    /// for `prepare`; a limiter wired up wrong, or not at all, would fail
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

        let base = Router::new().route("/x", get(|| async { "ok" }));
        let router = with_rate_limit(base).expect("with_rate_limit");

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

    /// The defaults are the ones the Loco configuration carried, stated once in
    /// code: the port a proxy already points at, the 8 MB body limit, and no
    /// database of Axa's.
    #[test]
    fn the_defaults_are_the_ports_and_limits_the_service_always_had() {
        let d = defaults();
        assert_eq!(d.database, "");
        assert_eq!(d.port, 5150);
        assert_eq!(d.body_limit, 8 * axa::config::MB);
        assert_eq!(d.request_timeout_seconds, Some(10));
    }

    /// The clinical routes are mounted without the health endpoints, and the
    /// service's routes add them: a status-code test must not also be a test
    /// of `/_health`.
    #[test]
    fn the_service_adds_health_routes_to_the_clinical_ones() {
        let clinical = routes().mounted();
        let served = service_routes().mounted();
        assert!(clinical.iter().all(|(_, path)| !path.contains("_health")));
        assert!(served.iter().any(|(_, path)| path.contains("_health")));
        for route in &clinical {
            assert!(served.contains(route), "{route:?} is not served");
        }
        assert!(clinical.len() >= 10, "the clinical routes were not mounted");
    }

    /// With no verification key configured, startup is refused — the
    /// fail-closed path (`db:PR12.16`) — and nothing is installed.
    #[test]
    fn startup_is_refused_when_no_verification_key_is_configured() {
        if std::env::var(crate::auth::KEYS_VAR).is_ok() {
            return; // the operator's own environment names a key; not this test's to unset
        }
        let ctx = axa::AppContext::for_test(()).expect("test context");
        assert!(prepare(&ctx).is_err());
        assert!(ctx.shared_store.get::<SharedOpenehrStore>().is_none());
    }

    /// The finished router answers its health check, and a limiter refusal
    /// carries the request id — which only holds if the limiter is layered
    /// *inside* Axa's standard middleware (the order `router` documents).
    #[tokio::test]
    async fn the_limiters_refusal_carries_the_standard_middlewares_request_id() {
        use axum::{
            body::Body,
            http::{Request, StatusCode},
        };
        use tower::ServiceExt as _;

        let app = router(&axa::AppContext::for_test(()).expect("test context")).expect("router");
        let get = |uri: &str| {
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .expect("request")
        };

        let live = app
            .clone()
            .oneshot(get("/_health/live"))
            .await
            .expect("infallible");
        assert_eq!(live.status(), StatusCode::OK);

        let mut last = None;
        for _ in 0..=super::RATE_LIMIT_BURST {
            last = Some(
                app.clone()
                    .oneshot(get("/_health/live"))
                    .await
                    .expect("infallible"),
            );
        }
        let refused = last.expect("requests were sent");
        assert_eq!(refused.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(
            refused.headers().contains_key("x-request-id"),
            "the 429 has no request id: the limiter sits outside the standard middleware"
        );
    }
}
