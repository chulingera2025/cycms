use std::sync::Arc;

use axum::extract::{FromRequestParts, Request, State};
use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, request::Parts};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use cycms_core::Error;

use crate::claims::AuthClaims;
use crate::service::AuthEngine;

/// axum 认证中间件：从 `Authorization: Bearer <token>` 头中解析 access token，
/// 校验通过后把 [`AuthClaims`] 注入到 `Request::extensions`，供下游
/// [`Authenticated`] 提取器和权限中间件取用。
///
/// 任何失败路径都返回 401 并携带 `{code, message}` JSON，**不泄露具体原因**。
pub async fn auth_middleware(
    State(engine): State<Arc<AuthEngine>>,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(token) = extract_bearer_token(request.headers()) else {
        return unauthorized_response("authentication required");
    };

    let Ok(claims) = engine.verify_access(token).await else {
        return unauthorized_response("invalid credentials");
    };

    request.extensions_mut().insert(claims);
    next.run(request).await
}

/// Handler 侧使用的 claims 提取器：`async fn handler(Authenticated(claims): Authenticated)`。
///
/// 中间件通过 `Request::extensions` 传递 claims，这里只需 `.get::<AuthClaims>().cloned()`。
#[derive(Debug, Clone)]
pub struct Authenticated(pub AuthClaims);

impl<S> FromRequestParts<S> for Authenticated
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthClaims>()
            .cloned()
            .map(Authenticated)
            .ok_or_else(|| unauthorized_response("authentication required"))
    }
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}

fn unauthorized_response(message: &str) -> Response {
    Error::Unauthorized {
        message: message.to_owned(),
    }
    .into_response()
}
