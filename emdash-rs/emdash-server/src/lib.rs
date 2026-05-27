pub mod handlers;
pub mod site;

use axum::{Router, routing::get, response::Html};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;

use emdash_core::{DatabaseProvider, LlmProvider, StorageProvider};
use emdash_sandbox::PluginRunner;

// ── Server context ────────────────────────────────────────────────────────────

/// Shared state injected into every request handler.
#[derive(Clone)]
pub struct ServerContext {
    pub db:            Arc<dyn DatabaseProvider>,
    pub storage:       Arc<dyn StorageProvider>,
    pub llm:           Arc<dyn LlmProvider>,
    pub plugin_runner: Arc<dyn PluginRunner>,
}

// ── OpenAPI document ──────────────────────────────────────────────────────────

#[derive(OpenApi)]
#[openapi(
    info(
        title       = "EmDash API",
        version     = "0.1.0",
        description = "Agent-portable CMS REST API — OpenAI-compatible, fully machine-readable."
    ),
    components(schemas(
        emdash_core::ApiError,
        emdash_core::ChatMessage,
        emdash_core::LlmOptions,
        emdash_core::portable_text::Block,
        emdash_core::portable_text::Span,
        emdash_core::portable_text::MarkDef,
        emdash_core::portable_text::ImageBlock,
        emdash_core::portable_text::ImageAsset,
        emdash_core::portable_text::Hotspot,
        emdash_core::portable_text::Crop,
        emdash_core::portable_text::ObjectBlock,
        emdash_schema::Collection,
        emdash_schema::Field,
        emdash_schema::FieldType,
        handlers::common::PaginationMeta,
        handlers::content::ContentItem,
        handlers::content::CreateContentBody,
        handlers::content::UpdateContentBody,
        handlers::schema::CreateCollectionBody,
        handlers::settings::SettingItem,
        handlers::media::MediaItem,
        handlers::taxonomies::Taxonomy,
        handlers::taxonomies::Term,
        handlers::menus::Menu,
        handlers::menus::MenuItem,
        handlers::redirects::Redirect,
        handlers::auth::ApiToken,
        handlers::comments::Comment,
        handlers::plugins::Plugin,
        handlers::dashboard::DashboardStats,
    )),
    tags(
        (name = "content",    description = "Content items (posts, pages, etc.)"),
        (name = "schema",     description = "Collection and field schema management"),
        (name = "media",      description = "Media library"),
        (name = "settings",   description = "Site settings"),
        (name = "taxonomies", description = "Tags, categories and terms"),
        (name = "menus",      description = "Navigation menus"),
        (name = "redirects",  description = "URL redirects and 404 tracking"),
        (name = "auth",       description = "Authentication and API tokens"),
        (name = "revisions",  description = "Content revision history"),
        (name = "comments",   description = "Content comments"),
        (name = "plugins",    description = "Plugin sandbox management"),
        (name = "dashboard",  description = "Aggregate statistics"),
        (name = "agent",      description = "Agent-discovery endpoints"),
    )
)]
pub struct ApiDoc;

// ── Router factory ────────────────────────────────────────────────────────────

/// Build the complete axum router wired to the given context.
pub fn create_router(ctx: Arc<ServerContext>) -> Router {
    // Resolve state on all state-aware routers → Router<()>
    let api = Router::new()
        .merge(handlers::content::router())
        .merge(handlers::schema::router())
        .merge(handlers::media::router())
        .merge(handlers::settings::router())
        .merge(handlers::taxonomies::router())
        .merge(handlers::menus::router())
        .merge(handlers::redirects::router())
        .merge(handlers::auth::router())
        .merge(handlers::revisions::router())
        .merge(handlers::comments::router())
        .merge(handlers::plugins::router())
        .merge(handlers::dashboard::router())
        .merge(handlers::agent::router())
        .with_state(ctx.clone());

    let site = site::router().with_state(ctx);

    Router::new()
        .route("/_emdash/health",           get(health_check))
        .route("/_emdash/api/openapi.json", get(openapi_json))
        .route("/_emdash/docs",             get(swagger_ui))
        .merge(api)
        .merge(site)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

/// Return the OpenAPI document (used by `emdash schema` CLI subcommand).
pub fn create_api_doc() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

// ── Internal handlers ─────────────────────────────────────────────────────────

async fn health_check() -> &'static str { "OK" }

/// Serve the OpenAPI 3.1 spec as JSON.
async fn openapi_json() -> axum::response::Response {
    let json = serde_json::to_string_pretty(&ApiDoc::openapi())
        .unwrap_or_else(|_| "{}".to_string());
    axum::response::Response::builder()
        .header("content-type", "application/json; charset=utf-8")
        .body(axum::body::Body::from(json))
        .unwrap()
}

/// Serve the Swagger UI HTML (loads swagger-ui from CDN — no extra dep needed).
async fn swagger_ui() -> Html<&'static str> {
    Html(r##"<!doctype html>
<html>
<head>
  <title>EmDash API — Swagger UI</title>
  <meta charset="utf-8">
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css">
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script>
    SwaggerUIBundle({
      url: "/_emdash/api/openapi.json",
      dom_id: "#swagger-ui",
      presets: [SwaggerUIBundle.presets.apis, SwaggerUIBundle.SwaggerUIStandalonePreset],
      layout: "BaseLayout"
    });
  </script>
</body>
</html>"##)
}
