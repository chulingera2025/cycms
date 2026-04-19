use std::path::PathBuf;
use std::sync::Arc;

use cycms_config::{DatabaseConfig, DatabaseDriver};
use cycms_core::Error;
use cycms_db::DatabasePool;
use cycms_migrate::MigrationEngine;
use cycms_settings::SettingsManager;
use serde_json::json;

fn system_migrations_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../cycms-migrate/migrations/system")
}

async fn fresh_sqlite_manager() -> SettingsManager {
    let pool = Arc::new(
        DatabasePool::connect(&DatabaseConfig {
            driver: DatabaseDriver::Sqlite,
            url: "sqlite::memory:".to_owned(),
            max_connections: 1,
            connect_timeout_secs: 5,
            idle_timeout_secs: 60,
        })
        .await
        .expect("sqlite pool connect"),
    );
    MigrationEngine::new(Arc::clone(&pool))
        .run_system_migrations(&system_migrations_root())
        .await
        .expect("run system migrations");
    SettingsManager::new(pool)
}

#[tokio::test]
async fn register_and_get_schema_roundtrips() {
    let mgr = fresh_sqlite_manager().await;

    let schema = json!({
        "type": "object",
        "properties": {
            "api_key": { "type": "string" }
        }
    });
    let registered = mgr
        .register_schema("official-auth", schema.clone())
        .await
        .unwrap();
    assert_eq!(registered.plugin_name, "official-auth");
    assert_eq!(registered.schema, schema);

    let fetched = mgr.get_schema("official-auth").await.unwrap().unwrap();
    assert_eq!(fetched.schema, schema);
}

#[tokio::test]
async fn register_updates_existing_schema() {
    let mgr = fresh_sqlite_manager().await;

    let v1 = mgr
        .register_schema("blog", json!({ "type": "object" }))
        .await
        .unwrap();
    let v2 = mgr
        .register_schema(
            "blog",
            json!({ "properties": { "title": { "type": "string" } } }),
        )
        .await
        .unwrap();

    assert_eq!(v1.created_at, v2.created_at, "created_at must be preserved");
    assert_eq!(v2.schema["properties"]["title"]["type"], json!("string"));
}

#[tokio::test]
async fn register_rejects_non_object_schema() {
    let mgr = fresh_sqlite_manager().await;
    let err = mgr
        .register_schema("bad", json!("just a string"))
        .await
        .unwrap_err();
    assert!(matches!(err, Error::ValidationError { .. }), "got: {err:?}");
}

#[tokio::test]
async fn register_rejects_object_without_type_or_properties() {
    let mgr = fresh_sqlite_manager().await;
    let err = mgr
        .register_schema("bad", json!({ "title": "only title" }))
        .await
        .unwrap_err();
    assert!(matches!(err, Error::ValidationError { .. }), "got: {err:?}");
}

#[tokio::test]
async fn unregister_removes_schema() {
    let mgr = fresh_sqlite_manager().await;
    mgr.register_schema("tmp", json!({ "type": "object" }))
        .await
        .unwrap();

    assert!(mgr.unregister_schema("tmp").await.unwrap());
    assert!(mgr.get_schema("tmp").await.unwrap().is_none());
    assert!(
        !mgr.unregister_schema("tmp").await.unwrap(),
        "second unregister must return false"
    );
}

#[tokio::test]
async fn list_schemas_sorted_by_plugin_name() {
    let mgr = fresh_sqlite_manager().await;
    mgr.register_schema("zeta", json!({ "type": "object" }))
        .await
        .unwrap();
    mgr.register_schema("alpha", json!({ "type": "object" }))
        .await
        .unwrap();
    mgr.register_schema("mu", json!({ "type": "object" }))
        .await
        .unwrap();

    let list = mgr.list_schemas().await.unwrap();
    let names: Vec<_> = list.iter().map(|s| s.plugin_name.as_str()).collect();
    assert_eq!(names, vec!["alpha", "mu", "zeta"]);
}
