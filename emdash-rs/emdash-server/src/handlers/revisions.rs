use axum::{Json, extract::{Path, Query, State}};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

use emdash_core::ApiError;
use axum::{Router, routing::get};

use super::common::ApiEnvelope;
use crate::ServerContext;

#[derive(Debug, Deserialize)]
pub struct RevisionQuery {
    pub content_id: String,
    pub table_name: String,
}

/// List revisions for a content item.
#[utoipa::path(
    get, path = "/_emdash/api/revisions",
    tag = "revisions",
    params(
        ("content_id" = String, Query, description = "Content item ID"),
        ("table_name" = String, Query, description = "Collection table (ec_xxx)"),
    ),
    responses((status = 200, description = "Success"))
)]
pub async fn list_revisions(
    State(ctx): State<Arc<ServerContext>>,
    Query(q): Query<RevisionQuery>,
) -> Result<ApiEnvelope<Vec<Value>>, ApiError> {
    let rows = ctx.db
        .query(
            "SELECT * FROM _emdash_revisions WHERE content_id = ? AND table_name = ? \
             ORDER BY created_at DESC",
            vec![
                Value::String(q.content_id),
                Value::String(q.table_name),
            ],
        )
        .await?;
    Ok(ApiEnvelope::with_total(rows.clone(), rows.len() as u64))
}

/// Get a specific revision.
#[utoipa::path(
    get, path = "/_emdash/api/revisions/{id}",
    tag = "revisions",
    params(("id" = String, Path, description = "Revision ID")),
    responses(
        (status = 200, description = "Success"),
        (status = 404, body = ApiError),
    )
)]
pub async fn get_revision(
    State(ctx): State<Arc<ServerContext>>,
    Path(id): Path<String>,
) -> Result<ApiEnvelope<Value>, ApiError> {
    let item = ctx.db.get_by_id("_emdash_revisions", &id).await?
        .ok_or_else(|| ApiError::NotFound(id))?;
    Ok(ApiEnvelope::new(item))
}

/// Helper: snapshot the current state of a content row as a revision.
pub async fn snapshot(
    ctx: &Arc<ServerContext>,
    table_name: &str,
    content_id: &str,
    created_by: Option<&str>,
) -> Result<(), ApiError> {
    let row = ctx.db.get_by_id(table_name, content_id).await?;
    if let Some(data) = row {
        let id  = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        ctx.db
            .execute(
                "INSERT INTO _emdash_revisions (id, content_id, table_name, data, created_by, created_at) \
                 VALUES (?, ?, ?, ?, ?, ?)",
                vec![
                    Value::String(id),
                    Value::String(content_id.into()),
                    Value::String(table_name.into()),
                    Value::String(data.to_string()),
                    created_by.map(|s| Value::String(s.into())).unwrap_or(Value::Null),
                    Value::String(now),
                ],
            )
            .await?;
    }
    Ok(())
}

pub fn router() -> Router<Arc<ServerContext>> {
    Router::new()
        .route("/_emdash/api/revisions", get(list_revisions))
        .route("/_emdash/api/revisions/{id}", get(get_revision))
}
