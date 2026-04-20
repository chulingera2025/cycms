use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Router, response::Response};
use cycms_auth::Authenticated;
use cycms_core::{Error, Result};
use cycms_media::{MediaAsset, UploadInput};
use serde::Serialize;

use crate::common::{created_json, require_permission};
use crate::query::parse_media_query;
use crate::state::ApiState;

pub fn routes() -> Router<Arc<ApiState>> {
    Router::new()
        .route("/", get(list_media))
        .route("/upload", post(upload_media))
        .route("/{id}", get(get_media).delete(delete_media))
}

#[derive(Debug, Clone, Serialize)]
pub struct MediaAssetResponse {
    pub id: String,
    pub filename: String,
    pub original_filename: String,
    pub mime_type: String,
    pub size: i64,
    pub storage_path: String,
    pub metadata: Option<serde_json::Value>,
    pub uploaded_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MediaListResponse {
    pub data: Vec<MediaAssetResponse>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
    pub page_count: u64,
}

pub async fn list_media(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<MediaListResponse>> {
    require_permission(&state, &claims, "media.asset.read", None).await?;
    let query = parse_media_query(&params)?;
    let result = state.media_manager.list(&query).await?;
    Ok(Json(MediaListResponse {
        data: result
            .data
            .into_iter()
            .map(MediaAssetResponse::from)
            .collect(),
        total: result.total,
        page: result.page,
        page_size: result.page_size,
        page_count: result.page_count,
    }))
}

pub async fn upload_media(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    mut multipart: Multipart,
) -> Result<impl IntoResponse> {
    require_permission(&state, &claims, "media.asset.upload", None).await?;

    let mut original_filename: Option<String> = None;
    let mut data: Option<Vec<u8>> = None;
    let mut mime_type: Option<String> = None;
    let mut metadata: Option<serde_json::Value> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| Error::BadRequest {
            message: format!("invalid multipart payload: {error}"),
            source: None,
        })?
    {
        let field_name = field.name().unwrap_or_default().to_owned();
        match field_name.as_str() {
            "file" => {
                original_filename = field.file_name().map(ToOwned::to_owned);
                if mime_type.is_none() {
                    mime_type = field.content_type().map(ToOwned::to_owned);
                }
                data = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|error| Error::BadRequest {
                            message: format!("failed to read uploaded file: {error}"),
                            source: None,
                        })?
                        .to_vec(),
                );
            }
            "metadata" => {
                let raw = field.text().await.map_err(|error| Error::BadRequest {
                    message: format!("failed to read metadata field: {error}"),
                    source: None,
                })?;
                metadata =
                    Some(
                        serde_json::from_str(&raw).map_err(|error| Error::ValidationError {
                            message: format!("invalid metadata JSON: {error}"),
                            details: None,
                        })?,
                    );
            }
            "mime_type" => {
                mime_type = Some(field.text().await.map_err(|error| Error::BadRequest {
                    message: format!("failed to read mime_type field: {error}"),
                    source: None,
                })?);
            }
            _ => {}
        }
    }

    let original_filename = original_filename.ok_or_else(|| Error::ValidationError {
        message: "multipart field `file` is required".to_owned(),
        details: None,
    })?;
    let data = data.ok_or_else(|| Error::ValidationError {
        message: "multipart field `file` is required".to_owned(),
        details: None,
    })?;

    let asset = state
        .media_manager
        .upload(UploadInput {
            original_filename,
            data,
            mime_type,
            uploaded_by: claims.sub.clone(),
            metadata,
        })
        .await?;
    Ok(created_json(MediaAssetResponse::from(asset)))
}

pub async fn get_media(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path(id): Path<String>,
) -> Result<Json<MediaAssetResponse>> {
    require_permission(&state, &claims, "media.asset.read", None).await?;
    let asset = state
        .media_manager
        .get(&id)
        .await?
        .ok_or_else(|| Error::NotFound {
            message: format!("media asset not found: {id}"),
        })?;
    Ok(Json(MediaAssetResponse::from(asset)))
}

pub async fn delete_media(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
    Path(id): Path<String>,
) -> Result<Response> {
    let asset = state
        .media_manager
        .get(&id)
        .await?
        .ok_or_else(|| Error::NotFound {
            message: format!("media asset not found: {id}"),
        })?;
    require_permission(
        &state,
        &claims,
        "media.asset.delete",
        Some(&asset.uploaded_by),
    )
    .await?;
    state.media_manager.delete(&id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

impl From<MediaAsset> for MediaAssetResponse {
    fn from(value: MediaAsset) -> Self {
        Self {
            id: value.id,
            filename: value.filename,
            original_filename: value.original_filename,
            mime_type: value.mime_type,
            size: value.size,
            storage_path: value.storage_path,
            metadata: value.metadata,
            uploaded_by: value.uploaded_by,
            created_at: value.created_at,
        }
    }
}
