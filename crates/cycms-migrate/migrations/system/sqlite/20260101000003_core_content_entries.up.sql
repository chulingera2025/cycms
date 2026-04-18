-- 内容实例、版本快照与关联关系
CREATE TABLE content_entries (
    id TEXT NOT NULL PRIMARY KEY,
    content_type_id TEXT NOT NULL REFERENCES content_types(id),
    slug TEXT,
    status TEXT NOT NULL DEFAULT 'draft',
    current_version_id TEXT,
    published_version_id TEXT,
    fields TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(fields)),
    created_by TEXT NOT NULL REFERENCES users(id),
    updated_by TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    published_at TEXT
);

CREATE INDEX idx_content_entries_type ON content_entries(content_type_id);
CREATE INDEX idx_content_entries_status ON content_entries(status);
CREATE INDEX idx_content_entries_slug ON content_entries(slug);

CREATE TABLE content_revisions (
    id TEXT NOT NULL PRIMARY KEY,
    content_entry_id TEXT NOT NULL REFERENCES content_entries(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL,
    snapshot TEXT NOT NULL CHECK (json_valid(snapshot)),
    change_summary TEXT,
    created_by TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    UNIQUE (content_entry_id, version_number)
);

CREATE TABLE content_relations (
    id TEXT NOT NULL PRIMARY KEY,
    source_entry_id TEXT NOT NULL REFERENCES content_entries(id) ON DELETE CASCADE,
    target_entry_id TEXT NOT NULL REFERENCES content_entries(id) ON DELETE CASCADE,
    field_api_id TEXT NOT NULL,
    relation_kind TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_content_relations_source ON content_relations(source_entry_id);
CREATE INDEX idx_content_relations_target ON content_relations(target_entry_id);
