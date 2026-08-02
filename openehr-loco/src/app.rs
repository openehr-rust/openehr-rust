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
        ctx.shared_store
            .insert::<SharedVerifier>(Arc::new(verifier));

        ctx.shared_store
            .insert::<SharedOpenehrStore>(Arc::new(Mutex::new(open_store()?)));
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
