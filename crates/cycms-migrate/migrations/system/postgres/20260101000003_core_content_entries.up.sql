-- 内容实例、版本快照与关联关系
CREATE TABLE content_entries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    content_type_id UUID NOT NULL REFERENCES content_types(id),
    slug VARCHAR(255),
    status VARCHAR(20) NOT NULL DEFAULT 'draft',
    current_version_id UUID,
    published_version_id UUID,
    fields JSONB NOT NULL DEFAULT '{}',
    created_by UUID NOT NULL REFERENCES users(id),
    updated_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    published_at TIMESTAMPTZ
);

CREATE INDEX idx_content_entries_type ON content_entries(content_type_id);
CREATE INDEX idx_content_entries_status ON content_entries(status);
CREATE INDEX idx_content_entries_slug ON content_entries(slug);
CREATE INDEX idx_content_entries_fields ON content_entries USING GIN (fields);

CREATE TABLE content_revisions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    content_entry_id UUID NOT NULL REFERENCES content_entries(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL,
    snapshot JSONB NOT NULL,
    change_summary TEXT,
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (content_entry_id, version_number)
);

CREATE TABLE content_relations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_entry_id UUID NOT NULL REFERENCES content_entries(id) ON DELETE CASCADE,
    target_entry_id UUID NOT NULL REFERENCES content_entries(id) ON DELETE CASCADE,
    field_api_id VARCHAR(255) NOT NULL,
    relation_kind VARCHAR(20) NOT NULL,
    position INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_content_relations_source ON content_relations(source_entry_id);
CREATE INDEX idx_content_relations_target ON content_relations(target_entry_id);
