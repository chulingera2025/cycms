use cycms_config::DatabaseConfig;
use cycms_core::Result;
use sqlx::{MySqlPool, PgPool, SqlitePool};

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

    /// 从配置创建连接池。
    ///
    /// # Errors
    /// TODO!!! MIG-1 之前仅返回未实现错误占位，DB-2 步骤实现真实连接逻辑。
    #[allow(clippy::unused_async)]
    pub async fn connect(_config: &DatabaseConfig) -> Result<Self> {
        todo!("TODO!!!: DB-2 实现三方言连接池创建")
    }
}
