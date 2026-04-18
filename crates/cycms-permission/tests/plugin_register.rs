use std::path::PathBuf;
use std::sync::Arc;

use cycms_config::{DatabaseConfig, DatabaseDriver};
use cycms_core::Error;
use cycms_db::DatabasePool;
use cycms_migrate::MigrationEngine;
use cycms_permission::{PermissionDefinition, PermissionEngine, PermissionScope};

fn system_migrations_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../cycms-migrate/migrations/system")
}

async fn fresh_sqlite_pool() -> Arc<DatabasePool> {
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
    pool
}

fn blog_defs() -> Vec<PermissionDefinition> {
    vec![
        PermissionDefinition {
            domain: "blog".to_owned(),
            resource: "post".to_owned(),
            action: "read".to_owned(),
            scope: PermissionScope::All,
        },
        PermissionDefinition {
            domain: "blog".to_owned(),
            resource: "post".to_owned(),
            action: "update".to_owned(),
            scope: PermissionScope::Own,
        },
    ]
}

#[tokio::test]
async fn register_plugin_permission_is_idempotent() {
    let pool = fresh_sqlite_pool().await;
    let engine = PermissionEngine::new(pool);

    let v1 = engine
        .register_permissions("blog", blog_defs())
        .await
        .unwrap();
    let v2 = engine
        .register_permissions("blog", blog_defs())
        .await
        .unwrap();

    assert_eq!(v1.len(), 2);
    assert_eq!(v2.len(), 2);
    assert_eq!(v1[0].id, v2[0].id);
    assert_eq!(v1[1].id, v2[1].id);
}

#[tokio::test]
async fn register_tags_source_on_every_row() {
    let pool = fresh_sqlite_pool().await;
    let engine = PermissionEngine::new(pool);
    let inserted = engine
        .register_permissions("blog", blog_defs())
        .await
        .unwrap();
    for p in &inserted {
        assert_eq!(p.source, "blog");
    }
    let other = engine.permissions().list_by_source("system").await.unwrap();
    assert!(other.is_empty(), "system namespace must remain untouched");
}

#[tokio::test]
async fn register_rejects_invalid_code() {
    let pool = fresh_sqlite_pool().await;
    let engine = PermissionEngine::new(pool);
    let defs = vec![PermissionDefinition {
        domain: "Blog".to_owned(), // 大写非法
        resource: "post".to_owned(),
        action: "read".to_owned(),
        scope: PermissionScope::All,
    }];
    let err = engine.register_permissions("blog", defs).await.unwrap_err();
    assert!(matches!(err, Error::ValidationError { .. }));
}

#[tokio::test]
async fn register_rejects_blank_source() {
    let pool = fresh_sqlite_pool().await;
    let engine = PermissionEngine::new(pool);
    let err = engine
        .register_permissions("   ", blog_defs())
        .await
        .unwrap_err();
    assert!(matches!(err, Error::ValidationError { .. }));
}

#[tokio::test]
async fn unregister_by_source_removes_all_matching_rows() {
    let pool = fresh_sqlite_pool().await;
    let engine = PermissionEngine::new(pool);
    engine
        .register_permissions("blog", blog_defs())
        .await
        .unwrap();
    let removed = engine
        .unregister_permissions_by_source("blog")
        .await
        .unwrap();
    assert_eq!(removed, 2);
    assert!(
        engine
            .permissions()
            .list_by_source("blog")
            .await
            .unwrap()
            .is_empty()
    );
}
