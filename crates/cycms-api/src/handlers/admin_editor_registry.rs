use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::routing::get;
use cycms_auth::Authenticated;
use cycms_core::Result;
use serde::Serialize;

use crate::common::require_permission;
use crate::state::ApiState;

pub fn protected_routes() -> Router<Arc<ApiState>> {
    Router::new().route("/", get(get_editor_registry))
}

#[derive(Serialize)]
pub struct EditorRegistryResponse {
    pub editors: Vec<EditorEntryResponse>,
}

#[derive(Serialize)]
pub struct EditorEntryResponse {
    pub id: String,
    pub editor: String,
    pub content_types: Vec<String>,
    pub field_types: Vec<String>,
    pub screen_targets: Vec<String>,
    pub modules: Vec<String>,
    pub styles: Vec<String>,
}

async fn get_editor_registry(
    State(state): State<Arc<ApiState>>,
    Authenticated(claims): Authenticated,
) -> Result<Json<EditorRegistryResponse>> {
    require_permission(&state, &claims, "admin.editor-registry.read", None).await?;
    let compiled = state.host_registry.compiled();
    let editors = compiled
        .editors
        .iter()
        .map(|editor| {
            let modules: Vec<String> = editor
                .asset_bundle_ids
                .iter()
                .flat_map(|bundle_id| {
                    let bundle_id = bundle_id.clone();
                    compiled
                        .assets
                        .iter()
                        .filter(move |bundle| bundle.id == bundle_id)
                        .flat_map(|bundle| bundle.modules.clone())
                })
                .collect();
            let styles: Vec<String> = editor
                .asset_bundle_ids
                .iter()
                .flat_map(|bundle_id| {
                    let bundle_id = bundle_id.clone();
                    compiled
                        .assets
                        .iter()
                        .filter(move |bundle| bundle.id == bundle_id)
                        .flat_map(|bundle| bundle.styles.clone())
                })
                .collect();
            EditorEntryResponse {
                id: editor.id.clone(),
                editor: editor.editor.clone(),
                content_types: editor.content_types.clone(),
                field_types: editor.field_types.clone(),
                screen_targets: editor.screen_targets.clone(),
                modules,
                styles,
            }
        })
        .collect();
    Ok(Json(EditorRegistryResponse { editors }))
}
