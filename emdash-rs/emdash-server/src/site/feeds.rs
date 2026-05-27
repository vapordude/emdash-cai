use serde_json::Value;
use std::sync::Arc;

use emdash_core::ApiError;
use crate::ServerContext;

/// Generate an RSS 2.0 feed for a collection that has `is_feed = true`.
pub async fn rss_for_collection(ctx: &Arc<ServerContext>, collection: &str) -> Result<String, ApiError> {
    let site_title = ctx.db
        .query("SELECT value FROM _emdash_settings WHERE key = 'site_title'", vec![])
        .await?
        .into_iter()
        .next()
        .and_then(|r| r["value"].as_str().map(String::from))
        .unwrap_or_else(|| "EmDash".to_string());

    let site_url = ctx.db
        .query("SELECT value FROM _emdash_settings WHERE key = 'site_url'", vec![])
        .await?
        .into_iter()
        .next()
        .and_then(|r| r["value"].as_str().map(String::from))
        .unwrap_or_else(|| "http://localhost:3000".to_string());

    // Verify the collection has is_feed = 1.
    let col_check = ctx.db
        .query(
            "SELECT id FROM _emdash_collections WHERE name = ? AND is_feed = 1",
            vec![Value::String(collection.to_string())],
        )
        .await?;
    if col_check.is_empty() {
        return Err(ApiError::NotFound(format!("feed collection '{collection}' not found")));
    }

    let table = format!("ec_{collection}");
    let items = ctx.db
        .query(
            &format!(
                "SELECT id, slug, data, published_at, created_at FROM {table} \
                 WHERE status = 'published' ORDER BY published_at DESC LIMIT 50"
            ),
            vec![],
        )
        .await?;

    let mut items_xml = String::new();
    for item in &items {
        let slug        = item["slug"].as_str().unwrap_or("");
        let link        = format!("{site_url}/{collection}/{slug}");
        let pub_date    = item["published_at"].as_str()
            .or_else(|| item["created_at"].as_str())
            .unwrap_or("");
        let description = item["data"].as_str().unwrap_or("").replace('<', "&lt;").replace('>', "&gt;");

        items_xml.push_str(&format!(
            "<item>\
              <title>{slug}</title>\
              <link>{link}</link>\
              <pubDate>{pub_date}</pubDate>\
              <description>{description}</description>\
              <guid>{link}</guid>\
            </item>\n"
        ));
    }

    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
<channel>
  <title>{site_title}</title>
  <link>{site_url}</link>
  <description>{site_title} — {collection}</description>
{items_xml}
</channel>
</rss>"#
    ))
}
