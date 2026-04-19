use std::path::PathBuf;
use std::sync::Arc;

use cycms_config::{DatabaseConfig, DatabaseDriver};
use cycms_db::DatabasePool;
use cycms_migrate::MigrationEngine;
use cycms_plugin_manager::{NewPluginRow, PluginKind, PluginRepository, PluginStatus};
use serde_json::json;

fn system_migrations_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../cycms-migrate/migrations/system")
}

async fn fresh_sqlite_repo() -> PluginRepository {
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
    PluginRepository::new(pool)
}

fn sample_manifest_value(name: &str) -> serde_json::Value {
    json!({
        "plugin": {
            "name": name,
            "version": "0.1.0",
            "kind": "native",
            "entry": format!("{name}.so"),
        },
        "compatibility": { "cycms": ">=0.1.0" }
    })
}

#[tokio::test]
async fn insert_and_find_roundtrip() {
    let repo = fresh_sqlite_repo().await;
    let row = NewPluginRow {
        name: "blog".into(),
        version: "0.1.0".into(),
        kind: PluginKind::Native,
        manifest: sample_manifest_value("blog"),
    };
    let inserted = repo.insert(row).await.unwrap();
    assert_eq!(inserted.name, "blog");
    assert_eq!(inserted.status, PluginStatus::Disabled);
    assert_eq!(inserted.kind, PluginKind::Native);

    let fetched = repo.find_by_name("blog").await.unwrap().unwrap();
    assert_eq!(fetched.id, inserted.id);
    assert_eq!(fetched.manifest, sample_manifest_value("blog"));
}

#[tokio::test]
async fn insert_duplicate_name_errors() {
    let repo = fresh_sqlite_repo().await;
    let row = NewPluginRow {
        name: "blog".into(),
        version: "0.1.0".into(),
        kind: PluginKind::Native,
        manifest: sample_manifest_value("blog"),
    };
    repo.insert(row.clone()).await.unwrap();
    assert!(repo.insert(row).await.is_err());
}

#[tokio::test]
async fn list_returns_alphabetical_order() {
    let repo = fresh_sqlite_repo().await;
    for name in ["zeta", "alpha", "mu"] {
        repo.insert(NewPluginRow {
            name: name.into(),
            version: "0.1.0".into(),
            kind: PluginKind::Wasm,
            manifest: sample_manifest_value(name),
        })
        .await
        .unwrap();
    }
    let records = repo.list().await.unwrap();
    let names: Vec<_> = records.iter().map(|r| r.name.clone()).collect();
    assert_eq!(names, vec!["alpha", "mu", "zeta"]);
}

#[tokio::test]
async fn update_status_toggles_row() {
    let repo = fresh_sqlite_repo().await;
    repo.insert(NewPluginRow {
        name: "blog".into(),
        version: "0.1.0".into(),
        kind: PluginKind::Native,
        manifest: sample_manifest_value("blog"),
    })
    .await
    .unwrap();

    assert!(
        repo.update_status("blog", PluginStatus::Enabled)
            .await
            .unwrap()
    );
    let after = repo.find_by_name("blog").await.unwrap().unwrap();
    assert_eq!(after.status, PluginStatus::Enabled);

    assert!(
        !repo
            .update_status("nonexistent", PluginStatus::Enabled)
            .await
            .unwrap(),
        "updating nonexistent plugin must report no-op"
    );
}

#[tokio::test]
async fn delete_removes_and_reports_miss() {
    let repo = fresh_sqlite_repo().await;
    repo.insert(NewPluginRow {
        name: "blog".into(),
        version: "0.1.0".into(),
        kind: PluginKind::Native,
        manifest: sample_manifest_value("blog"),
    })
    .await
    .unwrap();

    assert!(repo.delete("blog").await.unwrap());
    assert!(repo.find_by_name("blog").await.unwrap().is_none());
    assert!(!repo.delete("blog").await.unwrap());
}
