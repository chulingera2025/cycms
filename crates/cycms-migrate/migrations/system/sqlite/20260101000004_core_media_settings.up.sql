-- 媒体、系统设置、插件 settings schema 与 KV
CREATE TABLE media_assets (
    id TEXT NOT NULL PRIMARY KEY,
    filename TEXT NOT NULL,
    original_filename TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    size INTEGER NOT NULL,
    storage_path TEXT NOT NULL,
    metadata TEXT CHECK (metadata IS NULL OR json_valid(metadata)),
    uploaded_by TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX idx_media_assets_mime ON media_assets(mime_type);
CREATE INDEX idx_media_assets_uploaded_by ON media_assets(uploaded_by);

CREATE TABLE settings (
    id TEXT NOT NULL PRIMARY KEY,
    namespace TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL CHECK (json_valid(value)),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    UNIQUE (namespace, key)
);

CREATE TABLE plugin_settings_schemas (
    plugin_name TEXT NOT NULL PRIMARY KEY,
    schema TEXT NOT NULL CHECK (json_valid(schema)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE plugin_kv (
    plugin_name TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL CHECK (json_valid(value)),
    expires_at TEXT,
    PRIMARY KEY (plugin_name, key)
);

CREATE INDEX idx_plugin_kv_expires_at ON plugin_kv(expires_at);
