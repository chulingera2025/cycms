use std::time::Instant;

use axum::extract::Request;
use axum::http::HeaderValue;
use axum::http::header::HeaderName;
use axum::middleware::Next;
use axum::response::Response;
use tracing::Instrument;
use tracing::field::Empty;
use uuid::Uuid;

pub const REQUEST_ID_HEADER: &str = "x-request-id";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestContext {
    pub request_id: String,
}

pub async fn request_span_middleware(mut request: Request, next: Next) -> Response {
    let request_id = resolve_request_id(request.headers());
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let start = Instant::now();

    request.extensions_mut().insert(RequestContext {
        request_id: request_id.clone(),
    });

    let span = tracing::info_span!(
        "http.request",
        request_id = %request_id,
        method = %method,
        path = %path,
        status = Empty,
        latency_ms = Empty,
    );

    let mut response = next.run(request).instrument(span.clone()).await;

    let status = response.status().as_u16();
    let latency_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
    span.record("status", status);
    span.record("latency_ms", latency_ms);
    tracing::info!(parent: &span, status, latency_ms, "request completed");

    response.headers_mut().insert(
        HeaderName::from_static(REQUEST_ID_HEADER),
        HeaderValue::from_str(&request_id)
            .unwrap_or_else(|_| HeaderValue::from_static("invalid-request-id")),
    );

    response
}

fn resolve_request_id(headers: &axum::http::HeaderMap) -> String {
    incoming_request_id(headers).unwrap_or_else(|| Uuid::new_v4().to_string())
}

fn incoming_request_id(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| {
            !value.is_empty() && value.len() <= 128 && value.chars().all(|ch| ch.is_ascii_graphic())
        })
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use axum::Extension;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::middleware;
    use axum::routing::get;
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    use uuid::Uuid;

    use super::{REQUEST_ID_HEADER, RequestContext, request_span_middleware};

    async fn echo_request_id(Extension(ctx): Extension<RequestContext>) -> String {
        ctx.request_id
    }

    #[tokio::test]
    async fn reuses_incoming_request_id_header() {
        let app = Router::new()
            .route("/", get(echo_request_id))
            .layer(middleware::from_fn(request_span_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(REQUEST_ID_HEADER, "req-123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[REQUEST_ID_HEADER], "req-123");

        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, "req-123");
    }

    #[tokio::test]
    async fn generates_request_id_when_missing() {
        let app = Router::new()
            .route("/", get(echo_request_id))
            .layer(middleware::from_fn(request_span_middleware));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let header_value = response.headers()[REQUEST_ID_HEADER]
            .to_str()
            .unwrap()
            .to_owned();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_value = String::from_utf8(body.to_vec()).unwrap();
        assert_eq!(header_value, body_value);
        assert!(Uuid::parse_str(&body_value).is_ok());
    }
}
