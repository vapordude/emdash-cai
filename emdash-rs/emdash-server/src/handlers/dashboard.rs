use axum::extract::State;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use axum::{Router, routing::get};
use emdash_core::ApiError;

use super::common::ApiEnvelope;
use crate::ServerContext;

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DashboardStats {
    pub total_content: i64,
    pub published_content: i64,
    pub draft_content: i64,
    pub total_media: i64,
    pub total_comments: i64,
    pub pending_comments: i64,
    pub total_collections: i64,
    pub loaded_plugins: usize,
}

/// Aggregate site statistics for the admin dashboard.
#[utoipa::path(
    get, path = "/_emdash/api/dashboard",
    tag = "dashboard",
    responses((status = 200, body = inline(ApiEnvelope<DashboardStats>)))
)]
pub async fn get_dashboard(
    State(ctx): State<Arc<ServerContext>>,
) -> Result<ApiEnvelope<DashboardStats>, ApiError> {
    let count = |sql: &str| {
        let ctx = ctx.clone();
        let sql = sql.to_string();
        async move {
            ctx.db
                .query(&sql, vec![])
                .await
                .ok()
                .and_then(|rows| rows.into_iter().next())
                .and_then(|row| row["count"].as_i64().or_else(|| row["COUNT(*)"].as_i64()))
                .unwrap_or(0)
        }
    };

    // Collect counts for each collection's published / draft items.
    let collections = ctx.db.list("_emdash_collections").await?;
    let collection_count = collections.len() as i64;

    let mut total_content = 0i64;
    let mut published_content = 0i64;
    let mut draft_content = 0i64;

    for col in &collections {
        if let Some(name) = col["name"].as_str() {
            let all_sql = format!("SELECT COUNT(*) as count FROM ec_{name}");
            let pub_sql =
                format!("SELECT COUNT(*) as count FROM ec_{name} WHERE status = 'published'");
            let drft_sql =
                format!("SELECT COUNT(*) as count FROM ec_{name} WHERE status = 'draft'");

            total_content += count(&all_sql).await;
            published_content += count(&pub_sql).await;
            draft_content += count(&drft_sql).await;
        }
    }

    let total_media = count("SELECT COUNT(*) as count FROM _emdash_media").await;
    let total_comments = count("SELECT COUNT(*) as count FROM _emdash_comments").await;
    let pending_comments =
        count("SELECT COUNT(*) as count FROM _emdash_comments WHERE status = 'pending'").await;
    let loaded_plugins = ctx.plugin_runner.loaded_plugins().await.len();

    Ok(ApiEnvelope::new(DashboardStats {
        total_content,
        published_content,
        draft_content,
        total_media,
        total_comments,
        pending_comments,
        total_collections: collection_count,
        loaded_plugins,
    }))
}

pub fn router() -> Router<Arc<ServerContext>> {
    Router::new().route("/_emdash/api/dashboard", get(get_dashboard))
}
