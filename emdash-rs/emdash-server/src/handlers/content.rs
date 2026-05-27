use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use emdash_core::ApiError;
use axum::{Router, routing::{delete, get, patch, post}};

use super::common::{ApiEnvelope, PaginationMeta};
use crate::ServerContext;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContentItem {
    pub id:           String,
    pub collection:   String,
    pub status:       String,
    pub slug:         Option<String>,
    pub data:         serde_json::Value,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at:   DateTime<Utc>,
    pub updated_at:   DateTime<Utc>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateContentBody {
    pub collection: String,
    pub slug:       Option<String>,
    pub data:       serde_json::Value,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateContentBody {
    pub slug:   Option<String>,
    pub data:   Option<serde_json::Value>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub collection: String,
    pub status:     Option<String>,
    pub page:       Option<u32>,
    pub per_page:   Option<u32>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// List content items in a collection.
#[utoipa::path(
    get, path = "/_emdash/api/content",
    tag = "content",
    params(
        ("collection" = String, Query, description = "Collection name"),
        ("status"     = Option<String>, Query, description = "Filter by status"),
        ("page"       = Option<u32>,   Query, description = "Page number"),
        ("per_page"   = Option<u32>,   Query, description = "Items per page"),
    ),
    responses(
        (status = 200, body = inline(ApiEnvelope<Vec<ContentItem>>)),
        (status = 400, body = ApiError),
    )
)]
pub async fn list_content(
    State(ctx): State<Arc<ServerContext>>,
    Query(q): Query<ListQuery>,
) -> Result<ApiEnvelope<Vec<serde_json::Value>>, ApiError> {
    let table = format!("ec_{}", q.collection);
    let rows = ctx.db.list(&table).await?;
    let total = rows.len() as u64;
    Ok(ApiEnvelope::with_total(rows, total))
}

/// Get a single content item.
#[utoipa::path(
    get, path = "/_emdash/api/content/{id}",
    tag = "content",
    params(("id" = String, Path, description = "Content item ID")),
    responses(
        (status = 200, body = inline(ApiEnvelope<ContentItem>)),
        (status = 404, body = ApiError),
    )
)]
pub async fn get_content(
    State(ctx): State<Arc<ServerContext>>,
    Path(id): Path<String>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Result<ApiEnvelope<serde_json::Value>, ApiError> {
    let collection = q.get("collection").cloned().unwrap_or_default();
    let table = format!("ec_{collection}");
    let item = ctx.db.get_by_id(&table, &id).await?
        .ok_or_else(|| ApiError::NotFound(id.clone()))?;
    Ok(ApiEnvelope::new(item))
}

/// Create a content item.
#[utoipa::path(
    post, path = "/_emdash/api/content",
    tag = "content",
    request_body = CreateContentBody,
    responses(
        (status = 201, body = inline(ApiEnvelope<ContentItem>)),
        (status = 400, body = ApiError),
    )
)]
pub async fn create_content(
    State(ctx): State<Arc<ServerContext>>,
    Json(body): Json<CreateContentBody>,
) -> Result<(axum::http::StatusCode, ApiEnvelope<serde_json::Value>), ApiError> {
    let table = format!("ec_{}", body.collection);
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let data_str = body.data.to_string();

    ctx.db
        .execute(
            &format!(
                "INSERT INTO {table} (id, status, slug, data, created_at, updated_at) \
                 VALUES (?, 'draft', ?, ?, ?, ?)"
            ),
            vec![
                serde_json::Value::String(id.clone()),
                body.slug.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null),
                serde_json::Value::String(data_str),
                serde_json::Value::String(now.clone()),
                serde_json::Value::String(now),
            ],
        )
        .await?;

    let item = ctx.db.get_by_id(&table, &id).await?
        .ok_or_else(|| ApiError::Internal("insert failed".into()))?;

    Ok((axum::http::StatusCode::CREATED, ApiEnvelope::new(item)))
}

