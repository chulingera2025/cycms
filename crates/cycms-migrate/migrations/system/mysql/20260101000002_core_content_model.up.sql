-- 内容类型定义
CREATE TABLE content_types (
    id CHAR(36) NOT NULL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    api_id VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    kind VARCHAR(20) NOT NULL DEFAULT 'collection',
    fields JSON NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
);
