-- 内容类型定义
CREATE TABLE content_types (
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    api_id TEXT NOT NULL UNIQUE,
    description TEXT,
    kind TEXT NOT NULL DEFAULT 'collection',
    fields TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(fields)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
