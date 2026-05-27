use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

use axum::{Router, routing::get};
use emdash_core::ApiError;

use super::common::ApiEnvelope;
use crate::ServerContext;

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SettingItem {
    pub key: String,
    pub value: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpsertSettingBody {
    pub value: String,
}

/// List all settings.
#[utoipa::path(
    get, path = "/_emdash/api/settings",
    tag = "settings",
    responses((status = 200, description = "Success"))
)]
pub async fn list_settings(
    State(ctx): State<Arc<ServerContext>>,
) -> Result<ApiEnvelope<Vec<Value>>, ApiError> {
    let rows = ctx.db.list("_emdash_settings").await?;
    Ok(ApiEnvelope::with_total(rows.clone(), rows.len() as u64))
}

/// Get a single setting by key.
#[utoipa::path(
    get, path = "/_emdash/api/settings/{key}",
    tag = "settings",
    params(("key" = String, Path, description = "Setting key")),
    responses(
        (status = 200, description = "Success"),
        (status = 404, body = ApiError),
    )
)]
pub async fn get_setting(
    State(ctx): State<Arc<ServerContext>>,
    Path(key): Path<String>,
) -> Result<ApiEnvelope<Value>, ApiError> {
    let rows = ctx
        .db
        .query(
            "SELECT * FROM _emdash_settings WHERE key = ?",
            vec![Value::String(key.clone())],
        )
        .await?;
    let item = rows.into_iter().next().ok_or(ApiError::NotFound(key))?;
    Ok(ApiEnvelope::new(item))
}

/// Upsert a setting.
#[utoipa::path(
    put, path = "/_emdash/api/settings/{key}",
    tag = "settings",
    params(("key" = String, Path, description = "Setting key")),
    request_body = UpsertSettingBody,
    responses((status = 200, description = "Success"))
)]
pub async fn upsert_setting(
    State(ctx): State<Arc<ServerContext>>,
    Path(key): Path<String>,
    Json(body): Json<UpsertSettingBody>,
) -> Result<ApiEnvelope<Value>, ApiError> {
    let now = Utc::now().to_rfc3339();
    ctx.db
        .execute(
            "INSERT INTO _emdash_settings (key, value, updated_at) VALUES (?, ?, ?) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            vec![
                Value::String(key.clone()),
                Value::String(body.value),
                Value::String(now),
            ],
        )
        .await?;
    let rows = ctx
        .db
        .query(
            "SELECT * FROM _emdash_settings WHERE key = ?",
            vec![Value::String(key)],
        )
        .await?;
    Ok(ApiEnvelope::new(
        rows.into_iter().next().unwrap_or(Value::Null),
    ))
}

pub fn router() -> Router<Arc<ServerContext>> {
    Router::new()
        .route("/_emdash/api/settings", get(list_settings))
        .route(
            "/_emdash/api/settings/{key}",
            get(get_setting).put(upsert_setting),
        )
}
