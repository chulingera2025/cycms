use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path};

use cycms_core::{Error, Result};
use semver::VersionReq;
use serde::{Deserialize, Serialize};

use crate::manifest::FrontendSpec;

pub const ADMIN_SHELL_SDK_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminFrontendManifest {
    pub schema_version: u32,
    pub sdk_version: String,
    pub plugin_name: String,
    pub plugin_version: String,
    #[serde(default)]
    pub assets: Vec<FrontendAsset>,
    #[serde(default)]
    pub menus: Vec<MenuContribution>,
    #[serde(default)]
    pub routes: Vec<RouteContribution>,
    #[serde(default)]
    pub slots: Vec<SlotContribution>,
    #[serde(default)]
    pub field_renderers: Vec<FieldRendererContribution>,
    pub settings: Option<SettingsContribution>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendAsset {
    pub id: String,
    pub path: String,
    pub sha384: String,
    pub content_type: String,
    #[serde(default)]
    pub styles: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuContribution {
    pub id: String,
    pub label: String,
    pub zone: String,
    pub icon: Option<String>,
    #[serde(default)]
    pub order: i32,
    pub to: String,
    #[serde(default)]
    pub required_permissions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteContribution {
    pub id: String,
    pub path: String,
    pub module_asset_id: String,
    pub kind: String,
    pub title: String,
    #[serde(default)]
    pub required_permissions: Vec<String>,
    #[serde(default)]
    pub r#match: ContributionMatchSpec,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotContribution {
    pub id: String,
    pub slot: String,
    pub module_asset_id: String,
    #[serde(default)]
    pub order: i32,
    #[serde(default)]
    pub required_permissions: Vec<String>,
    #[serde(default)]
    pub r#match: ContributionMatchSpec,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldRendererContribution {
    pub id: String,
    pub type_name: String,
    pub module_asset_id: String,
    #[serde(default)]
    pub required_permissions: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContributionMatchSpec {
    #[serde(default)]
    pub content_type_api_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsContribution {
    pub namespace: String,
    #[serde(default)]
    pub required_permissions: Vec<String>,
    pub custom_page: Option<CustomPageContribution>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomPageContribution {
    pub path: String,
    pub module_asset_id: String,
}

pub fn load_frontend_manifest(
    plugin_dir: &Path,
    spec: &FrontendSpec,
) -> Result<AdminFrontendManifest> {
    let manifest_path = normalize_plugin_file_path(&spec.manifest, "frontend.manifest")?;
    let abs_path = plugin_dir.join(&manifest_path);
    let text = fs::read_to_string(&abs_path).map_err(|source| Error::ValidationError {
        message: format!(
            "failed to read frontend manifest {}: {source}",
            abs_path.display()
        ),
        details: None,
    })?;
    let manifest = serde_json::from_str(&text).map_err(|source| Error::ValidationError {
        message: format!(
            "failed to parse frontend manifest {}: {source}",
            abs_path.display()
        ),
        details: None,
    })?;
    Ok(manifest)
}

pub(crate) fn normalize_plugin_file_path(path: &str, field: &str) -> Result<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(Error::ValidationError {
            message: format!("{field} must not be empty"),
            details: None,
        });
    }

    let normalized = normalize_relative_components(trimmed, field)?;
    if normalized.is_empty() {
        return Err(Error::ValidationError {
            message: format!("{field} must not resolve to an empty path"),
            details: None,
        });
    }
    Ok(normalized)
}

pub(crate) fn normalize_namespace_path(path: &str, field: &str) -> Result<String> {
    let trimmed = path.trim();
    if !trimmed.starts_with('/') {
        return Err(Error::ValidationError {
            message: format!("{field} must start with '/' inside the plugin namespace"),
            details: None,
        });
    }
    if trimmed.starts_with("/admin") {
        return Err(Error::ValidationError {
            message: format!("{field} must stay inside the plugin namespace, got {trimmed:?}"),
            details: None,
        });
    }

    let stripped = trimmed.trim_start_matches('/');
    if stripped.is_empty() {
        return Ok("/".to_owned());
    }

    let normalized = normalize_relative_components(stripped, field)?;
    Ok(format!("/{normalized}"))
}

pub(crate) fn validate_permission_codes(codes: &[String], field: &str) -> Result<()> {
    for code in codes {
        let trimmed = code.trim();
        let segments: Vec<&str> = trimmed.split('.').collect();
        if segments.len() != 3 || segments.iter().any(|segment| segment.trim().is_empty()) {
            return Err(Error::ValidationError {
                message: format!("{field} contains invalid permission code {code:?}"),
                details: None,
            });
        }
    }
    Ok(())
}

pub(crate) fn validate_content_type_api_ids(ids: &[String], field: &str) -> Result<Vec<String>> {
    let mut normalized = Vec::with_capacity(ids.len());
    let mut seen = BTreeSet::new();
    for id in ids {
        let trimmed = id.trim();
        if trimmed.is_empty() {
            return Err(Error::ValidationError {
                message: format!("{field} contains an empty content type api id"),
                details: None,
            });
        }
        if !seen.insert(trimmed.to_owned()) {
            return Err(Error::ValidationError {
                message: format!("{field} contains duplicated content type api id {trimmed:?}"),
                details: None,
            });
        }
        normalized.push(trimmed.to_owned());
    }
    Ok(normalized)
}

pub(crate) fn validate_sdk_range(range: &str) -> Result<VersionReq> {
    VersionReq::parse(range).map_err(|source| Error::ValidationError {
        message: format!("frontend sdkVersion {range:?} is invalid: {source}"),
        details: None,
    })
}

fn normalize_relative_components(path: &str, field: &str) -> Result<String> {
    let mut segments = Vec::new();
    for component in Path::new(path).components() {
        match component {
            Component::Normal(segment) => {
                let segment = segment.to_str().ok_or_else(|| Error::ValidationError {
                    message: format!("{field} must be valid UTF-8"),
                    details: None,
                })?;
                if segment.is_empty() {
                    return Err(Error::ValidationError {
                        message: format!("{field} contains an empty path segment"),
                        details: None,
                    });
                }
                segments.push(segment.to_owned());
            }
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Err(Error::ValidationError {
                    message: format!("{field} must stay inside the plugin directory, got {path:?}"),
                    details: None,
                });
            }
        }
    }
    Ok(segments.join("/"))
}
