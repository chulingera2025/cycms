use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Router, response::Response};
use cycms_auth::Authenticated;
use cycms_core::{Error, Result};
use cycms_permission::NewRoleRow;
use serde::Deserialize;

use crate::common::{
    NullablePatch, RoleResponse, created_json, require_permission, sync_role_permissions, to_role_response,
    update_role_row,
};
use crate::state::ApiState;

pub fn routes() -> Router<Arc<ApiState>> {
    Router::new()
        .route("/", get(list_roles).post(create_role))
        .route("/permissions", get(list_permissions))
        .route("/{id}", get(get_role).put(update_role).delete(delete_role))
}

#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub permission_ids: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    #[serde(default)]
    pub description: NullablePatch<String>,
    pub permission_ids: Option<Vec<String>>,
}

pub async fn list_roles(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
) -> Result<Json<Vec<RoleResponse>>> {
    require_permission(&state, &claims, "system.role.read", None).await?;
    let roles = state.permission_engine.roles().list().await?;
    let mut response = Vec::with_capacity(roles.len());
    for role in roles {
        response.push(to_role_response(&state, role).await?);
    }
    Ok(Json(response))
}

pub async fn list_permissions(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
) -> Result<Json<Vec<cycms_permission::Permission>>> {
    require_permission(&state, &claims, "system.role.read", None).await?;
    Ok(Json(
        state.permission_engine.permissions().list_all().await?,
    ))
}

pub async fn create_role(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Json(request): Json<CreateRoleRequest>,
) -> Result<impl IntoResponse> {
    require_permission(&state, &claims, "system.role.manage", None).await?;
    let role = state
        .permission_engine
        .roles()
        .create(NewRoleRow {
            name: request.name,
            description: request.description,
            is_system: false,
        })
        .await?;
    if !request.permission_ids.is_empty() {
        sync_role_permissions(&state, &role.id, &request.permission_ids).await?;
    }
    let current = state
        .permission_engine
        .roles()
        .find_by_id(&role.id)
        .await?
        .ok_or_else(|| Error::Internal {
            message: "created role not found on read-back".to_owned(),
            source: None,
        })?;
    Ok(created_json(to_role_response(&state, current).await?))
}

pub async fn get_role(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path(id): Path<String>,
) -> Result<Json<RoleResponse>> {
    require_permission(&state, &claims, "system.role.read", None).await?;
    let role = state
        .permission_engine
        .roles()
        .find_by_id(&id)
        .await?
        .ok_or_else(|| Error::NotFound {
            message: format!("role not found: {id}"),
        })?;
    Ok(Json(to_role_response(&state, role).await?))
}

pub async fn update_role(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path(id): Path<String>,
    Json(request): Json<UpdateRoleRequest>,
) -> Result<Json<RoleResponse>> {
    require_permission(&state, &claims, "system.role.manage", None).await?;
    state
        .permission_engine
        .roles()
        .update(&id, update_role_row(request.name, request.description))
        .await?;
    if let Some(permission_ids) = request.permission_ids {
        sync_role_permissions(&state, &id, &permission_ids).await?;
    }
    let current = state
        .permission_engine
        .roles()
        .find_by_id(&id)
        .await?
        .ok_or_else(|| Error::Internal {
            message: "updated role not found on read-back".to_owned(),
            source: None,
        })?;
    Ok(Json(to_role_response(&state, current).await?))
}

pub async fn delete_role(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path(id): Path<String>,
) -> Result<Response> {
    require_permission(&state, &claims, "system.role.manage", None).await?;
    state.permission_engine.roles().delete(&id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
