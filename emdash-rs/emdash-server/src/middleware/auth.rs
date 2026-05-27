use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use emdash_core::{ApiError, RequestContext};

use crate::ServerContext;

/// SHA-256 hex digest of `data`.
fn sha256_hex(data: &[u8]) -> String {
    let hash = Sha256::digest(data);
    format!("{hash:x}")
}

/// Axum middleware that validates Bearer tokens against `_emdash_api_tokens`.
///
/// On success it injects a [`RequestContext`] extension into the request so
/// downstream handlers can read the authenticated user/role.
/// On failure it returns 401 immediately.
pub async fn require_auth(
    State(ctx): State<Arc<ServerContext>>,
    mut req: Request,
    next: Next,
) -> Response {
    let token = match extract_bearer(req.headers()) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({
                    "code": "UNAUTHORIZED",
                    "message": "missing or invalid Authorization header"
                })),
            )
                .into_response();
        }
    };

    let hash = sha256_hex(token.as_bytes());

    match validate_token(&ctx, &hash).await {
        Ok(rctx) => {
            req.extensions_mut().insert(rctx);
            next.run(req).await
        }
        Err(_) => (
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({
                "code": "UNAUTHORIZED",
                "message": "invalid or expired API token"
            })),
        )
            .into_response(),
    }
}

/// Extract the raw token string from `Authorization: Bearer <token>`.
fn extract_bearer(headers: &axum::http::HeaderMap) -> Option<String> {
    let value = headers.get(axum::http::header::AUTHORIZATION)?.to_str().ok()?;
    let token = value.strip_prefix("Bearer ")?;
    if token.is_empty() { None } else { Some(token.to_string()) }
}

/// Look up the token hash in the DB and return a populated [`RequestContext`].
async fn validate_token(
    ctx: &Arc<ServerContext>,
    hash: &str,
) -> Result<RequestContext, ApiError> {
    let rows = ctx
        .db
        .query(
            "SELECT t.id, t.user_id, u.role \
             FROM _emdash_api_tokens t \
             JOIN _emdash_users u ON u.id = t.user_id \
             WHERE t.token_hash = ? \
             LIMIT 1",
            vec![Value::String(hash.to_string())],
        )
        .await?;

    let row = rows.into_iter().next().ok_or(ApiError::Unauthorized)?;

    // Update last_used_at asynchronously — fire-and-forget; ignore error.
    let now = chrono::Utc::now().to_rfc3339();
    let _ = ctx
        .db
        .execute(
            "UPDATE _emdash_api_tokens SET last_used_at = ? WHERE token_hash = ?",
            vec![
                Value::String(now),
                Value::String(hash.to_string()),
            ],
        )
        .await;

    let user_id = row["user_id"]
        .as_str()
        .and_then(|s| s.parse().ok());
    let user_role = row["role"].as_str().map(|s| s.to_string());

    Ok(RequestContext {
        user_id,
        user_role,
        ..RequestContext::default()
    })
}
