use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Router, response::Response};
use cycms_auth::Authenticated;
use cycms_core::{Error, Result};
use serde::Deserialize;
use serde_json::Value;

use crate::common::require_permission;
use crate::state::ApiState;

pub fn routes() -> Router<Arc<ApiState>> {
    Router::new()
        .route("/schemas", get(list_schemas))
        .route("/{namespace}", get(list_namespace_settings))
        .route(
            "/{namespace}/{key}",
            get(get_setting).put(set_setting).delete(delete_setting),
        )
}

#[derive(Debug, Deserialize)]
pub struct SettingValueRequest {
    pub value: Value,
}

pub async fn list_schemas(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
) -> Result<Json<Vec<cycms_settings::PluginSchema>>> {
    require_permission(&state, &claims, "settings.namespace.read", None).await?;
    Ok(Json(state.settings_manager.list_schemas().await?))
}

pub async fn list_namespace_settings(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path(namespace): Path<String>,
) -> Result<Json<Vec<cycms_settings::SettingEntry>>> {
    require_permission(&state, &claims, "settings.namespace.read", None).await?;
    Ok(Json(
        state
            .settings_manager
            .settings()
            .list_by_namespace(&namespace)
            .await?,
    ))
}

pub async fn get_setting(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path((namespace, key)): Path<(String, String)>,
) -> Result<Json<cycms_settings::SettingEntry>> {
    require_permission(&state, &claims, "settings.namespace.read", None).await?;
    let entry = state
        .settings_manager
        .settings()
        .find(&namespace, &key)
        .await?
        .ok_or_else(|| Error::NotFound {
            message: format!("setting not found: {namespace}.{key}"),
        })?;
    Ok(Json(entry))
}

pub async fn set_setting(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path((namespace, key)): Path<(String, String)>,
    Json(request): Json<SettingValueRequest>,
) -> Result<Json<cycms_settings::SettingEntry>> {
    require_permission(&state, &claims, "settings.namespace.manage", None).await?;
    Ok(Json(state.settings_manager.set(&namespace, &key, request.value).await?))
}

pub async fn delete_setting(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path((namespace, key)): Path<(String, String)>,
) -> Result<Response> {
    require_permission(&state, &claims, "settings.namespace.manage", None).await?;
    if state.settings_manager.delete(&namespace, &key).await? {
        Ok(StatusCode::NO_CONTENT.into_response())
    } else {
        Err(Error::NotFound {
            message: format!("setting not found: {namespace}.{key}"),
        })
    }
}