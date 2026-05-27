pub mod feeds;
pub mod sitemap;
pub mod templates;

use axum::{
    Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

use crate::ServerContext;
use emdash_core::ApiError;

// ── Public REST API (WP REST API compatible shape) ────────────────────────────

#[derive(Deserialize)]
struct PublicListQuery {
    #[allow(dead_code)]
    page: Option<u32>,
    #[allow(dead_code)]
    per_page: Option<u32>,
    status: Option<String>,
}

/// Public content list — matches WordPress REST API shape at `/api/v1/{collection}`.
async fn public_list(
    State(ctx): State<Arc<ServerContext>>,
    Path(collection): Path<String>,
    Query(q): Query<PublicListQuery>,
) -> Result<axum::Json<Value>, (StatusCode, String)> {
    // Only return published content on the public API.
    let table = format!("ec_{collection}");
    let status = q.status.as_deref().unwrap_or("published");
    let rows = ctx
        .db
        .query(
            &format!("SELECT * FROM {table} WHERE status = ? ORDER BY created_at DESC"),
            vec![Value::String(status.to_string())],
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(axum::Json(Value::Array(rows)))
}

/// Public single-item fetch at `/api/v1/{collection}/{slug}`.
async fn public_get(
    State(ctx): State<Arc<ServerContext>>,
    Path((collection, slug)): Path<(String, String)>,
) -> Result<axum::Json<Value>, (StatusCode, String)> {
    let table = format!("ec_{collection}");
    let rows = ctx
        .db
        .query(
            &format!("SELECT * FROM {table} WHERE slug = ? AND status = 'published' LIMIT 1"),
            vec![Value::String(slug.clone())],
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    match rows.into_iter().next() {
        Some(item) => Ok(axum::Json(item)),
        None => Err((StatusCode::NOT_FOUND, format!("not found: {slug}"))),
    }
}

// ── HTML site routing ─────────────────────────────────────────────────────────

/// Site root — renders the first published item of the first collection, or
/// the template `home.html`.
async fn site_index(State(ctx): State<Arc<ServerContext>>) -> Response {
    match templates::render_home(&ctx).await {
        Ok(html) => Html(html).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Generic `/{collection}/{slug}` route — resolves and renders a content item.
async fn site_page(
    State(ctx): State<Arc<ServerContext>>,
    Path((collection, slug)): Path<(String, String)>,
) -> Response {
    match templates::render_page(&ctx, &collection, &slug).await {
        Ok(html) => Html(html).into_response(),
        Err(ApiError::NotFound(_)) => (StatusCode::NOT_FOUND, "Not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// RSS feed for any collection that has `is_feed = true`.
async fn rss_feed(
    State(ctx): State<Arc<ServerContext>>,
    Path(collection): Path<String>,
) -> Response {
    match feeds::rss_for_collection(&ctx, &collection).await {
        Ok(xml) => (
            [(header::CONTENT_TYPE, "application/rss+xml; charset=utf-8")],
            xml,
        )
            .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// XML sitemap for all published content.
async fn xml_sitemap(State(ctx): State<Arc<ServerContext>>) -> Response {
    match sitemap::generate(&ctx).await {
        Ok(xml) => (
            [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
            xml,
        )
            .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<ServerContext>> {
    Router::new()
        // Public WP-compatible REST API
        .route("/api/v1/:collection", get(public_list))
        .route("/api/v1/:collection/:slug", get(public_get))
        // HTML site
        .route("/", get(site_index))
        .route("/:collection/:slug", get(site_page))
        // Feeds & sitemap
        .route("/feeds/:collection.rss", get(rss_feed))
        .route("/sitemap.xml", get(xml_sitemap))
}
