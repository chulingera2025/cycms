//! `MigrationEngine` 主体（MIG-5 实现回滚后封顶）。

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use cycms_core::{Error, Result};
use cycms_db::{DatabasePool, DatabaseType};

use crate::discovery::{self, DiscoveredMigration};
use crate::record::{
    MYSQL_META_DDL, MigrationRecord, POSTGRES_META_DDL, SQLITE_META_DDL, SYSTEM_SOURCE,
};
use crate::runner;

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
    /// `migrations_root` 指向系统迁移根目录，engine 会按当前方言进入 `postgres/` /
    /// `mysql/` / `sqlite/` 子目录；未应用的迁移按 `version` 升序事务化执行。
    ///
    /// # Errors
    /// 元表初始化、文件发现或单条迁移执行失败时均返回错误。
    pub async fn run_system_migrations(
        &self,
        migrations_root: &Path,
    ) -> Result<Vec<MigrationRecord>> {
        self.run_migrations_for(SYSTEM_SOURCE, migrations_root).await
    }

    /// 执行指定插件的迁移。
    ///
    /// # Errors
    /// TODO!!! MIG-5 实现按 `plugin_name` 独立追踪并集成到 `PluginManager`。
    pub async fn run_plugin_migrations(
        &self,
        plugin_name: &str,
        migrations_dir: &Path,
    ) -> Result<Vec<MigrationRecord>> {
        self.run_migrations_for(plugin_name, migrations_dir).await
    }

    /// 回滚指定来源的最近 `count` 条迁移。
    ///
    /// # Errors
    /// TODO!!! MIG-5 实现基于 `.down.sql` 的倒序回滚。
    #[allow(clippy::unused_async)]
    pub async fn rollback(&self, _source: &str, _count: usize) -> Result<Vec<MigrationRecord>> {
        todo!("TODO!!!: MIG-5 实现回滚")
    }

    async fn run_migrations_for(
        &self,
        source: &str,
        migrations_root: &Path,
    ) -> Result<Vec<MigrationRecord>> {
        self.ensure_meta_table().await?;

        let dir = resolve_driver_dir(self.db.db_type(), migrations_root);
        let discovered: Vec<DiscoveredMigration> = discovery::scan(&dir)?;
        let applied: HashSet<i64> = runner::list_applied_versions(&self.db, source)
            .await?
            .into_iter()
            .collect();

        let mut records = Vec::new();
        for migration in discovered {
            if applied.contains(&migration.version) {
                continue;
            }
            let record = runner::apply_one(&self.db, source, &migration).await?;
            records.push(record);
        }
        Ok(records)
    }
}

fn resolve_driver_dir(db_type: DatabaseType, root: &Path) -> std::path::PathBuf {
    let sub = match db_type {
        DatabaseType::Postgres => "postgres",
        DatabaseType::MySql => "mysql",
        DatabaseType::Sqlite => "sqlite",
    };
    root.join(sub)
}
