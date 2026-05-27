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
pub struct ApiToken {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub scopes: Vec<String>,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateApiTokenBody {
    pub user_id: String,
    pub name: String,
    pub scopes: Option<Vec<String>>,
}

/// List API tokens (token hashes are never returned).
#[utoipa::path(
    get, path = "/_emdash/api/auth/tokens",
    tag = "auth",
    responses((status = 200, description = "Success"))
)]
pub async fn list_api_tokens(
    State(ctx): State<Arc<ServerContext>>,
) -> Result<ApiEnvelope<Vec<Value>>, ApiError> {
    let rows = ctx
        .db
        .query(
            "SELECT id, user_id, name, scopes, created_at, last_used_at \
             FROM _emdash_api_tokens ORDER BY created_at DESC",
            vec![],
        )
        .await?;
    Ok(ApiEnvelope::with_total(rows.clone(), rows.len() as u64))
}

/// Create an API token.
///
/// The plaintext token is returned **once** and never stored — only a hash is persisted.
#[utoipa::path(
    post, path = "/_emdash/api/auth/tokens",
    tag = "auth",
    request_body = CreateApiTokenBody,
    responses(
        (status = 201, description = "Returns the plaintext token once — store it securely."),
    )
)]
pub async fn create_api_token(
    State(ctx): State<Arc<ServerContext>>,
    Json(body): Json<CreateApiTokenBody>,
) -> Result<(axum::http::StatusCode, ApiEnvelope<Value>), ApiError> {
    let id = Uuid::new_v4().to_string();
    let token = Uuid::new_v4().to_string().replace('-', "");
    let now = Utc::now().to_rfc3339();
    let scopes = serde_json::to_string(&body.scopes.unwrap_or_default())
        .unwrap_or_else(|_| "[]".to_string());

    // SHA-256 hash of the plaintext token stored in the DB.
    let hash = sha256_hex(token.as_bytes());

    ctx.db
        .execute(
            "INSERT INTO _emdash_api_tokens (id, user_id, name, token_hash, scopes, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
            vec![
                Value::String(id.clone()),
                Value::String(body.user_id),
                Value::String(body.name),
                Value::String(hash),
                Value::String(scopes),
                Value::String(now),
            ],
        )
        .await?;

    Ok((
        axum::http::StatusCode::CREATED,
        ApiEnvelope::new(serde_json::json!({
            "id":    id,
            "token": token,
        })),
    ))
}

/// Revoke an API token.
#[utoipa::path(
    delete, path = "/_emdash/api/auth/tokens/{id}",
    tag = "auth",
    params(("id" = String, Path, description = "Token ID")),
    responses((status = 200, description = "Success"))
)]
pub async fn revoke_api_token(
    State(ctx): State<Arc<ServerContext>>,
    Path(id): Path<String>,
) -> Result<ApiEnvelope<Value>, ApiError> {
    ctx.db
        .execute(
            "DELETE FROM _emdash_api_tokens WHERE id = ?",
            vec![Value::String(id.clone())],
        )
        .await?;
    Ok(ApiEnvelope::new(serde_json::json!({ "revoked": id })))
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(data);
    format!("{hash:x}")
}

pub fn router() -> Router<Arc<ServerContext>> {
    Router::new()
        .route(
            "/_emdash/api/auth/tokens",
            get(list_api_tokens).post(create_api_token),
        )
        .route("/_emdash/api/auth/tokens/{id}", delete(revoke_api_token))
}
