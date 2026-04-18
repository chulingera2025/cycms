use std::fs;
use std::sync::Arc;

use cycms_config::{DatabaseConfig, DatabaseDriver};
use cycms_db::DatabasePool;
use cycms_migrate::MigrationEngine;
use tempfile::{TempDir, tempdir};

fn sqlite_memory_config() -> DatabaseConfig {
    DatabaseConfig {
        driver: DatabaseDriver::Sqlite,
        url: "sqlite::memory:".to_owned(),
        max_connections: 1,
        connect_timeout_secs: 5,
        idle_timeout_secs: 60,
    }
}

fn write_migration(
    dir: &TempDir,
    up_name: &str,
    up_sql: &str,
    down_name: Option<&str>,
    down_sql: Option<&str>,
) {
    let sqlite_dir = dir.path().join("sqlite");
    fs::create_dir_all(&sqlite_dir).unwrap();
    fs::write(sqlite_dir.join(up_name), up_sql).unwrap();
    if let (Some(n), Some(s)) = (down_name, down_sql) {
        fs::write(sqlite_dir.join(n), s).unwrap();
    }
}

#[tokio::test]
async fn plugin_migrations_are_tracked_independently_from_system() {
    let pool = Arc::new(DatabasePool::connect(&sqlite_memory_config()).await.unwrap());
    let engine = MigrationEngine::new(Arc::clone(&pool));

    let system_root = tempdir().unwrap();
    write_migration(
        &system_root,
        "20260101000001_sys.up.sql",
        "CREATE TABLE sys_t (id INTEGER PRIMARY KEY);",
        Some("20260101000001_sys.down.sql"),
        Some("DROP TABLE sys_t;"),
    );

    let plugin_root = tempdir().unwrap();
    // 插件故意使用与系统相同的 version，验证按 source 独立追踪。
    write_migration(
        &plugin_root,
        "20260101000001_plugin.up.sql",
        "CREATE TABLE plugin_t (id INTEGER PRIMARY KEY);",
        Some("20260101000001_plugin.down.sql"),
        Some("DROP TABLE plugin_t;"),
    );

    let sys_applied = engine
        .run_system_migrations(system_root.path())
        .await
        .unwrap();
    assert_eq!(sys_applied.len(), 1);

    let plugin_applied = engine
        .run_plugin_migrations("demo", plugin_root.path())
        .await
        .unwrap();
    assert_eq!(plugin_applied.len(), 1);

    let DatabasePool::Sqlite(inner) = pool.as_ref() else {
        panic!("expected sqlite pool");
    };
    let sources: Vec<(String,)> =
        sqlx::query_as("SELECT DISTINCT source FROM migration_records ORDER BY source")
            .fetch_all(inner)
            .await
            .unwrap();
    let sources: Vec<String> = sources.into_iter().map(|t| t.0).collect();
    assert_eq!(sources, vec!["demo".to_owned(), "system".to_owned()]);
}

#[tokio::test]
async fn rollback_reverses_latest_n_migrations() {
    let pool = Arc::new(DatabasePool::connect(&sqlite_memory_config()).await.unwrap());
    let engine = MigrationEngine::new(Arc::clone(&pool));

    let root = tempdir().unwrap();
    write_migration(
        &root,
        "20260101000001_first.up.sql",
        "CREATE TABLE t1 (id INTEGER PRIMARY KEY);",
        Some("20260101000001_first.down.sql"),
        Some("DROP TABLE t1;"),
    );
    write_migration(
        &root,
        "20260101000002_second.up.sql",
        "CREATE TABLE t2 (id INTEGER PRIMARY KEY);",
        Some("20260101000002_second.down.sql"),
        Some("DROP TABLE t2;"),
    );

    engine.run_system_migrations(root.path()).await.unwrap();

    let rolled = engine
        .rollback("system", root.path(), 1)
        .await
        .unwrap();
    assert_eq!(rolled.len(), 1);
    assert_eq!(rolled[0].version, 20_260_101_000_002);

    let DatabasePool::Sqlite(inner) = pool.as_ref() else {
        panic!("expected sqlite pool");
    };
    // t2 应被删除，t1 仍存在。
    let table_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('t1','t2')",
    )
    .fetch_one(inner)
    .await
    .unwrap();
    assert_eq!(table_count.0, 1);

    let statuses: Vec<(String,)> = sqlx::query_as(
        "SELECT status FROM migration_records WHERE source='system' ORDER BY version",
    )
    .fetch_all(inner)
    .await
    .unwrap();
    let statuses: Vec<String> = statuses.into_iter().map(|t| t.0).collect();
    assert_eq!(statuses, vec!["applied".to_owned(), "rolled_back".to_owned()]);
}

#[tokio::test]
async fn rollback_refuses_when_down_sql_missing() {
    let pool = Arc::new(DatabasePool::connect(&sqlite_memory_config()).await.unwrap());
    let engine = MigrationEngine::new(Arc::clone(&pool));

    let root = tempdir().unwrap();
    write_migration(
        &root,
        "20260101000001_nodown.up.sql",
        "CREATE TABLE nodown (id INTEGER PRIMARY KEY);",
        None,
        None,
    );

    engine.run_system_migrations(root.path()).await.unwrap();
    let err = engine.rollback("system", root.path(), 1).await.unwrap_err();
    assert!(err.to_string().contains("has no .down.sql"));
}
