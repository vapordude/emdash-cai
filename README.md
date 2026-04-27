# EmDash

A pure Rust serverless CMS. EmDash takes the ideas that made WordPress dominant -- extensibility, admin UX, a plugin ecosystem -- and rebuilds them using first-principles Rust for performance, safety, and universal portability. Plugins run in sandboxed WebAssembly execution environments.

## Get Started

EmDash is distributed as a Rust crate and Cargo workspace.

```bash
git clone https://github.com/emdash-cms/emdash.git && cd emdash/emdash-rs
cargo build
```

EmDash runs natively anywhere Rust runs. Because it depends entirely on async traits (`DatabaseProvider`, `StorageProvider`, `LlmProvider`, `PluginRunner`), you can supply bespoke implementations to map it to any database or deployment platform.

## Why EmDash?

**WordPress was built for a different era.** Running WordPress today means managing PHP alongside JavaScript, layering caches to get acceptable performance, and knowing that [96% of WordPress security vulnerabilities come from plugins](https://patchstack.com/whitepaper/state-of-wordpress-security-in-2024/). EmDash is what WordPress would look like if you started from scratch with today's tools.

**Universal OpenAI-Compatible AI Endpoints.** Moving beyond vendor lock-in, EmDash plugins now leverage universal, open OpenAI-compatible models for auditing, moderation, and embeddings. You are no longer tied specifically to Cloudflare Workers AI — just configure your API key and base URL, and run inference everywhere.

**Sandboxed plugins.** WordPress plugins have full access to the database, filesystem, and user data. A single vulnerable plugin can compromise the entire site. EmDash plugins run in isolated Worker sandboxes, each with a declared capability manifest. A plugin that requests `read:content` and `email:send` can do exactly that and nothing else.

```typescript
export default () =>
	definePlugin({
		id: "notify-on-publish",
		capabilities: ["read:content", "email:send"],
		hooks: {
			"content:afterSave": async (event, ctx) => {
				if (event.content.status !== "published") return;
				await ctx.email.send({
					to: "editors@example.com",
					subject: `New post: ${event.content.title}`,
				});
			},
		},
	});
```

**Structured content, not serialized HTML.** WordPress stores rich text as HTML with metadata embedded in comments -- tying your content to its DOM representation. EmDash uses [Portable Text](https://www.portabletext.org/), a structured JSON format that decouples content from presentation. Your content can render as a web page, a mobile app, an email, or an API response without parsing HTML.

**Built for agents.** EmDash ships with agent skills for building plugins and themes, a CLI that lets agents manage content and schema programmatically, and a built-in [MCP server](https://modelcontextprotocol.io/) so AI tools like Claude and ChatGPT can interact with your site directly.

**Runs anywhere.** EmDash uses portable abstractions at every layer that work with SQLite, D1, Turso, PostgreSQL, R2, AWS S3, or local files. It is designed to run natively in Rust on any server or serverless environment.

## Architecture

EmDash is built on a highly modular Rust `cargo` workspace:

- **Core Abstractions:** Business logic depends purely on async traits in `emdash-core` (`DatabaseProvider`, `LlmProvider`, `StorageProvider`, `PluginRunner`).
- **Web Framework:** Uses `axum` and `tokio` for ergonomic and highly performant HTTP routing.
- **Wasm Plugins:** Uses WebAssembly to provide secure sandboxing and lifecycle hook executions for untrusted plugins.
- **Universal LLM integration:** Abstracting AI generation behind an `OpenAICompatProvider` prevents vendor lock-in.

Content types are defined natively in the database via bespoke SQL generation based on dynamic `ColumnDef` parsing. Non-developers will be able to create and modify collections dynamically, and the `emdash-db` crate will securely generate matching SQL definitions.

## Features

**Content** -- Blog posts, pages, custom content types. Rich text editing via TipTap with Portable Text storage. Revisions, drafts, scheduled publishing, full-text search (FTS5), inline visual editing.

**Admin** -- Full admin panel with visual schema builder, media library (drag-drop uploads via signed URLs), navigation menus, taxonomies, widgets, and a WordPress import wizard.

**Auth** -- Passkey-first (WebAuthn) with OAuth and magic link fallbacks. Role-based access control: Administrator, Editor, Author, Contributor.

**Plugins** -- `definePlugin()` API with lifecycle hooks, KV storage, settings, admin pages, dashboard widgets, custom block types, and API routes. Sandboxed execution on Cloudflare via Dynamic Worker Loaders.

**Agents** -- Skill files for AI-assisted plugin and theme development. CLI for programmatic site management. Built-in MCP server for direct AI tool integration.

**WordPress migration** -- Import posts, pages, media, and taxonomies from WXR exports, the WordPress REST API, or WordPress.com. Agent skills help port plugins and themes.

## Portable Platforms

| Layer    | Provided Trait Implementations              | Also works with                                     |
| -------- | ------------------------------------------- | --------------------------------------------------- |
| Database | `BespokeDb` (`emdash-db`)                   | SQLite, Turso/libSQL, PostgreSQL                    |
| Storage  | `LocalStorage` (`emdash-storage`)           | AWS S3, any S3-compatible service, local filesystem |
| Sessions | In-memory                                   | Redis, file-based                                   |
| Plugins  | `PluginRunner` via WebAssembly              | In-process (safe mode)                              |
| AI       | `OpenAiCompatProvider` (`emdash-llm`)       | Any OpenAI-Compatible endpoint                      |

## Status

EmDash is currently being actively ported into a pure Rust ecosystem. We welcome contributions, feedback, plugins, themes, and ideas.

See the [documentation](https://github.com/emdash-cms/emdash/tree/main/docs) for legacy TS guides.

## Development

The project is structured as a standard Rust `cargo` workspace.

```bash
git clone https://github.com/emdash-cms/emdash.git && cd emdash/emdash-rs
cargo build
```

To run tests across the workspace:

```bash
cargo test
```

## Repository Structure

```
emdash-rs/
  emdash-core/     Async traits (DatabaseProvider, StorageProvider, LlmProvider), PortableText AST
  emdash-db/       First-principles dynamic schema database abstractions
  emdash-schema/   Validation and schema definitions
  emdash-sandbox/  Wasm plugin execution sandbox
  emdash-server/   Axum HTTP routing and server context wiring
  emdash-storage/  Local and in-memory storage implementations
  emdash-llm/      Universal OpenAI-compatible clients
```

For a detailed breakdown of the architecture, see [RUST_DESIGN.md](RUST_DESIGN.md).
