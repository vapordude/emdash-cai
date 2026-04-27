# EmDash Rust Port Design Document (emdash-rs)

## Overview
This document outlines the architectural design for porting the EmDash CMS (a TypeScript/Cloudflare-based CMS) into a pure Rust ecosystem. The goal is to build a high-performance, safe, and portable serverless-friendly framework using first-principles Rust, avoiding complex external dependencies where a minimal bespoke solution fits better.

## Core Pillars
1. **Web Framework:** Fast, ergonomic HTTP routing.
2. **Database & Schema:** Dynamic schema generation, SQLite/Postgres support.
3. **Plugin System:** Secure execution of untrusted third-party code.
4. **Admin UI:** Porting the React-based UI into Rust or a bundled hybrid approach.
5. **Content Representation:** Rust equivalent for Portable Text and the block system.

---

## 1. Web Framework
**Current Stack:** Astro + Cloudflare Pages/Workers + Hono/Vite middlewares.
**Rust Stack:** `axum` + `tower`

- **Why Axum?** It is the standard for high-performance Rust web backends. It integrates seamlessly with `tokio` and provides powerful type-safe extractors, which naturally map to EmDash's strict request/context handling.
- **Routing:** API routes (`/_emdash/api/*`) will be modeled as Axum routers. We will implement middleware to handle WebAuthn, OAuth, session management, and `RequestContext` injection.
- **Frontend Integration:** The resulting framework could run as a standalone server or compile via WebAssembly (if targeted properly) using something like `wasi-http` for Cloudflare Workers. However, natively, it will compile to a highly optimized binary deployable on standard container infrastructure (e.g., Docker, Fly.io).

## 2. Database and Schema Layer
**Current Stack:** SQLite (D1, libSQL) via `kysely` (TypeScript SQL query builder).
**Rust Stack:** `sqlx` (for static/typed query execution) + `sea-query` (for dynamic query building).

- **Dynamic Schema (`ec_*` tables):** EmDash generates database tables dynamically based on user-defined collections. `sea-query` provides an AST to dynamically construct `CREATE TABLE`, `ALTER TABLE`, `SELECT`, and `INSERT` statements.
- **Migrations:** EmDash has an embedded migration runner. We will write a custom migration runner using `sqlx` to apply pure SQL strings for system tables (`_emdash_*`).
- **Connection Pooling:** Abstracting the database connection pool so the system can run on SQLite locally, or Postgres in production via `sqlx`.

## 3. Plugin System and Sandbox
**Current Stack:** Cloudflare Dynamic Worker Loaders.
**Rust Stack:** `wasmtime` or `wasmer` + WebAssembly System Interface (WASI).

- **Why Wasm?** WebAssembly is the industry standard for executing untrusted code securely. Plugins will be compiled to `.wasm` modules.
- **Capabilities Manifest:** Just like the TypeScript version (`capabilities: ["read:content"]`), the Rust runtime will inspect a plugin manifest. Wasmtime allows configuring host functions. We will only expose host functions (e.g., `db_query`, `send_email`) to the plugin if the sandbox's config permits it.
- **Hooks:** EmDash triggers lifecycle hooks (`content:afterSave`). The main Axum app will hold a pool of pre-compiled Wasm modules. On an event, it will spin up a lightweight Wasmtime instance, inject the event data, and collect the result.

## 4. Content Representation (PortableText)
**Current Stack:** JSON-based `@portabletext/toolkit`.
**Rust Stack:** Custom AST via `serde`.

- **Implementation:** We will define deeply nested `enum`s and `struct`s to represent PortableText blocks.
- Example:
  ```rust
  #[derive(Serialize, Deserialize)]
  pub struct Block {
      _type: String,
      _key: String,
      children: Vec<Span>,
      style: Option<String>,
  }
  ```
- **Serialization:** `serde_json` will map these structs perfectly to the JSON columns in the SQLite database.

## 5. Admin UI
**Current Stack:** React, `@floating-ui/react`, `@tiptap/react` via Vite build.
**Rust Stack:** There are two approaches:
1. **Hybrid:** Compile the existing Vite/React app and embed it into the Rust binary using `rust-embed`. The Axum server just serves the static assets.
2. **Pure Rust:** Rewrite the admin dashboard using `Leptos` or `Dioxus`. These frameworks compile to WebAssembly for the browser and offer React-like reactivity. Given the complexity of TipTap (which relies heavily on Prosemirror DOM manipulation), a hybrid approach using embedded HTML/JS for the Rich Text Editor but Rust for the layout is advisable for the first iteration.

## 6. Architecture & Workspace Layout
The repository will be structured as a Cargo workspace to modularize concerns, similar to the `packages/` structure in the TypeScript monorepo.

- `emdash-core/`: Traits, PortableText AST, request context, and core business logic.
- `emdash-db/`: Database abstractions (`sqlx` + `sea-query`), dynamic schema registry.
- `emdash-schema/`: Validation, generation, Zod-equivalents via `validator` crate.
- `emdash-sandbox/`: The `wasmtime` runner for executing plugins safely.
- `emdash-server/`: The `axum` HTTP server wiring it all together.

## Conclusion
This architecture achieves full parity with EmDash while significantly lowering memory overhead and improving startup times by leveraging Rust's zero-cost abstractions and robust multithreading.
