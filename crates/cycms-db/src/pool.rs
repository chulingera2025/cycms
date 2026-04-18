use std::str::FromStr;
use std::time::Duration;

use cycms_config::{DatabaseConfig, DatabaseDriver};
use cycms_core::Result;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{MySqlPool, PgPool, SqlitePool};

use crate::error::map_sqlx_error;

/// 多数据库连接池统一枚举。
///
/// 对外只暴露方言标识与执行入口，具体数据库差异在 [`DatabaseType`] 驱动下由上层 API 决策。
#[derive(Debug, Clone)]
pub enum DatabasePool {
    Postgres(PgPool),
    MySql(MySqlPool),
    Sqlite(SqlitePool),
}

/// 当前连接池的方言标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseType {
    Postgres,
    MySql,
    Sqlite,
}

impl DatabasePool {
    /// 返回底层方言标识，用于路由到正确的 DDL/JSON 辅助实现。
    pub fn db_type(&self) -> DatabaseType {
        match self {
            Self::Postgres(_) => DatabaseType::Postgres,
            Self::MySql(_) => DatabaseType::MySql,
            Self::Sqlite(_) => DatabaseType::Sqlite,
        }
    }

    /// 根据配置中的驱动类型建立连接池。
    ///
    /// `connect_timeout_secs` 对应 sqlx 的 `acquire_timeout`，因为 sqlx 池在 acquire
    /// 时才执行实际连接动作，二者语义天然对齐；`idle_timeout_secs` 对应 `idle_timeout`。
    ///
    /// # Errors
    /// 底层 sqlx 连接失败时返回 [`cycms_core::Error`]，详见 [`map_sqlx_error`]。
    pub async fn connect(config: &DatabaseConfig) -> Result<Self> {
        let acquire_timeout = Duration::from_secs(config.connect_timeout_secs);
        let idle_timeout = Duration::from_secs(config.idle_timeout_secs);

        match config.driver {
            DatabaseDriver::Postgres => {
                let pool = PgPoolOptions::new()
                    .max_connections(config.max_connections)
                    .acquire_timeout(acquire_timeout)
                    .idle_timeout(idle_timeout)
                    .connect(&config.url)
                    .await
                    .map_err(map_sqlx_error)?;
                Ok(Self::Postgres(pool))
            }
            DatabaseDriver::MySql => {
                let pool = MySqlPoolOptions::new()
                    .max_connections(config.max_connections)
                    .acquire_timeout(acquire_timeout)
                    .idle_timeout(idle_timeout)
                    .connect(&config.url)
                    .await
                    .map_err(map_sqlx_error)?;
                Ok(Self::MySql(pool))
            }
            DatabaseDriver::Sqlite => {
                // WAL + create_if_missing 兼顾文件库与 `sqlite::memory:`；内存模式会忽略持久化选项。
                let connect_options = SqliteConnectOptions::from_str(&config.url)
                    .map_err(map_sqlx_error)?
                    .create_if_missing(true)
                    .foreign_keys(true)
                    .journal_mode(SqliteJournalMode::Wal)
                    .synchronous(SqliteSynchronous::Normal);

                let pool = SqlitePoolOptions::new()
                    .max_connections(config.max_connections)
                    .acquire_timeout(acquire_timeout)
                    .idle_timeout(idle_timeout)
                    .connect_with(connect_options)
                    .await
                    .map_err(map_sqlx_error)?;
                Ok(Self::Sqlite(pool))
            }
        }
    }
}
