use std::path::PathBuf;
use std::sync::Arc;

use cycms_config::{DatabaseConfig, DatabaseDriver};
use cycms_core::Error;
use cycms_db::DatabasePool;
use cycms_migrate::MigrationEngine;
use cycms_permission::{NewRoleRow, RoleRepository};

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
async fn create_then_find_round_trip() {
    let pool = fresh_sqlite_pool().await;
    let repo = RoleRepository::new(pool);

    let role = repo
        .create(NewRoleRow {
            name: "editor".to_owned(),
            description: Some("editorial staff".to_owned()),
            is_system: false,
        })
        .await
        .unwrap();
    assert_eq!(role.name, "editor");
    assert!(!role.is_system);
    assert_eq!(role.description.as_deref(), Some("editorial staff"));
    assert!(!role.id.is_empty());

    let by_id = repo.find_by_id(&role.id).await.unwrap().unwrap();
    assert_eq!(by_id.id, role.id);

    // 大小写与首尾空格均归一到 lowercase 后再比对
    let by_name = repo.find_by_name("  Editor  ").await.unwrap().unwrap();
    assert_eq!(by_name.id, role.id);

    let all = repo.list().await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].id, role.id);
}

#[tokio::test]
async fn duplicate_role_name_conflict() {
    let pool = fresh_sqlite_pool().await;
    let repo = RoleRepository::new(pool);

    repo.create(NewRoleRow {
        name: "editor".to_owned(),
        description: None,
        is_system: false,
    })
    .await
    .unwrap();

    let err = repo
        .create(NewRoleRow {
            name: "Editor".to_owned(),
            description: None,
            is_system: false,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Conflict { .. }), "got: {err:?}");
}

#[tokio::test]
async fn delete_system_role_returns_conflict() {
    let pool = fresh_sqlite_pool().await;
    let repo = RoleRepository::new(pool);

    let system_role = repo
        .create(NewRoleRow {
            name: "super_admin".to_owned(),
            description: None,
            is_system: true,
        })
        .await
        .unwrap();
    let err = repo.delete(&system_role.id).await.unwrap_err();
    assert!(matches!(err, Error::Conflict { .. }), "got: {err:?}");

    let plain_role = repo
        .create(NewRoleRow {
            name: "author".to_owned(),
            description: None,
            is_system: false,
        })
        .await
        .unwrap();
    repo.delete(&plain_role.id).await.unwrap();
    assert!(repo.find_by_id(&plain_role.id).await.unwrap().is_none());
}

#[tokio::test]
async fn delete_missing_role_returns_not_found() {
    let pool = fresh_sqlite_pool().await;
    let repo = RoleRepository::new(pool);
    let err = repo
        .delete("00000000-0000-0000-0000-000000000000")
        .await
        .unwrap_err();
    assert!(matches!(err, Error::NotFound { .. }), "got: {err:?}");
}

#[tokio::test]
async fn attach_detach_permission_idempotent() {
    let pool = fresh_sqlite_pool().await;
    let role_repo = RoleRepository::new(Arc::clone(&pool));

    // 通过原生 SQL 注入一条权限，避开对 PermissionRepository 的依赖
    let DatabasePool::Sqlite(inner) = pool.as_ref() else {
        panic!("expected sqlite pool");
    };
    sqlx::query(
        "INSERT INTO permissions (id, domain, resource, action, scope, source) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind("perm-0001-0000")
    .bind("system")
    .bind("post")
    .bind("read")
    .bind("all")
    .bind("system")
    .execute(inner)
    .await
    .unwrap();

    let role = role_repo
        .create(NewRoleRow {
            name: "editor".to_owned(),
            description: None,
            is_system: false,
        })
        .await
        .unwrap();

    role_repo
        .attach_permission(&role.id, "perm-0001-0000")
        .await
        .unwrap();
    role_repo
        .attach_permission(&role.id, "perm-0001-0000")
        .await
        .unwrap();

    let (count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM role_permissions WHERE role_id = ? AND permission_id = ?",
    )
    .bind(&role.id)
    .bind("perm-0001-0000")
    .fetch_one(inner)
    .await
    .unwrap();
    assert_eq!(count, 1, "attach must not create duplicate links");

    role_repo
        .detach_permission(&role.id, "perm-0001-0000")
        .await
        .unwrap();
    role_repo
        .detach_permission(&role.id, "perm-0001-0000")
        .await
        .unwrap();
}

#[tokio::test]
async fn bind_unbind_user_idempotent() {
    let pool = fresh_sqlite_pool().await;
    let role_repo = RoleRepository::new(Arc::clone(&pool));

    let DatabasePool::Sqlite(inner) = pool.as_ref() else {
        panic!("expected sqlite pool");
    };
    sqlx::query("INSERT INTO users (id, username, email, password_hash) VALUES (?, ?, ?, ?)")
        .bind("user-0000-0001")
        .bind("user_a")
        .bind("a@example.test")
        .bind("$argon2id$dummy")
        .execute(inner)
        .await
        .unwrap();

    let role = role_repo
        .create(NewRoleRow {
            name: "editor".to_owned(),
            description: None,
            is_system: false,
        })
        .await
        .unwrap();

    role_repo
        .bind_user("user-0000-0001", &role.id)
        .await
        .unwrap();
    role_repo
        .bind_user("user-0000-0001", &role.id)
        .await
        .unwrap();

    let (count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM user_roles WHERE user_id = ? AND role_id = ?")
            .bind("user-0000-0001")
            .bind(&role.id)
            .fetch_one(inner)
            .await
            .unwrap();
    assert_eq!(count, 1);

    role_repo
        .unbind_user("user-0000-0001", &role.id)
        .await
        .unwrap();
    role_repo
        .unbind_user("user-0000-0001", &role.id)
        .await
        .unwrap();
}
