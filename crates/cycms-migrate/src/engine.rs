//! `MigrationEngine` 主体骨架（MIG-2/4/5 逐步补齐）。

use std::path::Path;
use std::sync::Arc;

use cycms_core::Result;
use cycms_db::DatabasePool;

use crate::record::MigrationRecord;

/// 迁移执行入口，持有数据库连接池引用。
///
/// `MigrationEngine` 是启动期一次性组件，不放入 `AppContext`，避免运行时意外触发迁移。
pub struct MigrationEngine {
    #[allow(dead_code)]
    db: Arc<DatabasePool>,
}

impl MigrationEngine {
    pub fn new(db: Arc<DatabasePool>) -> Self {
        Self { db }
    }

    /// 执行系统迁移。
    ///
    /// # Errors
    /// TODO!!! MIG-4 实现真实事务化执行与元表记录。
    #[allow(clippy::unused_async)]
    pub async fn run_system_migrations(&self) -> Result<Vec<MigrationRecord>> {
        todo!("TODO!!!: MIG-4 实现系统迁移执行器")
    }

    /// 执行指定插件的迁移。
    ///
    /// # Errors
    /// TODO!!! MIG-5 实现按 `plugin_name` 独立追踪。
    #[allow(clippy::unused_async)]
    pub async fn run_plugin_migrations(
        &self,
        _plugin_name: &str,
        _migrations_dir: &Path,
    ) -> Result<Vec<MigrationRecord>> {
        todo!("TODO!!!: MIG-5 实现插件迁移")
    }

    /// 回滚指定来源的最近 `count` 条迁移。
    ///
    /// # Errors
    /// TODO!!! MIG-5 实现基于 `.down.sql` 的倒序回滚。
    #[allow(clippy::unused_async)]
    pub async fn rollback(&self, _source: &str, _count: usize) -> Result<Vec<MigrationRecord>> {
        todo!("TODO!!!: MIG-5 实现回滚")
    }
}
