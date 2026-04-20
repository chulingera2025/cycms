use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Router, response::Response};
use cycms_auth::Authenticated;
use cycms_config::DeleteMode;
use cycms_content_engine::{ContentEntry, CreateEntryInput, UpdateEntryInput};
use cycms_core::{Error, Result};
use cycms_revision::Revision;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::common::{created_json, require_permission};
use crate::query::parse_content_query;
use crate::state::ApiState;

pub fn routes() -> Router<Arc<ApiState>> {
    Router::new()
        .route("/{type_api_id}", get(list_entries).post(create_entry))
        .route(
            "/{type_api_id}/{id}",
            get(get_entry).put(update_entry).delete(delete_entry),
        )
        .route("/{type_api_id}/{id}/publish", post(publish_entry))
        .route("/{type_api_id}/{id}/unpublish", post(unpublish_entry))
        .route("/{type_api_id}/{id}/revisions", get(list_revisions))
        .route("/{type_api_id}/{id}/revisions/{version}", get(get_revision))
        .route(
            "/{type_api_id}/{id}/revisions/{version}/rollback",
            post(rollback_revision),
        )
}

#[derive(Debug, Deserialize)]
pub struct CreateEntryRequest {
    pub data: Value,
    pub slug: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateEntryRequest {
    pub data: Option<Value>,
    pub slug: Option<Option<String>>,
}

#[derive(Debug, Deserialize, Default)]
pub struct DeleteEntryQuery {
    pub mode: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct RevisionListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RevisionListResponse {
    pub data: Vec<Revision>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

pub async fn list_entries(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path(type_api_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<cycms_content_engine::PaginatedResponse<ContentEntry>>> {
    require_permission(&state, &claims, "content.entry.read", None).await?;
    let query = parse_content_query(&params)?;
    Ok(Json(state.content_engine.list(&type_api_id, &query).await?))
}

pub async fn create_entry(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path(type_api_id): Path<String>,
    Json(request): Json<CreateEntryRequest>,
) -> Result<impl IntoResponse> {
    require_permission(&state, &claims, "content.entry.create", None).await?;
    let entry = state
        .content_engine
        .create(CreateEntryInput {
            content_type_api_id: type_api_id,
            data: request.data,
            slug: request.slug,
			actor_id: claims.sub.clone(),
        })
        .await?;
    Ok(created_json(entry))
}

pub async fn get_entry(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path((type_api_id, id)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ContentEntry>> {
    require_permission(&state, &claims, "content.entry.read", None).await?;
    let populate = params
        .get("populate")
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let entry = state
        .content_engine
        .get(&type_api_id, &id, &populate)
        .await?
        .ok_or_else(|| Error::NotFound {
            message: format!("content entry not found: {id}"),
        })?;
    Ok(Json(entry))
}

pub async fn update_entry(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path((type_api_id, id)): Path<(String, String)>,
    Json(request): Json<UpdateEntryRequest>,
) -> Result<Json<ContentEntry>> {
	let existing = require_owned_entry(&state, &claims, &type_api_id, &id, "content.entry.update").await?;
    let _ = existing;
    let entry = state
        .content_engine
        .update(
            &type_api_id,
            &id,
            UpdateEntryInput {
                data: request.data,
                slug: request.slug,
				actor_id: claims.sub.clone(),
            },
        )
        .await?;
    Ok(Json(entry))
}

pub async fn delete_entry(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path((type_api_id, id)): Path<(String, String)>,
    Query(query): Query<DeleteEntryQuery>,
) -> Result<Response> {
    let existing = require_owned_entry(&state, &claims, &type_api_id, &id, "content.entry.delete").await?;
    let _ = existing;
    state
        .content_engine
        .delete(
            &type_api_id,
            &id,
            query.mode.as_deref().map(parse_delete_mode).transpose()?,
			&claims.sub,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

pub async fn list_revisions(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path((type_api_id, id)): Path<(String, String)>,
    Query(query): Query<RevisionListQuery>,
) -> Result<Json<RevisionListResponse>> {
    require_owned_entry(&state, &claims, &type_api_id, &id, "content.entry.read").await?;
    let revisions = state
        .revision_manager
        .list_revisions(&id, query.page.unwrap_or(1), query.page_size.unwrap_or(20))
        .await?;
    Ok(Json(RevisionListResponse {
        data: revisions.data,
        total: revisions.total,
        page: revisions.page,
        page_size: revisions.page_size,
    }))
}

pub async fn get_revision(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path((type_api_id, id, version)): Path<(String, String, i64)>,
) -> Result<Json<Revision>> {
    require_owned_entry(&state, &claims, &type_api_id, &id, "content.entry.read").await?;
    Ok(Json(state.revision_manager.get_revision(&id, version).await?))
}

pub async fn rollback_revision(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path((type_api_id, id, version)): Path<(String, String, i64)>,
) -> Result<Json<ContentEntry>> {
    require_owned_entry(&state, &claims, &type_api_id, &id, "content.entry.update").await?;
    state
        .revision_manager
		.rollback(&id, version, &claims.sub)
        .await?;
    let entry = state
        .content_engine
        .get(&type_api_id, &id, &[])
        .await?
        .ok_or_else(|| Error::NotFound {
            message: format!("content entry not found: {id}"),
        })?;
    Ok(Json(entry))
}

pub async fn publish_entry(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path((type_api_id, id)): Path<(String, String)>,
) -> Result<Json<ContentEntry>> {
    require_owned_entry(&state, &claims, &type_api_id, &id, "content.entry.publish").await?;
    Ok(Json(
        state
            .publish_manager
			.publish(&id, &type_api_id, &claims.sub)
            .await?,
    ))
}

pub async fn unpublish_entry(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path((type_api_id, id)): Path<(String, String)>,
) -> Result<Json<ContentEntry>> {
    require_owned_entry(&state, &claims, &type_api_id, &id, "content.entry.publish").await?;
    Ok(Json(
        state
            .publish_manager
			.unpublish(&id, &type_api_id, &claims.sub)
            .await?,
    ))
}

async fn require_owned_entry(
    state: &ApiState,
	claims: &cycms_auth::AuthClaims,
    type_api_id: &str,
    id: &str,
    code: &str,
) -> Result<ContentEntry> {
    let entry = state
        .content_engine
        .get(type_api_id, id, &[])
        .await?
        .ok_or_else(|| Error::NotFound {
            message: format!("content entry not found: {id}"),
        })?;
    require_permission(state, claims, code, Some(&entry.created_by)).await?;
    Ok(entry)
}

fn parse_delete_mode(raw: &str) -> Result<DeleteMode> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "soft" => Ok(DeleteMode::Soft),
        "hard" => Ok(DeleteMode::Hard),
        _ => Err(Error::ValidationError {
            message: format!("invalid delete mode: {raw}"),
            details: None,
        }),
    }
}