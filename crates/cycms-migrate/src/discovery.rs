//! 迁移文件发现与解析（占位，MIG-3 实现）。

#![allow(dead_code)]

/// 发现阶段解析出的单条迁移描述。
#[derive(Debug, Clone)]
pub struct DiscoveredMigration {
    pub version: i64,
    pub name: String,
    pub up_sql: String,
    pub down_sql: Option<String>,
    pub checksum: [u8; 32],
}
