//! `NativePluginRuntime` 生命周期集成测试。
//!
//! 用真实的 `PluginManager` + `NativePluginRuntime` 装配一条完整链路：
//! - install / enable：触发 `on_enable`、事件订阅、service 注册、route 收集
//! - disable：触发 `on_disable`、订阅 abort、service 注销
//! - `routes_of` / `all_routes`：Router 暴露能力
//! - re-enable：确认 runtime 状态可二次复用

use std::any::Any;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use axum::Router;
use axum::routing::get;
use cycms_auth::AuthEngine;
use cycms_config::{AuthConfig, ContentConfig, DatabaseConfig, DatabaseDriver, MediaConfig};
use cycms_content_engine::ContentEngine;
use cycms_content_model::{ContentModelRegistry, FieldTypeRegistry};
use cycms_core::Result;
use cycms_db::DatabasePool;
use cycms_events::{Event, EventBus, EventHandler, EventKind};
use cycms_media::MediaManager;
use cycms_migrate::MigrationEngine;
use cycms_permission::PermissionEngine;
use cycms_plugin_api::{Plugin, PluginContext, ServiceRegistry};
use cycms_plugin_manager::{
    PluginManager, PluginManagerConfig, PluginRuntime, PluginStatus, scan_plugins_dir,
};
use cycms_plugin_native::NativePluginRuntime;
use cycms_publish::PublishManager;
use cycms_revision::RevisionManager;
use cycms_settings::SettingsManager;
use semver::Version;
use tempfile::{TempDir, tempdir};

fn workspace_system_migrations() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../cycms-migrate/migrations/system")
}

struct CountingHandler {
    name: String,
    counter: Arc<AtomicU64>,
}

#[async_trait]
impl EventHandler for CountingHandler {
    fn name(&self) -> &str {
        &self.name
    }

    async fn handle(&self, _event: Arc<Event>) -> Result<()> {
        self.counter.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

/// 测试插件：每次 enable / disable 翻转 `enabled` 标志；
/// 订阅 `UserCreated`，handler 命中时递增 `event_counter`；
/// 暴露 `echo.counter` 服务（即上述 `AtomicU64`）；
/// 贡献一个 `/ping` 路由。
struct EchoPlugin {
    name: String,
    enabled: Arc<AtomicBool>,
    event_counter: Arc<AtomicU64>,
}

impl EchoPlugin {
    fn new(name: &str) -> Arc<Self> {
        Arc::new(Self {
            name: name.to_owned(),
            enabled: Arc::new(AtomicBool::new(false)),
            event_counter: Arc::new(AtomicU64::new(0)),
        })
    }

    fn enabled_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.enabled)
    }

    fn event_counter(&self) -> Arc<AtomicU64> {
        Arc::clone(&self.event_counter)
    }
}

#[async_trait]
impl Plugin for EchoPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &'static str {
        "0.1.0"
    }

    async fn on_enable(&self, _ctx: &PluginContext) -> Result<()> {
        self.enabled.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn on_disable(&self, _ctx: &PluginContext) -> Result<()> {
        self.enabled.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn routes(&self) -> Option<Router> {
        Some(Router::new().route("/ping", get(|| async { "pong" })))
    }

    fn event_handlers(&self) -> Vec<(EventKind, Arc<dyn EventHandler>)> {
        let handler: Arc<dyn EventHandler> = Arc::new(CountingHandler {
            name: format!("{}.counter", self.name),
            counter: Arc::clone(&self.event_counter),
        });
        vec![(EventKind::UserCreated, handler)]
    }

    fn services(&self) -> Vec<(String, Arc<dyn Any + Send + Sync>)> {
        let svc: Arc<dyn Any + Send + Sync> = self.event_counter.clone();
        vec![("counter".to_owned(), svc)]
    }
}

struct Harness {
    _tmp: TempDir,
    plugins_root: PathBuf,
    manager: PluginManager,
    runtime: Arc<NativePluginRuntime>,
    service_registry: Arc<ServiceRegistry>,
    event_bus: Arc<EventBus>,
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

    let runtime = Arc::new(NativePluginRuntime::new());
    let runtime_as_trait: Arc<dyn PluginRuntime> =
        Arc::clone(&runtime) as Arc<dyn PluginRuntime>;

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
            runtimes: vec![runtime_as_trait],
        },
    );

    Harness {
        _tmp: tmp,
        plugins_root,
        manager,
        runtime,
        service_registry,
        event_bus,
    }
}

fn write_plugin(root: &Path, name: &str, version: &str) {
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
"#
    );
    fs::write(dir.join("plugin.toml"), text).unwrap();
}

