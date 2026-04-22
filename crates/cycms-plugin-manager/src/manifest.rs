use std::collections::{BTreeSet, HashMap};

use cycms_host_types::{AdminPageMode, CompatibilityKind, OwnershipMode};
use cycms_permission::{PermissionDefinition, PermissionScope};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

use crate::error::PluginManagerError;

/// 插件清单根结构（对应 `plugin.toml`）。
///
/// 使用 [`PluginManifest::from_toml_str`] 统一入口，解析同时执行结构性校验，
/// 下游 `PluginManager` 拿到的都是语法合法的实例。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginManifest {
    pub plugin: PluginMeta,
    pub compatibility: CompatibilitySpec,
    #[serde(default)]
    pub dependencies: HashMap<String, DependencySpec>,
    pub permissions: Option<PermissionsSpec>,
    pub frontend: Option<FrontendSpec>,
    pub host: Option<HostManifestSpec>,
    #[serde(default)]
    pub migrations: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HostManifestSpec {
    #[serde(default)]
    pub public_pages: Vec<PublicPageSpec>,
    #[serde(default)]
    pub admin_pages: Vec<AdminPageSpec>,
    #[serde(default)]
    pub parsers: Vec<ParserSpec>,
    #[serde(default)]
    pub hooks: Vec<HookSpec>,
    #[serde(default)]
    pub assets: Vec<AssetBundleSpec>,
    #[serde(default)]
    pub editors: Vec<EditorSpec>,
    #[serde(default)]
    pub compatibility: Vec<CompatibilityBridgeSpec>,
}

impl HostManifestSpec {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.public_pages.is_empty()
            && self.admin_pages.is_empty()
            && self.parsers.is_empty()
            && self.hooks.is_empty()
            && self.assets.is_empty()
            && self.editors.is_empty()
            && self.compatibility.is_empty()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PublicPageSpec {
    pub id: String,
    pub path: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub ownership: OwnershipMode,
    pub handler: String,
    pub title: Option<String>,
    #[serde(default)]
    pub asset_bundle_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AdminPageSpec {
    pub id: String,
    pub path: String,
    pub title: String,
    #[serde(default)]
    pub mode: AdminPageMode,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub ownership: OwnershipMode,
    pub handler: String,
    pub menu_label: Option<String>,
    pub menu_zone: Option<String>,
    #[serde(default)]
    pub asset_bundle_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ParserSpec {
    pub id: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub ownership: OwnershipMode,
    pub parser: String,
    #[serde(default)]
    pub content_types: Vec<String>,
    #[serde(default)]
    pub field_names: Vec<String>,
    #[serde(default)]
    pub source_formats: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HookSpec {
    pub id: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub ownership: OwnershipMode,
    pub phase: String,
    pub handler: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetBundleSpec {
    pub id: String,
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EditorSpec {
    pub id: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompatibilityBridgeSpec {
    pub id: String,
    pub kind: CompatibilityKind,
    pub target: String,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// 插件元信息段（Req 20.1）：必填 `name` / `version` / `kind` / `entry`。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub kind: PluginKind,
    pub entry: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
}

/// 插件实现类型，决定 `PluginManager` 调用哪种 runtime 加载。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginKind {
    Native,
    Wasm,
}

impl PluginKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Wasm => "wasm",
        }
    }
}

impl std::fmt::Display for PluginKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for PluginKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "native" => Ok(Self::Native),
            "wasm" => Ok(Self::Wasm),
            other => Err(format!("unknown plugin kind: {other}")),
        }
    }
}

/// 宿主兼容性（Req 20.2）。`cycms` 为 `SemVer` range 字面量，安装期比对当前 cycms 版本。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompatibilitySpec {
    pub cycms: String,
}

/// 依赖其他插件的声明（Req 20.3）。`version` 为 `SemVer` range 字面量。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DependencySpec {
    pub version: String,
    #[serde(default)]
    pub optional: bool,
}

