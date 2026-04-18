use std::fs;
use std::sync::Arc;

use cycms_config::{DatabaseConfig, DatabaseDriver};
use cycms_db::DatabasePool;
use cycms_migrate::MigrationEngine;
use tempfile::tempdir;

fn sqlite_memory_config() -> DatabaseConfig {
    DatabaseConfig {
        driver: DatabaseDriver::Sqlite,
        url: "sqlite::memory:".to_owned(),
        max_connections: 1,
        connect_timeout_secs: 5,
        idle_timeout_secs: 60,
    }
}

async fn build_engine() -> (Arc<DatabasePool>, MigrationEngine) {
    let pool = Arc::new(
        DatabasePool::connect(&sqlite_memory_config())
            .await
            .expect("sqlite connect"),
    );
    let engine = MigrationEngine::new(Arc::clone(&pool));
    (pool, engine)
}

#[tokio::test]
async fn run_system_migrations_applies_pending_files_in_version_order() {
    let root = tempdir().unwrap();
    let sqlite_dir = root.path().join("sqlite");
    fs::create_dir(&sqlite_dir).unwrap();
    fs::write(
        sqlite_dir.join("20260101000001_first.up.sql"),
        "CREATE TABLE sample_a (id INTEGER PRIMARY KEY);",
    )
    .unwrap();
    fs::write(
        sqlite_dir.join("20260101000002_second.up.sql"),
        "CREATE TABLE sample_b (id INTEGER PRIMARY KEY);",
    )
    .unwrap();

    let (pool, engine) = build_engine().await;

    let applied = engine.run_system_migrations(root.path()).await.unwrap();
    assert_eq!(applied.len(), 2);
    assert_eq!(applied[0].version, 20_260_101_000_001);
    assert_eq!(applied[1].version, 20_260_101_000_002);

    // 第二次应幂等，无新迁移执行。
    let rerun = engine.run_system_migrations(root.path()).await.unwrap();
    assert!(rerun.is_empty());

    let DatabasePool::Sqlite(inner) = pool.as_ref() else {
        panic!("expected sqlite pool");
    };
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM migration_records WHERE source='system'")
            .fetch_one(inner)
            .await
            .unwrap();
    assert_eq!(count.0, 2);
}

#[tokio::test]
async fn run_system_migrations_rolls_back_on_failure_and_stops() {
    let root = tempdir().unwrap();
    let sqlite_dir = root.path().join("sqlite");
    fs::create_dir(&sqlite_dir).unwrap();
    fs::write(
        sqlite_dir.join("20260101000001_ok.up.sql"),
        "CREATE TABLE sample_ok (id INTEGER PRIMARY KEY);",
    )
    .unwrap();
    fs::write(
        sqlite_dir.join("20260101000002_broken.up.sql"),
        "CREATE TABLE sample_ok (id INTEGER PRIMARY KEY);", // 故意与上一迁移冲突
    )
    .unwrap();

    let (pool, engine) = build_engine().await;
    let err = engine.run_system_migrations(root.path()).await;
    assert!(err.is_err(), "second migration must fail on duplicate table");

    let DatabasePool::Sqlite(inner) = pool.as_ref() else {
        panic!("expected sqlite pool");
    };
    // 第一条成功（在独立事务里提交），第二条失败且未写入记录。
    let versions: Vec<(i64,)> =
        sqlx::query_as("SELECT version FROM migration_records WHERE source='system' ORDER BY version")
            .fetch_all(inner)
            .await
            .unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].0, 20_260_101_000_001);
}
