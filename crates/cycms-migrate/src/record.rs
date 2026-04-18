//! 迁移记录、状态以及三方言元表 DDL。

use chrono::{DateTime, Utc};

/// 系统迁移在 `migration_records.source` 列的保留值。
pub const SYSTEM_SOURCE: &str = "system";

/// 元表名；后续所有 SQL 片段统一通过此常量引用，便于将来重命名。
#[allow(dead_code)] // 在 MIG-4/5 的 INSERT/SELECT 中会被引用。
pub const META_TABLE: &str = "migration_records";

/// 单条迁移的执行状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationStatus {
    Applied,
    Failed,
    RolledBack,
}

impl MigrationStatus {
    pub const APPLIED: &'static str = "applied";
    pub const FAILED: &'static str = "failed";
    pub const ROLLED_BACK: &'static str = "rolled_back";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Applied => Self::APPLIED,
            Self::Failed => Self::FAILED,
            Self::RolledBack => Self::ROLLED_BACK,
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            Self::APPLIED => Some(Self::Applied),
            Self::FAILED => Some(Self::Failed),
            Self::ROLLED_BACK => Some(Self::RolledBack),
            _ => None,
        }
    }
}

/// 迁移记录的强类型视图。
#[derive(Debug, Clone)]
pub struct MigrationRecord {
    pub id: i64,
    pub version: i64,
    pub name: String,
    pub source: String,
    pub applied_at: DateTime<Utc>,
    pub execution_time_ms: i64,
    pub status: MigrationStatus,
}

pub(crate) const POSTGRES_META_DDL: &str = "\
CREATE TABLE IF NOT EXISTS migration_records (
    id BIGSERIAL PRIMARY KEY,
    version BIGINT NOT NULL,
    name VARCHAR(255) NOT NULL,
    source VARCHAR(255) NOT NULL,
    checksum BYTEA NOT NULL,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    execution_time_ms BIGINT NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'applied',
    UNIQUE (source, version)
);";

pub(crate) const MYSQL_META_DDL: &str = "\
CREATE TABLE IF NOT EXISTS migration_records (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    version BIGINT NOT NULL,
    name VARCHAR(255) NOT NULL,
    source VARCHAR(255) NOT NULL,
    checksum VARBINARY(64) NOT NULL,
    applied_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    execution_time_ms BIGINT NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'applied',
    UNIQUE (source, version)
);";

pub(crate) const SQLITE_META_DDL: &str = "\
CREATE TABLE IF NOT EXISTS migration_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    version INTEGER NOT NULL,
    name TEXT NOT NULL,
    source TEXT NOT NULL,
    checksum BLOB NOT NULL,
    applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    execution_time_ms INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'applied',
    UNIQUE (source, version)
);";
