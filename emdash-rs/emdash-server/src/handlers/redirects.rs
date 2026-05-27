use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

use axum::{
    Router,
    routing::{delete, get},
};
use emdash_core::ApiError;

use super::common::ApiEnvelope;
use crate::ServerContext;

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Redirect {
    pub id: String,
    pub from_path: String,
    pub to_path: String,
    pub status_code: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateRedirectBody {
    pub from_path: String,
    pub to_path: String,
    pub status_code: Option<i64>,
}

/// List redirects.
#[utoipa::path(
    get, path = "/_emdash/api/redirects",
    tag = "redirects",
    responses((status = 200, description = "Success"))
)]
pub async fn list_redirects(
    State(ctx): State<Arc<ServerContext>>,
) -> Result<ApiEnvelope<Vec<Value>>, ApiError> {
    let rows = ctx.db.list("_emdash_redirects").await?;
    Ok(ApiEnvelope::with_total(rows.clone(), rows.len() as u64))
}

/// Create a redirect.
#[utoipa::path(
    post, path = "/_emdash/api/redirects",
    tag = "redirects",
    request_body = CreateRedirectBody,
    responses((status = 201, description = "Success"))
)]
pub async fn create_redirect(
    State(ctx): State<Arc<ServerContext>>,
    Json(body): Json<CreateRedirectBody>,
) -> Result<(axum::http::StatusCode, ApiEnvelope<Value>), ApiError> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let status_code = body.status_code.unwrap_or(301);
    ctx.db
        .execute(
            "INSERT INTO _emdash_redirects (id, from_path, to_path, status_code, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
            vec![
                Value::String(id.clone()),
                Value::String(body.from_path),
                Value::String(body.to_path),
                Value::Number(status_code.into()),
                Value::String(now.clone()),
                Value::String(now),
            ],
        )
        .await?;
    let item = ctx
        .db
        .get_by_id("_emdash_redirects", &id)
        .await?
        .ok_or_else(|| ApiError::Internal("insert failed".into()))?;
    Ok((axum::http::StatusCode::CREATED, ApiEnvelope::new(item)))
}

/// Delete a redirect.
#[utoipa::path(
    delete, path = "/_emdash/api/redirects/{id}",
    tag = "redirects",
    params(("id" = String, Path, description = "Redirect ID")),
    responses((status = 200, description = "Success"))
)]
pub async fn delete_redirect(
    State(ctx): State<Arc<ServerContext>>,
    Path(id): Path<String>,
) -> Result<ApiEnvelope<Value>, ApiError> {
    ctx.db
        .execute(
            "DELETE FROM _emdash_redirects WHERE id = ?",
            vec![Value::String(id.clone())],
        )
        .await?;
    Ok(ApiEnvelope::new(serde_json::json!({ "deleted": id })))
}

/// List top 404 paths (for redirect suggestions).
#[utoipa::path(
    get, path = "/_emdash/api/redirects/not-found",
    tag = "redirects",
    responses((status = 200, description = "Success"))
)]
pub async fn list_not_found(
    State(ctx): State<Arc<ServerContext>>,
) -> Result<ApiEnvelope<Vec<Value>>, ApiError> {
    let rows = ctx
        .db
        .query(
            "SELECT * FROM _emdash_not_found ORDER BY hits DESC LIMIT 100",
            vec![],
        )
        .await?;
    Ok(ApiEnvelope::with_total(rows.clone(), rows.len() as u64))
}

/// Record a 404 hit (called by the site router).
pub async fn record_not_found(ctx: &Arc<ServerContext>, path: &str) {
    let _ = ctx
        .db
        .execute(
            "INSERT INTO _emdash_not_found (path, hits, last_seen) VALUES (?, 1, datetime('now')) \
             ON CONFLICT(path) DO UPDATE SET hits = hits + 1, last_seen = excluded.last_seen",
            vec![Value::String(path.to_string())],
        )
        .await;
}

pub fn router() -> Router<Arc<ServerContext>> {
    Router::new()
        .route(
            "/_emdash/api/redirects",
            get(list_redirects).post(create_redirect),
        )
        .route("/_emdash/api/redirects/not-found", get(list_not_found))
        .route("/_emdash/api/redirects/{id}", delete(delete_redirect))
}
