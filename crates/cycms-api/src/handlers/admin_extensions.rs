use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::Response;
use axum::routing::get;
use cycms_auth::Authenticated;
use cycms_core::{Error, Result};

use crate::common::require_permission;
use crate::state::ApiState;

pub fn protected_routes() -> Router<Arc<ApiState>> {
    Router::new()
        .route("/bootstrap", get(get_bootstrap))
        .route("/diagnostics", get(get_diagnostics))
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
) -> Result<Json<cycms_plugin_manager::AdminExtensionDiagnostics>> {
    require_permission(&state, &claims, "plugin.lifecycle.read", None).await?;
    Ok(Json(
        state.plugin_manager.admin_extension_diagnostics().await?,
    ))
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
