use std::collections::BTreeMap;

use http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const COMPILED_EXTENSION_REGISTRY_SCHEMA_VERSION: u32 = 1;
pub const CONTENT_DOCUMENT_SCHEMA_VERSION: u32 = 1;
pub const PAGE_DOCUMENT_SCHEMA_VERSION: u32 = 1;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum OwnershipMode {
    #[default]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistrationOriginKind {
    HostManifest,
    CompatibilityBridge,
    DynamicRuntime,
    FrontendCompatibility,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum AdminPageMode {
    Html,
    #[default]
    Hybrid,
    Island,
    Compatibility,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminMenuEntry {
    pub id: String,
    pub label: String,
    pub path: String,
    pub mode: AdminPageMode,
    pub plugin_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminMenuGroup {
    pub zone: String,
    #[serde(default)]
    pub entries: Vec<AdminMenuEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnershipDecision<T> {
    pub primary: Option<T>,
    pub wrappers: Vec<T>,
    pub appenders: Vec<T>,
    pub diagnostics: OwnershipDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentDiagnostic {
    pub severity: ContentDiagnosticSeverity,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSourceMeta {
    pub format: String,
    pub parser_id: String,
    pub origin_field: Option<String>,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadingNode {
    pub level: u8,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphNode {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockNode {
    pub kind: String,
    pub data: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotCallNode {
    pub slot: String,
    #[serde(default)]
    pub arguments: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbedNode {
    pub url: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContentNode {
    Heading(HeadingNode),
    Paragraph(ParagraphNode),
    Block(BlockNode),
    SlotCall(SlotCallNode),
    Embed(EmbedNode),
    RawHtml(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentDocument {
    pub schema_version: u32,
    pub source: ContentSourceMeta,
    #[serde(default)]
    pub nodes: Vec<ContentNode>,
    #[serde(default)]
    pub diagnostics: Vec<ContentDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HeadNode {
    Title { text: String },
    Meta { name: String, content: String },
    Link { rel: String, href: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageAction {
    pub id: String,
    pub label: String,
    pub method: String,
    pub href: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetReference {
    pub id: String,
    pub href: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineDataAsset {
    pub id: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IslandBootSpec {
    pub island_id: String,
    pub module: String,
    pub props: Value,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetGraph {
    #[serde(default)]
    pub styles: Vec<AssetReference>,
    #[serde(default)]
    pub scripts: Vec<AssetReference>,
    #[serde(default)]
    pub modules: Vec<AssetReference>,
    #[serde(default)]
    pub inline_data: Vec<InlineDataAsset>,
    #[serde(default)]
    pub island_boot: Vec<IslandBootSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutNode {
    pub name: String,
    #[serde(default)]
    pub children: Vec<PageNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionNode {
    pub name: String,
    #[serde(default)]
    pub children: Vec<PageNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FragmentNode {
    #[serde(default)]
    pub children: Vec<PageNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HtmlNode {
    pub tag: String,
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
    #[serde(default)]
    pub children: Vec<PageNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextNode {
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotOutletNode {
    pub slot: String,
    #[serde(default)]
    pub fallback: Vec<PageNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentOutletNode {
    pub content: ContentDocument,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IslandMount {
    pub id: String,
    pub component: String,
    pub props: Value,
    /// Per-island module URL override. When set, `build_island_boot` uses this
    /// URL directly instead of the page-level asset bundle module.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PageNode {
    Layout(LayoutNode),
    Region(RegionNode),
    Fragment(FragmentNode),
    Html(HtmlNode),
    Text(TextNode),
    SlotOutlet(SlotOutletNode),
    ContentOutlet(ContentOutletNode),
    Island(IslandMount),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageDocument {
    pub route_id: String,
    #[serde(with = "status_code_serde")]
    pub status: StatusCode,
    #[serde(default)]
    pub head: Vec<HeadNode>,
    #[serde(default)]
    pub body: Vec<PageNode>,
    #[serde(default)]
    pub actions: Vec<PageAction>,
    #[serde(default)]
    pub islands: Vec<IslandMount>,
    #[serde(default)]
    pub cache_tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedPage {
    #[serde(with = "status_code_serde")]
    pub status: StatusCode,
    pub html: String,
    pub assets: AssetGraph,
}

mod status_code_serde {
    use http::StatusCode;
    use serde::{Deserialize, Deserializer, Serializer, de::Error as _};

    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn serialize<S>(value: &StatusCode, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u16(value.as_u16())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<StatusCode, D::Error>
    where
        D: Deserializer<'de>,
    {
        let code = u16::deserialize(deserializer)?;
        StatusCode::from_u16(code).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use http::StatusCode;
    use serde_json::json;

    use super::*;

    #[test]
    fn page_document_status_round_trips_as_u16() {
        let page = PageDocument {
            route_id: "post.show".to_owned(),
            status: StatusCode::CREATED,
            head: vec![HeadNode::Title {
                text: "Hello".to_owned(),
            }],
            body: vec![PageNode::Text(TextNode {
                value: "Body".to_owned(),
            })],
            actions: vec![PageAction {
                id: "publish".to_owned(),
                label: "Publish".to_owned(),
                method: "post".to_owned(),
                href: "/actions/publish".to_owned(),
            }],
            islands: vec![IslandMount {
                id: "editor".to_owned(),
                component: "EditorIsland".to_owned(),
                props: json!({ "entryId": "post-1" }),
                module_url: None,
            }],
            cache_tags: vec!["post:1".to_owned()],
        };

        let value = serde_json::to_value(&page).unwrap();
        assert_eq!(value.get("status"), Some(&json!(201)));

        let decoded: PageDocument = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, page);
    }

    #[test]
    fn rendered_page_round_trips_with_asset_graph() {
        let rendered = RenderedPage {
            status: StatusCode::OK,
            html: "<html></html>".to_owned(),
            assets: AssetGraph {
                styles: vec![AssetReference {
                    id: "theme".to_owned(),
                    href: "/assets/theme.css".to_owned(),
                }],
                scripts: vec![AssetReference {
                    id: "legacy".to_owned(),
                    href: "/assets/legacy.js".to_owned(),
                }],
                modules: vec![AssetReference {
                    id: "runtime".to_owned(),
                    href: "/assets/runtime.js".to_owned(),
                }],
                inline_data: vec![InlineDataAsset {
                    id: "bootstrap".to_owned(),
                    value: json!({ "locale": "zh-CN" }),
                }],
                island_boot: vec![IslandBootSpec {
                    island_id: "editor".to_owned(),
                    module: "/assets/runtime.js".to_owned(),
                    props: json!({ "entryId": "post-1" }),
                }],
            },
        };

        let encoded = serde_json::to_string(&rendered).unwrap();
        let decoded: RenderedPage = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, rendered);
    }
}
