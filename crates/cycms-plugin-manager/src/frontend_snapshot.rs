use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD as BASE64_STANDARD, URL_SAFE_NO_PAD};
use cycms_core::{Error, Result};
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256, Sha384};

use crate::frontend_manifest::{
    ADMIN_SHELL_SDK_VERSION, AdminFrontendManifest, ContributionMatchSpec,
    normalize_namespace_path, normalize_plugin_file_path, validate_content_type_api_ids,
    validate_permission_codes, validate_sdk_range,
};
use crate::manifest::PluginManifest;

const FRONTEND_RUNTIME_KEY: &str = "frontend_runtime";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendRuntimeState {
    pub manifest_path: String,
    pub required: bool,
    pub compatibility: FrontendCompatibility,
    pub snapshot: NormalizedFrontendSnapshot,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendCompatibility {
    pub shell_sdk_version: String,
    pub plugin_sdk_range: String,
    pub compatible: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedFrontendSnapshot {
    pub schema_version: u32,
    pub sdk_version: String,
    pub plugin_name: String,
    pub plugin_version: String,
    pub files: Vec<NormalizedFrontendFile>,
    pub assets: Vec<NormalizedFrontendAsset>,
    pub menus: Vec<NormalizedMenuContribution>,
    pub routes: Vec<NormalizedRouteContribution>,
    pub slots: Vec<NormalizedSlotContribution>,
    pub field_renderers: Vec<NormalizedFieldRendererContribution>,
    pub settings: Option<NormalizedSettingsContribution>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedFrontendFile {
    pub path: String,
    pub content_type: String,
    pub integrity: String,
    pub url_hash: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedFrontendAsset {
    pub id: String,
    pub module_path: String,
    pub style_paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedMenuContribution {
    pub id: String,
    pub label: String,
    pub zone: String,
    pub icon: Option<String>,
    pub order: i32,
    pub to: String,
    pub required_permissions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedRouteContribution {
    pub id: String,
    pub path: String,
    pub module_asset_id: String,
    pub kind: String,
    pub title: String,
    pub required_permissions: Vec<String>,
    pub r#match: ContributionMatchSpec,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedSlotContribution {
    pub id: String,
    pub slot: String,
    pub module_asset_id: String,
    pub order: i32,
    pub required_permissions: Vec<String>,
    pub r#match: ContributionMatchSpec,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedFieldRendererContribution {
    pub id: String,
    pub type_name: String,
    pub module_asset_id: String,
    pub required_permissions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedSettingsContribution {
    pub namespace: String,
    pub required_permissions: Vec<String>,
    pub custom_page: Option<NormalizedSettingsPage>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedSettingsPage {
    pub path: String,
    pub module_asset_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminExtensionBootstrap {
    pub revision: String,
    pub shell_sdk_version: String,
    pub plugins: Vec<BootstrapPlugin>,
    pub diagnostics: Vec<ExtensionDiagnostic>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminExtensionDiagnostics {
    pub revision: String,
    pub diagnostics: Vec<ExtensionDiagnostic>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapPlugin {
    pub name: String,
    pub version: String,
    pub menus: Vec<BootstrapMenuContribution>,
    pub routes: Vec<BootstrapRouteContribution>,
    pub slots: Vec<BootstrapSlotContribution>,
    pub field_renderers: Vec<BootstrapFieldRendererContribution>,
    pub settings: Option<BootstrapSettingsContribution>,
}

impl BootstrapPlugin {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.menus.is_empty()
            && self.routes.is_empty()
            && self.slots.is_empty()
            && self.field_renderers.is_empty()
            && self.settings.is_none()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapMenuContribution {
    pub id: String,
    pub label: String,
    pub zone: String,
    pub icon: Option<String>,
    pub order: i32,
    pub to: String,
    pub full_path: String,
    pub required_permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapRouteContribution {
    pub id: String,
    pub path: String,
    pub full_path: String,
    pub module_url: String,
    pub styles: Vec<String>,
    pub kind: String,
    pub title: String,
    pub required_permissions: Vec<String>,
    pub r#match: ContributionMatchSpec,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapSlotContribution {
    pub id: String,
    pub slot: String,
    pub order: i32,
    pub module_url: String,
    pub styles: Vec<String>,
    pub required_permissions: Vec<String>,
    pub r#match: ContributionMatchSpec,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapFieldRendererContribution {
    pub id: String,
    pub type_name: String,
    pub module_url: String,
    pub styles: Vec<String>,
    pub required_permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapSettingsContribution {
    pub namespace: String,
    pub required_permissions: Vec<String>,
    pub custom_page: Option<BootstrapSettingsPage>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapSettingsPage {
    pub path: String,
    pub full_path: String,
    pub module_url: String,
    pub styles: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionDiagnostic {
    pub plugin_name: String,
    pub plugin_version: String,
    pub severity: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedPluginAsset {
    pub absolute_path: std::path::PathBuf,
    pub content_type: String,
    pub etag: String,
}

pub fn build_frontend_runtime_state(
    plugin_dir: &Path,
    plugin_manifest: &PluginManifest,
    frontend_manifest: AdminFrontendManifest,
) -> Result<FrontendRuntimeState> {
    let spec = plugin_manifest
        .frontend
        .as_ref()
        .ok_or_else(|| Error::Internal {
            message: format!(
                "plugin {} has no frontend spec while building runtime state",
                plugin_manifest.plugin.name
            ),
            source: None,
        })?;

    if frontend_manifest.schema_version != 1 {
        return Err(Error::ValidationError {
            message: format!(
                "plugin {} frontend schemaVersion {} is unsupported",
                plugin_manifest.plugin.name, frontend_manifest.schema_version
            ),
            details: None,
        });
    }
    if frontend_manifest.plugin_name != plugin_manifest.plugin.name {
        return Err(Error::ValidationError {
            message: format!(
                "plugin {} frontend manifest pluginName {:?} does not match plugin.toml",
                plugin_manifest.plugin.name, frontend_manifest.plugin_name
            ),
            details: None,
        });
    }
    if frontend_manifest.plugin_version != plugin_manifest.plugin.version {
        return Err(Error::ValidationError {
            message: format!(
                "plugin {} frontend manifest pluginVersion {:?} does not match plugin.toml",
                plugin_manifest.plugin.name, frontend_manifest.plugin_version
            ),
            details: None,
        });
    }

    let sdk_range = validate_sdk_range(&frontend_manifest.sdk_version)?;
    let shell_sdk = Version::parse(ADMIN_SHELL_SDK_VERSION).expect("valid admin shell sdk");
    let compatibility = if sdk_range.matches(&shell_sdk) {
        FrontendCompatibility {
            shell_sdk_version: ADMIN_SHELL_SDK_VERSION.to_owned(),
            plugin_sdk_range: frontend_manifest.sdk_version.clone(),
            compatible: true,
            reason: None,
        }
    } else {
        FrontendCompatibility {
            shell_sdk_version: ADMIN_SHELL_SDK_VERSION.to_owned(),
            plugin_sdk_range: frontend_manifest.sdk_version.clone(),
            compatible: false,
            reason: Some(format!(
                "plugin {} frontend requires admin shell sdk {}, host is {}",
                plugin_manifest.plugin.name, frontend_manifest.sdk_version, ADMIN_SHELL_SDK_VERSION
            )),
        }
    };

    let mut contribution_ids = BTreeSet::new();
    let mut asset_ids = BTreeSet::new();
    let mut file_map = BTreeMap::<String, NormalizedFrontendFile>::new();
    let mut assets = Vec::with_capacity(frontend_manifest.assets.len());
    for asset in frontend_manifest.assets {
        insert_unique(&mut contribution_ids, &asset.id, "asset id")?;
        insert_unique(&mut asset_ids, &asset.id, "asset id")?;
        if asset.content_type.trim().is_empty() {
            return Err(Error::ValidationError {
                message: format!(
                    "plugin {} frontend asset {} must declare contentType",
                    plugin_manifest.plugin.name, asset.id
                ),
                details: None,
            });
        }

        let module_file = load_file_descriptor(
            plugin_dir,
            &asset.path,
            &format!("assets.{}.path", asset.id),
            &asset.content_type,
            Some(&asset.sha384),
        )?;
        upsert_file_descriptor(&mut file_map, module_file.clone())?;

        let mut style_paths = Vec::with_capacity(asset.styles.len());
        for style in asset.styles {
            let style_file = load_file_descriptor(
                plugin_dir,
                &style,
                &format!("assets.{}.styles", asset.id),
                "text/css",
                None,
            )?;
            upsert_file_descriptor(&mut file_map, style_file.clone())?;
            style_paths.push(style_file.path);
        }
        assets.push(NormalizedFrontendAsset {
            id: asset.id,
            module_path: module_file.path,
            style_paths,
        });
    }

    let mut menus = Vec::with_capacity(frontend_manifest.menus.len());
    for menu in frontend_manifest.menus {
        insert_unique(&mut contribution_ids, &menu.id, "menu id")?;
        validate_permission_codes(
            &menu.required_permissions,
            &format!("menus.{}.requiredPermissions", menu.id),
        )?;
        let to = normalize_namespace_path(&menu.to, &format!("menus.{}.to", menu.id))?;
        if menu.label.trim().is_empty() || menu.zone.trim().is_empty() {
            return Err(Error::ValidationError {
                message: format!(
                    "plugin {} menu {} must declare label and zone",
                    plugin_manifest.plugin.name, menu.id
                ),
                details: None,
            });
        }
        menus.push(NormalizedMenuContribution {
            id: menu.id,
            label: menu.label.trim().to_owned(),
            zone: menu.zone.trim().to_owned(),
            icon: menu.icon.filter(|icon| !icon.trim().is_empty()),
            order: menu.order,
            to,
            required_permissions: menu.required_permissions,
        });
    }

    let mut routes = Vec::with_capacity(frontend_manifest.routes.len());
    for route in frontend_manifest.routes {
        insert_unique(&mut contribution_ids, &route.id, "route id")?;
        require_asset_reference(&asset_ids, &route.module_asset_id, &route.id)?;
        validate_permission_codes(
            &route.required_permissions,
            &format!("routes.{}.requiredPermissions", route.id),
        )?;
        let path = normalize_namespace_path(&route.path, &format!("routes.{}.path", route.id))?;
        let normalized_match = ContributionMatchSpec {
            content_type_api_ids: validate_content_type_api_ids(
                &route.r#match.content_type_api_ids,
                &format!("routes.{}.match.contentTypeApiIds", route.id),
            )?,
        };
        if route.kind.trim().is_empty() || route.title.trim().is_empty() {
            return Err(Error::ValidationError {
                message: format!(
                    "plugin {} route {} must declare kind and title",
                    plugin_manifest.plugin.name, route.id
                ),
                details: None,
            });
        }
        routes.push(NormalizedRouteContribution {
            id: route.id,
            path,
            module_asset_id: route.module_asset_id,
            kind: route.kind.trim().to_owned(),
            title: route.title.trim().to_owned(),
            required_permissions: route.required_permissions,
            r#match: normalized_match,
        });
    }

    let mut slots = Vec::with_capacity(frontend_manifest.slots.len());
    for slot in frontend_manifest.slots {
        insert_unique(&mut contribution_ids, &slot.id, "slot id")?;
        require_asset_reference(&asset_ids, &slot.module_asset_id, &slot.id)?;
        validate_permission_codes(
            &slot.required_permissions,
            &format!("slots.{}.requiredPermissions", slot.id),
        )?;
        let normalized_match = ContributionMatchSpec {
            content_type_api_ids: validate_content_type_api_ids(
                &slot.r#match.content_type_api_ids,
                &format!("slots.{}.match.contentTypeApiIds", slot.id),
            )?,
        };
        if slot.slot.trim().is_empty() {
            return Err(Error::ValidationError {
                message: format!(
                    "plugin {} slot {} must declare slot name",
                    plugin_manifest.plugin.name, slot.id
                ),
                details: None,
            });
        }
        slots.push(NormalizedSlotContribution {
            id: slot.id,
            slot: slot.slot.trim().to_owned(),
            module_asset_id: slot.module_asset_id,
            order: slot.order,
            required_permissions: slot.required_permissions,
            r#match: normalized_match,
        });
    }

    let mut field_type_names = BTreeSet::new();
    let mut field_renderers = Vec::with_capacity(frontend_manifest.field_renderers.len());
    for renderer in frontend_manifest.field_renderers {
        insert_unique(&mut contribution_ids, &renderer.id, "field renderer id")?;
        insert_unique(
            &mut field_type_names,
            &renderer.type_name,
            "field renderer typeName",
        )?;
        require_asset_reference(&asset_ids, &renderer.module_asset_id, &renderer.id)?;
        validate_permission_codes(
            &renderer.required_permissions,
            &format!("fieldRenderers.{}.requiredPermissions", renderer.id),
        )?;
        if renderer.type_name.trim().is_empty() {
            return Err(Error::ValidationError {
                message: format!(
                    "plugin {} field renderer {} must declare typeName",
                    plugin_manifest.plugin.name, renderer.id
                ),
                details: None,
            });
        }
        field_renderers.push(NormalizedFieldRendererContribution {
            id: renderer.id,
            type_name: renderer.type_name.trim().to_owned(),
            module_asset_id: renderer.module_asset_id,
            required_permissions: renderer.required_permissions,
        });
    }

    let settings = if let Some(settings) = frontend_manifest.settings {
        validate_permission_codes(
            &settings.required_permissions,
            "settings.requiredPermissions",
        )?;
        let namespace = settings.namespace.trim();
        if namespace.is_empty() {
            return Err(Error::ValidationError {
                message: format!(
                    "plugin {} settings namespace must not be empty",
                    plugin_manifest.plugin.name
                ),
                details: None,
            });
        }
        let custom_page = if let Some(page) = settings.custom_page {
            require_asset_reference(&asset_ids, &page.module_asset_id, "settings.customPage")?;
            Some(NormalizedSettingsPage {
                path: normalize_namespace_path(&page.path, "settings.customPage.path")?,
                module_asset_id: page.module_asset_id,
            })
        } else {
            None
        };
        Some(NormalizedSettingsContribution {
            namespace: namespace.to_owned(),
            required_permissions: settings.required_permissions,
            custom_page,
        })
    } else {
        None
    };

    let mut files: Vec<NormalizedFrontendFile> = file_map.into_values().collect();
    files.sort_by(|left, right| left.path.cmp(&right.path));
    assets.sort_by(|left, right| left.id.cmp(&right.id));
    menus.sort_by(|left, right| left.id.cmp(&right.id));
    routes.sort_by(|left, right| left.id.cmp(&right.id));
    slots.sort_by(|left, right| left.id.cmp(&right.id));
    field_renderers.sort_by(|left, right| left.id.cmp(&right.id));

    Ok(FrontendRuntimeState {
        manifest_path: normalize_plugin_file_path(&spec.manifest, "frontend.manifest")?,
        required: spec.required,
        compatibility,
        snapshot: NormalizedFrontendSnapshot {
            schema_version: frontend_manifest.schema_version,
            sdk_version: frontend_manifest.sdk_version,
            plugin_name: frontend_manifest.plugin_name,
            plugin_version: frontend_manifest.plugin_version,
            files,
            assets,
            menus,
            routes,
            slots,
            field_renderers,
            settings,
        },
    })
}

pub fn frontend_runtime_state(manifest_value: &Value) -> Result<Option<FrontendRuntimeState>> {
    let Some(raw_state) = manifest_value.get(FRONTEND_RUNTIME_KEY) else {
        return Ok(None);
    };
    serde_json::from_value(raw_state.clone())
        .map(Some)
        .map_err(|source| Error::Internal {
            message: format!("failed to decode persisted frontend runtime state: {source}"),
            source: None,
        })
}

pub fn insert_frontend_runtime_state(
    manifest_value: &mut Value,
    state: &FrontendRuntimeState,
) -> Result<()> {
    let object = manifest_value
        .as_object_mut()
        .ok_or_else(|| Error::Internal {
            message: "plugin manifest JSON must be an object".to_owned(),
            source: None,
        })?;
    let runtime_value = serde_json::to_value(state).map_err(|source| Error::Internal {
        message: format!("failed to encode frontend runtime state: {source}"),
        source: None,
    })?;
    object.insert(FRONTEND_RUNTIME_KEY.to_owned(), runtime_value);
    Ok(())
}

pub fn validate_cross_plugin_conflicts(states: &[&FrontendRuntimeState]) -> Result<()> {
    let mut route_ids = BTreeMap::<String, String>::new();
    let mut menu_ids = BTreeMap::<String, String>::new();
    let mut slot_ids = BTreeMap::<String, String>::new();
    let mut field_bindings = BTreeMap::<String, String>::new();
    let mut conflicts = Vec::new();

    for state in states {
        let plugin = state.snapshot.plugin_name.as_str();
        for route in &state.snapshot.routes {
            if let Some(previous) = route_ids.insert(route.id.clone(), plugin.to_owned()) {
                conflicts.push(format!(
                    "route id {} declared by {} and {}",
                    route.id, previous, plugin
                ));
            }
        }
        for menu in &state.snapshot.menus {
            if let Some(previous) = menu_ids.insert(menu.id.clone(), plugin.to_owned()) {
                conflicts.push(format!(
                    "menu id {} declared by {} and {}",
                    menu.id, previous, plugin
                ));
            }
        }
        for slot in &state.snapshot.slots {
            if let Some(previous) = slot_ids.insert(slot.id.clone(), plugin.to_owned()) {
                conflicts.push(format!(
                    "slot id {} declared by {} and {}",
                    slot.id, previous, plugin
                ));
            }
        }
        for renderer in &state.snapshot.field_renderers {
            if let Some(previous) =
                field_bindings.insert(renderer.type_name.clone(), plugin.to_owned())
            {
                conflicts.push(format!(
                    "field renderer type {} declared by {} and {}",
                    renderer.type_name, previous, plugin
                ));
            }
        }
    }

    if conflicts.is_empty() {
        Ok(())
    } else {
        Err(Error::Conflict {
            message: format!(
                "frontend contribution conflicts detected: {}",
                conflicts.join("; ")
            ),
        })
    }
}

pub fn extension_revision_token(states: &[&FrontendRuntimeState]) -> Result<String> {
    let mut ordered: Vec<&FrontendRuntimeState> = states.to_vec();
    ordered.sort_by(|left, right| left.snapshot.plugin_name.cmp(&right.snapshot.plugin_name));
    let bytes = serde_json::to_vec(&ordered).map_err(|source| Error::Internal {
        message: format!("failed to encode extension revision payload: {source}"),
        source: None,
    })?;
    let digest = Sha256::digest(bytes);
    Ok(format!("extrev:{digest:x}"))
}

pub fn plugin_admin_full_path(plugin_name: &str, relative_path: &str) -> String {
    let suffix = relative_path.trim_start_matches('/');
    if suffix.is_empty() {
        format!("/admin/x/{plugin_name}")
    } else {
        format!("/admin/x/{plugin_name}/{suffix}")
    }
}

pub fn resolve_plugin_asset(
    plugin_root: &Path,
    state: &FrontendRuntimeState,
    version: &str,
    url_hash: &str,
    asset_path: &str,
) -> Result<Option<ResolvedPluginAsset>> {
    if state.snapshot.plugin_version != version {
        return Ok(None);
    }
    let normalized_path = normalize_plugin_file_path(asset_path, "plugin asset path")?;
    let Some(file) = state
        .snapshot
        .files
        .iter()
        .find(|file| file.path == normalized_path && file.url_hash == url_hash)
    else {
        return Ok(None);
    };
    Ok(Some(ResolvedPluginAsset {
        absolute_path: plugin_root.join(&file.path),
        content_type: file.content_type.clone(),
        etag: format!("\"{}\"", file.url_hash),
    }))
}

fn insert_unique(seen: &mut BTreeSet<String>, value: &str, label: &str) -> Result<()> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(Error::ValidationError {
            message: format!("{label} must not be empty"),
            details: None,
        });
    }
    if !seen.insert(trimmed.to_owned()) {
        return Err(Error::ValidationError {
            message: format!("duplicate {label} {trimmed:?}"),
            details: None,
        });
    }
    Ok(())
}

fn require_asset_reference(
    asset_ids: &BTreeSet<String>,
    asset_id: &str,
    owner: &str,
) -> Result<()> {
    if asset_ids.contains(asset_id) {
        Ok(())
    } else {
        Err(Error::ValidationError {
            message: format!("{owner} references unknown moduleAssetId {asset_id:?}"),
            details: None,
        })
    }
}

fn load_file_descriptor(
    plugin_dir: &Path,
    relative_path: &str,
    field: &str,
    content_type: &str,
    expected_integrity: Option<&str>,
) -> Result<NormalizedFrontendFile> {
    let normalized = normalize_plugin_file_path(relative_path, field)?;
    let absolute = plugin_dir.join(&normalized);
    let bytes = fs::read(&absolute).map_err(|source| Error::ValidationError {
        message: format!(
            "failed to read declared asset {}: {source}",
            absolute.display()
        ),
        details: None,
    })?;
    let digest = Sha384::digest(&bytes);
    let integrity = format!("sha384-{}", BASE64_STANDARD.encode(digest));
    if let Some(expected) = expected_integrity
        && expected != integrity
    {
        return Err(Error::ValidationError {
            message: format!(
                "declared integrity for {} does not match on-disk asset",
                absolute.display()
            ),
            details: None,
        });
    }
    let url_hash = URL_SAFE_NO_PAD.encode(digest);
    Ok(NormalizedFrontendFile {
        path: normalized,
        content_type: content_type.trim().to_owned(),
        integrity,
        url_hash,
    })
}

fn upsert_file_descriptor(
    files: &mut BTreeMap<String, NormalizedFrontendFile>,
    descriptor: NormalizedFrontendFile,
) -> Result<()> {
    if let Some(previous) = files.get(&descriptor.path) {
        if previous == &descriptor {
            return Ok(());
        }
        return Err(Error::ValidationError {
            message: format!(
                "asset path {:?} is declared more than once with incompatible metadata",
                descriptor.path
            ),
            details: None,
        });
    }
    files.insert(descriptor.path.clone(), descriptor);
    Ok(())
}
