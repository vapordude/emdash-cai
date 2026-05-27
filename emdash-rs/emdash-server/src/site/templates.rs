use minijinja::Environment;
use serde_json::Value;
use std::sync::Arc;

use crate::ServerContext;
use emdash_core::ApiError;

// ── Template environment ──────────────────────────────────────────────────────

fn default_env() -> Environment<'static> {
    let mut env = Environment::new();

    // Built-in templates — overridable by mounting a theme directory later.
    env.add_template_owned(
        "base.html",
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{% block title %}EmDash{% endblock %}</title>
  {% block head %}{% endblock %}
</head>
<body>
  {% block body %}{% endblock %}
</body>
</html>"#
            .to_string(),
    )
    .ok();

    env.add_template_owned(
        "home.html",
        r#"{% extends "base.html" %}
{% block title %}{{ site_title }}{% endblock %}
{% block body %}
<main>
  <h1>{{ site_title }}</h1>
  {% for item in items %}
    <article>
      <h2><a href="/{{ collection }}/{{ item.slug }}">{{ item.slug }}</a></h2>
    </article>
  {% endfor %}
</main>
{% endblock %}"#
            .to_string(),
    )
    .ok();

    env.add_template_owned(
        "page.html",
        r#"{% extends "base.html" %}
{% block title %}{{ item.slug }} | {{ site_title }}{% endblock %}
{% block body %}
<main>
  <article>
    <h1>{{ item.slug }}</h1>
    <div class="content">{{ item.data }}</div>
  </article>
</main>
{% endblock %}"#
            .to_string(),
    )
    .ok();

    env
}

// ── Render helpers ────────────────────────────────────────────────────────────

/// Render the home page using the first feed-enabled collection.
pub async fn render_home(ctx: &Arc<ServerContext>) -> Result<String, ApiError> {
    let site_title = ctx
        .db
        .query(
            "SELECT value FROM _emdash_settings WHERE key = 'site_title'",
            vec![],
        )
        .await?
        .into_iter()
        .next()
        .and_then(|r| r["value"].as_str().map(String::from))
        .unwrap_or_else(|| "EmDash".to_string());

    // Find first feed-enabled collection for the home page.
    let cols = ctx
        .db
        .query(
            "SELECT name FROM _emdash_collections WHERE is_feed = 1 LIMIT 1",
            vec![],
        )
        .await?;

    let (collection, items) = if let Some(col) = cols.into_iter().next() {
        let name = col["name"].as_str().unwrap_or("").to_string();
        let rows = ctx.db
            .query(
                &format!("SELECT id, slug, created_at FROM ec_{name} WHERE status = 'published' ORDER BY created_at DESC LIMIT 20"),
                vec![],
            )
            .await
            .unwrap_or_default();
        (name, rows)
    } else {
        ("posts".to_string(), vec![])
    };

    let env = default_env();
    let tmpl = env
        .get_template("home.html")
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let ctx_val = minijinja::context! {
        site_title => site_title,
        collection => collection,
        items => serde_json::to_value(items).unwrap_or_default(),
    };

    tmpl.render(ctx_val)
        .map_err(|e| ApiError::Internal(e.to_string()))
}

/// Render a single content item.
pub async fn render_page(
    ctx: &Arc<ServerContext>,
    collection: &str,
    slug: &str,
) -> Result<String, ApiError> {
    let site_title = ctx
        .db
        .query(
            "SELECT value FROM _emdash_settings WHERE key = 'site_title'",
            vec![],
        )
        .await?
        .into_iter()
        .next()
        .and_then(|r| r["value"].as_str().map(String::from))
        .unwrap_or_else(|| "EmDash".to_string());

    let table = format!("ec_{collection}");
    let rows = ctx
        .db
        .query(
            &format!("SELECT * FROM {table} WHERE slug = ? AND status = 'published' LIMIT 1"),
            vec![Value::String(slug.to_string())],
        )
        .await?;

    let item = rows
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::NotFound(slug.to_string()))?;

    let env = default_env();
    let tmpl = env
        .get_template("page.html")
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let ctx_val = minijinja::context! {
        site_title => site_title,
        collection => collection,
        item => serde_json::to_value(&item).unwrap_or_default(),
    };

    tmpl.render(ctx_val)
        .map_err(|e| ApiError::Internal(e.to_string()))
}
