use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const COMPILED_EXTENSION_REGISTRY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnershipMode {
    Replace,
    Wrap,
    Append,
}

impl OwnershipMode {
    #[must_use]
    pub const fn precedence(self) -> u8 {
        match self {
            Self::Replace => 0,
            Self::Wrap => 1,
            Self::Append => 2,
        }
    }
}

impl Default for OwnershipMode {
    fn default() -> Self {
        Self::Replace
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistrationOriginKind {
    HostManifest,
    CompatibilityBridge,
    DynamicRuntime,
    FrontendCompatibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdminPageMode {
    Html,
    Hybrid,
    Island,
    Compatibility,
}

impl Default for AdminPageMode {
    fn default() -> Self {
        Self::Hybrid
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityKind {
    DynamicNativePlugin,
    DynamicWasmPlugin,
    AdminExtensionAssetBundle,
    AdminExtensionMenu,
    AdminExtensionRoute,
    AdminExtensionSlot,
    AdminExtensionFieldRenderer,
    AdminExtensionSettings,
    ManifestCompatibilityBridge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationSource {
    pub plugin_name: String,
    pub plugin_version: String,
    pub origin: RegistrationOriginKind,
    pub declaration_order: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledPluginDescriptor {
    pub name: String,
    pub version: String,
    pub plugin_kind: String,
    pub has_host_manifest: bool,
    pub has_frontend_manifest: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicPageRegistration {
    pub id: String,
    pub source: RegistrationSource,
    pub path: String,
    pub priority: i32,
    pub ownership: OwnershipMode,
    pub handler: String,
    pub title: Option<String>,
    #[serde(default)]
    pub asset_bundle_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminPageRegistration {
    pub id: String,
    pub source: RegistrationSource,
    pub path: String,
    pub title: String,
    pub mode: AdminPageMode,
    pub priority: i32,
    pub ownership: OwnershipMode,
    pub handler: String,
    pub menu_label: Option<String>,
    pub menu_zone: Option<String>,
    #[serde(default)]
    pub asset_bundle_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParserRegistration {
    pub id: String,
    pub source: RegistrationSource,
    pub priority: i32,
    pub ownership: OwnershipMode,
    pub parser: String,
    #[serde(default)]
    pub content_types: Vec<String>,
    #[serde(default)]
    pub field_names: Vec<String>,
    #[serde(default)]
    pub source_formats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookRegistration {
    pub id: String,
    pub source: RegistrationSource,
    pub priority: i32,
    pub ownership: OwnershipMode,
    pub phase: String,
    pub handler: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetBundleRegistration {
    pub id: String,
    pub source: RegistrationSource,
    #[serde(default)]
    pub apply_to: Vec<String>,
    #[serde(default)]
    pub modules: Vec<String>,
    #[serde(default)]
    pub scripts: Vec<String>,
    #[serde(default)]
    pub styles: Vec<String>,
    #[serde(default)]
    pub inline_data_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorRegistration {
    pub id: String,
    pub source: RegistrationSource,
    pub priority: i32,
    pub ownership: OwnershipMode,
    pub editor: String,
    #[serde(default)]
    pub content_types: Vec<String>,
    #[serde(default)]
    pub field_types: Vec<String>,
    #[serde(default)]
    pub screen_targets: Vec<String>,
    #[serde(default)]
    pub asset_bundle_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityRegistration {
    pub id: String,
    pub source: RegistrationSource,
    pub kind: CompatibilityKind,
    pub target: String,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledExtensionRegistry {
    pub schema_version: u32,
    #[serde(default)]
    pub plugins: Vec<CompiledPluginDescriptor>,
    #[serde(default)]
    pub public_pages: Vec<PublicPageRegistration>,
    #[serde(default)]
    pub admin_pages: Vec<AdminPageRegistration>,
    #[serde(default)]
    pub parsers: Vec<ParserRegistration>,
    #[serde(default)]
    pub hooks: Vec<HookRegistration>,
    #[serde(default)]
    pub assets: Vec<AssetBundleRegistration>,
    #[serde(default)]
    pub editors: Vec<EditorRegistration>,
    #[serde(default)]
    pub compatibility: Vec<CompatibilityRegistration>,
}

impl Default for CompiledExtensionRegistry {
    fn default() -> Self {
        Self {
            schema_version: COMPILED_EXTENSION_REGISTRY_SCHEMA_VERSION,
            plugins: Vec::new(),
            public_pages: Vec::new(),
            admin_pages: Vec::new(),
            parsers: Vec::new(),
            hooks: Vec::new(),
            assets: Vec::new(),
            editors: Vec::new(),
            compatibility: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostRequestTarget {
    pub path: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseTarget {
    pub content_type: Option<String>,
    pub field_name: Option<String>,
    pub source_format: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorTarget {
    pub content_type: Option<String>,
    pub field_type: Option<String>,
    pub screen_target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnershipCandidate {
    pub registration_id: String,
    pub plugin_name: String,
    pub plugin_version: String,
    pub origin: RegistrationOriginKind,
    pub ownership: OwnershipMode,
    pub priority: i32,
    pub declaration_order: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnershipDiagnostics {
    pub surface: String,
    pub target: String,
    pub candidates: Vec<OwnershipCandidate>,
    pub primary: Option<String>,
    #[serde(default)]
    pub wrappers: Vec<String>,
    #[serde(default)]
    pub appenders: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostRegistryDiagnosticsSnapshot {
    #[serde(default)]
    pub public_pages: Vec<OwnershipDiagnostics>,
    #[serde(default)]
    pub admin_pages: Vec<OwnershipDiagnostics>,
    #[serde(default)]
    pub parsers: Vec<OwnershipDiagnostics>,
    #[serde(default)]
    pub editors: Vec<OwnershipDiagnostics>,
    #[serde(default)]
    pub asset_bundles: Vec<AssetBundleRegistration>,
    #[serde(default)]
    pub hooks: Vec<HookRegistration>,
    #[serde(default)]
    pub compatibility: Vec<CompatibilityRegistration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnershipDecision<T> {
    pub primary: Option<T>,
    pub wrappers: Vec<T>,
    pub appenders: Vec<T>,
    pub diagnostics: OwnershipDiagnostics,
}