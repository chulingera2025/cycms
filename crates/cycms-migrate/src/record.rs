//! 迁移记录与元表 DDL 常量（占位，MIG-2 实现）。

/// 系统迁移在 `migration_records.source` 列的保留值。
pub const SYSTEM_SOURCE: &str = "system";

/// 单条迁移的执行状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationStatus {
    Applied,
    Failed,
    RolledBack,
}

/// 迁移记录的强类型视图。
///
/// TODO!!! MIG-2 补齐 DDL 常量与序列化字段。
#[derive(Debug, Clone)]
pub struct MigrationRecord {
    pub id: i64,
    pub version: i64,
    pub name: String,
    pub source: String,
    pub applied_at: chrono::DateTime<chrono::Utc>,
    pub execution_time_ms: i64,
    pub status: MigrationStatus,
}
