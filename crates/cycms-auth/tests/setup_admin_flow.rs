use std::path::PathBuf;
use std::sync::Arc;

use cycms_auth::{AuthEngine, CreateUserInput};
use cycms_config::{Argon2Config, AuthConfig, DatabaseConfig, DatabaseDriver};
use cycms_core::Error;
use cycms_db::DatabasePool;
use cycms_migrate::MigrationEngine;

const TEST_SECRET: &str = "test-jwt-secret";

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
        .unwrap(),
    );
    MigrationEngine::new(Arc::clone(&pool))
        .run_system_migrations(&system_migrations_root())
        .await
        .unwrap();
    pool
}

fn auth_config() -> AuthConfig {
    AuthConfig {
        jwt_secret: TEST_SECRET.to_owned(),
        access_token_ttl_secs: 900,
        refresh_token_ttl_secs: 1_209_600,
        argon2: Argon2Config {
            m_cost: 16,
            t_cost: 2,
            p_cost: 1,
        },
    }
}

fn sample_admin() -> CreateUserInput {
    CreateUserInput {
        username: "admin".to_owned(),
        email: "admin@example.test".to_owned(),
        password: "InitialAdmin1!".to_owned(),
    }
}

#[tokio::test]
async fn setup_admin_succeeds_on_empty_system() {
    let pool = fresh_sqlite_pool().await;
    let engine = AuthEngine::new(Arc::clone(&pool), auth_config()).unwrap();

    let user = engine.setup_admin(sample_admin()).await.unwrap();
    assert_eq!(user.username, "admin");
    assert!(user.is_active);
    assert_eq!(engine.users().count().await.unwrap(), 1);
}

#[tokio::test]
async fn setup_admin_rejects_when_any_user_exists() {
    let pool = fresh_sqlite_pool().await;
    let engine = AuthEngine::new(Arc::clone(&pool), auth_config()).unwrap();

    engine
        .create_user(CreateUserInput {
            username: "alice".to_owned(),
            email: "alice@example.test".to_owned(),
            password: "StrongPass1!".to_owned(),
        })
        .await
        .unwrap();

    let err = engine.setup_admin(sample_admin()).await.unwrap_err();
    match err {
        Error::Conflict { message } => assert_eq!(message, "initial admin already exists"),
        other => panic!("expected Conflict, got {other:?}"),
    }
}

#[tokio::test]
async fn setup_admin_rejects_weak_password() {
    let pool = fresh_sqlite_pool().await;
    let engine = AuthEngine::new(Arc::clone(&pool), auth_config()).unwrap();

    let err = engine
        .setup_admin(CreateUserInput {
            username: "admin".to_owned(),
            email: "admin@example.test".to_owned(),
            password: "weak".to_owned(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, Error::ValidationError { .. }));
    assert_eq!(engine.users().count().await.unwrap(), 0);
}

#[tokio::test]
async fn setup_admin_rejects_invalid_email() {
    let pool = fresh_sqlite_pool().await;
    let engine = AuthEngine::new(Arc::clone(&pool), auth_config()).unwrap();

    let err = engine
        .setup_admin(CreateUserInput {
            username: "admin".to_owned(),
            email: "no-at-sign".to_owned(),
            password: "InitialAdmin1!".to_owned(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, Error::ValidationError { .. }));
}

#[tokio::test]
async fn create_user_rejects_duplicate_username() {
    let pool = fresh_sqlite_pool().await;
    let engine = AuthEngine::new(Arc::clone(&pool), auth_config()).unwrap();

    engine.create_user(sample_admin()).await.unwrap();

    let err = engine
        .create_user(CreateUserInput {
            username: "admin".to_owned(),
            email: "different@example.test".to_owned(),
            password: "AnotherPass1!".to_owned(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Conflict { .. }));
}
