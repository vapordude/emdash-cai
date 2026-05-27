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
pub struct Menu {
    pub id:       String,
    pub name:     String,
    pub title:    String,
    pub location: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MenuItem {
    pub id:         String,
    pub menu_id:    String,
    pub parent_id:  Option<String>,
    pub label:      String,
    pub url:        Option<String>,
    pub content_id: Option<String>,
    pub position:   i64,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateMenuBody {
    pub name:     String,
    pub title:    String,
    pub location: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateMenuItemBody {
    pub parent_id:  Option<String>,
    pub label:      String,
    pub url:        Option<String>,
    pub content_id: Option<String>,
    pub position:   Option<i64>,
}

/// List menus.
#[utoipa::path(
    get, path = "/_emdash/api/menus",
    tag = "menus",
    responses((status = 200, description = "Success"))
)]
pub async fn list_menus(
    State(ctx): State<Arc<ServerContext>>,
) -> Result<ApiEnvelope<Vec<Value>>, ApiError> {
    let rows = ctx.db.list("_emdash_menus").await?;
    Ok(ApiEnvelope::with_total(rows.clone(), rows.len() as u64))
}

/// Create a menu.
#[utoipa::path(
    post, path = "/_emdash/api/menus",
    tag = "menus",
    request_body = CreateMenuBody,
    responses((status = 201, description = "Success"))
)]
pub async fn create_menu(
    State(ctx): State<Arc<ServerContext>>,
    Json(body): Json<CreateMenuBody>,
) -> Result<(axum::http::StatusCode, ApiEnvelope<Value>), ApiError> {
    let id  = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    ctx.db
        .execute(
            "INSERT INTO _emdash_menus (id, name, title, location, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
            vec![
                Value::String(id.clone()),
                Value::String(body.name),
                Value::String(body.title),
                body.location.map(Value::String).unwrap_or(Value::Null),
                Value::String(now.clone()),
                Value::String(now),
            ],
        )
        .await?;
    let item = ctx.db.get_by_id("_emdash_menus", &id).await?
        .ok_or_else(|| ApiError::Internal("insert failed".into()))?;
    Ok((axum::http::StatusCode::CREATED, ApiEnvelope::new(item)))
}

/// Get a menu with its items.
#[utoipa::path(
    get, path = "/_emdash/api/menus/{id}",
    tag = "menus",
    params(("id" = String, Path, description = "Menu ID")),
    responses(
        (status = 200, description = "Success"),
        (status = 404, body = ApiError),
    )
)]
pub async fn get_menu(
    State(ctx): State<Arc<ServerContext>>,
    Path(id): Path<String>,
) -> Result<ApiEnvelope<Value>, ApiError> {
    let menu = ctx.db.get_by_id("_emdash_menus", &id).await?
        .ok_or_else(|| ApiError::NotFound(id.clone()))?;
    let items = ctx.db
        .query(
            "SELECT * FROM _emdash_menu_items WHERE menu_id = ? ORDER BY position",
            vec![Value::String(id)],
        )
        .await?;
    let mut result = menu;
    result["items"] = Value::Array(items);
    Ok(ApiEnvelope::new(result))
}

/// Delete a menu.
#[utoipa::path(
    delete, path = "/_emdash/api/menus/{id}",
    tag = "menus",
    params(("id" = String, Path, description = "Menu ID")),
    responses((status = 200, description = "Success"))
)]
pub async fn delete_menu(
    State(ctx): State<Arc<ServerContext>>,
    Path(id): Path<String>,
) -> Result<ApiEnvelope<Value>, ApiError> {
    ctx.db
        .execute("DELETE FROM _emdash_menus WHERE id = ?", vec![Value::String(id.clone())])
        .await?;
    Ok(ApiEnvelope::new(serde_json::json!({ "deleted": id })))
}

/// Add an item to a menu.
#[utoipa::path(
    post, path = "/_emdash/api/menus/{id}/items",
    tag = "menus",
    params(("id" = String, Path, description = "Menu ID")),
    request_body = CreateMenuItemBody,
    responses((status = 201, description = "Success"))
)]
pub async fn create_menu_item(
    State(ctx): State<Arc<ServerContext>>,
    Path(menu_id): Path<String>,
    Json(body): Json<CreateMenuItemBody>,
) -> Result<(axum::http::StatusCode, ApiEnvelope<Value>), ApiError> {
    let id  = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let pos = body.position.unwrap_or(0);
    ctx.db
        .execute(
            "INSERT INTO _emdash_menu_items \
             (id, menu_id, parent_id, label, url, content_id, position, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            vec![
                Value::String(id.clone()),
                Value::String(menu_id),
                body.parent_id.map(Value::String).unwrap_or(Value::Null),
                Value::String(body.label),
                body.url.map(Value::String).unwrap_or(Value::Null),
                body.content_id.map(Value::String).unwrap_or(Value::Null),
                Value::Number(pos.into()),
                Value::String(now.clone()),
                Value::String(now),
            ],
        )
        .await?;
    let item = ctx.db.get_by_id("_emdash_menu_items", &id).await?
        .ok_or_else(|| ApiError::Internal("insert failed".into()))?;
    Ok((axum::http::StatusCode::CREATED, ApiEnvelope::new(item)))
}

pub fn router() -> Router<Arc<ServerContext>> {
    Router::new()
        .route("/_emdash/api/menus", get(list_menus).post(create_menu))
        .route("/_emdash/api/menus/{id}", get(get_menu).delete(delete_menu))
        .route("/_emdash/api/menus/{id}/items", post(create_menu_item))
}
