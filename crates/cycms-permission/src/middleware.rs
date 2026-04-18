use std::sync::Arc;

use axum::Json;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use cycms_auth::AuthClaims;

use crate::service::PermissionEngine;

/// axum 权限中间件 state，携带 `engine` 引用与要校验的权限 `code`。
///
/// 调用方使用 [`axum::middleware::from_fn_with_state`] 绑定到 Router：
///
/// ```ignore
/// use axum::middleware::from_fn_with_state;
/// use cycms_permission::{PermissionMiddlewareState, require_permission_middleware};
///
/// let layer = from_fn_with_state(
///     PermissionMiddlewareState::new(engine.clone(), "system.post.read"),
///     require_permission_middleware,
/// );
/// ```
#[derive(Clone)]
pub struct PermissionMiddlewareState {
    pub engine: Arc<PermissionEngine>,
    pub code: &'static str,
}

impl PermissionMiddlewareState {
    #[must_use]
    pub fn new(engine: Arc<PermissionEngine>, code: &'static str) -> Self {
        Self { engine, code }
    }
}

/// 权限校验中间件：从 `Request::extensions` 读取由 `auth_middleware` 注入的
/// [`AuthClaims`]，调用 [`PermissionEngine::check_permission`] 做粗粒度校验。
///
/// - 缺 claims → 401
/// - 有 claims 但无权限 → 403
/// - DB / 解析错误 → 500
///
/// `scope=own` 的细粒度校验需要 resource owner 信息，不在此中间件完成；
/// handler 应在业务逻辑处自行 `engine.require_permission(..., Some(owner_id))`。
// TODO!!!: 任务 18 实现统一 `IntoResponse for cycms_core::Error` 后，
//         本函数的 JSON 错误构造应迁移到该统一层。
pub async fn require_permission_middleware(
    State(state): State<PermissionMiddlewareState>,
    req: Request,
    next: Next,
) -> Response {
    let Some(claims) = req.extensions().get::<AuthClaims>().cloned() else {
        return unauthorized_response("missing authentication context");
    };

    match state
        .engine
        .check_permission(&claims.sub, &claims.roles, state.code, None)
        .await
    {
        Ok(true) => next.run(req).await,
        Ok(false) => forbidden_response(),
        Err(err) => internal_error_response(&err.to_string()),
    }
}

fn unauthorized_response(message: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({ "code": "unauthorized", "message": message })),
    )
        .into_response()
}

fn forbidden_response() -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(serde_json::json!({ "code": "forbidden", "message": "permission denied" })),
    )
        .into_response()
}

fn internal_error_response(message: &str) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "code": "internal_error", "message": message })),
    )
        .into_response()
}
