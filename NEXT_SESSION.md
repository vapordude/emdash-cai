# Next Session Handoff

> Branch merged to `main` from `copilot/rust-end-to-end`.
> All 8 integration tests green. This doc is the starting point for the next work block.

---

## Where We Are

The `emdash-rs` workspace is a working Rust CMS backend with full REST API, real SQLite, LLM, and filesystem storage. It is **not** a stub anymore.

| Crate | State |
|---|---|
| `emdash-server` | Full axum router — 13 API handler groups + site routes + auth middleware + OpenAPI/Swagger |
| `emdash-db` | Real SQLite via sqlx — connection pool, migration runner, CRUD, SQL injection prevention |
| `emdash-llm` | Full OpenAI-compatible client — chat, embed, env-configured |
| `emdash-storage` | Real filesystem — get/put/delete/list with path traversal guard |
| `emdash-sandbox` | NoopRunner (default) + wasmtime `WasmPluginRunner` (feature-gated) |
| `emdash-core` | PortableText Block/Span AST, shared error/provider traits |
| `emdash-schema` | Collection + Field models |
| `emdash-cli` | Entry point wiring everything together |

Run tests: `cd emdash-rs && cargo test`

---

## Priority Order for Next Session

### 1. PortableText AST — `emdash-core/src/portable_text.rs`
Expand to full spec parity with `@portabletext/toolkit`:
- Typed list + list-item blocks
- Custom object blocks (`_type` dispatch)
- Complex marks and annotations
- Inline block support

### 2. Wasm Data ABI — `emdash-sandbox/src/lib.rs`
The `WasmPluginRunner` links and runs modules but data exchange is a stub.
- Allocate guest memory, write JSON payload, read result pointer
- Wire `content:afterSave` / `content:beforeDelete` lifecycle events from content handlers

### 3. WebAuthn / OAuth — `emdash-server/src/handlers/auth.rs`
Token-based auth is done. Add:
- WebAuthn passkey registration + assertion (use the `webauthn-rs` crate)
- OAuth2/OIDC (GitHub, Google) — `oauth2` crate
- Session cookie support for browser admin access

### 4. Admin UI — decision required
Pick one:
- **Hybrid (recommended to ship faster):** compile existing React admin, embed via `rust-embed`, serve from axum
- **Pure Rust:** Leptos or Dioxus (larger scope)

### 5. S3/R2 Storage Provider
Add behind the existing `StorageProvider` trait. `LocalStorage` is done; cloud is the gap.

---

## Key Files

```
emdash-rs/
  Cargo.toml                      # workspace manifest
  REMAINING_WORK.md               # detailed gap list (kept up to date)
  RUST_DESIGN.md                  # original architecture spec
  emdash-server/src/
    lib.rs                        # router + OpenAPI doc
    handlers/                     # one file per resource group
    middleware/auth.rs            # bearer token validation
    site/                         # feeds, sitemap, templates
  emdash-db/src/lib.rs            # BespokeDb (sqlx SQLite)
  emdash-db/migrations/001_init.sql
  emdash-llm/src/lib.rs           # OpenAiCompatProvider
  emdash-storage/src/lib.rs       # LocalStorage
  emdash-sandbox/src/lib.rs       # NoopRunner + WasmPluginRunner
  emdash-core/src/
    lib.rs                        # shared traits + error types
    portable_text.rs              # AST (needs expansion)
```

---

## Notes

- Auth is token-only for now — tokens are SHA-256 hashed in `_emdash_api_tokens`.
- The `wasmtime-sandbox` Cargo feature gates the real Wasm runner; disabled by default to keep compile times fast.
- LLM is configured entirely via env vars (`LLM_BASE_URL`, `LLM_API_KEY`, `LLM_MODEL`) — no platform lock-in.
- `REMAINING_WORK.md` is the detailed technical gap doc; this file is the session entry point.
