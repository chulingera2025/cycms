use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use cycms_api::{ApiState, build_router};
use cycms_kernel::Kernel;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tempfile::tempdir;
use tower::ServiceExt;

fn system_migrations_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../cycms-migrate/migrations/system")
}

async fn build_test_app() -> Router {
    let temp = tempdir().unwrap();
    let uploads_dir = temp.path().join("uploads");
    let plugins_dir = temp.path().join("plugins");
    fs::create_dir_all(&uploads_dir).unwrap();
    fs::create_dir_all(&plugins_dir).unwrap();

    let config_path = temp.path().join("cycms.toml");
    fs::write(
        &config_path,
        format!(
            r#"
[database]
driver = "sqlite"
url = "sqlite::memory:"
max_connections = 1
connect_timeout_secs = 5
idle_timeout_secs = 60

[auth]
jwt_secret = "test-jwt-secret"
access_token_ttl_secs = 900
refresh_token_ttl_secs = 3600

[auth.argon2]
m_cost = 16
t_cost = 2
p_cost = 1

[media]
upload_dir = "{}"

[plugins]
directory = "{}"
wasm_enabled = true
"#,
            uploads_dir.display(),
            plugins_dir.display(),
        ),
    )
    .unwrap();

    let kernel = Kernel::build(Some(&config_path)).await.unwrap();
    let ctx = kernel
        .bootstrap(Some(&system_migrations_root()))
        .await
        .unwrap();

    let state = Arc::new(ApiState::new(
        Arc::clone(&ctx.config),
        Arc::clone(&ctx.auth_engine),
        Arc::clone(&ctx.permission_engine),
        Arc::clone(&ctx.content_model),
        Arc::clone(&ctx.content_engine),
        Arc::clone(&ctx.revision_manager),
        Arc::clone(&ctx.publish_manager),
        Arc::clone(&ctx.media_manager),
        Arc::clone(&ctx.plugin_manager),
        Arc::clone(&ctx.settings_manager),
        Arc::clone(&ctx.service_registry),
        Arc::clone(&ctx.native_runtime),
        Arc::clone(&ctx.wasm_runtime),
    ));

    build_router(state)
}

fn json_request(method: Method, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap()
}

fn authorized_json_request(method: Method, uri: &str, token: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap()
}

fn authorized_empty_request(method: Method, uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

async fn read_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn bootstrap_admin(app: Router) -> (Router, String) {
    let response = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/auth/register",
            json!({
                "username": "admin",
                "email": "admin@example.test",
                "password": "StrongPass1!"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/auth/login",
            json!({
                "username": "admin",
                "password": "StrongPass1!"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json(response).await;
    let token = body["access_token"].as_str().unwrap().to_owned();
    (app, token)
}

#[tokio::test]
async fn docs_endpoint_is_public_and_unauthorized_errors_are_unified() {
    let app = build_test_app().await;

    let response = app
        .clone()
        .oneshot(Request::builder().uri("/api/docs").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json(response).await;
    assert_eq!(body["openapi"], json!("3.1.0"));
    assert!(body["paths"]["/api/v1/auth/login"].is_object());

    let response = app
        .oneshot(Request::builder().uri("/api/v1/auth/me").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = read_json(response).await;
    assert_eq!(body["error"]["status"], json!(401));
    assert_eq!(body["error"]["name"], json!("unauthorized"));
    assert_eq!(body["error"]["code"], json!("unauthorized"));
}

#[tokio::test]
async fn register_login_and_me_flow_returns_current_user() {
    let app = build_test_app().await;
    let (app, token) = bootstrap_admin(app).await;

    let response = app
        .oneshot(authorized_empty_request(Method::GET, "/api/v1/auth/me", &token))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json(response).await;
    assert_eq!(body["username"], json!("admin"));
    assert_eq!(body["email"], json!("admin@example.test"));
    assert!(body["roles"].as_array().unwrap().iter().any(|role| role == "super_admin"));
}

#[tokio::test]
async fn creating_content_type_updates_openapi_document() {
    let app = build_test_app().await;
    let (app, token) = bootstrap_admin(app).await;

    let response = app
        .clone()
        .oneshot(authorized_json_request(
            Method::POST,
            "/api/v1/content-types",
            &token,
            json!({
                "name": "Article",
                "api_id": "article",
                "description": "Article content type",
                "kind": "collection",
                "fields": [
                    {
                        "name": "Title",
                        "api_id": "title",
                        "field_type": { "kind": "text" },
                        "required": true,
                        "unique": false,
                        "validations": [],
                        "position": 0
                    }
                ]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = app
        .oneshot(Request::builder().uri("/api/docs").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json(response).await;
    assert!(body["paths"]["/api/v1/content/article"].is_object());
    assert!(body["paths"]["/api/v1/content/article/{id}"].is_object());
    assert!(body["components"]["schemas"]["ContentEntryArticle"].is_object());
    assert!(body["components"]["schemas"]["ContentFieldsArticle"].is_object());
}