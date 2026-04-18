-- 插件注册表与审计日志
CREATE TABLE plugins (
    id CHAR(36) NOT NULL PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    version VARCHAR(50) NOT NULL,
    kind VARCHAR(20) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'disabled',
    manifest JSON NOT NULL,
    installed_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
);

CREATE TABLE audit_logs (
    id CHAR(36) NOT NULL PRIMARY KEY,
    actor_id CHAR(36) NOT NULL,
    action VARCHAR(255) NOT NULL,
    resource_type VARCHAR(255) NOT NULL,
    resource_id VARCHAR(255),
    details JSON,
    result VARCHAR(20) NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    KEY idx_audit_logs_actor (actor_id),
    KEY idx_audit_logs_action (action),
    KEY idx_audit_logs_created (created_at)
);
