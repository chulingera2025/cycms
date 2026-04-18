//! `MigrationEngine` 主体骨架（MIG-4/5 逐步补齐）。

use std::path::Path;
use std::sync::Arc;

use cycms_core::{Error, Result};
use cycms_db::{DatabasePool, DatabaseType};

use crate::record::{
    MYSQL_META_DDL, MigrationRecord, POSTGRES_META_DDL, SQLITE_META_DDL,
};

/// 迁移执行入口，持有数据库连接池引用。
///
/// `MigrationEngine` 是启动期一次性组件，不放入 `AppContext`，避免运行时意外触发迁移。
pub struct MigrationEngine {
    db: Arc<DatabasePool>,
}

impl MigrationEngine {
    pub fn new(db: Arc<DatabasePool>) -> Self {
        Self { db }
    }

    /// 确保元表 `migration_records` 存在；已存在时为幂等操作。
    ///
    /// # Errors
    /// 执行 DDL 失败（如权限不足）时返回 `Error::Internal`。
    pub async fn ensure_meta_table(&self) -> Result<()> {
        let ddl = match self.db.db_type() {
            DatabaseType::Postgres => POSTGRES_META_DDL,
            DatabaseType::MySql => MYSQL_META_DDL,
            DatabaseType::Sqlite => SQLITE_META_DDL,
        };

        let outcome = match self.db.as_ref() {
            DatabasePool::Postgres(pool) => sqlx::raw_sql(ddl).execute(pool).await.map(|_| ()),
            DatabasePool::MySql(pool) => sqlx::raw_sql(ddl).execute(pool).await.map(|_| ()),
            DatabasePool::Sqlite(pool) => sqlx::raw_sql(ddl).execute(pool).await.map(|_| ()),
        };

        outcome.map_err(|source| Error::Internal {
            message: "failed to ensure migration_records table".to_owned(),
            source: Some(Box::new(source)),
        })
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
