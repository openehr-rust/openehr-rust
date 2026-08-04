//! The Loco application.

use async_trait::async_trait;
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

use crate::access::{AccessLog, SharedAccessLog};
use crate::auth::{PasetoVerifier, SharedVerifier};

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
}