/// 插件权限点列表容器（Req 20.4）。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PermissionsSpec {
    pub definitions: Vec<PermissionEntry>,
}

/// 单条权限定义（Req 20.4）。`scope` 省略时默认为 `all`。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PermissionEntry {
    pub domain: String,
    pub resource: String,
    pub action: String,
    #[serde(default = "default_scope")]
    pub scope: PermissionScope,
}

const fn default_scope() -> PermissionScope {
    PermissionScope::All
}

/// 前端入口信息（Req 20.5）。v0.1 仅存储路径字面量，AdminWeb 负责动态加载。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FrontendSpec {
    pub manifest: String,
    #[serde(default)]
    pub required: bool,
}

impl PluginManifest {
    /// 从 TOML 字符串解析并执行结构性校验。
    ///
    /// 校验范围：
    /// - `plugin.name` 非空、不含 `.` 与空白、只允许 `[A-Za-z0-9_-]`
    /// - `plugin.entry` 非空
    /// - `plugin.version` 为合法 `SemVer`
    /// - `compatibility.cycms` 为合法 `SemVer` range
    /// - 每个 `dependencies.<name>.version` 为合法 `SemVer` range
    /// - `permissions.definitions[*]` 的 `domain/resource/action` 均非空且不含 `.`
    ///
    /// # Errors
    /// 任一校验失败均返回 [`PluginManagerError::InvalidManifest`]。
    pub fn from_toml_str(text: &str) -> Result<Self, PluginManagerError> {
        let manifest: Self = toml::from_str(text)
            .map_err(|e| PluginManagerError::InvalidManifest(format!("toml parse: {e}")))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// 对已构造的 manifest 执行同 [`PluginManifest::from_toml_str`] 的校验链。
    ///
    /// # Errors
    /// 见 [`PluginManifest::from_toml_str`]。
    pub fn validate(&self) -> Result<(), PluginManagerError> {
        validate_plugin_name(&self.plugin.name)?;
        if self.plugin.entry.trim().is_empty() {
            return Err(PluginManagerError::InvalidManifest(
                "plugin.entry must not be empty".into(),
            ));
        }
        Version::parse(&self.plugin.version).map_err(|e| {
            PluginManagerError::InvalidManifest(format!(
                "plugin.version {:?}: {e}",
                self.plugin.version
            ))
        })?;
        VersionReq::parse(&self.compatibility.cycms).map_err(|e| {
            PluginManagerError::InvalidManifest(format!(
                "compatibility.cycms {:?}: {e}",
                self.compatibility.cycms
            ))
        })?;
        for (dep_name, spec) in &self.dependencies {
            if dep_name.trim().is_empty() {
                return Err(PluginManagerError::InvalidManifest(
                    "dependencies entry name must not be empty".into(),
                ));
            }
            VersionReq::parse(&spec.version).map_err(|e| {
                PluginManagerError::InvalidManifest(format!(
                    "dependencies.{dep_name}.version {:?}: {e}",
                    spec.version
                ))
            })?;
        }
        if let Some(perms) = &self.permissions {
            for entry in &perms.definitions {
                check_permission_segment("permissions.domain", &entry.domain)?;
                check_permission_segment("permissions.resource", &entry.resource)?;
                check_permission_segment("permissions.action", &entry.action)?;
            }
        }
        if let Some(frontend) = &self.frontend
            && frontend.manifest.trim().is_empty()
        {
            return Err(PluginManagerError::InvalidManifest(
                "frontend.manifest must not be empty".into(),
            ));
        }
        if let Some(host) = &self.host {
            validate_host_manifest(host)?;
        }
        Ok(())
    }

    /// 已校验后的 `SemVer` 版本。
    ///
    /// # Panics
    /// 仅当 manifest 未经过 [`PluginManifest::validate`] 时才会 panic（内部 API 不可达）。
    #[must_use]
    pub fn parsed_version(&self) -> Version {
        Version::parse(&self.plugin.version).expect("manifest.validate enforces SemVer")
    }

    /// 已校验后的兼容性范围。
    ///
    /// # Panics
    /// 同 [`PluginManifest::parsed_version`]。
    #[must_use]
    pub fn parsed_compatibility(&self) -> VersionReq {
        VersionReq::parse(&self.compatibility.cycms).expect("manifest.validate enforces VersionReq")
    }

    /// 展开 permissions 段为 [`PermissionDefinition`] 列表，
    /// 可直接作为 `PermissionEngine::register_permissions` 的入参。
    #[must_use]
    pub fn permission_definitions(&self) -> Vec<PermissionDefinition> {
        self.permissions
            .as_ref()
            .map(|p| {
                p.definitions
                    .iter()
                    .map(|e| PermissionDefinition {
                        domain: e.domain.clone(),
                        resource: e.resource.clone(),
                        action: e.action.clone(),
                        scope: e.scope,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn validate_plugin_name(name: &str) -> Result<(), PluginManagerError> {
    if name.is_empty() {
        return Err(PluginManagerError::InvalidManifest(
            "plugin.name must not be empty".into(),
        ));
    }
    if name.contains('.') {
        return Err(PluginManagerError::InvalidManifest(
            "plugin.name must not contain '.' (collides with ServiceRegistry key separator)".into(),
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(PluginManagerError::InvalidManifest(format!(
            "plugin.name must match [A-Za-z0-9_-]+, got {name:?}"
        )));
    }
    Ok(())
}

fn check_permission_segment(field: &str, value: &str) -> Result<(), PluginManagerError> {
    if value.trim().is_empty() {
        return Err(PluginManagerError::InvalidManifest(format!(
            "{field} must not be empty"
        )));
    }
    if value.contains('.') {
        return Err(PluginManagerError::InvalidManifest(format!(
            "{field} {value:?} must not contain '.'"
        )));
    }
    Ok(())
}

fn validate_host_manifest(host: &HostManifestSpec) -> Result<(), PluginManagerError> {
    let asset_ids = validate_unique_ids(
        host.assets.iter().map(|asset| asset.id.as_str()),
        "host.assets.id",
    )?;
    validate_unique_ids(
        host.public_pages.iter().map(|page| page.id.as_str()),
        "host.public_pages.id",
    )?;
    validate_unique_ids(
        host.admin_pages.iter().map(|page| page.id.as_str()),
        "host.admin_pages.id",
    )?;
    validate_unique_ids(
        host.parsers.iter().map(|parser| parser.id.as_str()),
        "host.parsers.id",
    )?;
    validate_unique_ids(
        host.hooks.iter().map(|hook| hook.id.as_str()),
        "host.hooks.id",
    )?;
    validate_unique_ids(
        host.editors.iter().map(|editor| editor.id.as_str()),
        "host.editors.id",
    )?;
    validate_unique_ids(
        host.compatibility.iter().map(|item| item.id.as_str()),
        "host.compatibility.id",
    )?;

    for page in &host.public_pages {
        validate_non_empty("host.public_pages.id", &page.id)?;
        validate_non_empty("host.public_pages.handler", &page.handler)?;
        validate_host_path("host.public_pages.path", &page.path)?;
        validate_asset_bundle_refs(
            &asset_ids,
            &page.asset_bundle_ids,
            "host.public_pages.asset_bundle_ids",
        )?;
    }

    for page in &host.admin_pages {
        validate_non_empty("host.admin_pages.id", &page.id)?;
        validate_non_empty("host.admin_pages.title", &page.title)?;
        validate_non_empty("host.admin_pages.handler", &page.handler)?;
        validate_host_path("host.admin_pages.path", &page.path)?;
        validate_asset_bundle_refs(
            &asset_ids,
            &page.asset_bundle_ids,
            "host.admin_pages.asset_bundle_ids",
        )?;
    }

    for parser in &host.parsers {
        validate_non_empty("host.parsers.id", &parser.id)?;
        validate_non_empty("host.parsers.parser", &parser.parser)?;
        if parser.content_types.is_empty()
            && parser.field_names.is_empty()
            && parser.source_formats.is_empty()
        {
            return Err(PluginManagerError::InvalidManifest(
                "host.parsers must declare at least one content_types, field_names, or source_formats selector"
                    .into(),
            ));
        }
        validate_non_empty_items(&parser.content_types, "host.parsers.content_types")?;
        validate_non_empty_items(&parser.field_names, "host.parsers.field_names")?;
        validate_non_empty_items(&parser.source_formats, "host.parsers.source_formats")?;
    }

    for hook in &host.hooks {
        validate_non_empty("host.hooks.id", &hook.id)?;
        validate_non_empty("host.hooks.phase", &hook.phase)?;
        validate_non_empty("host.hooks.handler", &hook.handler)?;
    }

    for asset in &host.assets {
        validate_non_empty("host.assets.id", &asset.id)?;
        if asset.modules.is_empty()
            && asset.scripts.is_empty()
            && asset.styles.is_empty()
            && asset.inline_data_keys.is_empty()
        {
            return Err(PluginManagerError::InvalidManifest(
                "host.assets entries must declare at least one module, script, style, or inline_data_keys entry"
                    .into(),
            ));
        }
        validate_non_empty_items(&asset.apply_to, "host.assets.apply_to")?;
        validate_non_empty_items(&asset.modules, "host.assets.modules")?;
        validate_non_empty_items(&asset.scripts, "host.assets.scripts")?;
        validate_non_empty_items(&asset.styles, "host.assets.styles")?;
        validate_non_empty_items(&asset.inline_data_keys, "host.assets.inline_data_keys")?;
    }

    for editor in &host.editors {
        validate_non_empty("host.editors.id", &editor.id)?;
        validate_non_empty("host.editors.editor", &editor.editor)?;
        if editor.content_types.is_empty()
            && editor.field_types.is_empty()
            && editor.screen_targets.is_empty()
        {
            return Err(PluginManagerError::InvalidManifest(
                "host.editors must declare at least one content_types, field_types, or screen_targets selector"
                    .into(),
            ));
        }
        validate_non_empty_items(&editor.content_types, "host.editors.content_types")?;
        validate_non_empty_items(&editor.field_types, "host.editors.field_types")?;
        validate_non_empty_items(&editor.screen_targets, "host.editors.screen_targets")?;
        validate_asset_bundle_refs(&asset_ids, &editor.asset_bundle_ids, "host.editors.asset_bundle_ids")?;
    }

    for item in &host.compatibility {
        validate_non_empty("host.compatibility.id", &item.id)?;
        validate_non_empty("host.compatibility.target", &item.target)?;
        if item.kind == CompatibilityKind::ManifestCompatibilityBridge {
            continue;
        }
    }

    Ok(())
}

fn validate_non_empty(field: &str, value: &str) -> Result<(), PluginManagerError> {
    if value.trim().is_empty() {
        return Err(PluginManagerError::InvalidManifest(format!(
            "{field} must not be empty"
        )));
    }
    Ok(())
}

fn validate_non_empty_items(items: &[String], field: &str) -> Result<(), PluginManagerError> {
    for item in items {
        validate_non_empty(field, item)?;
    }
    Ok(())
}

fn validate_unique_ids<'a>(
    ids: impl Iterator<Item = &'a str>,
    field: &str,
) -> Result<BTreeSet<String>, PluginManagerError> {
    let mut seen = BTreeSet::new();
    for id in ids {
        validate_non_empty(field, id)?;
        if !seen.insert(id.to_owned()) {
            return Err(PluginManagerError::InvalidManifest(format!(
                "duplicate {field} value {id:?}"
            )));
        }
    }
    Ok(seen)
}

fn validate_host_path(field: &str, path: &str) -> Result<(), PluginManagerError> {
    let trimmed = path.trim();
    if !trimmed.starts_with('/') {
        return Err(PluginManagerError::InvalidManifest(format!(
            "{field} must start with '/'"
        )));
    }
    if trimmed.contains(' ') {
        return Err(PluginManagerError::InvalidManifest(format!(
            "{field} must not contain spaces"
        )));
    }
    Ok(())
}

fn validate_asset_bundle_refs(
    known_ids: &BTreeSet<String>,
    refs: &[String],
    field: &str,
) -> Result<(), PluginManagerError> {
    for reference in refs {
        validate_non_empty(field, reference)?;
        if !known_ids.contains(reference) {
            return Err(PluginManagerError::InvalidManifest(format!(
                "{field} references unknown asset bundle {reference:?}"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_TOML: &str = r#"
[plugin]
name = "blog"
version = "0.1.0"
kind = "native"
entry = "blog.so"

[compatibility]
cycms = ">=0.1.0"
"#;

    #[test]
    fn minimal_manifest_parses() {
        let m = PluginManifest::from_toml_str(MINIMAL_TOML).unwrap();
        assert_eq!(m.plugin.name, "blog");
        assert_eq!(m.plugin.kind, PluginKind::Native);
        assert!(m.dependencies.is_empty());
        assert!(m.permissions.is_none());
        assert!(m.frontend.is_none());
        assert!(m.migrations.is_empty());
        assert_eq!(m.parsed_version().to_string(), "0.1.0");
        assert!(
            m.parsed_compatibility()
                .matches(&Version::parse("0.1.5").unwrap())
        );
    }

    #[test]
    fn full_manifest_parses() {
        let toml_text = r#"
migrations = ["migrations"]

[plugin]
name = "blog"
version = "1.2.3"
kind = "wasm"
entry = "dist/blog.wasm"
description = "A blog plugin"
author = "someone"
license = "Apache-2.0"

[compatibility]
cycms = ">=0.1.0, <0.2.0"

[dependencies]
auth-oauth = { version = "^0.1" }
billing = { version = "^0.2", optional = true }

[permissions]
definitions = [
  { domain = "blog", resource = "post", action = "create" },
  { domain = "blog", resource = "post", action = "update", scope = "own" },
]

[frontend]
manifest = "admin/manifest.json"
required = true

[host]

[[host.assets]]
id = "blog-admin"
modules = ["admin/main.js"]
styles = ["admin/main.css"]

[[host.public_pages]]
id = "blog-home"
path = "/blog"
handler = "blog::public::home"
asset_bundle_ids = ["blog-admin"]

[[host.admin_pages]]
id = "blog-editor"
path = "/admin/blog/editor"
title = "Blog Editor"
mode = "hybrid"
handler = "blog::admin::editor"
asset_bundle_ids = ["blog-admin"]

[[host.parsers]]
id = "blog-markdown"
parser = "blog::parse_markdown"
source_formats = ["markdown"]

[[host.hooks]]
id = "blog-before-send"
phase = "before_send"
handler = "blog::hooks::before_send"

[[host.editors]]
id = "blog-post-editor"
editor = "blog::editor::post"
content_types = ["post"]
asset_bundle_ids = ["blog-admin"]

[[host.compatibility]]
id = "legacy-blog-api"
kind = "manifest_compatibility_bridge"
target = "/api/v1/public/blog"
"#;
        let m = PluginManifest::from_toml_str(toml_text).unwrap();
        assert_eq!(m.plugin.kind, PluginKind::Wasm);
        assert_eq!(m.dependencies.len(), 2);
        assert!(!m.dependencies["auth-oauth"].optional);
        assert!(m.dependencies["billing"].optional);
        let defs = m.permission_definitions();
        assert_eq!(defs.len(), 2);
        assert_eq!(defs[0].scope, PermissionScope::All);
        assert_eq!(defs[1].scope, PermissionScope::Own);
        assert_eq!(m.frontend.as_ref().unwrap().manifest, "admin/manifest.json");
        assert!(m.frontend.as_ref().unwrap().required);
        let host = m.host.as_ref().unwrap();
        assert_eq!(host.assets.len(), 1);
        assert_eq!(host.public_pages.len(), 1);
        assert_eq!(host.admin_pages.len(), 1);
        assert_eq!(host.parsers.len(), 1);
        assert_eq!(host.hooks.len(), 1);
        assert_eq!(host.editors.len(), 1);
        assert_eq!(host.compatibility.len(), 1);
        assert_eq!(m.migrations, vec!["migrations".to_owned()]);
    }

    #[test]
    fn host_asset_reference_must_exist() {
        let mut bad = MINIMAL_TOML.to_owned();
        bad.push_str(
            r#"
[host]

[[host.public_pages]]
id = "blog-home"
path = "/blog"
handler = "blog::public::home"
asset_bundle_ids = ["missing"]
"#,
        );
        assert!(PluginManifest::from_toml_str(&bad).is_err());
    }

    #[test]
    fn name_with_space_rejected() {
        let bad = MINIMAL_TOML.replace(r#"name = "blog""#, r#"name = "has space""#);
        let err = PluginManifest::from_toml_str(&bad).unwrap_err();
        assert!(matches!(err, PluginManagerError::InvalidManifest(_)));
    }

    #[test]
    fn name_with_dot_rejected() {
        let bad = MINIMAL_TOML.replace(r#"name = "blog""#, r#"name = "my.plugin""#);
        assert!(PluginManifest::from_toml_str(&bad).is_err());
    }

    #[test]
    fn empty_entry_rejected() {
        let bad = MINIMAL_TOML.replace(r#"entry = "blog.so""#, r#"entry = """#);
        assert!(PluginManifest::from_toml_str(&bad).is_err());
    }

    #[test]
    fn invalid_semver_version_rejected() {
        let bad = MINIMAL_TOML.replace(r#"version = "0.1.0""#, r#"version = "not-a-version""#);
        assert!(PluginManifest::from_toml_str(&bad).is_err());
    }

    #[test]
    fn invalid_compatibility_range_rejected() {
        let bad = MINIMAL_TOML.replace(r#"cycms = ">=0.1.0""#, r#"cycms = "???""#);
        assert!(PluginManifest::from_toml_str(&bad).is_err());
    }

    #[test]
    fn unknown_kind_rejected() {
        let bad = MINIMAL_TOML.replace(r#"kind = "native""#, r#"kind = "python""#);
        assert!(PluginManifest::from_toml_str(&bad).is_err());
    }

    #[test]
    fn invalid_dependency_version_rejected() {
        let mut bad = MINIMAL_TOML.to_owned();
        bad.push_str(
            r#"
[dependencies]
auth = { version = "bad" }
"#,
        );
        assert!(PluginManifest::from_toml_str(&bad).is_err());
    }

    #[test]
    fn permission_empty_segment_rejected() {
        let mut bad = MINIMAL_TOML.to_owned();
        bad.push_str(
            r#"
[permissions]
definitions = [
  { domain = "", resource = "post", action = "read" },
]
"#,
        );
        assert!(PluginManifest::from_toml_str(&bad).is_err());
    }

    #[test]
    fn permission_segment_with_dot_rejected() {
        let mut bad = MINIMAL_TOML.to_owned();
        bad.push_str(
            r#"
[permissions]
definitions = [
  { domain = "blog.extra", resource = "post", action = "read" },
]
"#,
        );
        assert!(PluginManifest::from_toml_str(&bad).is_err());
    }
}