/// Update a content item.
#[utoipa::path(
    patch, path = "/_emdash/api/content/{id}",
    tag = "content",
    params(("id" = String, Path, description = "Content item ID")),
    request_body = UpdateContentBody,
    responses(
        (status = 200, body = inline(ApiEnvelope<ContentItem>)),
        (status = 404, body = ApiError),
    )
)]
pub async fn update_content(
    State(ctx): State<Arc<ServerContext>>,
    Path(id): Path<String>,
    Query(q): Query<std::collections::HashMap<String, String>>,
    Json(body): Json<UpdateContentBody>,
) -> Result<ApiEnvelope<serde_json::Value>, ApiError> {
    let collection = q.get("collection").cloned().unwrap_or_default();
    let table = format!("ec_{collection}");
    let now = Utc::now().to_rfc3339();

    let mut sets = vec!["updated_at = ?".to_string()];
    let mut params: Vec<serde_json::Value> = vec![serde_json::Value::String(now)];

    if let Some(slug) = body.slug {
        sets.push("slug = ?".into());
        params.push(serde_json::Value::String(slug));
    }
    if let Some(status) = body.status {
        sets.push("status = ?".into());
        params.push(serde_json::Value::String(status));
    }
    if let Some(data) = body.data {
        sets.push("data = ?".into());
        params.push(serde_json::Value::String(data.to_string()));
    }

    params.push(serde_json::Value::String(id.clone()));
    let sql = format!("UPDATE {table} SET {} WHERE id = ?", sets.join(", "));
    ctx.db.execute(&sql, params).await?;

    let item = ctx.db.get_by_id(&table, &id).await?
        .ok_or_else(|| ApiError::NotFound(id.clone()))?;
    Ok(ApiEnvelope::new(item))
}

/// Delete a content item (soft-delete — sets status to 'trashed').
#[utoipa::path(
    delete, path = "/_emdash/api/content/{id}",
    tag = "content",
    params(("id" = String, Path, description = "Content item ID")),
    responses(
        (status = 200, body = inline(ApiEnvelope<serde_json::Value>)),
        (status = 404, body = ApiError),
    )
)]
pub async fn delete_content(
    State(ctx): State<Arc<ServerContext>>,
    Path(id): Path<String>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Result<ApiEnvelope<serde_json::Value>, ApiError> {
    let collection = q.get("collection").cloned().unwrap_or_default();
    let table = format!("ec_{collection}");
    let now = Utc::now().to_rfc3339();
    ctx.db
        .execute(
            &format!("UPDATE {table} SET status = 'trashed', updated_at = ? WHERE id = ?"),
            vec![
                serde_json::Value::String(now),
                serde_json::Value::String(id.clone()),
            ],
        )
        .await?;
    Ok(ApiEnvelope::new(serde_json::json!({ "id": id, "status": "trashed" })))
}

/// Publish a content item.
#[utoipa::path(
    post, path = "/_emdash/api/content/{id}/publish",
    tag = "content",
    params(("id" = String, Path, description = "Content item ID")),
    responses((status = 200, body = inline(ApiEnvelope<serde_json::Value>)))
)]
pub async fn publish_content(
    State(ctx): State<Arc<ServerContext>>,
    Path(id): Path<String>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Result<ApiEnvelope<serde_json::Value>, ApiError> {
    let collection = q.get("collection").cloned().unwrap_or_default();
    let table = format!("ec_{collection}");
    let now = Utc::now().to_rfc3339();
    ctx.db
        .execute(
            &format!(
                "UPDATE {table} SET status = 'published', published_at = ?, updated_at = ? WHERE id = ?"
            ),
            vec![
                serde_json::Value::String(now.clone()),
                serde_json::Value::String(now),
                serde_json::Value::String(id.clone()),
            ],
        )
        .await?;
    let item = ctx.db.get_by_id(&table, &id).await?
        .ok_or_else(|| ApiError::NotFound(id.clone()))?;
    Ok(ApiEnvelope::new(item))
}

/// Unpublish a content item.
#[utoipa::path(
    post, path = "/_emdash/api/content/{id}/unpublish",
    tag = "content",
    params(("id" = String, Path, description = "Content item ID")),
    responses((status = 200, body = inline(ApiEnvelope<serde_json::Value>)))
)]
pub async fn unpublish_content(
    State(ctx): State<Arc<ServerContext>>,
    Path(id): Path<String>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Result<ApiEnvelope<serde_json::Value>, ApiError> {
    let collection = q.get("collection").cloned().unwrap_or_default();
    let table = format!("ec_{collection}");
    let now = Utc::now().to_rfc3339();
    ctx.db
        .execute(
            &format!("UPDATE {table} SET status = 'draft', updated_at = ? WHERE id = ?"),
            vec![
                serde_json::Value::String(now),
                serde_json::Value::String(id.clone()),
            ],
        )
        .await?;
    let item = ctx.db.get_by_id(&table, &id).await?
        .ok_or_else(|| ApiError::NotFound(id.clone()))?;
    Ok(ApiEnvelope::new(item))
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<ServerContext>> {
    Router::new()
        .route("/_emdash/api/content", get(list_content).post(create_content))
        .route("/_emdash/api/content/{id}", get(get_content).patch(update_content).delete(delete_content))
        .route("/_emdash/api/content/{id}/publish", post(publish_content))
        .route("/_emdash/api/content/{id}/unpublish", post(unpublish_content))
}
