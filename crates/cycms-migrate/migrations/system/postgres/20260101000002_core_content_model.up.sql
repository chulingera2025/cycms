-- 内容类型定义
CREATE TABLE content_types (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    api_id VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    kind VARCHAR(20) NOT NULL DEFAULT 'collection',
    fields JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
