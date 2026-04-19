use std::fs;
use std::path::PathBuf;

use cycms_config::DatabaseDriver;
use cycms_db::DatabasePool;
use cycms_events::{DEFAULT_CHANNEL_CAPACITY, Event, EventKind};
use cycms_kernel::Kernel;
use tempfile::tempdir;

fn workspace_system_migrations() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../cycms-migrate/migrations/system")
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

    // AuthEngine 已挂入上下文：count_users 应可调用并返回 0
    assert_eq!(ctx.auth_engine.users().count().await.unwrap(), 0);

    // PermissionEngine 已挂入上下文：空迁移环境下 roles 表可读，初始零行
    assert!(
        ctx.permission_engine
            .roles()
            .list()
            .await
            .unwrap()
            .is_empty()
    );

    // EventBus 已挂入上下文：默认容量生效，无订阅者时 publish 静默 no-op
    assert_eq!(ctx.event_bus.capacity(), DEFAULT_CHANNEL_CAPACITY);
    ctx.event_bus.publish(Event::new(EventKind::ContentCreated));
    assert_eq!(ctx.event_bus.receiver_count(&EventKind::ContentCreated), 0);
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
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table'")
        .fetch_one(inner)
        .await
        .unwrap();
    assert_eq!(count, 0, "no migrations run => no tables");
}
