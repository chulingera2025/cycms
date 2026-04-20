use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Router, response::IntoResponse};
use cycms_auth::Authenticated;
use cycms_content_model::{ContentTypeKind, FieldDefinition};
use cycms_core::{Error, Result};
use serde::Deserialize;

use crate::common::{NullablePatch, created_json, require_permission};
use crate::state::ApiState;

pub fn routes() -> Router<Arc<ApiState>> {
    Router::new()
        .route("/", get(list_content_types).post(create_content_type))
        .route(
            "/{api_id}",
            get(get_content_type)
                .put(update_content_type)
                .delete(delete_content_type),
        )
}

#[derive(Debug, Deserialize)]
pub struct CreateContentTypeRequest {
    pub name: String,
    pub api_id: String,
    pub description: Option<String>,
    pub kind: ContentTypeKind,
    pub fields: Vec<FieldDefinition>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateContentTypeRequest {
    pub name: Option<String>,
    #[serde(default)]
    pub description: NullablePatch<String>,
    pub kind: Option<ContentTypeKind>,
    pub fields: Option<Vec<FieldDefinition>>,
}

pub async fn list_content_types(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
) -> Result<Json<Vec<cycms_content_model::ContentTypeDefinition>>> {
    require_permission(&state, &claims, "content.type.read", None).await?;
    Ok(Json(state.content_model.list_types().await?))
}

pub async fn create_content_type(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Json(request): Json<CreateContentTypeRequest>,
) -> Result<impl IntoResponse> {
    require_permission(&state, &claims, "content.type.create", None).await?;
    let content_type = state
        .content_model
        .create_type(cycms_content_model::CreateContentTypeInput {
            name: request.name,
            api_id: request.api_id,
            description: request.description,
            kind: request.kind,
            fields: request.fields,
        })
        .await?;
    Ok(created_json(content_type))
}

pub async fn get_content_type(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path(api_id): Path<String>,
) -> Result<Json<cycms_content_model::ContentTypeDefinition>> {
    require_permission(&state, &claims, "content.type.read", None).await?;
    let content_type = state
        .content_model
        .get_type(&api_id)
        .await?
        .ok_or_else(|| Error::NotFound {
            message: format!("content type `{api_id}` not found"),
        })?;
    Ok(Json(content_type))
}

pub async fn update_content_type(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path(api_id): Path<String>,
    Json(request): Json<UpdateContentTypeRequest>,
) -> Result<Json<cycms_content_model::ContentTypeDefinition>> {
    require_permission(&state, &claims, "content.type.update", None).await?;
    let content_type = state
        .content_model
        .update_type(
            &api_id,
            cycms_content_model::UpdateContentTypeInput {
                name: request.name,
                description: match request.description {
                    NullablePatch::Missing => None,
                    NullablePatch::Null => Some(None),
                    NullablePatch::Value(value) => Some(Some(value)),
                },
                kind: request.kind,
                fields: request.fields,
            },
        )
        .await?;
    Ok(Json(content_type))
}

pub async fn delete_content_type(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path(api_id): Path<String>,
) -> Result<impl IntoResponse> {
    require_permission(&state, &claims, "content.type.delete", None).await?;
    if state.content_model.delete_type(&api_id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(Error::NotFound {
            message: format!("content type `{api_id}` not found"),
        })
    }
}
