use std::path::PathBuf;
use std::sync::Arc;

use cycms_auth::{
    AuthClaims, AuthEngine, JwtCodec, LoginRequest, NewUserRow, TokenType, UserRepository,
    hash_password,
};
use cycms_config::{Argon2Config, AuthConfig, DatabaseConfig, DatabaseDriver};
use cycms_core::Error;
use cycms_db::DatabasePool;
use cycms_migrate::MigrationEngine;

const TEST_SECRET: &str = "test-jwt-secret";

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

async fn seed_user(
    repo: &UserRepository,
    username: &str,
    password: &str,
    is_active: bool,
    cfg: &Argon2Config,
) -> String {
    let phc = hash_password(password, cfg).unwrap();
    let user = repo
        .create(NewUserRow {
            username: username.to_owned(),
            email: format!("{username}@example.test"),
            password_hash: phc,
            is_active,
        })
        .await
        .unwrap();
    user.id
}

#[tokio::test]
async fn login_with_valid_credentials_returns_token_pair_with_roles() {
    let pool = fresh_sqlite_pool().await;
    let cfg = auth_config();
    let engine = AuthEngine::new(Arc::clone(&pool), cfg.clone()).unwrap();

    let user_id = seed_user(engine.users(), "alice", "StrongPass1!", true, &cfg.argon2).await;

    // 绑定一个角色用于验证 roles 字段
    let DatabasePool::Sqlite(inner) = pool.as_ref() else {
        panic!("expected sqlite pool");
    };
    sqlx::query("INSERT INTO roles (id, name) VALUES (?, ?)")
        .bind("role-editor-0000")
        .bind("editor")
        .execute(inner)
        .await
        .unwrap();
    sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES (?, ?)")
        .bind(&user_id)
        .bind("role-editor-0000")
        .execute(inner)
        .await
        .unwrap();

    let pair = engine
        .login(LoginRequest {
            username: "alice".to_owned(),
            password: "StrongPass1!".to_owned(),
        })
        .await
        .unwrap();
    assert_eq!(pair.expires_in, 900);

    let codec = JwtCodec::new(TEST_SECRET, 900, 1_209_600);
    let claims: AuthClaims = codec.decode(&pair.access_token, TokenType::Access).unwrap();
    assert_eq!(claims.sub, user_id);
    assert_eq!(claims.roles, vec!["editor".to_owned()]);
}

#[tokio::test]
async fn login_with_wrong_password_returns_unauthorized() {
    let pool = fresh_sqlite_pool().await;
    let cfg = auth_config();
    let engine = AuthEngine::new(Arc::clone(&pool), cfg.clone()).unwrap();
    seed_user(engine.users(), "bob", "StrongPass1!", true, &cfg.argon2).await;

    let err = engine
        .login(LoginRequest {
            username: "bob".to_owned(),
            password: "WrongPass1!".to_owned(),
        })
        .await
        .unwrap_err();
    match err {
        Error::Unauthorized { message } => assert_eq!(message, "invalid credentials"),
        other => panic!("expected Unauthorized, got {other:?}"),
    }
}

#[tokio::test]
async fn login_with_missing_user_returns_unauthorized() {
    let pool = fresh_sqlite_pool().await;
    let cfg = auth_config();
    let engine = AuthEngine::new(Arc::clone(&pool), cfg).unwrap();

    let err = engine
        .login(LoginRequest {
            username: "ghost".to_owned(),
            password: "anything-1A!".to_owned(),
        })
        .await
        .unwrap_err();
    match err {
        Error::Unauthorized { message } => assert_eq!(message, "invalid credentials"),
        other => panic!("expected Unauthorized, got {other:?}"),
    }
}

#[tokio::test]
async fn login_with_disabled_user_returns_unauthorized() {
    let pool = fresh_sqlite_pool().await;
    let cfg = auth_config();
    let engine = AuthEngine::new(Arc::clone(&pool), cfg.clone()).unwrap();
    seed_user(engine.users(), "carol", "StrongPass1!", false, &cfg.argon2).await;

    let err = engine
        .login(LoginRequest {
            username: "carol".to_owned(),
            password: "StrongPass1!".to_owned(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Unauthorized { .. }));
}

#[tokio::test]
async fn verify_access_rejects_revoked_token() {
    let pool = fresh_sqlite_pool().await;
    let cfg = auth_config();
    let engine = AuthEngine::new(Arc::clone(&pool), cfg.clone()).unwrap();
    seed_user(engine.users(), "dave", "StrongPass1!", true, &cfg.argon2).await;

    let pair = engine
        .login(LoginRequest {
            username: "dave".to_owned(),
            password: "StrongPass1!".to_owned(),
        })
        .await
        .unwrap();

    // 先校验成功一次
    let claims = engine.verify_access(&pair.access_token).await.unwrap();

    // 将其 jti 加入黑名单，再次校验应失败
    engine
        .revoked_tokens()
        .revoke(&claims.jti, chrono::Utc::now() + chrono::Duration::hours(1), "test")
        .await
        .unwrap();
    let err = engine.verify_access(&pair.access_token).await.unwrap_err();
    assert!(matches!(err, Error::Unauthorized { .. }));
}

#[tokio::test]
async fn verify_access_rejects_refresh_token() {
    let pool = fresh_sqlite_pool().await;
    let cfg = auth_config();
    let engine = AuthEngine::new(Arc::clone(&pool), cfg.clone()).unwrap();
    seed_user(engine.users(), "eva", "StrongPass1!", true, &cfg.argon2).await;

    let pair = engine
        .login(LoginRequest {
            username: "eva".to_owned(),
            password: "StrongPass1!".to_owned(),
        })
        .await
        .unwrap();

    let err = engine.verify_access(&pair.refresh_token).await.unwrap_err();
    assert!(matches!(err, Error::Unauthorized { .. }));
}
