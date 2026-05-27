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
pub struct Taxonomy {
    pub id: String,
    pub name: String,
    pub title: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Term {
    pub id: String,
    pub taxonomy_id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub parent_id: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateTaxonomyBody {
    pub name: String,
    pub title: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateTermBody {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub parent_id: Option<String>,
}

// ── Taxonomy CRUD ─────────────────────────────────────────────────────────────

/// List taxonomies.
#[utoipa::path(
    get, path = "/_emdash/api/taxonomies",
    tag = "taxonomies",
    responses((status = 200, description = "Success"))
)]
pub async fn list_taxonomies(
    State(ctx): State<Arc<ServerContext>>,
) -> Result<ApiEnvelope<Vec<Value>>, ApiError> {
    let rows = ctx.db.list("_emdash_taxonomies").await?;
    Ok(ApiEnvelope::with_total(rows.clone(), rows.len() as u64))
}

/// Create a taxonomy.
#[utoipa::path(
    post, path = "/_emdash/api/taxonomies",
    tag = "taxonomies",
    request_body = CreateTaxonomyBody,
    responses((status = 201, description = "Success"))
)]
pub async fn create_taxonomy(
    State(ctx): State<Arc<ServerContext>>,
    Json(body): Json<CreateTaxonomyBody>,
) -> Result<(axum::http::StatusCode, ApiEnvelope<Value>), ApiError> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    ctx.db
        .execute(
            "INSERT INTO _emdash_taxonomies (id, name, title, description, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
            vec![
                Value::String(id.clone()),
                Value::String(body.name),
                Value::String(body.title),
                body.description.map(Value::String).unwrap_or(Value::Null),
                Value::String(now.clone()),
                Value::String(now),
            ],
        )
        .await?;
    let item = ctx
        .db
        .get_by_id("_emdash_taxonomies", &id)
        .await?
        .ok_or_else(|| ApiError::Internal("insert failed".into()))?;
    Ok((axum::http::StatusCode::CREATED, ApiEnvelope::new(item)))
}

/// Delete a taxonomy.
#[utoipa::path(
    delete, path = "/_emdash/api/taxonomies/{id}",
    tag = "taxonomies",
    params(("id" = String, Path, description = "Taxonomy ID")),
    responses((status = 200, description = "Success"))
)]
pub async fn delete_taxonomy(
    State(ctx): State<Arc<ServerContext>>,
    Path(id): Path<String>,
) -> Result<ApiEnvelope<Value>, ApiError> {
    ctx.db
        .execute(
            "DELETE FROM _emdash_taxonomies WHERE id = ?",
            vec![Value::String(id.clone())],
        )
        .await?;
    Ok(ApiEnvelope::new(serde_json::json!({ "deleted": id })))
}

// ── Term CRUD ─────────────────────────────────────────────────────────────────

/// List terms in a taxonomy.
#[utoipa::path(
    get, path = "/_emdash/api/taxonomies/{taxonomy_id}/terms",
    tag = "taxonomies",
    params(("taxonomy_id" = String, Path, description = "Taxonomy ID")),
    responses((status = 200, description = "Success"))
)]
pub async fn list_terms(
    State(ctx): State<Arc<ServerContext>>,
    Path(taxonomy_id): Path<String>,
) -> Result<ApiEnvelope<Vec<Value>>, ApiError> {
    let rows = ctx
        .db
        .query(
            "SELECT * FROM _emdash_terms WHERE taxonomy_id = ? ORDER BY name",
            vec![Value::String(taxonomy_id)],
        )
        .await?;
    Ok(ApiEnvelope::with_total(rows.clone(), rows.len() as u64))
}

/// Create a term.
#[utoipa::path(
    post, path = "/_emdash/api/taxonomies/{taxonomy_id}/terms",
    tag = "taxonomies",
    params(("taxonomy_id" = String, Path, description = "Taxonomy ID")),
    request_body = CreateTermBody,
    responses((status = 201, description = "Success"))
)]
pub async fn create_term(
    State(ctx): State<Arc<ServerContext>>,
    Path(taxonomy_id): Path<String>,
    Json(body): Json<CreateTermBody>,
) -> Result<(axum::http::StatusCode, ApiEnvelope<Value>), ApiError> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    ctx.db
        .execute(
            "INSERT INTO _emdash_terms (id, taxonomy_id, name, slug, description, parent_id, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            vec![
                Value::String(id.clone()),
                Value::String(taxonomy_id),
                Value::String(body.name),
                Value::String(body.slug),
                body.description.map(Value::String).unwrap_or(Value::Null),
                body.parent_id.map(Value::String).unwrap_or(Value::Null),
                Value::String(now.clone()),
                Value::String(now),
            ],
        )
        .await?;
    let item = ctx
        .db
        .get_by_id("_emdash_terms", &id)
        .await?
        .ok_or_else(|| ApiError::Internal("insert failed".into()))?;
    Ok((axum::http::StatusCode::CREATED, ApiEnvelope::new(item)))
}

pub fn router() -> Router<Arc<ServerContext>> {
    Router::new()
        .route(
            "/_emdash/api/taxonomies",
            get(list_taxonomies).post(create_taxonomy),
        )
        .route("/_emdash/api/taxonomies/{id}", delete(delete_taxonomy))
        .route(
            "/_emdash/api/taxonomies/{taxonomy_id}/terms",
            get(list_terms).post(create_term),
        )
}
