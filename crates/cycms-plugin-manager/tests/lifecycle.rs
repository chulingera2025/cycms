//! `PluginManager` 生命周期集成测试。
//!
//! 使用 [`MockRuntime`] 替代 Native / Wasm 运行时，
//! 覆盖：
//! - install → enable → disable → uninstall 的完整状态机
//! - 权限点随 install 落库、随 uninstall 清理
//! - 反向依赖存在时 `disable(force=false)` 被拒绝，`force=true` 级联
//! - `ServiceRegistry` 可用性随 enable / disable 切换

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use cycms_auth::AuthEngine;
use cycms_config::{AuthConfig, ContentConfig, DatabaseConfig, DatabaseDriver, MediaConfig};
use cycms_content_engine::ContentEngine;
use cycms_content_model::{ContentModelRegistry, FieldTypeRegistry};
use cycms_core::Result;
use cycms_db::DatabasePool;
use cycms_events::EventBus;
use cycms_media::MediaManager;
use cycms_migrate::MigrationEngine;
use cycms_permission::PermissionEngine;
use cycms_plugin_api::{PluginContext, ServiceRegistry};
use cycms_plugin_manager::{
    PluginKind, PluginManager, PluginManagerConfig, PluginManifest, PluginRuntime, PluginStatus,
    scan_plugins_dir,
};
use cycms_publish::PublishManager;
use cycms_revision::RevisionManager;
use cycms_settings::SettingsManager;
use cycms_permission::NewRoleRow;
use semver::Version;
use tempfile::{TempDir, tempdir};

fn workspace_system_migrations() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../cycms-migrate/migrations/system")
}

/// 记录 `load` / `unload` 调用顺序的替身 runtime，供状态机断言。
struct MockRuntime {
    kind: PluginKind,
    loaded: Mutex<Vec<String>>,
}

