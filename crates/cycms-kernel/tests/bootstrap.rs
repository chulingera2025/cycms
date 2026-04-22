use std::fs;
use std::path::PathBuf;

use cycms_auth::AuthEngine;
use cycms_config::DatabaseDriver;
use cycms_content_engine::{ContentEngine, ContentQuery};
use cycms_content_model::{ContentModelRegistry, ContentTypeKind};
use cycms_db::DatabasePool;
use cycms_events::{DEFAULT_CHANNEL_CAPACITY, Event, EventBus, EventKind};
use cycms_kernel::Kernel;
use cycms_media::MediaManager;
use cycms_permission::PermissionEngine;
use cycms_publish::PublishManager;
use cycms_revision::RevisionManager;
use cycms_settings::SettingsManager;
use serde_json::json;
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
[server]
host = "127.0.0.1"

[database]
driver = "sqlite"
url = "sqlite::memory:"
max_connections = 1
connect_timeout_secs = 5
idle_timeout_secs = 60

[events]
channel_capacity = 128
handler_timeout_secs = 2

[observability]
audit_enabled = false
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

    // EventBus 已挂入上下文：配置值生效，无订阅者时 publish 静默 no-op
    assert_ne!(ctx.event_bus.capacity(), DEFAULT_CHANNEL_CAPACITY);
    assert_eq!(ctx.event_bus.capacity(), 128);
    assert_eq!(ctx.event_bus.handler_timeout().as_secs(), 2);
    ctx.event_bus.publish(Event::new(EventKind::ContentCreated));
    assert_eq!(ctx.event_bus.receiver_count(&EventKind::ContentCreated), 0);

    // SettingsManager 已挂入上下文：set 后 get 读回一致
    ctx.settings_manager
        .set("system", "locale", json!("en_US"))
        .await
        .unwrap();
    assert_eq!(
        ctx.settings_manager.get("system", "locale").await.unwrap(),
        Some(json!("en_US"))
    );

    // ServiceRegistry 已挂入上下文：核心服务批量注册且可按 typed key 查询
    let keys = ctx.service_registry.list_by_prefix("system");
    assert_eq!(
        keys,
        vec![
            "system.auth",
            "system.content_engine",
            "system.content_model",
            "system.db",
            "system.events",
            "system.media",
            "system.permission",
            "system.plugin_manager",
            "system.publish",
            "system.revision",
            "system.settings",
        ]
    );
    ctx.service_registry
        .get::<AuthEngine>("system.auth")
        .unwrap();
    ctx.service_registry
        .get::<PermissionEngine>("system.permission")
        .unwrap();
    ctx.service_registry
        .get::<EventBus>("system.events")
        .unwrap();
    ctx.service_registry
        .get::<SettingsManager>("system.settings")
        .unwrap();
    ctx.service_registry
        .get::<DatabasePool>("system.db")
        .unwrap();
    ctx.service_registry
        .get::<ContentModelRegistry>("system.content_model")
        .unwrap();
    ctx.service_registry
        .get::<ContentEngine>("system.content_engine")
        .unwrap();
    ctx.service_registry
        .get::<RevisionManager>("system.revision")
        .unwrap();
    ctx.service_registry
        .get::<PublishManager>("system.publish")
        .unwrap();
    ctx.service_registry
        .get::<MediaManager>("system.media")
        .unwrap();

    // ContentModel 已挂入上下文并完成默认博客预设。
    let category = ctx
        .content_model
        .get_type("category")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(category.kind, ContentTypeKind::Collection);
    let page = ctx.content_model.get_type("page").await.unwrap().unwrap();
    assert_eq!(page.kind, ContentTypeKind::Collection);
    let post = ctx.content_model.get_type("post").await.unwrap().unwrap();
    assert_eq!(post.kind, ContentTypeKind::Collection);
    let site_settings = ctx
        .content_model
        .get_type("site_settings")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(site_settings.kind, ContentTypeKind::Single);

    // ContentEngine 已挂入上下文：可对 seeded 类型执行空 list 查询
    let res = ctx
        .content_engine
        .list("post", &ContentQuery::default())
        .await
        .unwrap();
    assert_eq!(res.meta.total, 0);
    assert!(res.data.is_empty());
}

#[tokio::test]
async fn bootstrap_without_migrations_does_not_create_tables() {
    let temp = tempdir().unwrap();
    let config_path = temp.path().join("cycms.toml");
    fs::write(
        &config_path,
        r#"
[server]
host = "127.0.0.1"

[database]
driver = "sqlite"
url = "sqlite::memory:"
max_connections = 1
connect_timeout_secs = 5
idle_timeout_secs = 60

[observability]
audit_enabled = false
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
