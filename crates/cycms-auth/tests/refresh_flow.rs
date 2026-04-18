use std::path::PathBuf;
use std::sync::Arc;

use cycms_auth::{
    AuthEngine, JwtCodec, LoginRequest, NewUserRow, TokenType, UserRepository, hash_password,
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

async fn login_fresh(engine: &AuthEngine, username: &str, password: &str) -> cycms_auth::TokenPair {
    engine
        .login(LoginRequest {
            username: username.to_owned(),
            password: password.to_owned(),
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn refresh_rotates_and_old_refresh_is_rejected() {
    let pool = fresh_sqlite_pool().await;
    let cfg = auth_config();
    let engine = AuthEngine::new(Arc::clone(&pool), cfg.clone()).unwrap();
    seed_user(engine.users(), "alice", "StrongPass1!", true, &cfg.argon2).await;

    let pair_v1 = login_fresh(&engine, "alice", "StrongPass1!").await;
    let pair_v2 = engine.refresh(&pair_v1.refresh_token).await.unwrap();
    assert_ne!(pair_v1.access_token, pair_v2.access_token);
    assert_ne!(pair_v1.refresh_token, pair_v2.refresh_token);

    // 新 pair 的 access 仍然可以校验
    engine.verify_access(&pair_v2.access_token).await.unwrap();

    // 旧 refresh 被吊销，第二次使用应失败
    let err = engine.refresh(&pair_v1.refresh_token).await.unwrap_err();
    assert!(matches!(err, Error::Unauthorized { .. }));

    // 新 refresh 也可以继续轮换
    engine.refresh(&pair_v2.refresh_token).await.unwrap();
}

#[tokio::test]
async fn refresh_with_tampered_token_is_rejected() {
    let pool = fresh_sqlite_pool().await;
    let cfg = auth_config();
    let engine = AuthEngine::new(Arc::clone(&pool), cfg.clone()).unwrap();
    seed_user(engine.users(), "bob", "StrongPass1!", true, &cfg.argon2).await;
    let pair = login_fresh(&engine, "bob", "StrongPass1!").await;

    let mut tampered = pair.refresh_token.clone();
    let last = tampered.pop().unwrap();
    // 替换为任何与原字符不同的合法 base64url 字符，避免 pop+push 同字符造成无效篡改
    tampered.push(if last == 'A' { 'B' } else { 'A' });
    let err = engine.refresh(&tampered).await.unwrap_err();
    assert!(matches!(err, Error::Unauthorized { .. }));
}

#[tokio::test]
async fn refresh_rejects_access_token_as_refresh() {
    let pool = fresh_sqlite_pool().await;
    let cfg = auth_config();
    let engine = AuthEngine::new(Arc::clone(&pool), cfg.clone()).unwrap();
    seed_user(engine.users(), "carol", "StrongPass1!", true, &cfg.argon2).await;
    let pair = login_fresh(&engine, "carol", "StrongPass1!").await;

    let err = engine.refresh(&pair.access_token).await.unwrap_err();
    assert!(matches!(err, Error::Unauthorized { .. }));
}

#[tokio::test]
async fn refresh_rejects_disabled_user() {
    let pool = fresh_sqlite_pool().await;
    let cfg = auth_config();
    let engine = AuthEngine::new(Arc::clone(&pool), cfg.clone()).unwrap();
    seed_user(engine.users(), "dave", "StrongPass1!", true, &cfg.argon2).await;
    let pair = login_fresh(&engine, "dave", "StrongPass1!").await;

    // 在 DB 层禁用用户
    let DatabasePool::Sqlite(inner) = pool.as_ref() else {
        panic!("expected sqlite pool");
    };
    sqlx::query("UPDATE users SET is_active = 0 WHERE username = ?")
        .bind("dave")
        .execute(inner)
        .await
        .unwrap();

    let err = engine.refresh(&pair.refresh_token).await.unwrap_err();
    assert!(matches!(err, Error::Unauthorized { .. }));
}

#[tokio::test]
async fn refresh_uses_latest_roles() {
    let pool = fresh_sqlite_pool().await;
    let cfg = auth_config();
    let engine = AuthEngine::new(Arc::clone(&pool), cfg.clone()).unwrap();
    let user_id = seed_user(engine.users(), "eva", "StrongPass1!", true, &cfg.argon2).await;

    let pair_v1 = login_fresh(&engine, "eva", "StrongPass1!").await;
    // 初始 roles = []
    let codec = JwtCodec::new(TEST_SECRET, 900, 1_209_600);
    let claims_v1 = codec.decode(&pair_v1.access_token, TokenType::Access).unwrap();
    assert!(claims_v1.roles.is_empty());

    // 之后授予 editor 角色
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

    // refresh 后新 token 中 roles 应被刷新
    let pair_v2 = engine.refresh(&pair_v1.refresh_token).await.unwrap();
    let claims_v2 = codec.decode(&pair_v2.access_token, TokenType::Access).unwrap();
    assert_eq!(claims_v2.roles, vec!["editor".to_owned()]);
}
