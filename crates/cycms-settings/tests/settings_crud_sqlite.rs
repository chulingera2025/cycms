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
async fn set_and_get_roundtrips() {
    let mgr = fresh_sqlite_manager().await;

    let entry = mgr
        .set("ui", "theme", json!({ "mode": "dark" }))
        .await
        .unwrap();
    assert_eq!(entry.namespace, "ui");
    assert_eq!(entry.key, "theme");
    assert_eq!(entry.value, json!({ "mode": "dark" }));

    let fetched = mgr.get("ui", "theme").await.unwrap().unwrap();
    assert_eq!(fetched, json!({ "mode": "dark" }));
}

#[tokio::test]
async fn set_overwrites_existing_value() {
    let mgr = fresh_sqlite_manager().await;

    let v1 = mgr.set("system", "locale", json!("en_US")).await.unwrap();
    let v2 = mgr.set("system", "locale", json!("zh_CN")).await.unwrap();
    assert_eq!(v1.id, v2.id, "upsert must keep the same id");
    assert_eq!(v2.value, json!("zh_CN"));
}

#[tokio::test]
async fn delete_removes_setting() {
    let mgr = fresh_sqlite_manager().await;
    mgr.set("system", "flag", json!(true)).await.unwrap();

    assert!(mgr.delete("system", "flag").await.unwrap());
    assert!(mgr.get("system", "flag").await.unwrap().is_none());
}

#[tokio::test]
async fn delete_nonexistent_returns_false() {
    let mgr = fresh_sqlite_manager().await;
    assert!(!mgr.delete("nope", "nope").await.unwrap());
}

#[tokio::test]
async fn get_all_lists_namespace() {
    let mgr = fresh_sqlite_manager().await;
    mgr.set("ui", "theme", json!("dark")).await.unwrap();
    mgr.set("ui", "sidebar", json!({ "collapsed": true }))
        .await
        .unwrap();
    mgr.set("other", "x", json!(1)).await.unwrap();

    let all = mgr.get_all("ui").await.unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all.get("theme"), Some(&json!("dark")));
    assert_eq!(all.get("sidebar"), Some(&json!({ "collapsed": true })));
}

#[tokio::test]
async fn namespace_isolation_is_strict() {
    let mgr = fresh_sqlite_manager().await;
    mgr.set("ns_a", "shared_key", json!(1)).await.unwrap();
    mgr.set("ns_b", "shared_key", json!(2)).await.unwrap();

    assert_eq!(mgr.get("ns_a", "shared_key").await.unwrap(), Some(json!(1)));
    assert_eq!(mgr.get("ns_b", "shared_key").await.unwrap(), Some(json!(2)));
}

#[tokio::test]
async fn empty_namespace_or_key_is_rejected() {
    let mgr = fresh_sqlite_manager().await;
    let err = mgr.set("   ", "k", json!(1)).await.unwrap_err();
    assert!(matches!(err, Error::ValidationError { .. }), "got: {err:?}");

    let err = mgr.set("ns", "", json!(1)).await.unwrap_err();
    assert!(matches!(err, Error::ValidationError { .. }), "got: {err:?}");
}
