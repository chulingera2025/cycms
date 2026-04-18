use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::from_fn_with_state;
use axum::routing::get;
use cycms_auth::{
    AuthEngine, Authenticated, LoginRequest, NewUserRow, UserRepository, auth_middleware,
    hash_password,
};
use cycms_config::{Argon2Config, AuthConfig, DatabaseConfig, DatabaseDriver};
use cycms_db::DatabasePool;
use cycms_migrate::MigrationEngine;
use http_body_util::BodyExt;
use tower::ServiceExt;

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

async fn seed_user(
    repo: &UserRepository,
    username: &str,
    password: &str,
    cfg: &Argon2Config,
) -> String {
    let phc = hash_password(password, cfg).unwrap();
    repo.create(NewUserRow {
        username: username.to_owned(),
        email: format!("{username}@example.test"),
        password_hash: phc,
        is_active: true,
    })
    .await
    .unwrap()
    .id
}

async fn protected_handler(Authenticated(claims): Authenticated) -> String {
    claims.sub
}

fn build_router(engine: Arc<AuthEngine>) -> Router {
    Router::new()
        .route("/me", get(protected_handler))
        .layer(from_fn_with_state(engine, auth_middleware))
}

async fn read_body(response: axum::response::Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn valid_token_is_allowed_and_sets_claims() {
    let pool = fresh_sqlite_pool().await;
    let cfg = auth_config();
    let engine = Arc::new(AuthEngine::new(Arc::clone(&pool), cfg.clone()).unwrap());
    let user_id = seed_user(engine.users(), "alice", "StrongPass1!", &cfg.argon2).await;

    let pair = engine
        .login(LoginRequest {
            username: "alice".to_owned(),
            password: "StrongPass1!".to_owned(),
        })
        .await
        .unwrap();

    let app = build_router(Arc::clone(&engine));
    let request = Request::builder()
        .uri("/me")
        .header("authorization", format!("Bearer {}", pair.access_token))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(read_body(response).await, user_id);
}

#[tokio::test]
async fn missing_authorization_returns_401() {
    let pool = fresh_sqlite_pool().await;
    let engine = Arc::new(AuthEngine::new(Arc::clone(&pool), auth_config()).unwrap());
    let app = build_router(Arc::clone(&engine));

    let request = Request::builder().uri("/me").body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn wrong_scheme_returns_401() {
    let pool = fresh_sqlite_pool().await;
    let engine = Arc::new(AuthEngine::new(Arc::clone(&pool), auth_config()).unwrap());
    let app = build_router(Arc::clone(&engine));

    let request = Request::builder()
        .uri("/me")
        .header("authorization", "Basic dXNlcjpwYXNz")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn expired_or_tampered_token_returns_401() {
    let pool = fresh_sqlite_pool().await;
    let engine = Arc::new(AuthEngine::new(Arc::clone(&pool), auth_config()).unwrap());
    let app = build_router(Arc::clone(&engine));

    let request = Request::builder()
        .uri("/me")
        .header("authorization", "Bearer not-a-real-jwt")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn refresh_token_is_rejected_by_access_middleware() {
    let pool = fresh_sqlite_pool().await;
    let cfg = auth_config();
    let engine = Arc::new(AuthEngine::new(Arc::clone(&pool), cfg.clone()).unwrap());
    seed_user(engine.users(), "bob", "StrongPass1!", &cfg.argon2).await;

    let pair = engine
        .login(LoginRequest {
            username: "bob".to_owned(),
            password: "StrongPass1!".to_owned(),
        })
        .await
        .unwrap();

    let app = build_router(Arc::clone(&engine));
    let request = Request::builder()
        .uri("/me")
        .header("authorization", format!("Bearer {}", pair.refresh_token))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
