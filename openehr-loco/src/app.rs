//! The Loco application.

use async_trait::async_trait;
use loco_rs::{
    Result,
    app::{AppContext, Hooks, Initializer},
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

        let mut store = SqliteStore::open(std::path::Path::new(
            &std::env::var("OPENEHR_SQLITE_PATH").unwrap_or_else(|_| "openehr.sqlite3".to_owned()),
        ))
        .map_err(|e| loco_rs::Error::Message(e.to_string()))?;
        store
            .install()
            .map_err(|e| loco_rs::Error::Message(e.to_string()))?;
        ctx.shared_store
            .insert::<SharedOpenehrStore>(Arc::new(Mutex::new(store)));
        Ok(())
    }

    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![Box::new(crate::initializers::RequestIdInitializer)])
    }

    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        // No background work. openEHR commits are synchronous by design: a
        // caller told 201 must be able to read the version back immediately.
        Ok(())
    }

    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(crate::tasks::Checkpoint);
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
}
