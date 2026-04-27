use axum::{routing::get, Router};
use std::sync::Arc;

use emdash_core::{DatabaseProvider, LlmProvider, PluginRunner, StorageProvider};

/// A universal context that can be injected into any request,
/// abstracting away the underlying implementations (e.g., SQLite vs Postgres).
#[derive(Clone)]
pub struct ServerContext {
    pub db: Arc<dyn DatabaseProvider + Send + Sync>,
    pub storage: Arc<dyn StorageProvider + Send + Sync>,
    pub llm: Arc<dyn LlmProvider + Send + Sync>,
    pub plugin_runner: Arc<dyn PluginRunner + Send + Sync>,
}

/// A basic health check handler
async fn health_check() -> &'static str {
    "OK"
}

/// Create the universal API router
pub fn create_router(context: ServerContext) -> Router {
    // Basic setup showing state injection.
    // In the future we will bind concrete paths to the providers.
    Router::new()
        .route("/_emdash/health", get(health_check))
        .with_state(context)
}
