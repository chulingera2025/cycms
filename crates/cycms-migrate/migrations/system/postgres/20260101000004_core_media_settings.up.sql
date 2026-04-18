-- 媒体、系统设置、插件 settings schema 与 KV
CREATE TABLE media_assets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    filename VARCHAR(255) NOT NULL,
    original_filename VARCHAR(255) NOT NULL,
    mime_type VARCHAR(127) NOT NULL,
    size BIGINT NOT NULL,
    storage_path TEXT NOT NULL,
    metadata JSONB,
    uploaded_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_media_assets_mime ON media_assets(mime_type);
CREATE INDEX idx_media_assets_uploaded_by ON media_assets(uploaded_by);

CREATE TABLE settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    namespace VARCHAR(255) NOT NULL,
    key VARCHAR(255) NOT NULL,
    value JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (namespace, key)
);

CREATE TABLE plugin_settings_schemas (
    plugin_name VARCHAR(255) PRIMARY KEY,
    schema JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE plugin_kv (
    plugin_name VARCHAR(255) NOT NULL,
    key VARCHAR(255) NOT NULL,
    value JSONB NOT NULL,
    expires_at TIMESTAMPTZ,
    PRIMARY KEY (plugin_name, key)
);

CREATE INDEX idx_plugin_kv_expires_at ON plugin_kv(expires_at);
