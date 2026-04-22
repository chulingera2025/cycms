mod admin_extensions_observability;
mod common;
mod handlers;
mod query;
mod state;

use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::middleware;
use axum::routing::get;
use cycms_auth::auth_middleware;
use cycms_core::Result;

pub use admin_extensions_observability::{
    AdminExtensionClientEventPayload, AdminExtensionDiagnosticsResponse, AdminExtensionEventRecord,
    AdminExtensionEventStore, AdminExtensionSecurityState, SharedAdminExtensionEventStore,
    build_admin_extension_csp_policy, build_admin_extension_security_state, build_csp_report_event,
    normalize_csp_report_payload, with_request_context,
};
pub use state::ApiState;

pub fn build_router(state: Arc<ApiState>) -> Router {
    let auth_layer =
        middleware::from_fn_with_state(Arc::clone(&state.auth_engine), auth_middleware);

    let auth_router = handlers::auth::public_routes()
        .merge(handlers::auth::protected_routes().route_layer(auth_layer.clone()));

    let protected_v1 = Router::new()
        .nest(
            "/admin/extensions",
            handlers::admin_extensions::protected_routes(),
        )
        .nest(
            "/admin/editor-registry",
            handlers::admin_editor_registry::protected_routes(),
        )
        .nest("/content-types", handlers::content_types::routes())
        .nest("/content", handlers::content::routes())
        .nest("/media", handlers::media::routes())
        .nest("/plugins", handlers::plugins::routes())
        .nest("/settings", handlers::settings::routes())
        .nest("/users", handlers::users::routes())
        .nest("/roles", handlers::roles::routes())
        .route_layer(auth_layer);

    let mut api: Router<Arc<ApiState>> = Router::new()
        .route("/docs", get(openapi_docs))
        .nest("/v1/auth", auth_router)
        .nest(
            "/v1/plugin-assets",
            handlers::admin_extensions::public_routes(),
        )
        .nest("/v1/public", handlers::public::routes())
        .nest("/v1", protected_v1);

    for (plugin_name, router) in state.native_runtime.all_routes() {
        api = api.nest_service(&format!("/v1/x/{plugin_name}"), router);
    }
    for (plugin_name, router) in state.wasm_runtime.all_routes() {
        api = api.nest_service(&format!("/v1/x/{plugin_name}"), router);
    }

    Router::new().nest("/api", api.with_state(state))
}

async fn openapi_docs(State(state): State<Arc<ApiState>>) -> Result<Json<serde_json::Value>> {
    let document = cycms_openapi::build_openapi_json(
        &state.content_model,
        &state.native_runtime,
        &state.wasm_runtime,
    )
    .await?;
    Ok(Json(document))
}
