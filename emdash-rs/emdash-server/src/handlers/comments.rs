use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

use axum::{
    Router,
    routing::{delete, get, patch},
};
use emdash_core::ApiError;

use super::common::ApiEnvelope;
use crate::ServerContext;

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Comment {
    pub id: String,
    pub content_id: String,
    pub table_name: String,
    pub parent_id: Option<String>,
    pub author_name: String,
    pub author_email: String,
    pub body: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateCommentBody {
    pub content_id: String,
    pub table_name: String,
    pub parent_id: Option<String>,
    pub author_name: String,
    pub author_email: String,
    pub body: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateCommentStatusBody {
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct CommentQuery {
    pub content_id: String,
    pub table_name: String,
    pub status: Option<String>,
}

/// List comments for a content item.
#[utoipa::path(
    get, path = "/_emdash/api/comments",
    tag = "comments",
    params(
        ("content_id" = String, Query, description = "Content item ID"),
        ("table_name" = String, Query, description = "Collection table"),
        ("status"     = Option<String>, Query, description = "Filter by status"),
    ),
    responses((status = 200, description = "Success"))
)]
pub async fn list_comments(
    State(ctx): State<Arc<ServerContext>>,
    Query(q): Query<CommentQuery>,
) -> Result<ApiEnvelope<Vec<Value>>, ApiError> {
    let (sql, params) = if let Some(status) = q.status {
        (
            "SELECT * FROM _emdash_comments WHERE content_id = ? AND table_name = ? AND status = ? \
             ORDER BY created_at",
            vec![
                Value::String(q.content_id),
                Value::String(q.table_name),
                Value::String(status),
            ],
        )
    } else {
        (
            "SELECT * FROM _emdash_comments WHERE content_id = ? AND table_name = ? \
             ORDER BY created_at",
            vec![Value::String(q.content_id), Value::String(q.table_name)],
        )
    };
    let rows = ctx.db.query(sql, params).await?;
    Ok(ApiEnvelope::with_total(rows.clone(), rows.len() as u64))
}

/// Create a comment (starts as 'pending').
#[utoipa::path(
    post, path = "/_emdash/api/comments",
    tag = "comments",
    request_body = CreateCommentBody,
    responses((status = 201, description = "Success"))
)]
pub async fn create_comment(
    State(ctx): State<Arc<ServerContext>>,
    Json(body): Json<CreateCommentBody>,
) -> Result<(axum::http::StatusCode, ApiEnvelope<Value>), ApiError> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    ctx.db
        .execute(
            "INSERT INTO _emdash_comments \
             (id, content_id, table_name, parent_id, author_name, author_email, body, status, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, 'pending', ?, ?)",
            vec![
                Value::String(id.clone()),
                Value::String(body.content_id),
                Value::String(body.table_name),
                body.parent_id.map(Value::String).unwrap_or(Value::Null),
                Value::String(body.author_name),
                Value::String(body.author_email),
                Value::String(body.body),
                Value::String(now.clone()),
                Value::String(now),
            ],
        )
        .await?;
    let item = ctx
        .db
        .get_by_id("_emdash_comments", &id)
        .await?
        .ok_or_else(|| ApiError::Internal("insert failed".into()))?;
    Ok((axum::http::StatusCode::CREATED, ApiEnvelope::new(item)))
}

/// Update comment status (approve / spam / etc.).
#[utoipa::path(
    patch, path = "/_emdash/api/comments/{id}/status",
    tag = "comments",
    params(("id" = String, Path, description = "Comment ID")),
    request_body = UpdateCommentStatusBody,
    responses((status = 200, description = "Success"))
)]
pub async fn update_comment_status(
    State(ctx): State<Arc<ServerContext>>,
    Path(id): Path<String>,
    Json(body): Json<UpdateCommentStatusBody>,
) -> Result<ApiEnvelope<Value>, ApiError> {
    let now = Utc::now().to_rfc3339();
    ctx.db
        .execute(
            "UPDATE _emdash_comments SET status = ?, updated_at = ? WHERE id = ?",
            vec![
                Value::String(body.status),
                Value::String(now),
                Value::String(id.clone()),
            ],
        )
        .await?;
    let item = ctx
        .db
        .get_by_id("_emdash_comments", &id)
        .await?
        .ok_or(ApiError::NotFound(id))?;
    Ok(ApiEnvelope::new(item))
}

/// Delete a comment.
#[utoipa::path(
    delete, path = "/_emdash/api/comments/{id}",
    tag = "comments",
    params(("id" = String, Path, description = "Comment ID")),
    responses((status = 200, description = "Success"))
)]
pub async fn delete_comment(
    State(ctx): State<Arc<ServerContext>>,
    Path(id): Path<String>,
) -> Result<ApiEnvelope<Value>, ApiError> {
    ctx.db
        .execute(
            "DELETE FROM _emdash_comments WHERE id = ?",
            vec![Value::String(id.clone())],
        )
        .await?;
    Ok(ApiEnvelope::new(serde_json::json!({ "deleted": id })))
}

pub fn router() -> Router<Arc<ServerContext>> {
    Router::new()
        .route(
            "/_emdash/api/comments",
            get(list_comments).post(create_comment),
        )
        .route(
            "/_emdash/api/comments/{id}/status",
            patch(update_comment_status),
        )
        .route("/_emdash/api/comments/{id}", delete(delete_comment))
}
