use std::sync::Arc;

use cycms_auth::AuthEngine;
use cycms_config::AppConfig;
use cycms_content_engine::ContentEngine;
use cycms_content_model::ContentModelRegistry;
use cycms_events::EventBus;
use cycms_media::MediaManager;
use cycms_permission::PermissionEngine;
use cycms_plugin_api::ServiceRegistry;
use cycms_plugin_manager::PluginManager;
use cycms_plugin_native::NativePluginRuntime;
use cycms_plugin_wasm::WasmPluginRuntime;
use cycms_publish::PublishManager;
use cycms_revision::RevisionManager;
use cycms_settings::SettingsManager;

#[derive(Clone)]
pub struct ApiState {
    pub config: Arc<AppConfig>,
    pub auth_engine: Arc<AuthEngine>,
    pub permission_engine: Arc<PermissionEngine>,
    pub event_bus: Arc<EventBus>,
    pub content_model: Arc<ContentModelRegistry>,
    pub content_engine: Arc<ContentEngine>,
    pub revision_manager: Arc<RevisionManager>,
    pub publish_manager: Arc<PublishManager>,
    pub media_manager: Arc<MediaManager>,
    pub plugin_manager: Arc<PluginManager>,
    pub settings_manager: Arc<SettingsManager>,
    pub service_registry: Arc<ServiceRegistry>,
    pub native_runtime: Arc<NativePluginRuntime>,
    pub wasm_runtime: Arc<WasmPluginRuntime>,
}

impl ApiState {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        config: Arc<AppConfig>,
        auth_engine: Arc<AuthEngine>,
        permission_engine: Arc<PermissionEngine>,
        event_bus: Arc<EventBus>,
        content_model: Arc<ContentModelRegistry>,
        content_engine: Arc<ContentEngine>,
        revision_manager: Arc<RevisionManager>,
        publish_manager: Arc<PublishManager>,
        media_manager: Arc<MediaManager>,
        plugin_manager: Arc<PluginManager>,
        settings_manager: Arc<SettingsManager>,
        service_registry: Arc<ServiceRegistry>,
        native_runtime: Arc<NativePluginRuntime>,
        wasm_runtime: Arc<WasmPluginRuntime>,
    ) -> Self {
        Self {
            config,
            auth_engine,
            permission_engine,
            event_bus,
            content_model,
            content_engine,
            revision_manager,
            publish_manager,
            media_manager,
            plugin_manager,
            settings_manager,
            service_registry,
            native_runtime,
            wasm_runtime,
        }
    }
}
