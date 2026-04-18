-- 媒体、系统设置、插件 settings schema 与 KV
CREATE TABLE media_assets (
    id CHAR(36) NOT NULL PRIMARY KEY,
    filename VARCHAR(255) NOT NULL,
    original_filename VARCHAR(255) NOT NULL,
    mime_type VARCHAR(127) NOT NULL,
    size BIGINT NOT NULL,
    storage_path TEXT NOT NULL,
    metadata JSON,
    uploaded_by CHAR(36) NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    KEY idx_media_assets_mime (mime_type),
    KEY idx_media_assets_uploaded_by (uploaded_by),
    CONSTRAINT fk_media_assets_uploader FOREIGN KEY (uploaded_by) REFERENCES users(id)
);

CREATE TABLE settings (
    id CHAR(36) NOT NULL PRIMARY KEY,
    namespace VARCHAR(255) NOT NULL,
    `key` VARCHAR(255) NOT NULL,
    value JSON NOT NULL,
    updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    UNIQUE KEY uq_settings_namespace_key (namespace, `key`)
);

CREATE TABLE plugin_settings_schemas (
    plugin_name VARCHAR(255) NOT NULL PRIMARY KEY,
    `schema` JSON NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
);

CREATE TABLE plugin_kv (
    plugin_name VARCHAR(255) NOT NULL,
    `key` VARCHAR(255) NOT NULL,
    value JSON NOT NULL,
    expires_at DATETIME(6),
    PRIMARY KEY (plugin_name, `key`),
    KEY idx_plugin_kv_expires_at (expires_at)
);
