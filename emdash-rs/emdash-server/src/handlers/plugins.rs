use axum::{Json, extract::{Path, State}};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

use emdash_core::ApiError;
use axum::{Router, routing::{delete, get, post}};

use super::common::ApiEnvelope;
use crate::ServerContext;

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Plugin {
    pub id:           String,
    pub plugin_id:    String,
    pub name:         String,
    pub wasm_path:    String,
    pub capabilities: Vec<String>,
    pub enabled:      bool,
    pub created_at:   String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct InstallPluginBody {
    pub plugin_id:    String,
    pub name:         String,
    pub wasm_path:    String,
    pub capabilities: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ExecuteHookBody {
    pub hook_name: String,
    pub payload:   Value,
}

/// List installed plugins.
#[utoipa::path(
    get, path = "/_emdash/api/plugins",
    tag = "plugins",
    responses((status = 200, description = "Success"))
)]
pub async fn list_plugins(
    State(ctx): State<Arc<ServerContext>>,
) -> Result<ApiEnvelope<Vec<Value>>, ApiError> {
    let rows = ctx.db.list("_emdash_plugins").await?;
    Ok(ApiEnvelope::with_total(rows.clone(), rows.len() as u64))
}

/// Install (register) a plugin.
///
/// After registering, the server loads the Wasm module into the sandbox.
#[utoipa::path(
    post, path = "/_emdash/api/plugins",
    tag = "plugins",
    request_body = InstallPluginBody,
    responses((status = 201, description = "Success"))
)]
pub async fn install_plugin(
    State(ctx): State<Arc<ServerContext>>,
    Json(body): Json<InstallPluginBody>,
) -> Result<(axum::http::StatusCode, ApiEnvelope<Value>), ApiError> {
    let id   = Uuid::new_v4().to_string();
    let now  = Utc::now().to_rfc3339();
    let caps = serde_json::to_string(&body.capabilities.unwrap_or_default())
        .unwrap_or_else(|_| "[]".to_string());

    ctx.db
        .execute(
            "INSERT INTO _emdash_plugins \
             (id, plugin_id, name, wasm_path, capabilities, enabled, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, 1, ?, ?)",
            vec![
                Value::String(id.clone()),
                Value::String(body.plugin_id.clone()),
                Value::String(body.name),
                Value::String(body.wasm_path.clone()),
                Value::String(caps),
                Value::String(now.clone()),
                Value::String(now),
            ],
        )
        .await?;

    // Load Wasm bytes from storage and register in the sandbox.
    match ctx.storage.get_file(&body.wasm_path).await {
        Ok(wasm_bytes) => {
            ctx.plugin_runner
                .load_plugin(&body.plugin_id, &wasm_bytes)
                .await
                .ok(); // Non-fatal: sandbox may not be enabled.
        }
        Err(e) => {
            tracing::warn!(plugin_id = %body.plugin_id, error = %e, "could not load wasm bytes");
        }
    }

    let item = ctx.db.get_by_id("_emdash_plugins", &id).await?
        .ok_or_else(|| ApiError::Internal("insert failed".into()))?;
    Ok((axum::http::StatusCode::CREATED, ApiEnvelope::new(item)))
}

/// Uninstall a plugin.
#[utoipa::path(
    delete, path = "/_emdash/api/plugins/{plugin_id}",
    tag = "plugins",
    params(("plugin_id" = String, Path, description = "Plugin identifier")),
    responses((status = 200, description = "Success"))
)]
pub async fn uninstall_plugin(
    State(ctx): State<Arc<ServerContext>>,
    Path(plugin_id): Path<String>,
) -> Result<ApiEnvelope<Value>, ApiError> {
    ctx.plugin_runner.unload_plugin(&plugin_id).await.ok();
    ctx.db
        .execute(
            "DELETE FROM _emdash_plugins WHERE plugin_id = ?",
            vec![Value::String(plugin_id.clone())],
        )
        .await?;
    Ok(ApiEnvelope::new(serde_json::json!({ "uninstalled": plugin_id })))
}

/// Manually invoke a lifecycle hook on a plugin (for testing).
#[utoipa::path(
    post, path = "/_emdash/api/plugins/{plugin_id}/hooks",
    tag = "plugins",
    params(("plugin_id" = String, Path, description = "Plugin identifier")),
    request_body = ExecuteHookBody,
    responses((status = 200, description = "Success"))
)]
pub async fn execute_hook(
    State(ctx): State<Arc<ServerContext>>,
    Path(plugin_id): Path<String>,
    Json(body): Json<ExecuteHookBody>,
) -> Result<ApiEnvelope<Value>, ApiError> {
    let result = ctx.plugin_runner
        .execute_hook(&plugin_id, &body.hook_name, body.payload)
        .await?;
    Ok(ApiEnvelope::new(result))
}

/// List currently loaded plugin IDs (sandbox runtime state).
#[utoipa::path(
    get, path = "/_emdash/api/plugins/loaded",
    tag = "plugins",
    responses((status = 200, body = inline(ApiEnvelope<Vec<String>>)))
)]
pub async fn list_loaded_plugins(
    State(ctx): State<Arc<ServerContext>>,
) -> ApiEnvelope<Vec<String>> {
    let loaded = ctx.plugin_runner.loaded_plugins().await;
    ApiEnvelope::new(loaded)
}

pub fn router() -> Router<Arc<ServerContext>> {
    Router::new()
        .route("/_emdash/api/plugins", get(list_plugins).post(install_plugin))
        .route("/_emdash/api/plugins/loaded", get(list_loaded_plugins))
        .route("/_emdash/api/plugins/{plugin_id}", delete(uninstall_plugin))
        .route("/_emdash/api/plugins/{plugin_id}/hooks", post(execute_hook))
}
