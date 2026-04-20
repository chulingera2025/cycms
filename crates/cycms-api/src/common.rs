use std::collections::HashSet;

use axum::Json;
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use cycms_auth::{AuthClaims, UpdateUserRow, User, hash_password};
use cycms_core::{Error, Result};
use cycms_permission::{Permission, Role, UpdateRoleRow};
use serde::{Deserialize, Deserializer, Serialize};

use crate::state::ApiState;

#[derive(Debug, Clone, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub email: String,
    pub is_active: bool,
    pub role_ids: Vec<String>,
    pub roles: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoleResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub created_at: DateTime<Utc>,
    pub permissions: Vec<Permission>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum NullablePatch<T> {
    #[default]
    Missing,
    Null,
    Value(T),
}

impl<'de, T> Deserialize<'de> for NullablePatch<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match Option::<Option<T>>::deserialize(deserializer)? {
            None => Self::Missing,
            Some(None) => Self::Null,
            Some(Some(value)) => Self::Value(value),
        })
    }
}

pub fn created_json<T: Serialize>(value: T) -> (StatusCode, Json<T>) {
    (StatusCode::CREATED, Json(value))
}

pub async fn require_permission(
    state: &ApiState,
    claims: &AuthClaims,
    code: &str,
    owner_id: Option<&str>,
) -> Result<()> {
    state
        .permission_engine
        .require_permission(&claims.sub, &claims.roles, code, owner_id)
        .await
}

pub async fn hash_password_for_api(state: &ApiState, password: &str) -> Result<String> {
    Ok(hash_password(password, &state.config.auth.argon2)?)
}

pub async fn sync_user_roles(state: &ApiState, user_id: &str, role_ids: &[String]) -> Result<()> {
    let desired: HashSet<String> = role_ids.iter().cloned().collect();
    let current_roles = state
        .permission_engine
        .roles()
        .list_by_user_id(user_id)
        .await?;
    let current_ids: HashSet<String> = current_roles.iter().map(|role| role.id.clone()).collect();

    for role_id in &desired {
        if state
            .permission_engine
            .roles()
            .find_by_id(role_id)
            .await?
            .is_none()
        {
            return Err(Error::NotFound {
                message: format!("role not found: {role_id}"),
            });
        }
    }

    for role_id in current_ids.difference(&desired) {
        state
            .permission_engine
            .roles()
            .unbind_user(user_id, role_id)
            .await?;
    }

    for role_id in desired.difference(&current_ids) {
        state
            .permission_engine
            .roles()
            .bind_user(user_id, role_id)
            .await?;
    }

    Ok(())
}

pub async fn sync_role_permissions(
    state: &ApiState,
    role_id: &str,
    permission_ids: &[String],
) -> Result<()> {
    let desired: HashSet<String> = permission_ids.iter().cloned().collect();
    let current_permissions = state
        .permission_engine
        .permissions()
        .list_by_role_id(role_id)
        .await?;
    let current_ids: HashSet<String> = current_permissions
        .iter()
        .map(|permission| permission.id.clone())
        .collect();

    for permission_id in &desired {
        if state
            .permission_engine
            .permissions()
            .find_by_id(permission_id)
            .await?
            .is_none()
        {
            return Err(Error::NotFound {
                message: format!("permission not found: {permission_id}"),
            });
        }
    }

    for permission_id in current_ids.difference(&desired) {
        state
            .permission_engine
            .roles()
            .detach_permission(role_id, permission_id)
            .await?;
    }

    for permission_id in desired.difference(&current_ids) {
        state
            .permission_engine
            .roles()
            .attach_permission(role_id, permission_id)
            .await?;
    }

    Ok(())
}

pub async fn to_user_response(state: &ApiState, user: User) -> Result<UserResponse> {
    let roles = state
        .permission_engine
        .roles()
        .list_by_user_id(&user.id)
        .await?;
    Ok(UserResponse {
        id: user.id,
        username: user.username,
        email: user.email,
        is_active: user.is_active,
        role_ids: roles.iter().map(|role| role.id.clone()).collect(),
        roles: roles.iter().map(|role| role.name.clone()).collect(),
        created_at: user.created_at,
        updated_at: user.updated_at,
    })
}

pub async fn to_role_response(state: &ApiState, role: Role) -> Result<RoleResponse> {
    let permissions = state
        .permission_engine
        .permissions()
        .list_by_role_id(&role.id)
        .await?;
    Ok(RoleResponse {
        id: role.id,
        name: role.name,
        description: role.description,
        is_system: role.is_system,
        created_at: role.created_at,
        permissions,
    })
}

pub fn update_user_row(
    username: Option<String>,
    email: Option<String>,
    password_hash: Option<String>,
    is_active: Option<bool>,
) -> UpdateUserRow {
    UpdateUserRow {
        username,
        email,
        password_hash,
        is_active,
    }
}

pub fn update_role_row(name: Option<String>, description: NullablePatch<String>) -> UpdateRoleRow {
    UpdateRoleRow {
        name,
        description: match description {
            NullablePatch::Missing => None,
            NullablePatch::Null => Some(None),
            NullablePatch::Value(value) => Some(Some(value)),
        },
    }
}