impl MockRuntime {
    fn new(kind: PluginKind) -> Self {
        Self {
            kind,
            loaded: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl PluginRuntime for MockRuntime {
    fn kind(&self) -> PluginKind {
        self.kind
    }

    async fn load(
        &self,
        manifest: &PluginManifest,
        _entry: &Path,
        _ctx: Arc<PluginContext>,
    ) -> Result<()> {
        self.loaded
            .lock()
            .unwrap()
            .push(manifest.plugin.name.clone());
        Ok(())
    }

    async fn unload(&self, name: &str) -> Result<()> {
        self.loaded.lock().unwrap().retain(|n| n != name);
        Ok(())
    }

    fn loaded_plugins(&self) -> Vec<String> {
        self.loaded.lock().unwrap().clone()
    }
}

struct Harness {
    _tmp: TempDir,
    pool: Arc<DatabasePool>,
    plugins_root: PathBuf,
    manager: PluginManager,
    mock: Arc<MockRuntime>,
    permission_engine: Arc<PermissionEngine>,
    service_registry: Arc<ServiceRegistry>,
}

async fn fresh_harness() -> Harness {
    let tmp = tempdir().unwrap();
    let plugins_root = tmp.path().to_owned();

    let pool = Arc::new(
        DatabasePool::connect(&DatabaseConfig {
            driver: DatabaseDriver::Sqlite,
            url: "sqlite::memory:".into(),
            max_connections: 1,
            connect_timeout_secs: 5,
            idle_timeout_secs: 60,
        })
        .await
        .unwrap(),
    );
    let migration_engine = Arc::new(MigrationEngine::new(Arc::clone(&pool)));
    migration_engine
        .run_system_migrations(&workspace_system_migrations())
        .await
        .unwrap();

    let event_bus = Arc::new(EventBus::new());
    let permission_engine = Arc::new(PermissionEngine::new(Arc::clone(&pool)));
    let settings_manager = Arc::new(SettingsManager::new(Arc::clone(&pool)));
    let service_registry = Arc::new(ServiceRegistry::new());

    let auth_engine = Arc::new(AuthEngine::new(Arc::clone(&pool), AuthConfig::default()).unwrap());
    let field_type_registry = Arc::new(FieldTypeRegistry::new());
    let content_model = Arc::new(ContentModelRegistry::new(
        Arc::clone(&pool),
        Arc::clone(&field_type_registry),
    ));
    let revision_manager = Arc::new(RevisionManager::new(Arc::clone(&pool)));
    let publish_manager = Arc::new(PublishManager::new(&pool, Arc::clone(&event_bus)));
    let media_manager = Arc::new(MediaManager::new(
        &pool,
        Arc::clone(&event_bus),
        &MediaConfig::default(),
    ));
    let content_engine = Arc::new(ContentEngine::new(
        Arc::clone(&pool),
        Arc::clone(&content_model),
        Arc::clone(&event_bus),
        ContentConfig::default(),
        Arc::clone(&revision_manager),
    ));

    let plugin_context = Arc::new(PluginContext::new(
        Arc::clone(&pool),
        Arc::clone(&auth_engine),
        Arc::clone(&permission_engine),
        Arc::clone(&event_bus),
        Arc::clone(&settings_manager),
        Arc::clone(&content_model),
        Arc::clone(&content_engine),
        Arc::clone(&revision_manager),
        Arc::clone(&publish_manager),
        Arc::clone(&media_manager),
        Arc::clone(&service_registry),
    ));

    let mock = Arc::new(MockRuntime::new(PluginKind::Native));
    let mock_as_runtime: Arc<dyn PluginRuntime> = Arc::clone(&mock) as Arc<dyn PluginRuntime>;

    let manager = PluginManager::new(
        Arc::clone(&pool),
        migration_engine,
        Arc::clone(&permission_engine),
        Arc::clone(&settings_manager),
        Arc::clone(&service_registry),
        Arc::clone(&event_bus),
        plugin_context,
        PluginManagerConfig {
            cycms_version: Version::parse("0.1.0").unwrap(),
            plugins_root: plugins_root.clone(),
            runtimes: vec![mock_as_runtime],
        },
    );

    Harness {
        _tmp: tmp,
        pool,
        plugins_root,
        manager,
        mock,
        permission_engine,
        service_registry,
    }
}

fn write_plugin(root: &Path, name: &str, version: &str, extras: &str) {
    let dir = root.join(name);
    fs::create_dir_all(&dir).unwrap();
    let text = format!(
        r#"
[plugin]
name = "{name}"
version = "{version}"
kind = "native"
entry = "{name}.so"

[compatibility]
cycms = ">=0.1.0"
{extras}"#
    );
    fs::write(dir.join("plugin.toml"), text).unwrap();
}

#[tokio::test]
async fn install_enable_disable_uninstall_cycle() {
    let harness = fresh_harness().await;
    write_plugin(
        &harness.plugins_root,
        "blog",
        "0.1.0",
        r#"
[permissions]
definitions = [
  { domain = "blog", resource = "post", action = "create" },
]
"#,
    );
    let discovered = scan_plugins_dir(&harness.plugins_root).unwrap();
    assert_eq!(discovered.len(), 1);

    // install: 状态 disabled，runtime 未加载，权限已注册
    let info = harness.manager.install(&discovered[0]).await.unwrap();
    assert_eq!(info.status, PluginStatus::Disabled);
    assert_eq!(info.permissions, vec!["blog.post.create"]);
    assert!(harness.mock.loaded_plugins().is_empty());
    let perm_rows = harness
        .permission_engine
        .permissions()
        .list_by_source("blog")
        .await
        .unwrap();
    assert_eq!(perm_rows.len(), 1);

    // enable: runtime.load 被调用，状态翻转
    harness.manager.enable("blog").await.unwrap();
    assert_eq!(harness.mock.loaded_plugins(), vec!["blog".to_owned()]);
    let list = harness.manager.list().await.unwrap();
    assert_eq!(list[0].status, PluginStatus::Enabled);

    // disable: runtime.unload 被调用，状态回滚
    harness.manager.disable("blog", false).await.unwrap();
    assert!(harness.mock.loaded_plugins().is_empty());
    let list = harness.manager.list().await.unwrap();
    assert_eq!(list[0].status, PluginStatus::Disabled);

    // uninstall: 记录被删，权限清理
    harness.manager.uninstall("blog").await.unwrap();
    let list = harness.manager.list().await.unwrap();
    assert!(list.is_empty());
    let perm_rows = harness
        .permission_engine
        .permissions()
        .list_by_source("blog")
        .await
        .unwrap();
    assert!(perm_rows.is_empty());
}

#[tokio::test]
async fn install_rejects_duplicate() {
    let harness = fresh_harness().await;
    write_plugin(&harness.plugins_root, "blog", "0.1.0", "");
    let discovered = scan_plugins_dir(&harness.plugins_root).unwrap();

    harness.manager.install(&discovered[0]).await.unwrap();
    let err = harness.manager.install(&discovered[0]).await.unwrap_err();
    assert!(matches!(err, cycms_core::Error::Conflict { .. }));
}

#[tokio::test]
async fn enable_rejects_if_dependency_not_enabled() {
    let harness = fresh_harness().await;
    write_plugin(&harness.plugins_root, "auth", "0.1.0", "");
    write_plugin(
        &harness.plugins_root,
        "blog",
        "0.1.0",
        r#"
[dependencies]
auth = { version = "^0.1" }
"#,
    );
    let discovered = scan_plugins_dir(&harness.plugins_root).unwrap();
    for d in &discovered {
        harness.manager.install(d).await.unwrap();
    }

    // auth 未 enable，blog enable 应该失败
    let err = harness.manager.enable("blog").await.unwrap_err();
    assert!(
        matches!(err, cycms_core::Error::ValidationError { .. }),
        "got: {err:?}"
    );

    harness.manager.enable("auth").await.unwrap();
    harness.manager.enable("blog").await.unwrap();
    assert_eq!(
        harness.mock.loaded_plugins(),
        vec!["auth".to_owned(), "blog".to_owned()]
    );
}

#[tokio::test]
async fn disable_requires_force_when_dependents_exist() {
    let harness = fresh_harness().await;
    write_plugin(&harness.plugins_root, "auth", "0.1.0", "");
    write_plugin(
        &harness.plugins_root,
        "blog",
        "0.1.0",
        r#"
[dependencies]
auth = { version = "^0.1" }
"#,
    );
    let discovered = scan_plugins_dir(&harness.plugins_root).unwrap();
    for d in &discovered {
        harness.manager.install(d).await.unwrap();
    }
    harness.manager.enable("auth").await.unwrap();
    harness.manager.enable("blog").await.unwrap();

    // auth 被 blog 依赖，普通 disable 必须报 Conflict
    let err = harness.manager.disable("auth", false).await.unwrap_err();
    assert!(matches!(err, cycms_core::Error::Conflict { .. }));

    // force 级联禁用
    harness.manager.disable("auth", true).await.unwrap();
    assert!(harness.mock.loaded_plugins().is_empty());
    let list = harness.manager.list().await.unwrap();
    for info in list {
        assert_eq!(info.status, PluginStatus::Disabled);
    }
}

#[tokio::test]
async fn service_registry_toggles_with_enable_disable() {
    #[derive(Debug)]
    struct Svc;

    let harness = fresh_harness().await;
    write_plugin(&harness.plugins_root, "blog", "0.1.0", "");
    let discovered = scan_plugins_dir(&harness.plugins_root).unwrap();
    harness.manager.install(&discovered[0]).await.unwrap();

    // 注册一个命名空间在 blog.* 下的假服务，验证可用性切换
    let svc: Arc<Svc> = Arc::new(Svc);
    harness
        .service_registry
        .register("blog.render", Arc::clone(&svc))
        .unwrap();

    harness.manager.enable("blog").await.unwrap();
    harness.service_registry.get::<Svc>("blog.render").unwrap();

    harness.manager.disable("blog", false).await.unwrap();
    let err = harness
        .service_registry
        .get::<Svc>("blog.render")
        .unwrap_err();
    assert!(matches!(
        err,
        cycms_plugin_api::RegistryError::ServiceUnavailable { .. }
    ));
}

#[tokio::test]
async fn uninstall_cleans_role_permission_links() {
    let harness = fresh_harness().await;
    write_plugin(
        &harness.plugins_root,
        "blog",
        "0.1.0",
        r#"
[permissions]
definitions = [
  { domain = "blog", resource = "post", action = "create" },
]
"#,
    );
    let discovered = scan_plugins_dir(&harness.plugins_root).unwrap();
    harness.manager.install(&discovered[0]).await.unwrap();

    let role = harness
        .permission_engine
        .roles()
        .create(NewRoleRow {
            name: "reviewer".into(),
            description: None,
            is_system: false,
        })
        .await
        .unwrap();
    let permission = harness
        .permission_engine
        .permissions()
        .list_by_source("blog")
        .await
        .unwrap()
        .pop()
        .unwrap();
    harness
        .permission_engine
        .roles()
        .attach_permission(&role.id, &permission.id)
        .await
        .unwrap();

    let DatabasePool::Sqlite(pool) = harness.pool.as_ref() else {
        panic!("expected sqlite");
    };
    let before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM role_permissions WHERE role_id = ? AND permission_id = ?",
    )
    .bind(&role.id)
    .bind(&permission.id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(before, 1);

    harness.manager.uninstall("blog").await.unwrap();

    let after: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM role_permissions WHERE role_id = ? AND permission_id = ?",
    )
    .bind(&role.id)
    .bind(&permission.id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(after, 0);
}

#[tokio::test]
async fn install_rolls_back_applied_plugin_migrations_on_failure() {
    let harness = fresh_harness().await;
    let plugin_dir = harness.plugins_root.join("broken");
    fs::create_dir_all(plugin_dir.join("migrations/sqlite")).unwrap();
    fs::write(
        plugin_dir.join("plugin.toml"),
        r#"
migrations = ["migrations"]

[plugin]
name = "broken"
version = "0.1.0"
kind = "native"
entry = "broken.so"

[compatibility]
cycms = ">=0.1.0"
"#,
    )
    .unwrap();
    fs::write(
        plugin_dir.join("migrations/sqlite/20260102000001_create_demo.up.sql"),
        "CREATE TABLE broken_install_demo (id INTEGER NOT NULL);",
    )
    .unwrap();
    fs::write(
        plugin_dir.join("migrations/sqlite/20260102000001_create_demo.down.sql"),
        "DROP TABLE IF EXISTS broken_install_demo;",
    )
    .unwrap();
    fs::write(
        plugin_dir.join("migrations/sqlite/20260102000002_fail.up.sql"),
        "THIS IS NOT VALID SQL;",
    )
    .unwrap();
    fs::write(
        plugin_dir.join("migrations/sqlite/20260102000002_fail.down.sql"),
        "SELECT 1;",
    )
    .unwrap();

    let discovered = scan_plugins_dir(&harness.plugins_root).unwrap();
    let err = harness.manager.install(&discovered[0]).await.unwrap_err();
    assert!(matches!(err, cycms_core::Error::Internal { .. }), "got: {err:?}");

    let DatabasePool::Sqlite(pool) = harness.pool.as_ref() else {
        panic!("expected sqlite");
    };
    let applied_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM migration_records WHERE source = ? AND status = 'applied'",
    )
    .bind("broken")
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(applied_count, 0);

    let demo_table_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'broken_install_demo'",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(demo_table_count, 0);
    assert!(harness.manager.list().await.unwrap().is_empty());
}

#[tokio::test]
async fn install_blocks_when_host_incompatible() {
    let harness = fresh_harness().await;
    write_plugin(&harness.plugins_root, "future", "9.0.0", "");
    // 覆盖掉 compatibility 让它声明只兼容未来版本
    let plugin_toml = harness.plugins_root.join("future/plugin.toml");
    let text = fs::read_to_string(&plugin_toml)
        .unwrap()
        .replace(r#"cycms = ">=0.1.0""#, r#"cycms = ">=9.0.0""#);
    fs::write(&plugin_toml, text).unwrap();

    let discovered = scan_plugins_dir(&harness.plugins_root).unwrap();
    let err = harness.manager.install(&discovered[0]).await.unwrap_err();
    assert!(matches!(err, cycms_core::Error::ValidationError { .. }));
}
