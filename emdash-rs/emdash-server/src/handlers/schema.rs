use axum::{Json, extract::{Path, State}};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

use emdash_core::ApiError;
use emdash_db::{BespokeDb, ColumnDef, ColumnType};
use emdash_schema::{Collection, Field, FieldType};
use axum::{Router, routing::{delete, get, post}};

use super::common::ApiEnvelope;
use crate::ServerContext;

// ── Request bodies ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateCollectionBody {
    pub name:        String,
    pub title:       String,
    pub description: Option<String>,
    pub is_feed:     Option<bool>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateFieldBody {
    pub name:       String,
    pub title:      String,
    pub field_type: FieldType,
    pub required:   Option<bool>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn field_type_to_col(ft: &FieldType) -> ColumnType {
    match ft {
        FieldType::Number    => ColumnType::Float,
        FieldType::Boolean   => ColumnType::Boolean,
        FieldType::Date | FieldType::Datetime => ColumnType::Timestamp,
        _                    => ColumnType::Text,
    }
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// List all collections.
#[utoipa::path(
    get, path = "/_emdash/api/schema/collections",
    tag = "schema",
    responses((status = 200, description = "Success"))
)]
pub async fn list_collections(
    State(ctx): State<Arc<ServerContext>>,
) -> Result<ApiEnvelope<Vec<Value>>, ApiError> {
    let rows = ctx.db.list("_emdash_collections").await?;
    Ok(ApiEnvelope::with_total(rows.clone(), rows.len() as u64))
}

/// Get a single collection with its fields.
#[utoipa::path(
    get, path = "/_emdash/api/schema/collections/{name}",
    tag = "schema",
    params(("name" = String, Path, description = "Collection machine name")),
    responses(
        (status = 200, description = "Success"),
        (status = 404, body = ApiError),
    )
)]
pub async fn get_collection(
    State(ctx): State<Arc<ServerContext>>,
    Path(name): Path<String>,
) -> Result<ApiEnvelope<Value>, ApiError> {
    let rows = ctx.db
        .query(
            "SELECT * FROM _emdash_collections WHERE name = ?",
            vec![Value::String(name.clone())],
        )
        .await?;
    let col = rows.into_iter().next()
        .ok_or_else(|| ApiError::NotFound(name))?;
    Ok(ApiEnvelope::new(col))
}

/// Create a new collection and its backing database table.
#[utoipa::path(
    post, path = "/_emdash/api/schema/collections",
    tag = "schema",
    request_body = CreateCollectionBody,
    responses(
        (status = 201, description = "Success"),
        (status = 400, body = ApiError),
        (status = 409, body = ApiError),
    )
)]
pub async fn create_collection(
    State(ctx): State<Arc<ServerContext>>,
    Json(body): Json<CreateCollectionBody>,
) -> Result<(axum::http::StatusCode, ApiEnvelope<Value>), ApiError> {
    emdash_db::validate_identifier(&body.name)?;

    let id  = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let is_feed = body.is_feed.unwrap_or(false) as i64;

    ctx.db
        .execute(
            "INSERT INTO _emdash_collections (id, name, title, description, is_feed, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            vec![
                Value::String(id.clone()),
                Value::String(body.name.clone()),
                Value::String(body.title),
                body.description.map(Value::String).unwrap_or(Value::Null),
                Value::Number(is_feed.into()),
                Value::String(now.clone()),
                Value::String(now),
            ],
        )
        .await
        .map_err(|_| ApiError::Conflict(format!("collection '{}' already exists", body.name)))?;

    // Create the backing `ec_<name>` table with standard columns.
    ctx.db
        .execute(
            &format!(
                "CREATE TABLE IF NOT EXISTS ec_{name} (\
                    id TEXT PRIMARY KEY, \
                    status TEXT NOT NULL DEFAULT 'draft', \
                    slug TEXT, \
                    data TEXT, \
                    published_at TEXT, \
                    created_at TEXT NOT NULL DEFAULT (datetime('now')), \
                    updated_at TEXT NOT NULL DEFAULT (datetime('now'))\
                )",
                name = body.name
            ),
            vec![],
        )
        .await?;

    let item = ctx.db.get_by_id("_emdash_collections", &id).await?
        .ok_or_else(|| ApiError::Internal("insert failed".into()))?;

    Ok((axum::http::StatusCode::CREATED, ApiEnvelope::new(item)))
}