async fn wait_until(counter: &Arc<AtomicU64>, target: u64) {
    for _ in 0..100 {
        if counter.load(Ordering::SeqCst) >= target {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("counter never reached {target}");
}

#[tokio::test]
async fn lifecycle_subscribes_events_and_exposes_services_and_routes() {
    let harness = fresh_harness().await;
    write_plugin(&harness.plugins_root, "echo", "0.1.0");

    let plugin = EchoPlugin::new("echo");
    let enabled_flag = plugin.enabled_flag();
    let event_counter = plugin.event_counter();
    harness
        .runtime
        .register_plugin(Arc::clone(&plugin) as Arc<dyn Plugin>);

    let discovered = scan_plugins_dir(&harness.plugins_root).unwrap();
    harness.manager.install(&discovered[0]).await.unwrap();
    assert!(harness.runtime.loaded_plugins().is_empty());
    assert!(!enabled_flag.load(Ordering::SeqCst));

    // enable: on_enable 触发 + 订阅 + 服务 + 路由 都就位
    harness.manager.enable("echo").await.unwrap();
    assert!(enabled_flag.load(Ordering::SeqCst));
    assert_eq!(harness.runtime.loaded_plugins(), vec!["echo".to_owned()]);

    // 服务命名空间已注册；PluginManager.enable 也已 set_available
    let svc: Arc<AtomicU64> = harness.service_registry.get("echo.counter").unwrap();
    assert!(Arc::ptr_eq(&svc, &event_counter));

    // 路由通过 runtime 暴露
    assert!(harness.runtime.routes_of("echo").is_some());
    let all_routes = harness.runtime.all_routes();
    assert_eq!(all_routes.len(), 1);
    assert_eq!(all_routes[0].0, "echo");

    // 事件订阅生效：发布 UserCreated 会命中 handler
    harness.event_bus.publish(Event::new(EventKind::UserCreated));
    wait_until(&event_counter, 1).await;

    // disable: on_disable 触发 + 订阅 abort + 服务注销
    harness.manager.disable("echo", false).await.unwrap();
    assert!(!enabled_flag.load(Ordering::SeqCst));
    assert!(harness.runtime.loaded_plugins().is_empty());
    assert!(harness.runtime.routes_of("echo").is_none());

    // 再次发布事件不应再递增（留些时间让分发尝试投递）
    harness.event_bus.publish(Event::new(EventKind::UserCreated));
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(event_counter.load(Ordering::SeqCst), 1);

    // 服务被 runtime.unregister 掉，查询应为 ServiceNotFound
    let err = harness
        .service_registry
        .get::<AtomicU64>("echo.counter")
        .unwrap_err();
    assert!(matches!(
        err,
        cycms_plugin_api::RegistryError::ServiceNotFound { .. }
    ));

    // re-enable：同一 runtime 下允许再次拉起，事件计数能从断点继续递增
    harness.manager.enable("echo").await.unwrap();
    assert!(enabled_flag.load(Ordering::SeqCst));
    harness.event_bus.publish(Event::new(EventKind::UserCreated));
    wait_until(&event_counter, 2).await;

    // uninstall：runtime 状态清空；plugin 记录清空
    harness.manager.uninstall("echo").await.unwrap();
    assert!(harness.runtime.loaded_plugins().is_empty());
    assert!(harness.manager.list().await.unwrap().is_empty());
    // 服务同样被 unload 清理
    assert!(
        harness
            .service_registry
            .get::<AtomicU64>("echo.counter")
            .is_err()
    );
}

#[tokio::test]
async fn enable_errors_if_plugin_not_registered() {
    let harness = fresh_harness().await;
    write_plugin(&harness.plugins_root, "missing", "0.1.0");

    let discovered = scan_plugins_dir(&harness.plugins_root).unwrap();
    harness.manager.install(&discovered[0]).await.unwrap();

    // runtime 里没有 register_plugin，enable 应该报 PluginError（无工厂）
    let err = harness.manager.enable("missing").await.unwrap_err();
    assert!(
        matches!(err, cycms_core::Error::PluginError { .. }),
        "got: {err:?}"
    );
    let list = harness.manager.list().await.unwrap();
    assert_eq!(list[0].status, PluginStatus::Disabled);
}

#[tokio::test]
async fn register_plugin_replace_warns_and_uses_latest() {
    let harness = fresh_harness().await;
    write_plugin(&harness.plugins_root, "echo", "0.1.0");
    let discovered = scan_plugins_dir(&harness.plugins_root).unwrap();
    harness.manager.install(&discovered[0]).await.unwrap();

    let v1 = EchoPlugin::new("echo");
    harness
        .runtime
        .register_plugin(Arc::clone(&v1) as Arc<dyn Plugin>);

    // 第二次注册覆盖（Kernel 热替换 / 测试夹具常见行为）
    let v2 = EchoPlugin::new("echo");
    let v2_enabled = v2.enabled_flag();
    harness
        .runtime
        .register_plugin(Arc::clone(&v2) as Arc<dyn Plugin>);

    harness.manager.enable("echo").await.unwrap();
    assert!(v2_enabled.load(Ordering::SeqCst));
    assert!(!v1.enabled_flag().load(Ordering::SeqCst));
}
