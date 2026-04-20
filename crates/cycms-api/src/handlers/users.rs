use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Router, response::Response};
use cycms_auth::{Authenticated, CreateUserInput};
use cycms_core::{Error, Result};
use serde::Deserialize;

use crate::common::{
    UserResponse, created_json, hash_password_for_api, require_permission, sync_user_roles,
    to_user_response, update_user_row,
};
use crate::state::ApiState;

pub fn routes() -> Router<Arc<ApiState>> {
    Router::new()
        .route("/", get(list_users).post(create_user))
        .route("/{id}", get(get_user).put(update_user).delete(delete_user))
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub is_active: Option<bool>,
    #[serde(default)]
    pub role_ids: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateUserRequest {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub is_active: Option<bool>,
    pub role_ids: Option<Vec<String>>,
}

pub async fn list_users(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
) -> Result<Json<Vec<UserResponse>>> {
    require_permission(&state, &claims, "system.user.read", None).await?;
    let users = state.auth_engine.users().list().await?;
    let mut response = Vec::with_capacity(users.len());
    for user in users {
        response.push(to_user_response(&state, user).await?);
    }
    Ok(Json(response))
}

pub async fn create_user(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Json(request): Json<CreateUserRequest>,
) -> Result<impl IntoResponse> {
    require_permission(&state, &claims, "system.user.manage", None).await?;
    let user = state
        .auth_engine
        .create_user(CreateUserInput {
            username: request.username,
            email: request.email,
            password: request.password,
        })
        .await?;

    if request.is_active == Some(false) {
        state
            .auth_engine
            .users()
            .update(
                &user.id,
                update_user_row(None, None, None, Some(false)),
            )
            .await?;
    }
    if !request.role_ids.is_empty() {
        sync_user_roles(&state, &user.id, &request.role_ids).await?;
    }

    let current = state
        .auth_engine
        .users()
        .find_by_id(&user.id)
        .await?
        .ok_or_else(|| Error::Internal {
            message: "created user not found on read-back".to_owned(),
            source: None,
        })?;
    Ok(created_json(to_user_response(&state, current).await?))
}

pub async fn get_user(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path(id): Path<String>,
) -> Result<Json<UserResponse>> {
    require_permission(&state, &claims, "system.user.read", None).await?;
    let user = state.auth_engine.users().find_by_id(&id).await?.ok_or_else(|| Error::NotFound {
        message: format!("user not found: {id}"),
    })?;
    Ok(Json(to_user_response(&state, user).await?))
}

pub async fn update_user(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path(id): Path<String>,
    Json(request): Json<UpdateUserRequest>,
) -> Result<Json<UserResponse>> {
    require_permission(&state, &claims, "system.user.manage", None).await?;
    let password_hash = match request.password.as_deref() {
        Some(password) => Some(hash_password_for_api(&state, password).await?),
        None => None,
    };
    let user = state
        .auth_engine
        .users()
        .update(
            &id,
            update_user_row(request.username, request.email, password_hash, request.is_active),
        )
        .await?;
    if let Some(role_ids) = request.role_ids {
        sync_user_roles(&state, &id, &role_ids).await?;
    }
    let current = state.auth_engine.users().find_by_id(&id).await?.ok_or_else(|| Error::Internal {
        message: "updated user not found on read-back".to_owned(),
        source: None,
    })?;
    let _ = user;
    Ok(Json(to_user_response(&state, current).await?))
}

pub async fn delete_user(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path(id): Path<String>,
) -> Result<Response> {
    require_permission(&state, &claims, "system.user.manage", None).await?;
	if claims.sub == id {
        return Err(Error::Conflict {
            message: "cannot delete the current authenticated user".to_owned(),
        });
    }
    state.auth_engine.users().delete(&id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}