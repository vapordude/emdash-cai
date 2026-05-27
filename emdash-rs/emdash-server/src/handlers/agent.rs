/// Agent-discovery endpoints.
///
/// These are not part of the WordPress-parity feature set — they are
/// **additional** endpoints that make EmDash natively agent-friendly.
///
/// - `GET /_emdash/api/openapi.json` → served automatically by utoipa-swagger-ui
/// - `GET /_emdash/api/manifest`     → machine-readable description of all
///                                     registered collections, their fields,
///                                     enabled capabilities, and plugin hooks.
use axum::extract::State;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

use emdash_core::ApiError;
use axum::{Router, routing::get};

use super::common::ApiEnvelope;
use crate::ServerContext;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct Manifest {
    pub version:      &'static str,
    pub collections:  Vec<CollectionManifest>,
    pub plugins:      Vec<String>,
    pub capabilities: Vec<&'static str>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CollectionManifest {
    pub name:    String,
    pub title:   String,
    pub is_feed: bool,
    pub fields:  Vec<FieldManifest>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct FieldManifest {
    pub name:       String,
    pub title:      String,
    pub field_type: String,
    pub required:   bool,
}

/// Machine-readable manifest for agent / pipeline discovery.
///
/// Agents can `GET /_emdash/api/manifest` to understand what collections
/// exist, what fields they have, which plugins are active, and what
/// capabilities the server exposes — all without reading source code.
#[utoipa::path(
    get, path = "/_emdash/api/manifest",
    tag = "agent",
    responses((status = 200, body = inline(ApiEnvelope<Manifest>)))
)]
pub async fn get_manifest(
    State(ctx): State<Arc<ServerContext>>,
) -> Result<ApiEnvelope<Manifest>, ApiError> {
    let col_rows = ctx.db.list("_emdash_collections").await?;

    let mut collections = Vec::new();
    for col in &col_rows {
        let name    = col["name"].as_str().unwrap_or("").to_string();
        let title   = col["title"].as_str().unwrap_or("").to_string();
        let is_feed = col["is_feed"].as_i64().unwrap_or(0) == 1;
        let col_id  = col["id"].as_str().unwrap_or("").to_string();

        let field_rows = ctx.db
            .query(
                "SELECT name, title, field_type, required FROM _emdash_fields \
                 WHERE collection_id = ? ORDER BY position",
                vec![Value::String(col_id)],
            )
            .await
            .unwrap_or_default();

        let fields = field_rows
            .into_iter()
            .map(|f| FieldManifest {
                name:       f["name"].as_str().unwrap_or("").to_string(),
                title:      f["title"].as_str().unwrap_or("").to_string(),
                field_type: f["field_type"].as_str().unwrap_or("unknown").to_string(),
                required:   f["required"].as_i64().unwrap_or(0) == 1,
            })
            .collect();

        collections.push(CollectionManifest { name, title, is_feed, fields });
    }

    let plugins = ctx.plugin_runner.loaded_plugins().await;

    Ok(ApiEnvelope::new(Manifest {
        version:      env!("CARGO_PKG_VERSION"),
        collections,
        plugins,
        capabilities: vec![
            "content:crud",
            "schema:dynamic",
            "media:library",
            "taxonomies",
            "menus",
            "redirects",
            "revisions",
            "comments",
            "plugins:wasm",
            "llm:chat",
            "llm:embed",
            "site:render",
            "site:rss",
            "site:sitemap",
        ],
    }))
}

pub fn router() -> Router<Arc<ServerContext>> {
    Router::new()
        .route("/_emdash/api/manifest", get(get_manifest))
}
