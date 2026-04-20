use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use cycms_auth::{CreateUserInput, LoginRequest};
use cycms_content_engine::{ContentEntry, ContentQuery, ContentStatus, PaginatedResponse};
use cycms_core::{Error, Result};
use cycms_events::{Event, EventKind};
use cycms_permission::seed_defaults;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::common::{created_json, to_user_response};
use crate::handlers::auth::TokenPairResponse;
use crate::query::parse_content_query;
use crate::state::ApiState;

pub fn routes() -> Router<Arc<ApiState>> {
    Router::new()
        .route("/content-types", get(list_content_types))
        .route("/content/{type_api_id}", get(list_published_entries))
        .route(
            "/content/{type_api_id}/{id_or_slug}",
            get(get_published_entry),
        )
        .route("/auth/login", post(member_login))
        .route("/auth/register", post(member_register))
        .route("/auth/refresh", post(member_refresh))
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicContentTypeResponse {
    pub id: String,
    pub name: String,
    pub api_id: String,
    pub description: Option<String>,
}

/// 列出所有内容类型（公开，不含字段详情）。
async fn list_content_types(
    State(state): State<Arc<ApiState>>,
) -> Result<Json<Vec<PublicContentTypeResponse>>> {
    let types = state.content_model.list_types().await?;
    let response: Vec<PublicContentTypeResponse> = types
        .into_iter()
        .map(|ct| PublicContentTypeResponse {
            id: ct.id,
            name: ct.name,
            api_id: ct.api_id,
            description: ct.description,
        })
        .collect();
    Ok(Json(response))
}

/// 列出指定类型下 `status=published` 的内容条目。
async fn list_published_entries(
    State(state): State<Arc<ApiState>>,
    Path(type_api_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<PaginatedResponse<ContentEntry>>> {
    let mut query = parse_content_query(&params)?;
    // 公开接口强制 status=published
    query.status = Some(ContentStatus::Published);
    Ok(Json(state.content_engine.list(&type_api_id, &query).await?))
}

/// 按 ID 或 slug 获取已发布的内容条目。
async fn get_published_entry(
    State(state): State<Arc<ApiState>>,
    Path((type_api_id, id_or_slug)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ContentEntry>> {
    let populate = params
        .get("populate")
        .map(|v| {
            v.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // 优先按 ID 查找
    if let Some(entry) = state
        .content_engine
        .get(&type_api_id, &id_or_slug, &populate)
        .await?
    {
        if entry.status == ContentStatus::Published {
            return Ok(Json(entry));
        }
    }

    // 回退按 slug 查找
    let slug_query = ContentQuery {
        status: Some(ContentStatus::Published),
        filters: vec![cycms_content_engine::FilterSpec {
            field: cycms_content_engine::FieldRef::Column(cycms_content_engine::ColumnField::Slug),
            op: cycms_content_engine::FilterOperator::Eq,
            value: serde_json::Value::String(id_or_slug.clone()),
        }],
        populate: populate.clone(),
        page: Some(1),
        page_size: Some(1),
        ..ContentQuery::default()
    };
    let result = state.content_engine.list(&type_api_id, &slug_query).await?;
    let entry = result.data.into_iter().next().ok_or(Error::NotFound {
        message: format!("published content entry not found: {id_or_slug}"),
    })?;
    Ok(Json(entry))
}

// ── 会员认证 ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct MemberRegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

/// 会员自助注册（需系统已有管理员后方可使用）。
async fn member_register(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<MemberRegisterRequest>,
) -> Result<axum::response::Response> {
    // 只有管理员已存在时才允许会员注册
    if state.auth_engine.users().count().await? == 0 {
        return Err(Error::Conflict {
            message: "system not initialized; use /api/v1/auth/register to create admin first"
                .to_owned(),
        });
    }

    seed_defaults(&state.permission_engine).await?;

    let user = state
        .auth_engine
        .create_user(CreateUserInput {
            username: request.username,
            email: request.email,
            password: request.password,
        })
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
                "member_register": true,
                "result": "success",
            })),
    );
    Ok(created_json(response).into_response())
}

async fn member_login(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<TokenPairResponse>> {
    let pair = state.auth_engine.login(request).await?;
    Ok(Json(TokenPairResponse::from(pair)))
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

async fn member_refresh(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<RefreshRequest>,
) -> Result<Json<TokenPairResponse>> {
    let pair = state.auth_engine.refresh(&request.refresh_token).await?;
    Ok(Json(TokenPairResponse::from(pair)))
}

use axum::response::IntoResponse;
