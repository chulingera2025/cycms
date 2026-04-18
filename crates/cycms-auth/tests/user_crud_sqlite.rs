use std::path::PathBuf;
use std::sync::Arc;

use cycms_auth::{NewUserRow, UserRepository};
use cycms_config::{DatabaseConfig, DatabaseDriver};
use cycms_core::Error;
use cycms_db::DatabasePool;
use cycms_migrate::MigrationEngine;

fn system_migrations_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../cycms-migrate/migrations/system")
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

fn sample_input(suffix: &str) -> NewUserRow {
    NewUserRow {
        username: format!("alice_{suffix}"),
        email: format!("alice_{suffix}@example.test"),
        password_hash: "$argon2id$dummy".to_owned(),
        is_active: true,
    }
}

#[tokio::test]
async fn create_then_find_round_trip() {
    let pool = fresh_sqlite_pool().await;
    let repo = UserRepository::new(pool);

    assert_eq!(repo.count().await.unwrap(), 0);

    let user = repo.create(sample_input("1")).await.unwrap();
    assert_eq!(user.username, "alice_1");
    assert_eq!(user.email, "alice_1@example.test");
    assert!(user.is_active);
    assert!(!user.id.is_empty());
    // SQLite 默认 strftime 产出 UTC ISO8601
    assert!(user.created_at <= user.updated_at || user.created_at == user.updated_at);

    assert_eq!(repo.count().await.unwrap(), 1);

    let by_id = repo.find_by_id(&user.id).await.unwrap().unwrap();
    assert_eq!(by_id.id, user.id);

    let by_name = repo.find_by_username(&user.username).await.unwrap().unwrap();
    assert_eq!(by_name.id, user.id);
}

#[tokio::test]
async fn find_missing_returns_none() {
    let pool = fresh_sqlite_pool().await;
    let repo = UserRepository::new(pool);

    assert!(repo.find_by_id("00000000-0000-0000-0000-000000000000").await.unwrap().is_none());
    assert!(repo.find_by_username("ghost").await.unwrap().is_none());
}

#[tokio::test]
async fn duplicate_username_returns_conflict() {
    let pool = fresh_sqlite_pool().await;
    let repo = UserRepository::new(pool);

    repo.create(sample_input("dup")).await.unwrap();

    let mut second = sample_input("dup");
    second.email = "different@example.test".to_owned();

    let err = repo.create(second).await.unwrap_err();
    assert!(matches!(err, Error::Conflict { .. }), "got: {err:?}");
}

#[tokio::test]
async fn duplicate_email_returns_conflict() {
    let pool = fresh_sqlite_pool().await;
    let repo = UserRepository::new(pool);

    repo.create(sample_input("a")).await.unwrap();

    let mut second = sample_input("b");
    second.email = "alice_a@example.test".to_owned();

    let err = repo.create(second).await.unwrap_err();
    assert!(matches!(err, Error::Conflict { .. }), "got: {err:?}");
}

#[tokio::test]
async fn fetch_roles_returns_empty_for_unlinked_user() {
    let pool = fresh_sqlite_pool().await;
    let repo = UserRepository::new(Arc::clone(&pool));

    let user = repo.create(sample_input("roleless")).await.unwrap();
    let roles = repo.fetch_roles(&user.id).await.unwrap();
    assert!(roles.is_empty());
}

#[tokio::test]
async fn fetch_roles_returns_linked_role_names() {
    let pool = fresh_sqlite_pool().await;
    let repo = UserRepository::new(Arc::clone(&pool));
    let user = repo.create(sample_input("staff")).await.unwrap();

    let DatabasePool::Sqlite(inner) = pool.as_ref() else {
        panic!("expected sqlite pool");
    };

    // 建两个角色并关联用户，验证排序与多行读取
    let role_editor_id = "role-editor-0000";
    let role_admin_id = "role-admin-0000";
    sqlx::query("INSERT INTO roles (id, name) VALUES (?, ?), (?, ?)")
        .bind(role_admin_id)
        .bind("admin")
        .bind(role_editor_id)
        .bind("editor")
        .execute(inner)
        .await
        .unwrap();
    sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES (?, ?), (?, ?)")
        .bind(&user.id)
        .bind(role_admin_id)
        .bind(&user.id)
        .bind(role_editor_id)
        .execute(inner)
        .await
        .unwrap();

    let roles = repo.fetch_roles(&user.id).await.unwrap();
    assert_eq!(roles, vec!["admin".to_owned(), "editor".to_owned()]);
}
