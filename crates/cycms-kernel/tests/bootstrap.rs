use std::fs;
use std::path::PathBuf;

use cycms_config::DatabaseDriver;
use cycms_db::DatabasePool;
use cycms_kernel::Kernel;
use tempfile::tempdir;

fn workspace_system_migrations() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../cycms-migrate/migrations/system")
}

#[tokio::test]
async fn bootstrap_with_migrations_creates_core_tables() {
    let temp = tempdir().unwrap();
    let config_path = temp.path().join("cycms.toml");
    fs::write(
        &config_path,
        r#"
[database]
driver = "sqlite"
url = "sqlite::memory:"
max_connections = 1
connect_timeout_secs = 5
idle_timeout_secs = 60
"#,
    )
    .unwrap();

    let kernel = Kernel::build(Some(&config_path)).await.unwrap();
    let ctx = kernel
        .bootstrap(Some(&workspace_system_migrations()))
        .await
        .unwrap();

    assert_eq!(ctx.config.database.driver, DatabaseDriver::Sqlite);

    let DatabasePool::Sqlite(inner) = ctx.db.as_ref() else {
        panic!("expected sqlite pool");
    };
    let (count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='users'")
            .fetch_one(inner)
            .await
            .unwrap();
    assert_eq!(count, 1, "users table must exist after bootstrap migration");
}

#[tokio::test]
async fn bootstrap_without_migrations_does_not_create_tables() {
    let temp = tempdir().unwrap();
    let config_path = temp.path().join("cycms.toml");
    fs::write(
        &config_path,
        r#"
[database]
driver = "sqlite"
url = "sqlite::memory:"
max_connections = 1
connect_timeout_secs = 5
idle_timeout_secs = 60
"#,
    )
    .unwrap();

    let kernel = Kernel::build(Some(&config_path)).await.unwrap();
    let ctx = kernel.bootstrap(None).await.unwrap();

    let DatabasePool::Sqlite(inner) = ctx.db.as_ref() else {
        panic!("expected sqlite pool");
    };
    let (count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table'")
            .fetch_one(inner)
            .await
            .unwrap();
    assert_eq!(count, 0, "no migrations run => no tables");
}
