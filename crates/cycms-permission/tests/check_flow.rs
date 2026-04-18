use std::path::PathBuf;
use std::sync::Arc;

use cycms_config::{DatabaseConfig, DatabaseDriver};
use cycms_core::Error;
use cycms_db::DatabasePool;
use cycms_migrate::MigrationEngine;
use cycms_permission::{
    NewRoleRow, PermissionDefinition, PermissionEngine, PermissionScope,
};

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

async fn seed_role_with_permission(
    engine: &PermissionEngine,
    role_name: &str,
    domain: &str,
    resource: &str,
    action: &str,
    scope: PermissionScope,
) -> String {
    let role = engine
        .roles()
        .create(NewRoleRow {
            name: role_name.to_owned(),
            description: None,
            is_system: false,
        })
        .await
        .unwrap();
    let defs = vec![PermissionDefinition {
        domain: domain.to_owned(),
        resource: resource.to_owned(),
        action: action.to_owned(),
        scope,
    }];
    let perms = engine
        .permissions()
        .upsert_many("system", &defs)
        .await
        .unwrap();
    engine
        .roles()
        .attach_permission(&role.id, &perms[0].id)
        .await
        .unwrap();
    role.id
}

#[tokio::test]
async fn super_admin_short_circuits_any_code() {
    let pool = fresh_sqlite_pool().await;
    let engine = PermissionEngine::new(pool);
    // 不预置任何权限表记录，super_admin 仍可通行
    let pass = engine
        .check_permission(
            "user-xxx",
            &["super_admin".to_owned()],
            "any.thing.do",
            None,
        )
        .await
        .unwrap();
    assert!(pass);
}

#[tokio::test]
async fn role_without_matching_permission_is_denied() {
    let pool = fresh_sqlite_pool().await;
    let engine = PermissionEngine::new(pool);
    engine
        .roles()
        .create(NewRoleRow {
            name: "empty".to_owned(),
            description: None,
            is_system: false,
        })
        .await
        .unwrap();
    let pass = engine
        .check_permission("user-x", &["empty".to_owned()], "system.post.read", None)
        .await
        .unwrap();
    assert!(!pass);
}

#[tokio::test]
async fn role_with_all_scope_passes_regardless_of_owner() {
    let pool = fresh_sqlite_pool().await;
    let engine = PermissionEngine::new(pool);
    seed_role_with_permission(
        &engine,
        "editor",
        "system",
        "post",
        "read",
        PermissionScope::All,
    )
    .await;

    assert!(
        engine
            .check_permission("user-1", &["editor".to_owned()], "system.post.read", None)
            .await
            .unwrap()
    );
    assert!(
        engine
            .check_permission(
                "user-1",
                &["editor".to_owned()],
                "system.post.read",
                Some("user-2"),
            )
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn role_with_own_scope_passes_when_owner_matches() {
    let pool = fresh_sqlite_pool().await;
    let engine = PermissionEngine::new(pool);
    seed_role_with_permission(
        &engine,
        "author",
        "system",
        "post",
        "update",
        PermissionScope::Own,
    )
    .await;

    let pass = engine
        .check_permission(
            "user-1",
            &["author".to_owned()],
            "system.post.update",
            Some("user-1"),
        )
        .await
        .unwrap();
    assert!(pass);
}

#[tokio::test]
async fn role_with_own_scope_denied_when_owner_differs() {
    let pool = fresh_sqlite_pool().await;
    let engine = PermissionEngine::new(pool);
    seed_role_with_permission(
        &engine,
        "author",
        "system",
        "post",
        "update",
        PermissionScope::Own,
    )
    .await;

    let pass = engine
        .check_permission(
            "user-1",
            &["author".to_owned()],
            "system.post.update",
            Some("user-2"),
        )
        .await
        .unwrap();
    assert!(!pass);
}

#[tokio::test]
async fn role_with_own_scope_denied_when_owner_missing() {
    let pool = fresh_sqlite_pool().await;
    let engine = PermissionEngine::new(pool);
    seed_role_with_permission(
        &engine,
        "author",
        "system",
        "post",
        "update",
        PermissionScope::Own,
    )
    .await;

    let pass = engine
        .check_permission(
            "user-1",
            &["author".to_owned()],
            "system.post.update",
            None,
        )
        .await
        .unwrap();
    assert!(!pass);
}

#[tokio::test]
async fn mixed_own_and_all_scope_returns_pass_on_all() {
    let pool = fresh_sqlite_pool().await;
    let engine = PermissionEngine::new(pool);
    seed_role_with_permission(
        &engine,
        "editor_all",
        "system",
        "post",
        "update",
        PermissionScope::All,
    )
    .await;
    seed_role_with_permission(
        &engine,
        "author_own",
        "system",
        "post",
        "update",
        PermissionScope::Own,
    )
    .await;
    let roles = vec!["editor_all".to_owned(), "author_own".to_owned()];

    // owner 不匹配，但因为存在 All scope 仍放行
    assert!(
        engine
            .check_permission("user-1", &roles, "system.post.update", Some("user-2"))
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn role_name_is_normalized_case_and_whitespace() {
    let pool = fresh_sqlite_pool().await;
    let engine = PermissionEngine::new(pool);
    seed_role_with_permission(
        &engine,
        "editor",
        "system",
        "post",
        "read",
        PermissionScope::All,
    )
    .await;

    // 调用方传入带大小写与空格的角色名仍能命中
    let roles = vec!["  Editor ".to_owned(), String::new()];
    assert!(
        engine
            .check_permission("user-1", &roles, "system.post.read", None)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn invalid_code_returns_validation_error() {
    let pool = fresh_sqlite_pool().await;
    let engine = PermissionEngine::new(pool);
    let err = engine
        .check_permission("user-1", &["editor".to_owned()], "INVALID.code", None)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::ValidationError { .. }));
}

#[tokio::test]
async fn require_permission_returns_forbidden_on_deny() {
    let pool = fresh_sqlite_pool().await;
    let engine = PermissionEngine::new(pool);
    let err = engine
        .require_permission(
            "user-1",
            &["no_role".to_owned()],
            "system.post.read",
            None,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Forbidden { .. }));
}
