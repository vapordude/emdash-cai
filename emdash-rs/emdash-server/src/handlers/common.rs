use axum::Json;
use axum::response::IntoResponse;
use serde::Serialize;

// ── Response envelope ─────────────────────────────────────────────────────────

/// Standard JSON envelope for every API response.
///
/// Agents can rely on this shape unconditionally:
/// `{ "data": <T>, "meta": { ... } }`
#[derive(Serialize, utoipa::ToSchema)]
pub struct ApiEnvelope<T: Serialize> {
    pub data: T,
    pub meta: PaginationMeta,
}

#[derive(Serialize, Default, utoipa::ToSchema)]
pub struct PaginationMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

impl<T: Serialize> ApiEnvelope<T> {
    pub fn new(data: T) -> Self {
        Self {
            data,
            meta: PaginationMeta::default(),
        }
    }

    pub fn with_total(data: T, total: u64) -> Self {
        Self {
            data,
            meta: PaginationMeta {
                total: Some(total),
                ..Default::default()
            },
        }
    }
}

impl<T: Serialize> IntoResponse for ApiEnvelope<T> {
    fn into_response(self) -> axum::response::Response {
        Json(self).into_response()
    }
}
