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
    /// 构造 `MigrationEngine`。
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
        self.run_migrations_for(SYSTEM_SOURCE, migrations_root)
            .await
    }

    /// 执行指定插件的迁移。
    ///
    /// `source` 固定为插件名，因此同一套迁移文件在不同插件命名空间下可并存；
    /// `migrations_dir` 是插件目录下的 `migrations/` 根（内部同样按方言分子目录）。
    ///
    /// # Errors
    /// 元表初始化、文件发现或单条迁移执行失败时均返回错误。
    pub async fn run_plugin_migrations(
        &self,
        plugin_name: &str,
        migrations_dir: &Path,
    ) -> Result<Vec<MigrationRecord>> {
        self.run_migrations_for(plugin_name, migrations_dir).await
    }

    /// 回滚指定来源的最近 `count` 条迁移（按 `version` 从大到小）。
    ///
    /// `migrations_root` 与 `run_*_migrations` 相同，用于在回滚时重新读取对应的
    /// `.down.sql`；若 `.down.sql` 缺失则整体拒绝回滚。
    ///
    /// # Errors
    /// 缺失 `.down.sql`、执行失败或元表更新失败均会返回错误。
    pub async fn rollback(
        &self,
        source: &str,
        migrations_root: &Path,
        count: usize,
    ) -> Result<Vec<MigrationRecord>> {
        self.ensure_meta_table().await?;

        let dir = resolve_driver_dir(self.db.db_type(), migrations_root);
        let discovered = discovery::scan(&dir)?;
        let by_version: std::collections::HashMap<i64, &DiscoveredMigration> =
            discovered.iter().map(|m| (m.version, m)).collect();

        let limit = i64::try_from(count).map_err(|_| Error::BadRequest {
            message: "rollback count exceeds i64 range".to_owned(),
            source: None,
        })?;
        let targets = runner::list_recent_applied(&self.db, source, limit).await?;
        if targets.is_empty() {
            return Ok(Vec::new());
        }

        let mut rolled = Vec::with_capacity(targets.len());
        for (record_id, version, name) in targets {
            let migration = by_version.get(&version).ok_or_else(|| Error::NotFound {
                message: format!(
                    "migration file missing for rollback: source={source} version={version}"
                ),
            })?;
            let down_sql = migration
                .down_sql
                .as_deref()
                .ok_or_else(|| Error::BadRequest {
                    message: format!(
                        "migration has no .down.sql, cannot rollback: source={source} version={version}"
                    ),
                    source: None,
                })?;

            runner::rollback_one(&self.db, record_id, down_sql).await?;

            rolled.push(MigrationRecord {
                id: record_id,
                version,
                name,
                source: source.to_owned(),
                applied_at: chrono::Utc::now(),
                execution_time_ms: 0,
                status: crate::record::MigrationStatus::RolledBack,
            });
        }
        Ok(rolled)
    }

    /// 返回指定 `source` 当前处于 `applied` 状态的迁移数量。
    ///
    /// # Errors
    /// 元表查询失败时返回错误。
    pub async fn applied_count(&self, source: &str) -> Result<usize> {
        self.ensure_meta_table().await?;
        let versions = runner::list_applied_versions(&self.db, source).await?;
        Ok(versions.len())
    }

    /// 返回指定 `source` 当前仍处于 `applied` 状态的迁移版本列表。
    ///
    /// # Errors
    /// 元表查询失败时返回错误。
    pub async fn applied_versions(&self, source: &str) -> Result<Vec<i64>> {
        self.ensure_meta_table().await?;
        runner::list_applied_versions(&self.db, source).await
    }

    /// 返回指定 `source` 在某个迁移根目录下、当前仍处于 `applied` 状态的版本列表。
    ///
    /// 该方法会使用与 [`MigrationEngine::rollback`] 相同的方言目录解析规则，只统计
    /// 当前数据库方言实际会读取到的迁移文件。
    ///
    /// # Errors
    /// 元表查询或迁移目录扫描失败时返回错误。
    pub async fn applied_versions_in_dir(
        &self,
        source: &str,
        migrations_root: &Path,
    ) -> Result<Vec<i64>> {
        self.ensure_meta_table().await?;

        let applied: HashSet<i64> = runner::list_applied_versions(&self.db, source)
            .await?
            .into_iter()
            .collect();
        if applied.is_empty() {
            return Ok(Vec::new());
        }

        let dir = resolve_driver_dir(self.db.db_type(), migrations_root);
        let mut versions: Vec<i64> = discovery::scan(&dir)?
            .into_iter()
            .map(|migration| migration.version)
            .filter(|version| applied.contains(version))
            .collect();
        versions.sort_unstable();
        Ok(versions)
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
