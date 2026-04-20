//! `WasmPluginRuntime` 集成测试。
//!
//! 使用预编译的 `hello_plugin.wasm` guest fixture 验证完整 load/unload 生命周期、
//! host function 调用链（settings/kv/event/route/log）、事件订阅转发、路由合成。

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use cycms_auth::AuthEngine;
use cycms_config::{AuthConfig, ContentConfig, DatabaseConfig, DatabaseDriver, MediaConfig};
use cycms_content_engine::ContentEngine;
use cycms_content_model::{ContentModelRegistry, FieldTypeRegistry};
use cycms_db::DatabasePool;
use cycms_events::{Event, EventBus, EventKind};
use cycms_media::MediaManager;
use cycms_migrate::MigrationEngine;
use cycms_permission::PermissionEngine;
use cycms_plugin_api::{PluginContext, ServiceRegistry};
use cycms_plugin_manager::{PluginKind, PluginManifest, PluginRuntime};
use cycms_plugin_wasm::WasmPluginRuntime;
use cycms_publish::PublishManager;
use cycms_revision::RevisionManager;
use cycms_settings::SettingsManager;

fn workspace_system_migrations() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../cycms-migrate/migrations/system")
}

fn fixture_wasm() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hello_plugin.wasm")
}

fn test_manifest(name: &str) -> PluginManifest {
    let toml_text = format!(
        r#"
[plugin]
name = "{name}"
version = "0.1.0"
kind = "wasm"
entry = "hello_plugin.wasm"

[compatibility]
cycms = ">=0.1.0"
"#
    );
    PluginManifest::from_toml_str(&toml_text).expect("parse test manifest")
}

struct Harness {
    ctx: Arc<PluginContext>,
    runtime: WasmPluginRuntime,
    event_bus: Arc<EventBus>,
    settings: Arc<SettingsManager>,
}

async fn fresh_harness() -> Harness {
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

    let auth_engine =
        Arc::new(AuthEngine::new(Arc::clone(&pool), AuthConfig::default()).unwrap());
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

    let ctx = Arc::new(PluginContext::new(
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

    let runtime = WasmPluginRuntime::new().expect("create WasmPluginRuntime");

    Harness {
        ctx,
        runtime,
        event_bus,
        settings: settings_manager,
    }
}

#[tokio::test]
async fn kind_is_wasm() {
    let harness = fresh_harness().await;
    assert_eq!(harness.runtime.kind(), PluginKind::Wasm);
}

#[tokio::test]
async fn load_calls_on_enable_and_host_functions() {
    let harness = fresh_harness().await;
    let manifest = test_manifest("hello");
    let wasm_path = fixture_wasm();

    harness
        .runtime
        .load(&manifest, &wasm_path, Arc::clone(&harness.ctx))
        .await
        .unwrap();

    // on-enable 调用了 settings.set("enabled", "true")
    let val = harness.settings.get("hello", "enabled").await.unwrap();
    assert!(val.is_some(), "settings key 'enabled' should exist");

    // on-enable 调用了 kv.set("init", "done") — namespace 为 plugin:hello:kv
    let kv_val = harness.settings.get("plugin:hello:kv", "init").await.unwrap();
    assert!(kv_val.is_some(), "kv key 'init' should exist");

    // loaded_plugins 含 hello
    assert_eq!(harness.runtime.loaded_plugins(), vec!["hello".to_owned()]);

    // route.register("/hello", "GET") → all_routes 非空
    let routes = harness.runtime.all_routes();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].0, "hello");
}

#[tokio::test]
async fn event_subscription_forwards_to_guest() {
    let harness = fresh_harness().await;
    let manifest = test_manifest("hello");
    let wasm_path = fixture_wasm();

    harness
        .runtime
        .load(&manifest, &wasm_path, Arc::clone(&harness.ctx))
        .await
        .unwrap();

    // guest 在 on-enable 中 subscribe("content.created")；
    // 发布该事件后 on-event 应调 kv.set("last-event-kind", "content.created")
    harness
        .event_bus
        .publish(Event::new(EventKind::ContentCreated));

    // 等待异步事件分发
    for _ in 0..100 {
        let val = harness
            .settings
            .get("plugin:hello:kv", "last-event-kind")
            .await
            .unwrap();
        if let Some(v) = val {
            assert_eq!(v.as_str().unwrap(), "content.created");
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("event handler did not fire within timeout");
}

#[tokio::test]
async fn unload_calls_on_disable() {
    let harness = fresh_harness().await;
    let manifest = test_manifest("hello");
    let wasm_path = fixture_wasm();

    harness
        .runtime
        .load(&manifest, &wasm_path, Arc::clone(&harness.ctx))
        .await
        .unwrap();

    harness.runtime.unload("hello").await.unwrap();

    // on-disable 调用了 settings.set("enabled", "false")
    let val = harness.settings.get("hello", "enabled").await.unwrap();
    assert!(val.is_some());
    let json_val = val.unwrap();
    assert_eq!(json_val.as_str().unwrap(), "false");

    // loaded_plugins 为空
    assert!(harness.runtime.loaded_plugins().is_empty());

    // all_routes 为空
    assert!(harness.runtime.all_routes().is_empty());
}

#[tokio::test]
async fn load_duplicate_returns_conflict() {
    let harness = fresh_harness().await;
    let manifest = test_manifest("hello");
    let wasm_path = fixture_wasm();

    harness
        .runtime
        .load(&manifest, &wasm_path, Arc::clone(&harness.ctx))
        .await
        .unwrap();

    let err = harness
        .runtime
        .load(&manifest, &wasm_path, Arc::clone(&harness.ctx))
        .await
        .unwrap_err();

    assert!(
        matches!(err, cycms_core::Error::Conflict { .. }),
        "expected Conflict, got: {err:?}"
    );
}

#[tokio::test]
async fn unload_nonexistent_is_noop() {
    let harness = fresh_harness().await;
    // 卸载不存在的插件应静默成功
    harness.runtime.unload("nonexistent").await.unwrap();
}
