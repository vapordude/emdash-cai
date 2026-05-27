# emdash-rs

Rust implementation of the EmDash CMS — headless, agent-first, WordPress-parity.

## Workspace crates

| Crate | Purpose |
|---|---|
| `emdash-core` | Shared traits (`DatabaseProvider`, `StorageProvider`, `LlmProvider`) and types (`ApiError`, `PortableText`, `RequestContext`) |
| `emdash-db` | SQLite implementation via `sqlx` with migrations runner |
| `emdash-storage` | Local filesystem storage via `tokio::fs` |
| `emdash-llm` | OpenAI-compatible LLM provider (`/v1/chat/completions`, `/v1/embeddings`) |
| `emdash-sandbox` | Wasm plugin runner (`wasmtime` behind `wasmtime-sandbox` feature flag) |
| `emdash-schema` | Collection + field schema types |
| `emdash-server` | Axum HTTP server — all REST routes, OpenAPI 3.1, site rendering |
| `emdash-cli` | `emdash` binary — `serve`, `migrate`, `export`, `schema` subcommands |

## Quick start

```bash
# Run the server (SQLite file created automatically)
DATABASE_URL=emdash.db cargo run -p emdash-cli -- serve

# Dump the OpenAPI spec
cargo run -p emdash-cli -- schema

# Export static site
cargo run -p emdash-cli -- export --out ./dist
```

## Environment variables

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | `emdash.db` | SQLite file path (prefix `sqlite:` optional) |
| `STORAGE_PATH` | `storage` | Root directory for uploaded files |
| `LLM_BASE_URL` | `https://api.openai.com` | OpenAI-compatible endpoint |
| `LLM_API_KEY` | _(none)_ | API key (omit for local Ollama) |
| `LLM_MODEL` | `gpt-4o` | Model identifier |
| `PORT` | `3000` | HTTP listen port |
| `RUST_LOG` | `info` | Log level filter |

## API endpoints

The full machine-readable spec is available at `/_emdash/api/openapi.json`.
Swagger UI is at `/_emdash/docs`.

| Domain | Base path |
|---|---|
| Content | `/_emdash/api/content` |
| Schema | `/_emdash/api/schema/collections` |
| Media | `/_emdash/api/media` |
| Settings | `/_emdash/api/settings` |
| Taxonomies | `/_emdash/api/taxonomies` |
| Menus | `/_emdash/api/menus` |
| Redirects | `/_emdash/api/redirects` |
| Auth / tokens | `/_emdash/api/auth/tokens` |
| Revisions | `/_emdash/api/revisions` |
| Comments | `/_emdash/api/comments` |
| Plugins | `/_emdash/api/plugins` |
| Dashboard | `/_emdash/api/dashboard` |
| Agent manifest | `/_emdash/api/manifest` |
| Health | `/_emdash/health` |

Public WP-compatible read API: `/api/v1/{collection}`, `/api/v1/{collection}/{slug}`

## Building

```bash
cargo build --release          # default (SQLite + local storage)
cargo build --release --features wasmtime-sandbox   # enable Wasm plugins
```

## Testing

```bash
cargo test
cargo clippy --deny warnings
cargo fmt --check
```

## Feature flags

| Flag | Enables |
|---|---|
| `wasmtime-sandbox` | Wasmtime Wasm plugin runner (adds ~50 MB to binary) |

## Deployment

See `fly.toml` (Fly.io) and `railway.json` (Railway) in the repo root.
Single-binary + SQLite is the default deployment target — no external database required.
