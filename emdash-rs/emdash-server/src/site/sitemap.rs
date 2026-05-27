use std::sync::Arc;

use crate::ServerContext;
use emdash_core::ApiError;

/// Generate an XML sitemap covering all published content across all collections.
pub async fn generate(ctx: &Arc<ServerContext>) -> Result<String, ApiError> {
    let site_url = ctx
        .db
        .query(
            "SELECT value FROM _emdash_settings WHERE key = 'site_url'",
            vec![],
        )
        .await?
        .into_iter()
        .next()
        .and_then(|r| r["value"].as_str().map(String::from))
        .unwrap_or_else(|| "http://localhost:3000".to_string());

    let collections = ctx.db.list("_emdash_collections").await?;

    let mut urls = String::new();

    // Homepage
    urls.push_str(&format!("<url><loc>{site_url}/</loc></url>\n"));

    for col in &collections {
        let name = match col["name"].as_str() {
            Some(n) => n,
            None => continue,
        };
        let table = format!("ec_{name}");
        let items = ctx.db
            .query(
                &format!(
                    "SELECT slug, updated_at FROM {table} WHERE status = 'published' AND slug IS NOT NULL"
                ),
                vec![],
            )
            .await
            .unwrap_or_default();

        for item in &items {
            let slug = item["slug"].as_str().unwrap_or("");
            let lastmod = item["updated_at"].as_str().unwrap_or("");
            urls.push_str(&format!(
                "<url><loc>{site_url}/{name}/{slug}</loc><lastmod>{lastmod}</lastmod></url>\n"
            ));
        }
    }

    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
{urls}</urlset>"#
    ))
}
