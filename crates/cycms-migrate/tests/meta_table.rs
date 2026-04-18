use std::sync::Arc;

use cycms_config::{DatabaseConfig, DatabaseDriver};
use cycms_db::DatabasePool;
use cycms_migrate::MigrationEngine;

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
async fn ensure_meta_table_creates_migration_records() {
    let pool = Arc::new(
        DatabasePool::connect(&sqlite_memory_config())
            .await
            .expect("sqlite connect"),
    );
    let engine = MigrationEngine::new(Arc::clone(&pool));

    engine
        .ensure_meta_table()
        .await
        .expect("first call should succeed");
    // 幂等：第二次不应报错。
    engine
        .ensure_meta_table()
        .await
        .expect("second call should be idempotent");

    let DatabasePool::Sqlite(inner) = pool.as_ref() else {
        panic!("expected sqlite pool");
    };
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='migration_records'",
    )
    .fetch_one(inner)
    .await
    .expect("query sqlite_master");
    assert_eq!(row.0, 1);
}
