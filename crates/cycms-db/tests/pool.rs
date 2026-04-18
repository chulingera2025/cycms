use cycms_config::{DatabaseConfig, DatabaseDriver};
use cycms_db::{DatabasePool, DatabaseType};
use sqlx::Row;

fn sqlite_memory_config() -> DatabaseConfig {
    DatabaseConfig {
        driver: DatabaseDriver::Sqlite,
        url: "sqlite::memory:".to_owned(),
        max_connections: 1,
        connect_timeout_secs: 5,
        idle_timeout_secs: 60,
    }
}

#[tokio::test]
async fn sqlite_memory_pool_executes_simple_query() {
    let pool = DatabasePool::connect(&sqlite_memory_config())
        .await
        .expect("sqlite in-memory connection should succeed");

    assert_eq!(pool.db_type(), DatabaseType::Sqlite);

    let DatabasePool::Sqlite(inner) = &pool else {
        panic!("expected sqlite pool");
    };
    let row = sqlx::query("SELECT 1 AS n")
        .fetch_one(inner)
        .await
        .expect("select 1 should succeed");
    let value: i64 = row.get("n");
    assert_eq!(value, 1);
}
