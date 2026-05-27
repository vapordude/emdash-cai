//! Integration tests for emdash-server.
//!
//! Spins up a real in-memory SQLite database + LocalStorage + NoopPluginRunner
//! and makes HTTP requests through the full axum router.

use std::sync::Arc;

use axum::{body::Body, http};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt; // for `oneshot`

use emdash_db::BespokeDb;
use emdash_sandbox::NoopPluginRunner;
use emdash_server::{ServerContext, create_router};
use emdash_storage::LocalStorage;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a fresh in-memory server context with an empty database.
async fn make_ctx() -> Arc<ServerContext> {
    let db = BespokeDb::connect("sqlite::memory:")
        .await
        .expect("in-memory db");
    let storage = LocalStorage::new(std::env::temp_dir().join("emdash-test-storage"));
    let llm = emdash_llm::OpenAiCompatProvider::from_env();

    Arc::new(ServerContext {
        db: Arc::new(db),
        storage: Arc::new(storage),
        llm: Arc::new(llm),
        plugin_runner: Arc::new(NoopPluginRunner),
    })
}

/// Issue a one-shot request through the router and return the parsed JSON body.
async fn request(
    ctx: Arc<ServerContext>,
    method: &str,
    uri: &str,
    body: Option<Value>,
    token: Option<&str>,
) -> (u16, Value) {
    let router = create_router(ctx);

    let mut builder = http::Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");

    if let Some(t) = token {
        builder = builder.header("authorization", format!("Bearer {t}"));
    }

    let body_bytes = match body {
        Some(v) => serde_json::to_vec(&v).unwrap(),
        None => vec![],
    };

    let req = builder.body(Body::from(body_bytes)).unwrap();

    let resp = router.oneshot(req).await.expect("router failed");
    let status = resp.status().as_u16();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

/// Create an API token and return its plaintext value.
async fn create_token(ctx: Arc<ServerContext>) -> String {
    // Seed a user first (tokens require a user_id FK).
    let user_id = uuid::Uuid::new_v4().to_string();
    ctx.db
        .execute(
            "INSERT INTO _emdash_users (id, email, name, role) VALUES (?, ?, ?, ?)",
            vec![
                Value::String(user_id.clone()),
                Value::String("test@example.com".into()),
                Value::String("Test User".into()),
                Value::String("admin".into()),
            ],
        )
        .await
        .unwrap();

    let token_plain = uuid::Uuid::new_v4().to_string().replace('-', "");
    let hash = sha256_hex(token_plain.as_bytes());
    let token_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    ctx.db
        .execute(
            "INSERT INTO _emdash_api_tokens \
             (id, user_id, name, token_hash, scopes, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
            vec![
                Value::String(token_id),
                Value::String(user_id),
                Value::String("test-token".into()),
                Value::String(hash),
                Value::String("[]".into()),
                Value::String(now),
            ],
        )
        .await
        .unwrap();

    token_plain
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(data);
    format!("{hash:x}")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn health_check_returns_ok() {
    let ctx = make_ctx().await;
    let (status, _) = request(ctx, "GET", "/_emdash/health", None, None).await;
    assert_eq!(status, 200);
}

#[tokio::test]
async fn openapi_json_is_public() {
    let ctx = make_ctx().await;
    let (status, body) = request(ctx, "GET", "/_emdash/api/openapi.json", None, None).await;
    assert_eq!(status, 200);
    assert!(body.get("openapi").is_some(), "should have openapi key");
}

#[tokio::test]
async fn protected_route_requires_auth() {
    let ctx = make_ctx().await;
    // No token → 401
    let (status, _) = request(ctx, "GET", "/_emdash/api/schema/collections", None, None).await;
    assert_eq!(status, 401);
}

#[tokio::test]
async fn protected_route_with_bad_token_is_rejected() {
    let ctx = make_ctx().await;
    let (status, _) = request(
        ctx,
        "GET",
        "/_emdash/api/schema/collections",
        None,
        Some("totally-wrong-token"),
    )
    .await;
    assert_eq!(status, 401);
}

#[tokio::test]
async fn create_and_list_collection() {
    let ctx = make_ctx().await;
    let token = create_token(ctx.clone()).await;
    let tok = Some(token.as_str());

    // Create collection
    let (status, body) = request(
        ctx.clone(),
        "POST",
        "/_emdash/api/schema/collections",
        Some(json!({ "name": "posts", "title": "Posts" })),
        tok,
    )
    .await;
    assert_eq!(status, 201, "create collection: {body}");

    // List collections
    let (status, body) = request(
        ctx.clone(),
        "GET",
        "/_emdash/api/schema/collections",
        None,
        tok,
    )
    .await;
    assert_eq!(status, 200, "list collections: {body}");
    let items = body["data"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["name"], "posts");
}

#[tokio::test]
async fn content_crud_lifecycle() {
    let ctx = make_ctx().await;
    let token = create_token(ctx.clone()).await;
    let tok = Some(token.as_str());

    // Create collection
    request(
        ctx.clone(),
        "POST",
        "/_emdash/api/schema/collections",
        Some(json!({ "name": "articles", "title": "Articles" })),
        tok,
    )
    .await;

    // Create content item
    let (status, body) = request(
        ctx.clone(),
        "POST",
        "/_emdash/api/content/articles",
        Some(json!({ "slug": "hello-world", "data": { "title": "Hello World" } })),
        tok,
    )
    .await;
    assert_eq!(status, 201, "create content: {body}");
    let id = body["data"]["id"].as_str().unwrap().to_string();

    // Get content item
    let (status, body) = request(
        ctx.clone(),
        "GET",
        &format!("/_emdash/api/content/articles/{id}"),
        None,
        tok,
    )
    .await;
    assert_eq!(status, 200, "get content: {body}");
    assert_eq!(body["data"]["id"], id);
    assert_eq!(body["data"]["status"], "draft");

    // List — should have 1 item with pagination meta
    let (status, body) = request(
        ctx.clone(),
        "GET",
        "/_emdash/api/content/articles",
        None,
        tok,
    )
    .await;
    assert_eq!(status, 200, "list content: {body}");
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(body["meta"]["total"], 1);
    assert_eq!(body["meta"]["page"], 1);

    // Update
    let (status, body) = request(
        ctx.clone(),
        "PATCH",
        &format!("/_emdash/api/content/articles/{id}"),
        Some(json!({ "data": { "title": "Updated" } })),
        tok,
    )
    .await;
    assert_eq!(status, 200, "update content: {body}");

    // Publish
    let (status, body) = request(
        ctx.clone(),
        "POST",
        &format!("/_emdash/api/content/articles/{id}/publish"),
        None,
        tok,
    )
    .await;
    assert_eq!(status, 200, "publish content: {body}");
    assert_eq!(body["data"]["status"], "published");

    // Unpublish
    let (status, body) = request(
        ctx.clone(),
        "POST",
        &format!("/_emdash/api/content/articles/{id}/unpublish"),
        None,
        tok,
    )
    .await;
    assert_eq!(status, 200, "unpublish content: {body}");
    assert_eq!(body["data"]["status"], "draft");

    // Delete (soft)
    let (status, body) = request(
        ctx.clone(),
        "DELETE",
        &format!("/_emdash/api/content/articles/{id}"),
        None,
        tok,
    )
    .await;
    assert_eq!(status, 200, "delete content: {body}");
    assert_eq!(body["data"]["status"], "trashed");
}

#[tokio::test]
async fn pagination_returns_correct_page() {
    let ctx = make_ctx().await;
    let token = create_token(ctx.clone()).await;
    let tok = Some(token.as_str());

    // Create collection
    request(
        ctx.clone(),
        "POST",
        "/_emdash/api/schema/collections",
        Some(json!({ "name": "pages", "title": "Pages" })),
        tok,
    )
    .await;

    // Insert 5 items
    for i in 0..5_u32 {
        request(
            ctx.clone(),
            "POST",
            "/_emdash/api/content/pages",
            Some(json!({ "slug": format!("page-{i}"), "data": { "n": i } })),
            tok,
        )
        .await;
    }

    // Page 1 with per_page=2
    let (status, body) = request(
        ctx.clone(),
        "GET",
        "/_emdash/api/content/pages?page=1&per_page=2",
        None,
        tok,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["data"].as_array().unwrap().len(), 2);
    assert_eq!(body["meta"]["total"], 5);
    assert_eq!(body["meta"]["per_page"], 2);

    // Page 3 — only 1 item left
    let (status, body) = request(
        ctx.clone(),
        "GET",
        "/_emdash/api/content/pages?page=3&per_page=2",
        None,
        tok,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn invalid_collection_name_is_rejected() {
    let ctx = make_ctx().await;
    let token = create_token(ctx.clone()).await;
    let tok = Some(token.as_str());

    let (status, _) = request(
        ctx.clone(),
        "GET",
        "/_emdash/api/content/bad-name!",
        None,
        tok,
    )
    .await;
    assert_eq!(status, 400);
}