/// Delete a collection (and its backing table).
#[utoipa::path(
    delete, path = "/_emdash/api/schema/collections/{name}",
    tag = "schema",
    params(("name" = String, Path, description = "Collection machine name")),
    responses(
        (status = 200, description = "Success"),
        (status = 404, body = ApiError),
    )
)]
pub async fn delete_collection(
    State(ctx): State<Arc<ServerContext>>,
    Path(name): Path<String>,
) -> Result<ApiEnvelope<Value>, ApiError> {
    emdash_db::validate_identifier(&name)?;
    ctx.db
        .execute(
            "DELETE FROM _emdash_collections WHERE name = ?",
            vec![Value::String(name.clone())],
        )
        .await?;
    ctx.db
        .execute(&format!("DROP TABLE IF EXISTS ec_{name}"), vec![])
        .await?;
    Ok(ApiEnvelope::new(serde_json::json!({ "deleted": name })))
}

/// List fields for a collection.
#[utoipa::path(
    get, path = "/_emdash/api/schema/collections/{name}/fields",
    tag = "schema",
    params(("name" = String, Path, description = "Collection machine name")),
    responses((status = 200, description = "Success"))
)]
pub async fn list_fields(
    State(ctx): State<Arc<ServerContext>>,
    Path(name): Path<String>,
) -> Result<ApiEnvelope<Vec<Value>>, ApiError> {
    let rows = ctx.db
        .query(
            "SELECT f.* FROM _emdash_fields f \
             JOIN _emdash_collections c ON c.id = f.collection_id \
             WHERE c.name = ? ORDER BY f.position",
            vec![Value::String(name)],
        )
        .await?;
    Ok(ApiEnvelope::with_total(rows.clone(), rows.len() as u64))
}

/// Add a field to a collection (also adds the column to the live table).
#[utoipa::path(
    post, path = "/_emdash/api/schema/collections/{name}/fields",
    tag = "schema",
    params(("name" = String, Path, description = "Collection machine name")),
    request_body = CreateFieldBody,
    responses(
        (status = 201, description = "Success"),
        (status = 400, body = ApiError),
    )
)]
pub async fn create_field(
    State(ctx): State<Arc<ServerContext>>,
    Path(name): Path<String>,
    Json(body): Json<CreateFieldBody>,
) -> Result<(axum::http::StatusCode, ApiEnvelope<Value>), ApiError> {
    emdash_db::validate_identifier(&body.name)?;
    emdash_db::validate_identifier(&name)?;

    // Resolve the collection ID.
    let cols = ctx.db
        .query(
            "SELECT id FROM _emdash_collections WHERE name = ?",
            vec![Value::String(name.clone())],
        )
        .await?;
    let col = cols.into_iter().next()
        .ok_or_else(|| ApiError::NotFound(name.clone()))?;
    let collection_id = col["id"]
        .as_str()
        .ok_or_else(|| ApiError::Internal("missing id".into()))?
        .to_string();

    let id       = Uuid::new_v4().to_string();
    let now      = Utc::now().to_rfc3339();
    let ft_json  = serde_json::to_string(&body.field_type)
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let required = body.required.unwrap_or(false) as i64;

    ctx.db
        .execute(
            "INSERT INTO _emdash_fields \
             (id, collection_id, name, title, field_type, required, position, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, \
               (SELECT COALESCE(MAX(position), 0) + 1 FROM _emdash_fields WHERE collection_id = ?), \
               ?, ?)",
            vec![
                Value::String(id.clone()),
                Value::String(collection_id.clone()),
                Value::String(body.name.clone()),
                Value::String(body.title),
                Value::String(ft_json),
                Value::Number(required.into()),
                Value::String(collection_id.clone()),
                Value::String(now.clone()),
                Value::String(now),
            ],
        )
        .await?;

    // ALTER TABLE to add the column to the live collection table.
    let col_type = field_type_to_col(&body.field_type);
    ctx.db
        .execute(
            &format!(
                "ALTER TABLE ec_{name} ADD COLUMN {} {}",
                body.name, col_type.to_sql()
            ),
            vec![],
        )
        .await
        .ok(); // Ignore error if column already exists (SQLite doesn't have IF NOT EXISTS for ALTER)

    let item = ctx.db.get_by_id("_emdash_fields", &id).await?
        .ok_or_else(|| ApiError::Internal("insert failed".into()))?;

    Ok((axum::http::StatusCode::CREATED, ApiEnvelope::new(item)))
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<ServerContext>> {
    Router::new()
        .route("/_emdash/api/schema/collections", get(list_collections).post(create_collection))
        .route("/_emdash/api/schema/collections/{name}", get(get_collection).delete(delete_collection))
        .route("/_emdash/api/schema/collections/{name}/fields", get(list_fields).post(create_field))
}
