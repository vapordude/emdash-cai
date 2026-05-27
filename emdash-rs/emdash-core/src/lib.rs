pub mod portable_text;

use async_trait::async_trait;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

// ── Error ────────────────────────────────────────────────────────────────────

/// Unified API error type.  Implements `axum::IntoResponse` so handlers can
/// return `Result<T, ApiError>` directly.
#[derive(Debug, Error, Serialize, utoipa::ToSchema)]
#[serde(tag = "code", content = "details", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApiError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden")]
    Forbidden,

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("unprocessable entity: {0}")]
    Unprocessable(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl ApiError {
    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Unprocessable(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// Wire shape of an error response body.
#[derive(Serialize, utoipa::ToSchema)]
struct ErrorBody<'a> {
    code: &'a str,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        let code = match &self {
            Self::NotFound(_) => "NOT_FOUND",
            Self::BadRequest(_) => "BAD_REQUEST",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Forbidden => "FORBIDDEN",
            Self::Conflict(_) => "CONFLICT",
            Self::Unprocessable(_) => "UNPROCESSABLE_ENTITY",
            Self::Internal(_) => "INTERNAL_ERROR",
        };
        let body = ErrorBody {
            code,
            message: self.to_string(),
        };
        (status, Json(body)).into_response()
    }
}

// ── Request context ──────────────────────────────────────────────────────────

/// Injected by auth middleware into every authenticated request.
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: Uuid,
    pub user_id: Option<Uuid>,
    pub user_role: Option<String>,
    pub locale: String,
    pub started_at: DateTime<Utc>,
}

impl Default for RequestContext {
    fn default() -> Self {
        Self {
            request_id: Uuid::new_v4(),
            user_id: None,
            user_role: None,
            locale: "en".into(),
            started_at: Utc::now(),
        }
    }
}

// ── Provider traits ──────────────────────────────────────────────────────────

#[async_trait]
pub trait StorageProvider: Send + Sync {
    async fn get_file(&self, path: &str) -> Result<Vec<u8>, ApiError>;
    async fn put_file(&self, path: &str, data: &[u8]) -> Result<(), ApiError>;
    async fn delete_file(&self, path: &str) -> Result<(), ApiError>;
    async fn list_files(&self, prefix: &str) -> Result<Vec<String>, ApiError>;
}

#[async_trait]
pub trait DatabaseProvider: Send + Sync {
    async fn query(
        &self,
        sql: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<Vec<serde_json::Value>, ApiError>;
    async fn execute(&self, sql: &str, params: Vec<serde_json::Value>) -> Result<u64, ApiError>;
    async fn get_by_id(&self, table: &str, id: &str)
    -> Result<Option<serde_json::Value>, ApiError>;
    /// Return all rows in `table` as JSON objects (unbounded — prefer `list_page`).
    async fn list(&self, table: &str) -> Result<Vec<serde_json::Value>, ApiError>;
    /// Return a page of rows from `table` ordered by `created_at DESC`.
    async fn list_page(
        &self,
        table: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<serde_json::Value>, ApiError>;
    /// Return the total row count for `table`.
    async fn count(&self, table: &str) -> Result<u64, ApiError>;
    /// Create a dynamic collection table (`ec_<name>`) with standard base columns.
    /// Extra column definitions are appended after the base set.
    async fn create_collection_table(
        &self,
        collection_name: &str,
        extra_columns: &[CollectionColumn],
    ) -> Result<(), ApiError>;
}

/// Portable column definition used by `DatabaseProvider::create_collection_table`.
/// Kept in `emdash-core` so all crates can reference it without depending on `emdash-db`.
#[derive(Debug, Clone)]
pub struct CollectionColumn {
    pub name: String,
    pub sql_type: &'static str,
    pub required: bool,
    pub unique: bool,
}

/// A single chat turn.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Options forwarded verbatim to the LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct LlmOptions {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stop: Option<Vec<String>>,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Single-prompt shorthand (wraps chat_completion internally).
    async fn generate_text(&self, prompt: &str, opts: LlmOptions) -> Result<String, ApiError>;

    /// Full multi-turn chat completion — primary agent interface.
    async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        opts: LlmOptions,
    ) -> Result<String, ApiError>;

    /// Dense embedding vector for semantic search / RAG.
    async fn embed(&self, input: &str) -> Result<Vec<f32>, ApiError>;
}
