use std::sync::Arc;

use axum::extract::Request;
use axum::Json;
use axum::Router;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::Response;
use axum::routing::{get, post};
use cycms_auth::Authenticated;
use cycms_core::{Error, Result};
use cycms_observability::RequestContext;
use serde_json::Value;
use tracing::{info, warn};

use crate::common::require_permission;
use crate::{
    AdminExtensionClientEventPayload, AdminExtensionDiagnosticsResponse,
    build_admin_extension_security_state, build_csp_report_event,
    normalize_csp_report_payload, with_request_context,
};
use crate::state::ApiState;

pub fn protected_routes() -> Router<Arc<ApiState>> {
    Router::new()
        .route("/bootstrap", get(get_bootstrap))
        .route("/diagnostics", get(get_diagnostics))
        .route("/events", post(post_event))
}

pub fn public_routes() -> Router<Arc<ApiState>> {
    Router::new().route(
        "/{plugin}/{version}/{url_hash}/{*asset_path}",
        get(get_plugin_asset),
    )
}

pub async fn get_bootstrap(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
) -> Result<Json<cycms_plugin_manager::AdminExtensionBootstrap>> {
    Ok(Json(
        state
            .plugin_manager
            .admin_extension_bootstrap(&claims.sub, &claims.roles)
            .await?,
    ))
}

pub async fn get_diagnostics(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
) -> Result<Json<AdminExtensionDiagnosticsResponse>> {
    require_permission(&state, &claims, "plugin.lifecycle.read", None).await?;
    let diagnostics = state.plugin_manager.admin_extension_diagnostics().await?;
    let recent_events = state.admin_extension_events.snapshot().await;
    let security = build_admin_extension_security_state(&state.config.admin_extensions);
    Ok(Json(AdminExtensionDiagnosticsResponse {
        revision: diagnostics.revision,
        diagnostics: diagnostics.diagnostics,
        recent_events,
        security,
    }))
}

pub async fn post_event(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    headers: HeaderMap,
    request: Request,
) -> Result<StatusCode> {
    let request_id = request
        .extensions()
        .get::<RequestContext>()
        .map(|ctx| ctx.request_id.clone());
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let body = axum::body::to_bytes(request.into_body(), usize::MAX)
        .await
        .map_err(|source| Error::BadRequest {
            message: format!("failed to read admin extension event body: {source}"),
            source: None,
        })?;

    if body.is_empty() {
        return Ok(StatusCode::NO_CONTENT);
    }

    if content_type.contains("application/csp-report")
        || content_type.contains("application/reports+json")
    {
        let payload = serde_json::from_slice::<Value>(&body).map_err(|source| Error::BadRequest {
            message: format!("invalid csp report payload: {source}"),
            source: None,
        })?;
        for report in normalize_csp_report_payload(payload) {
            let event = with_request_context(
                build_csp_report_event(report),
                Some(&claims.sub),
                request_id.as_deref(),
            );
            let record = state.admin_extension_events.record(event).await;
            warn!(
                target: "admin_extensions.telemetry",
                source = %record.source,
                level = %record.level,
                event_name = %record.event_name,
                message = %record.message,
                actor_id = record.actor_id.as_deref().unwrap_or("-"),
                request_id = record.request_id.as_deref().unwrap_or("-"),
                full_path = record.full_path.as_deref().unwrap_or("-"),
                "admin extension event recorded"
            );
        }
        return Ok(StatusCode::NO_CONTENT);
    }

    let payload = serde_json::from_slice::<AdminExtensionClientEventPayload>(&body).map_err(
        |source| Error::BadRequest {
            message: format!("invalid admin extension telemetry payload: {source}"),
            source: None,
        },
    )?;
    let event = with_request_context(
        crate::admin_extensions_observability::AdminExtensionRecordedEvent::client(payload),
        Some(&claims.sub),
        request_id.as_deref(),
    );
    let record = state.admin_extension_events.record(event).await;
    info!(
        target: "admin_extensions.telemetry",
        source = %record.source,
        level = %record.level,
        event_name = %record.event_name,
        message = %record.message,
        actor_id = record.actor_id.as_deref().unwrap_or("-"),
        request_id = record.request_id.as_deref().unwrap_or("-"),
        plugin_name = record.plugin_name.as_deref().unwrap_or("-"),
        contribution_id = record.contribution_id.as_deref().unwrap_or("-"),
        contribution_kind = record.contribution_kind.as_deref().unwrap_or("-"),
        full_path = record.full_path.as_deref().unwrap_or("-"),
        "admin extension event recorded"
    );
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_plugin_asset(
    State(state): State<Arc<ApiState>>,
    Path((plugin, version, url_hash, asset_path)): Path<(String, String, String, String)>,
    headers: HeaderMap,
) -> Result<Response> {
    let asset_path = asset_path.trim_start_matches('/');
    let asset = state
        .plugin_manager
        .resolve_frontend_asset(&plugin, &version, &url_hash, asset_path)
        .await?
        .ok_or_else(|| Error::NotFound {
            message: format!("plugin asset not found: {plugin}/{version}/{asset_path}"),
        })?;

    if headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == asset.etag)
    {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::NOT_MODIFIED;
        response
            .headers_mut()
            .insert(header::ETAG, header_value(&asset.etag)?);
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        );
        response.headers_mut().insert(
            header::HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        );
        return Ok(response);
    }

    let body = tokio::fs::read(&asset.absolute_path)
        .await
        .map_err(|source| Error::Internal {
            message: format!(
                "read plugin asset {}: {source}",
                asset.absolute_path.display()
            ),
            source: None,
        })?;

    let mut response = Response::new(Body::from(body));
    *response.status_mut() = StatusCode::OK;
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, header_value(&asset.content_type)?);
    response
        .headers_mut()
        .insert(header::ETAG, header_value(&asset.etag)?);
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    response.headers_mut().insert(
        header::HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}

fn header_value(value: &str) -> Result<HeaderValue> {
    HeaderValue::from_str(value).map_err(|source| Error::Internal {
        message: format!("invalid response header value {value:?}: {source}"),
        source: None,
    })
}
