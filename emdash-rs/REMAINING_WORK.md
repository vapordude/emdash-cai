# Remaining Work for Feature Parity

This document outlines the gaps between the architectural vision detailed in `RUST_DESIGN.md` and the current stubbed implementations in the `emdash-rs` workspace. These items represent the remaining tasks required to achieve full feature parity with the legacy EmDash TypeScript implementation.

## 1. Web Framework (`emdash-server`)
*   **Current State:** Basic `axum` router with a single `/_emdash/health` check endpoint and state injection.
*   **Remaining Work:**
    *   Implement complete API routes (`/_emdash/api/*`) for dynamic collections and content management.
    *   Develop custom `axum` extractors and middlewares for:
        *   Authentication (WebAuthn, OAuth).
        *   Session management.
        *   `RequestContext` injection for isolated request processing.

## 2. Database and Schema Layer (`emdash-db`, `emdash-schema`)
*   **Current State:** Abstract `DatabaseProvider` trait, basic `Collection`/`Field` models, and rudimentary table generation logic (`create_table` string building) in `BespokeDb`. Methods return empty stubs.
*   **Remaining Work:**
    *   Implement actual connection pooling for SQLite and/or PostgreSQL in `BespokeDb` without relying on heavy ORMs.
    *   Implement full bespoke dynamic SQL query builder (CRUD operations based on `ec_*` tables).
    *   **Security Priority:** Ensure strict SQL identifier validation (e.g., limiting table/column names to alphanumeric chars and underscores) and proper quoting to prevent SQL injection in the bespoke dynamic SQL generator.
    *   Implement a lightweight migration runner for system tables (`_emdash_*`).
    *   Implement data persistence in `get_by_id`, `query`, and `execute` methods.

## 3. Plugin System and Sandbox (`emdash-sandbox`)
*   **Current State:** `PluginRunner` trait definition and `SandboxConfig` structure.
*   **Remaining Work:**
    *   Integrate a Wasm runtime (e.g., `wasmtime`).
    *   Implement Wasm module compilation, loading, and unloading.
    *   Implement host function exposure mapped securely based on `SandboxConfig` capabilities (e.g., `db_query`, `send_email`).
    *   Implement EmDash lifecycle hook triggers (e.g., `content:afterSave`) that spin up lightweight Wasm instances and pass data safely.

## 4. Universal AI and Storage (`emdash-llm`, `emdash-storage`)
*   **Current State:** `LlmProvider` and `StorageProvider` traits, with empty `OpenAiCompatProvider` and `LocalStorage` stubs.
*   **Remaining Work:**
    *   **LLM:** Implement HTTP requests in `OpenAiCompatProvider` to integrate with OpenAI-compatible endpoints, ensuring universal compatibility without platform-specific bindings.
    *   **Storage:** Implement actual filesystem operations in `LocalStorage` (`get_file`, `put_file`, `delete_file`).
    *   Potentially add cloud storage provider implementations.

## 5. Content Representation (`emdash-core`)
*   **Current State:** Basic PortableText `Block` and `Span` structs using `serde`.
*   **Remaining Work:**
    *   Expand the AST to fully support the PortableText specification, including lists, list items, custom object blocks, and complex marks, matching the parity of `@portabletext/toolkit`.

## 6. Admin UI
*   **Current State:** No frontend implementation present in the `emdash-rs` workspace.
*   **Remaining Work:**
    *   Decide between and implement either:
        1.  **Hybrid Approach:** Bundle a compiled React/Vite admin dashboard and serve it via `rust-embed` with `axum`.
        2.  **Pure Rust:** Rewrite the dashboard using WebAssembly-based UI frameworks like Leptos or Dioxus.
