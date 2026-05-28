# Remaining Work for Feature Parity

This document tracks gaps between `RUST_DESIGN.md` and the current `emdash-rs` workspace.
Updated to reflect actual implementation state as of 2026-05-28.

---

## What Is Already Done ✅

| Area | Status |
|---|---|
| **`emdash-server`** | Full axum router — all 13 handler groups wired (content, schema, media, settings, taxonomies, menus, redirects, auth, revisions, comments, plugins, dashboard, agent) + site routes (feeds, sitemap, templates) |
| **`emdash-db`** | Real SQLite via `sqlx` with connection pooling, migration runner (`001_init.sql`), full `query`/`execute` CRUD, identifier validation against SQL injection |
| **`emdash-llm`** | Full OpenAI-compatible HTTP client — `generate_text`, `chat_completion`, `embed` all implemented via `reqwest`; reads `LLM_BASE_URL` / `LLM_API_KEY` / `LLM_MODEL` from env |
| **`emdash-storage`** | Real filesystem (`LocalStorage`) — `get_file`, `put_file`, `delete_file`, `list_files`; path traversal protection included |
| **`emdash-sandbox`** | `NoopPluginRunner` (always compiled) + feature-gated `WasmPluginRunner` via `wasmtime`; capability-based host function linking (`db_query`, `db_execute`) |
| **Auth middleware** | ****** validation against `_emdash_api_tokens` (SHA-256 hash), injects `RequestContext` into request extensions |
| **OpenAPI / Swagger UI** | `/_emdash/api/openapi.json` + `/_emdash/docs` Swagger UI served directly |
| **Integration tests** | 8/8 passing — health, collection CRUD, content lifecycle, auth, pagination, OpenAPI visibility |

---

## What Remains 🔲

### 1. PortableText AST (`emdash-core`)
*   **Current State:** Basic `Block` and `Span` structs.
*   **Remaining:**
    *   Expand to full PortableText spec — typed list/list-item blocks, custom object blocks,
        complex marks (annotations), nested spans, inline block support.
    *   Target parity with `@portabletext/toolkit`.

### 2. Wasm Data ABI (`emdash-sandbox`)
*   **Current State:** `WasmPluginRunner` compiles and runs Wasm modules; host functions are
    linked and capability-gated, but data exchange with the guest is a stub (payload returned unchanged).
*   **Remaining:**
    *   Implement full data exchange via Wasm linear memory (allocate, write JSON, call hook,
        read result pointer).
    *   Wire lifecycle hooks (`content:afterSave`, `content:beforeDelete`, etc.) to fire from
        the relevant DB/content handler paths.

### 3. WebAuthn / OAuth (`emdash-server`)
*   **Current State:** Token-based auth only (API keys).
*   **Remaining:**
    *   Add WebAuthn passkey registration + assertion flow.
    *   Add OAuth2/OIDC provider support (GitHub, Google, etc.).
    *   Session cookie management for browser-based admin access.

### 4. Admin UI
*   **Current State:** None. API is fully machine-readable and Swagger UI is served.
*   **Decision needed — pick one:**
    1.  **Hybrid:** Compile the existing React/Vite admin panel, embed via `rust-embed`, serve from axum.
    2.  **Pure Rust:** Build with Leptos or Dioxus (larger scope, no JS build step).

### 5. Cloud Storage Provider
*   `LocalStorage` is complete. Add S3-compatible (or R2) provider behind the same `StorageProvider` trait for production deployments.
