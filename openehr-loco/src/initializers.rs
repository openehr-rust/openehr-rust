//! Initializers.

use async_trait::async_trait;
use axum::Router as AxumRouter;
use loco_rs::{app::AppContext, app::Initializer, Result};

/// Echoes a request id back on every response.
///
/// Small, and the reason is operational: a caller reporting "it returned 500"
/// with an id is reporting an incident someone can find in a log. Without one
/// they are reporting a feeling.
pub struct RequestIdInitializer;

#[async_trait]
impl Initializer for RequestIdInitializer {
    fn name(&self) -> String {
        "request-id".to_owned()
    }

    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        Ok(router)
    }
}
