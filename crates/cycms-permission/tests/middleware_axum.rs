use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::{Next, from_fn, from_fn_with_state};
use axum::routing::get;
use cycms_auth::{AuthClaims, TokenType};
use cycms_config::{DatabaseConfig, DatabaseDriver};
use cycms_db::DatabasePool;
use cycms_migrate::MigrationEngine;
use cycms_permission::{
    NewRoleRow, PermissionDefinition, PermissionEngine, PermissionMiddlewareState, PermissionScope,
    SUPER_ADMIN_ROLE, require_permission_middleware,
};
use tower::ServiceExt;

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
    role: &str,
    domain: &str,
    resource: &str,
    action: &str,
    scope: PermissionScope,
) {
    let r = engine
        .roles()
        .create(NewRoleRow {
            name: role.to_owned(),
            description: None,
            is_system: false,
        })
        .await
        .unwrap();
    let perms = engine
        .permissions()
        .upsert_many(
            "system",
            &[PermissionDefinition {
                domain: domain.to_owned(),
                resource: resource.to_owned(),
                action: action.to_owned(),
                scope,
            }],
        )
        .await
        .unwrap();
    engine
        .roles()
        .attach_permission(&r.id, &perms[0].id)
        .await
        .unwrap();
}

fn make_claims(sub: &str, roles: Vec<String>) -> AuthClaims {
    let now = chrono::Utc::now().timestamp();
    AuthClaims {
        sub: sub.to_owned(),
        exp: now + 900,
        iat: now,
        jti: "test-jti".to_owned(),
        token_type: TokenType::Access,
        roles,
    }
}

fn build_router(
    engine: Arc<PermissionEngine>,
    code: &'static str,
    claims: Option<AuthClaims>,
) -> Router {
    let base = Router::new()
        .route("/protected", get(|| async { "ok" }))
        .layer(from_fn_with_state(
            PermissionMiddlewareState::new(engine, code),
            require_permission_middleware,
        ));
    if let Some(c) = claims {
        base.layer(from_fn(move |mut req: Request, next: Next| {
            let c = c.clone();
            async move {
                req.extensions_mut().insert(c);
                next.run(req).await
            }
        }))
    } else {
        base
    }
}

#[tokio::test]
async fn request_without_auth_claims_returns_401() {
    let pool = fresh_sqlite_pool().await;
    let engine = Arc::new(PermissionEngine::new(pool));
    let app = build_router(engine, "system.post.read", None);
    let req = Request::builder()
        .uri("/protected")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn request_with_auth_but_without_permission_returns_403() {
    let pool = fresh_sqlite_pool().await;
    let engine = Arc::new(PermissionEngine::new(pool));
    seed_role_with_permission(
        &engine,
        "editor",
        "system",
        "post",
        "read",
        PermissionScope::All,
    )
    .await;

    let claims = make_claims("u1", vec!["ghost".to_owned()]);
    let app = build_router(engine, "system.post.read", Some(claims));
    let req = Request::builder()
        .uri("/protected")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn request_with_permission_returns_200() {
    let pool = fresh_sqlite_pool().await;
    let engine = Arc::new(PermissionEngine::new(pool));
    seed_role_with_permission(
        &engine,
        "editor",
        "system",
        "post",
        "read",
        PermissionScope::All,
    )
    .await;

    let claims = make_claims("u1", vec!["editor".to_owned()]);
    let app = build_router(engine, "system.post.read", Some(claims));
    let req = Request::builder()
        .uri("/protected")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn super_admin_bypasses_middleware_check() {
    let pool = fresh_sqlite_pool().await;
    let engine = Arc::new(PermissionEngine::new(pool));
    // 不 seed 任何权限表行
    let claims = make_claims("u1", vec![SUPER_ADMIN_ROLE.to_owned()]);
    let app = build_router(engine, "system.anything.do", Some(claims));
    let req = Request::builder()
        .uri("/protected")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn invalid_code_in_middleware_returns_500() {
    // 不合法的权限字符串会被 parser 抛 ValidationError，中间件统一转 500。
    // 这一行为让运维能在日志里区分"权限配置错误"与"权限拒绝"。
    let pool = fresh_sqlite_pool().await;
    let engine = Arc::new(PermissionEngine::new(pool));
    let claims = make_claims("u1", vec!["editor".to_owned()]);
    let app = build_router(engine, "INVALID.code", Some(claims));
    let req = Request::builder()
        .uri("/protected")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
