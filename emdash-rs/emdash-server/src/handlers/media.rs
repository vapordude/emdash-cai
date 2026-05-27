use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

use axum::{Router, routing::get};
use emdash_core::ApiError;

use super::common::ApiEnvelope;
use crate::ServerContext;

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MediaItem {
    pub id: String,
    pub filename: String,
    pub mime_type: String,
    pub size: i64,
    pub storage_path: String,
    pub alt: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateMediaBody {
    pub filename: String,
    pub mime_type: String,
    pub size: i64,
    pub storage_path: String,
    pub alt: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateMediaBody {
    pub alt: Option<String>,
}

/// List media library items.
#[utoipa::path(
    get, path = "/_emdash/api/media",
    tag = "media",
    responses((status = 200, description = "Success"))
)]
pub async fn list_media(
    State(ctx): State<Arc<ServerContext>>,
) -> Result<ApiEnvelope<Vec<Value>>, ApiError> {
    let rows = ctx.db.list("_emdash_media").await?;
    Ok(ApiEnvelope::with_total(rows.clone(), rows.len() as u64))
}

/// Get a media item.
#[utoipa::path(
    get, path = "/_emdash/api/media/{id}",
    tag = "media",
    params(("id" = String, Path, description = "Media ID")),
    responses(
        (status = 200, description = "Success"),
        (status = 404, body = ApiError),
    )
)]
pub async fn get_media(
    State(ctx): State<Arc<ServerContext>>,
    Path(id): Path<String>,
) -> Result<ApiEnvelope<Value>, ApiError> {
    let item = ctx
        .db
        .get_by_id("_emdash_media", &id)
        .await?
        .ok_or(ApiError::NotFound(id))?;
    Ok(ApiEnvelope::new(item))
}

/// Register a new media item (after upload via presigned URL).
#[utoipa::path(
    post, path = "/_emdash/api/media",
    tag = "media",
    request_body = CreateMediaBody,
    responses(
        (status = 201, description = "Success"),
        (status = 400, body = ApiError),
    )
)]
pub async fn create_media(
    State(ctx): State<Arc<ServerContext>>,
    Json(body): Json<CreateMediaBody>,
) -> Result<(axum::http::StatusCode, ApiEnvelope<Value>), ApiError> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    ctx.db
        .execute(
            "INSERT INTO _emdash_media (id, filename, mime_type, size, storage_path, alt, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            vec![
                Value::String(id.clone()),
                Value::String(body.filename),
                Value::String(body.mime_type),
                Value::Number(body.size.into()),
                Value::String(body.storage_path),
                body.alt.map(Value::String).unwrap_or(Value::Null),
                Value::String(now.clone()),
                Value::String(now),
            ],
        )
        .await?;
    let item = ctx
        .db
        .get_by_id("_emdash_media", &id)
        .await?
        .ok_or_else(|| ApiError::Internal("insert failed".into()))?;
    Ok((axum::http::StatusCode::CREATED, ApiEnvelope::new(item)))
}

/// Update media metadata.
#[utoipa::path(
    patch, path = "/_emdash/api/media/{id}",
    tag = "media",
    params(("id" = String, Path, description = "Media ID")),
    request_body = UpdateMediaBody,
    responses((status = 200, description = "Success"))
)]
pub async fn update_media(
    State(ctx): State<Arc<ServerContext>>,
    Path(id): Path<String>,
    Json(body): Json<UpdateMediaBody>,
) -> Result<ApiEnvelope<Value>, ApiError> {
    let now = Utc::now().to_rfc3339();
    if let Some(alt) = body.alt {
        ctx.db
            .execute(
                "UPDATE _emdash_media SET alt = ?, updated_at = ? WHERE id = ?",
                vec![
                    Value::String(alt),
                    Value::String(now),
                    Value::String(id.clone()),
                ],
            )
            .await?;
    }
    let item = ctx
        .db
        .get_by_id("_emdash_media", &id)
        .await?
        .ok_or(ApiError::NotFound(id))?;
    Ok(ApiEnvelope::new(item))
}

/// Delete a media item (also removes the file from storage).
#[utoipa::path(
    delete, path = "/_emdash/api/media/{id}",
    tag = "media",
    params(("id" = String, Path, description = "Media ID")),
    responses((status = 200, description = "Success"))
)]
pub async fn delete_media(
    State(ctx): State<Arc<ServerContext>>,
    Path(id): Path<String>,
) -> Result<ApiEnvelope<Value>, ApiError> {
    // Fetch storage path before deleting the record.
    if let Some(item) = ctx.db.get_by_id("_emdash_media", &id).await?
        && let Some(path) = item["storage_path"].as_str()
    {
        ctx.storage.delete_file(path).await.ok();
    }
    ctx.db
        .execute(
            "DELETE FROM _emdash_media WHERE id = ?",
            vec![Value::String(id.clone())],
        )
        .await?;
    Ok(ApiEnvelope::new(serde_json::json!({ "deleted": id })))
}

pub fn router() -> Router<Arc<ServerContext>> {
    Router::new()
        .route("/_emdash/api/media", get(list_media).post(create_media))
        .route(
            "/_emdash/api/media/{id}",
            get(get_media).patch(update_media).delete(delete_media),
        )
}
