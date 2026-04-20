use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Router, response::IntoResponse};
use cycms_auth::{Authenticated, CreateUserInput, LoginRequest, TokenPair};
use cycms_core::{Error, Result};
use cycms_events::{Event, EventKind};
use cycms_permission::{SUPER_ADMIN_ROLE, seed_defaults};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::common::{UserResponse, created_json, to_user_response};
use crate::state::ApiState;

pub fn public_routes() -> Router<Arc<ApiState>> {
    Router::new()
        .route("/login", post(login))
        .route("/register", post(register))
        .route("/refresh", post(refresh))
}

pub fn protected_routes() -> Router<Arc<ApiState>> {
    Router::new().route("/me", get(me))
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenPairResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

pub async fn login(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<TokenPairResponse>> {
    let pair = state.auth_engine.login(request).await?;
    Ok(Json(TokenPairResponse::from(pair)))
}

pub async fn register(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<RegisterRequest>,
) -> Result<impl IntoResponse> {
    if state.auth_engine.users().count().await? > 0 {
        return Err(Error::Conflict {
            message: "initial administrator already exists; use the user management API".to_owned(),
        });
    }

    seed_defaults(&state.permission_engine).await?;

    let user = state
        .auth_engine
        .setup_admin(CreateUserInput {
            username: request.username,
            email: request.email,
            password: request.password,
        })
        .await?;

    let super_admin = state
        .permission_engine
        .roles()
        .find_by_name(SUPER_ADMIN_ROLE)
        .await?
        .ok_or_else(|| Error::Internal {
            message: "super_admin role not found after seeding defaults".to_owned(),
            source: None,
        })?;
    state
        .permission_engine
        .roles()
        .bind_user(&user.id, &super_admin.id)
        .await?;

    let response = to_user_response(&state, user).await?;
    state.event_bus.publish(
        Event::new(EventKind::UserCreated)
            .with_actor(&response.id)
            .with_payload(json!({
                "id": response.id.clone(),
                "username": response.username.clone(),
                "email": response.email.clone(),
                "role_ids": response.role_ids.clone(),
                "bootstrap": true,
                "result": "success",
            })),
    );
    Ok(created_json(response))
}

pub async fn refresh(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<RefreshRequest>,
) -> Result<Json<TokenPairResponse>> {
    let pair = state.auth_engine.refresh(&request.refresh_token).await?;
    Ok(Json(TokenPairResponse::from(pair)))
}

pub async fn me(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
) -> Result<Json<UserResponse>> {
    let user = state
        .auth_engine
        .users()
        .find_by_id(&claims.sub)
        .await?
        .ok_or_else(|| Error::Unauthorized {
            message: "invalid credentials".to_owned(),
        })?;
    Ok(Json(to_user_response(&state, user).await?))
}

impl From<TokenPair> for TokenPairResponse {
    fn from(value: TokenPair) -> Self {
        Self {
            access_token: value.access_token,
            refresh_token: value.refresh_token,
            expires_in: value.expires_in,
        }
    }
}
