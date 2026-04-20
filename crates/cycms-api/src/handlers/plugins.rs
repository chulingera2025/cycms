use std::path::PathBuf;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::Router;
use cycms_auth::Authenticated;
use cycms_core::{Error, Result};
use cycms_plugin_manager::{PluginInfo, discover_plugin_dir};
use serde::{Deserialize, Serialize};

use crate::common::{created_json, require_permission};
use crate::state::ApiState;

pub fn routes() -> Router<Arc<ApiState>> {
    Router::new()
        .route("/", get(list_plugins))
        .route("/install", post(install_plugin))
        .route("/{name}", get(get_plugin).delete(uninstall_plugin))
        .route("/{name}/enable", post(enable_plugin))
        .route("/{name}/disable", post(disable_plugin))
}

#[derive(Debug, Deserialize)]
pub struct InstallPluginRequest {
    pub path: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct DisablePluginRequest {
    pub force: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginInfoResponse {
    pub name: String,
    pub version: String,
    pub kind: String,
    pub status: String,
    pub dependencies: Vec<String>,
    pub permissions: Vec<String>,
}

pub async fn list_plugins(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
) -> Result<Json<Vec<PluginInfoResponse>>> {
    require_permission(&state, &claims, "plugin.lifecycle.read", None).await?;
    let plugins = state.plugin_manager.list().await?;
    Ok(Json(plugins.into_iter().map(PluginInfoResponse::from).collect()))
}

pub async fn get_plugin(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path(name): Path<String>,
) -> Result<Json<PluginInfoResponse>> {
    require_permission(&state, &claims, "plugin.lifecycle.read", None).await?;
    let plugin = state
        .plugin_manager
        .list()
        .await?
        .into_iter()
        .find(|plugin| plugin.name == name)
        .ok_or_else(|| Error::NotFound {
            message: format!("plugin not found: {name}"),
        })?;
    Ok(Json(PluginInfoResponse::from(plugin)))
}

pub async fn install_plugin(
    Authenticated(claims): Authenticated,
    State(state): State<Arc<ApiState>>,
    Json(request): Json<InstallPluginRequest>,
) -> Result<(StatusCode, Json<PluginInfoResponse>)> {
    require_permission(&state, &claims, "plugin.lifecycle.manage", None).await?;
    let plugin_manager = Arc::clone(&state.plugin_manager);
    let runtime_handle = tokio::runtime::Handle::current();
    let info = tokio::task::spawn_blocking(move || -> Result<PluginInfo> {
        let directory = PathBuf::from(request.path);
        let discovered = discover_plugin_dir(&directory)?;
        runtime_handle.block_on(async move { plugin_manager.install(&discovered).await })
    })
    .await
    .map_err(|error| Error::Internal {
        message: format!("plugin install task join failed: {error}"),
        source: None,
    })??;
    Ok(created_json(PluginInfoResponse::from(info)))
}

pub async fn enable_plugin(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path(name): Path<String>,
) -> Result<Json<PluginInfoResponse>> {
    require_permission(&state, &claims, "plugin.lifecycle.manage", None).await?;
    state.plugin_manager.enable(&name).await?;
    let plugin = state
        .plugin_manager
        .list()
        .await?
        .into_iter()
        .find(|plugin| plugin.name == name)
        .ok_or_else(|| Error::NotFound {
            message: format!("plugin not found: {name}"),
        })?;
    Ok(Json(PluginInfoResponse::from(plugin)))
}

pub async fn disable_plugin(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path(name): Path<String>,
    Json(request): Json<DisablePluginRequest>,
) -> Result<Json<PluginInfoResponse>> {
    require_permission(&state, &claims, "plugin.lifecycle.manage", None).await?;
    state
        .plugin_manager
        .disable(&name, request.force.unwrap_or(false))
        .await?;
    let plugin = state
        .plugin_manager
        .list()
        .await?
        .into_iter()
        .find(|plugin| plugin.name == name)
        .ok_or_else(|| Error::NotFound {
            message: format!("plugin not found: {name}"),
        })?;
    Ok(Json(PluginInfoResponse::from(plugin)))
}

pub async fn uninstall_plugin(
    Path(name): Path<String>,
    Authenticated(claims): Authenticated,
    State(state): State<Arc<ApiState>>,
) -> Result<StatusCode> {
    require_permission(&state, &claims, "plugin.lifecycle.manage", None).await?;
    let plugin_manager = Arc::clone(&state.plugin_manager);
    let runtime_handle = tokio::runtime::Handle::current();
    tokio::task::spawn_blocking(move || -> Result<()> {
        runtime_handle.block_on(async move { plugin_manager.uninstall(&name).await })
    })
    .await
    .map_err(|error| Error::Internal {
        message: format!("plugin uninstall task join failed: {error}"),
        source: None,
    })??;
    Ok(StatusCode::NO_CONTENT)
}

impl From<PluginInfo> for PluginInfoResponse {
    fn from(value: PluginInfo) -> Self {
        Self {
            name: value.name,
            version: value.version,
            kind: value.kind.to_string(),
            status: value.status.to_string(),
            dependencies: value.dependencies,
            permissions: value.permissions,
        }
    }
}