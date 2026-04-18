-- 内容实例、版本快照与关联关系
-- MySQL 8 的 JSON 列不能使用字面量 DEFAULT，应用层写入时必须显式传入默认值
CREATE TABLE content_entries (
    id CHAR(36) NOT NULL PRIMARY KEY,
    content_type_id CHAR(36) NOT NULL,
    slug VARCHAR(255),
    status VARCHAR(20) NOT NULL DEFAULT 'draft',
    current_version_id CHAR(36),
    published_version_id CHAR(36),
    fields JSON NOT NULL,
    created_by CHAR(36) NOT NULL,
    updated_by CHAR(36) NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    published_at DATETIME(6),
    KEY idx_content_entries_type (content_type_id),
    KEY idx_content_entries_status (status),
    KEY idx_content_entries_slug (slug),
    CONSTRAINT fk_content_entries_type FOREIGN KEY (content_type_id) REFERENCES content_types(id),
    CONSTRAINT fk_content_entries_creator FOREIGN KEY (created_by) REFERENCES users(id),
    CONSTRAINT fk_content_entries_updater FOREIGN KEY (updated_by) REFERENCES users(id)
);

CREATE TABLE content_revisions (
    id CHAR(36) NOT NULL PRIMARY KEY,
    content_entry_id CHAR(36) NOT NULL,
    version_number INT NOT NULL,
    snapshot JSON NOT NULL,
    change_summary TEXT,
    created_by CHAR(36) NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    UNIQUE KEY uq_content_revisions_entry_version (content_entry_id, version_number),
    CONSTRAINT fk_content_revisions_entry FOREIGN KEY (content_entry_id) REFERENCES content_entries(id) ON DELETE CASCADE,
    CONSTRAINT fk_content_revisions_creator FOREIGN KEY (created_by) REFERENCES users(id)
);

CREATE TABLE content_relations (
    id CHAR(36) NOT NULL PRIMARY KEY,
    source_entry_id CHAR(36) NOT NULL,
    target_entry_id CHAR(36) NOT NULL,
    field_api_id VARCHAR(255) NOT NULL,
    relation_kind VARCHAR(20) NOT NULL,
    position INT NOT NULL DEFAULT 0,
    KEY idx_content_relations_source (source_entry_id),
    KEY idx_content_relations_target (target_entry_id),
    CONSTRAINT fk_content_relations_source FOREIGN KEY (source_entry_id) REFERENCES content_entries(id) ON DELETE CASCADE,
    CONSTRAINT fk_content_relations_target FOREIGN KEY (target_entry_id) REFERENCES content_entries(id) ON DELETE CASCADE
);
