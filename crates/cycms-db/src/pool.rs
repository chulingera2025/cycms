use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use cycms_config::{DatabaseConfig, DatabaseDriver};
use cycms_core::{Error, Result};
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
                ensure_sqlite_parent_dir(&config.url)?;

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

/// 保证文件型 `SQLite` URL 的父目录存在，避免 sqlx `create_if_missing` 因缺目录失败。
///
/// `:memory:` 及其变体、以及无父目录的相对文件名会直接跳过。
fn ensure_sqlite_parent_dir(url: &str) -> Result<()> {
    if url.contains(":memory:") {
        return Ok(());
    }

    let path_part = url
        .strip_prefix("sqlite://")
        .or_else(|| url.strip_prefix("sqlite:"))
        .unwrap_or(url);
    let path_part = path_part.split('?').next().unwrap_or("");
    if path_part.is_empty() {
        return Ok(());
    }

    let Some(parent) = Path::new(path_part).parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }

    fs::create_dir_all(parent).map_err(|source| Error::Internal {
        message: format!(
            "failed to create sqlite parent directory: {}",
            parent.display()
        ),
        source: Some(Box::new(source)),
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::process;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::ensure_sqlite_parent_dir;

    static NEXT_TEMP_ID: AtomicUsize = AtomicUsize::new(0);

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let unique_id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("cycms-db-tests-{}-{unique_id}", process::id()));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn creates_missing_parent_directory_for_file_url() {
        let temp = TempDir::new();
        let target_dir = temp.path.join("nested/inner");
        let url = format!("sqlite://{}/cycms.db", target_dir.display());

        ensure_sqlite_parent_dir(&url).unwrap();

        assert!(target_dir.is_dir());
    }

    #[test]
    fn strips_query_parameters_before_computing_parent() {
        let temp = TempDir::new();
        let target_dir = temp.path.join("with-query");
        let url = format!(
            "sqlite://{}/cycms.db?mode=rwc&cache=shared",
            target_dir.display()
        );

        ensure_sqlite_parent_dir(&url).unwrap();

        assert!(target_dir.is_dir());
    }

    #[test]
    fn skips_in_memory_urls() {
        for url in [
            "sqlite::memory:",
            "sqlite://:memory:",
            "sqlite://:memory:?cache=shared",
        ] {
            ensure_sqlite_parent_dir(url).unwrap();
        }
    }

    #[test]
    fn no_op_when_path_has_no_parent() {
        ensure_sqlite_parent_dir("sqlite://cycms.db").unwrap();
        ensure_sqlite_parent_dir("sqlite:cycms.db").unwrap();
    }
}
