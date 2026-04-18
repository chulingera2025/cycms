//! `cycms-migrate` 提供系统与插件的数据库迁移执行器。
//!
//! - 自定义元表 `migration_records` 同时追踪系统（`source = "system"`）与插件迁移，
//!   满足 tasks.md §4.3 的多 source 追踪诉求。
//! - 文件命名沿用 sqlx-cli 约定 `{version}_{name}.up.sql` / `.down.sql`，可无缝使用
//!   `sqlx migrate add -r` 生成初始模板。
//! - 执行器单条迁移均包裹在事务中，失败后自动回滚并写入 `status = 'failed'` 记录。

mod checksum;
mod discovery;
mod engine;
mod record;
mod runner;

pub use discovery::{DiscoveredMigration, scan};
pub use engine::MigrationEngine;
pub use record::{MigrationRecord, MigrationStatus, SYSTEM_SOURCE};
