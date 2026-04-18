use std::path::PathBuf;
use std::sync::Arc;

use cycms_config::{DatabaseConfig, DatabaseDriver};
use cycms_core::Error;
use cycms_db::DatabasePool;
use cycms_migrate::MigrationEngine;
use cycms_permission::{PermissionDefinition, PermissionRepository, PermissionScope};

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

#[tokio::test]
async fn create_permission_roundtrips() {
    let pool = fresh_sqlite_pool().await;
    let repo = PermissionRepository::new(pool);

    let p = repo
        .create("system", "post", "read", PermissionScope::All, "system")
        .await
        .unwrap();
    assert_eq!(p.domain, "system");
    assert_eq!(p.scope, PermissionScope::All);

    let found = repo
        .find_by_code_and_scope("system", "post", "read", PermissionScope::All)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.id, p.id);

    let by_id = repo.find_by_id(&p.id).await.unwrap().unwrap();
    assert_eq!(by_id.id, p.id);
}

#[tokio::test]
async fn create_duplicate_returns_conflict() {
    let pool = fresh_sqlite_pool().await;
    let repo = PermissionRepository::new(pool);

    repo.create("system", "post", "read", PermissionScope::All, "system")
        .await
        .unwrap();
    let err = repo
        .create("system", "post", "read", PermissionScope::All, "system")
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Conflict { .. }), "got: {err:?}");
}

#[tokio::test]
async fn upsert_many_is_idempotent() {
    let pool = fresh_sqlite_pool().await;
    let repo = PermissionRepository::new(pool);

    let defs = vec![
        PermissionDefinition {
            domain: "system".to_owned(),
            resource: "post".to_owned(),
            action: "read".to_owned(),
            scope: PermissionScope::All,
        },
        PermissionDefinition {
            domain: "system".to_owned(),
            resource: "post".to_owned(),
            action: "update".to_owned(),
            scope: PermissionScope::Own,
        },
    ];

    let v1 = repo.upsert_many("system", &defs).await.unwrap();
    let v2 = repo.upsert_many("system", &defs).await.unwrap();
    assert_eq!(v1.len(), 2);
    assert_eq!(v2.len(), 2);
    // 第二次 upsert 应拿到既有行，同一 id
    assert_eq!(v1[0].id, v2[0].id);
    assert_eq!(v1[1].id, v2[1].id);

    let listed = repo.list_by_source("system").await.unwrap();
    assert_eq!(listed.len(), 2);
}

#[tokio::test]
async fn list_by_source_filters_correctly() {
    let pool = fresh_sqlite_pool().await;
    let repo = PermissionRepository::new(pool);

    let sys = vec![PermissionDefinition {
        domain: "system".to_owned(),
        resource: "user".to_owned(),
        action: "read".to_owned(),
        scope: PermissionScope::All,
    }];
    let plugin = vec![PermissionDefinition {
        domain: "blog".to_owned(),
        resource: "post".to_owned(),
        action: "read".to_owned(),
        scope: PermissionScope::All,
    }];
    repo.upsert_many("system", &sys).await.unwrap();
    repo.upsert_many("blog", &plugin).await.unwrap();

    let sys_list = repo.list_by_source("system").await.unwrap();
    assert_eq!(sys_list.len(), 1);
    assert_eq!(sys_list[0].domain, "system");

    let plugin_list = repo.list_by_source("blog").await.unwrap();
    assert_eq!(plugin_list.len(), 1);
    assert_eq!(plugin_list[0].domain, "blog");
}

#[tokio::test]
async fn delete_by_source_removes_matching_rows() {
    let pool = fresh_sqlite_pool().await;
    let repo = PermissionRepository::new(pool);

    let plugin = vec![
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
    ];
    repo.upsert_many("blog", &plugin).await.unwrap();

    let removed = repo.delete_by_source("blog").await.unwrap();
    assert_eq!(removed, 2);
    assert!(repo.list_by_source("blog").await.unwrap().is_empty());
}
