-- Core EmDash system tables.  All system tables are prefixed `_emdash_`.
-- Dynamic collection tables are prefixed `ec_` and created at runtime.

PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

-- Applied migrations log (used by the migration runner itself)
CREATE TABLE IF NOT EXISTS _emdash_migrations (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT    NOT NULL UNIQUE,
    applied_at TEXT    NOT NULL DEFAULT (datetime('now'))
);

-- Site-wide key/value settings
CREATE TABLE IF NOT EXISTS _emdash_settings (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Users
CREATE TABLE IF NOT EXISTS _emdash_users (
    id         TEXT PRIMARY KEY,
    email      TEXT NOT NULL UNIQUE,
    name       TEXT NOT NULL,
    role       TEXT NOT NULL DEFAULT 'editor',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Auth sessions (short-lived, token stored as a SHA-256 hex hash)
CREATE TABLE IF NOT EXISTS _emdash_sessions (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES _emdash_users(id) ON DELETE CASCADE,
    token_hash  TEXT NOT NULL UNIQUE,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at  TEXT NOT NULL
);

-- Long-lived API tokens
CREATE TABLE IF NOT EXISTS _emdash_api_tokens (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES _emdash_users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    token_hash  TEXT NOT NULL UNIQUE,
    scopes      TEXT NOT NULL DEFAULT '[]',   -- JSON array of scope strings
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    last_used_at TEXT
);

-- Collection definitions (dynamic schema)
CREATE TABLE IF NOT EXISTS _emdash_collections (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,   -- machine name, e.g. "posts"
    title       TEXT NOT NULL,
    description TEXT,
    is_feed     INTEGER NOT NULL DEFAULT 0,   -- 1 → auto RSS feed
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Field definitions within a collection
CREATE TABLE IF NOT EXISTS _emdash_fields (
    id            TEXT PRIMARY KEY,
    collection_id TEXT NOT NULL REFERENCES _emdash_collections(id) ON DELETE CASCADE,
    name          TEXT NOT NULL,
    title         TEXT NOT NULL,
    field_type    TEXT NOT NULL,   -- JSON-encoded FieldType
    required      INTEGER NOT NULL DEFAULT 0,
    position      INTEGER NOT NULL DEFAULT 0,
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at    TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (collection_id, name)
);

-- Media library
CREATE TABLE IF NOT EXISTS _emdash_media (
    id           TEXT PRIMARY KEY,
    filename     TEXT NOT NULL,
    mime_type    TEXT NOT NULL,
    size         INTEGER NOT NULL,
    storage_path TEXT NOT NULL,
    alt          TEXT,
    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Taxonomies (tags, categories…)
CREATE TABLE IF NOT EXISTS _emdash_taxonomies (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    title       TEXT NOT NULL,
    description TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Terms within a taxonomy
CREATE TABLE IF NOT EXISTS _emdash_terms (
    id          TEXT PRIMARY KEY,
    taxonomy_id TEXT NOT NULL REFERENCES _emdash_taxonomies(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    slug        TEXT NOT NULL,
    description TEXT,
    parent_id   TEXT REFERENCES _emdash_terms(id),
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (taxonomy_id, slug)
);

-- Menus
CREATE TABLE IF NOT EXISTS _emdash_menus (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL UNIQUE,
    title      TEXT NOT NULL,
    location   TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Menu items (tree via parent_id)
CREATE TABLE IF NOT EXISTS _emdash_menu_items (
    id         TEXT PRIMARY KEY,
    menu_id    TEXT NOT NULL REFERENCES _emdash_menus(id) ON DELETE CASCADE,
    parent_id  TEXT REFERENCES _emdash_menu_items(id),
    label      TEXT NOT NULL,
    url        TEXT,
    content_id TEXT,
    position   INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 301/302 redirects
CREATE TABLE IF NOT EXISTS _emdash_redirects (
    id          TEXT PRIMARY KEY,
    from_path   TEXT NOT NULL UNIQUE,
    to_path     TEXT NOT NULL,
    status_code INTEGER NOT NULL DEFAULT 301,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 404 log (for auto-redirect suggestions)
CREATE TABLE IF NOT EXISTS _emdash_not_found (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    path       TEXT NOT NULL,
    hits       INTEGER NOT NULL DEFAULT 1,
    last_seen  TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (path)
);

-- Installed plugins
CREATE TABLE IF NOT EXISTS _emdash_plugins (
    id           TEXT PRIMARY KEY,
    plugin_id    TEXT NOT NULL UNIQUE,
    name         TEXT NOT NULL,
    wasm_path    TEXT NOT NULL,
    capabilities TEXT NOT NULL DEFAULT '[]',   -- JSON array
    enabled      INTEGER NOT NULL DEFAULT 1,
    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Content revision history
CREATE TABLE IF NOT EXISTS _emdash_revisions (
    id         TEXT PRIMARY KEY,
    content_id TEXT NOT NULL,
    table_name TEXT NOT NULL,
    data       TEXT NOT NULL,   -- JSON snapshot of the row
    created_by TEXT REFERENCES _emdash_users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Comments
CREATE TABLE IF NOT EXISTS _emdash_comments (
    id           TEXT PRIMARY KEY,
    content_id   TEXT NOT NULL,
    table_name   TEXT NOT NULL,
    parent_id    TEXT REFERENCES _emdash_comments(id),
    author_name  TEXT NOT NULL,
    author_email TEXT NOT NULL,
    body         TEXT NOT NULL,
    status       TEXT NOT NULL DEFAULT 'pending',   -- pending | approved | spam
    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at   TEXT NOT NULL DEFAULT (datetime('now'))
);
