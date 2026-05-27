use clap::{Parser, Subcommand};
use emdash_db::validate_identifier;
use std::sync::Arc;
use tracing::info;

// ── CLI definition ─────────────────────────────────────────────────────────────

/// EmDash — headless CMS with agent-first API
#[derive(Parser)]
#[command(name = "emdash", version, author, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Start the EmDash server
    Serve {
        /// TCP port to listen on
        #[arg(long, env = "PORT", default_value = "3000")]
        port: u16,
    },

    /// Run pending database migrations and exit
    Migrate,

    /// Export all published content to static HTML
    Export {
        /// Output directory for generated files
        #[arg(long, short, default_value = "dist")]
        out: std::path::PathBuf,
    },

    /// Print the current OpenAPI 3.1 schema and exit
    Schema,
}

// ── Entry point ────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // JSON structured logging for agent pipeline compatibility.
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "info".to_string())
                .as_str(),
        )
        .init();

    // ── Shared infrastructure (all sub-commands need DB + storage) ─────────────
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "emdash.db".to_string());
    let store_path = std::env::var("STORAGE_PATH").unwrap_or_else(|_| "storage".to_string());

    let db = Arc::new(emdash_db::BespokeDb::connect(&db_url).await?);
    let sto = Arc::new(emdash_storage::LocalStorage::new(&store_path));
    let llm = Arc::new(emdash_llm::OpenAiCompatProvider::from_env());
    let plg = Arc::new(emdash_sandbox::NoopPluginRunner);

    match cli.cmd {
        // ── serve ────────────────────────────────────────────────────────────
        Cmd::Serve { port } => {
            info!(port, "starting emdash server");
            let ctx = emdash_server::ServerContext {
                db,
                storage: sto,
                llm,
                plugin_runner: plg,
            };
            let router = emdash_server::create_router(Arc::new(ctx));
            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
            info!(addr = %listener.local_addr()?, "listening");
            axum::serve(listener, router).await?;
        }

        // ── migrate ──────────────────────────────────────────────────────────
        Cmd::Migrate => {
            info!("running migrations");
            // BespokeDb::connect already runs migrations — this command just
            // confirms they ran successfully and exits 0.
            info!("migrations complete");
        }

        // ── export ───────────────────────────────────────────────────────────
        Cmd::Export { out } => {
            info!(dir = %out.display(), "exporting static site");
            tokio::fs::create_dir_all(&out).await?;

            let ctx = Arc::new(emdash_server::ServerContext {
                db: db.clone(),
                storage: sto,
                llm,
                plugin_runner: plg,
            });

            // Export each published item from every collection.
            let collections = ctx.db.list("_emdash_collections").await?;
            for col in &collections {
                let name = match col["name"].as_str() {
                    Some(n) => n,
                    None => continue,
                };
                let table = format!("ec_{name}");
                if validate_identifier(name).is_err() {
                    continue;
                }
                let items = ctx.db
                    .query(
                        &format!(
                            "SELECT slug FROM {table} WHERE status = 'published' AND slug IS NOT NULL"
                        ),
                        vec![],
                    )
                    .await
                    .unwrap_or_default();

                let col_dir = out.join(name);
                tokio::fs::create_dir_all(&col_dir).await?;

                for item in &items {
                    let slug = item["slug"].as_str().unwrap_or("index");
                    let html = emdash_server::site::templates::render_page(&ctx, name, slug)
                        .await
                        .unwrap_or_else(|e| format!("<p>Error: {e}</p>"));
                    let file = col_dir.join(format!("{slug}.html"));
                    tokio::fs::write(&file, html).await?;
                    info!(file = %file.display(), "exported");
                }
            }

            // Home page
            let home = emdash_server::site::templates::render_home(&ctx)
                .await
                .unwrap_or_else(|e| format!("<p>Error: {e}</p>"));
            tokio::fs::write(out.join("index.html"), home).await?;

            info!("export complete");
        }

        Cmd::Schema => {
            let api = emdash_server::create_api_doc();
            println!("{}", serde_json::to_string_pretty(&api)?);
        }
    }

    Ok(())
}
