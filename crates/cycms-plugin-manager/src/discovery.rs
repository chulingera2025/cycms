//! 插件目录扫描：`<root>/<plugin_name>/plugin.toml` 结构。
//!
//! - 根目录不存在 → 返回空列表（零插件是合法启动状态）
//! - 根目录存在但子条目非目录 → 跳过（兼容日志文件等干扰物）
//! - 子目录存在但缺 `plugin.toml` → 跳过（约定：不完整的目录被视为脚手架占位）
//! - `plugin.toml` 解析失败 → 返回 [`PluginManagerError::InvalidManifest`]

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::PluginManagerError;
use crate::manifest::PluginManifest;

/// 已扫描并解析通过的插件项：目录与 manifest 成对返回。
#[derive(Debug, Clone)]
pub struct DiscoveredPlugin {
    pub directory: PathBuf,
    pub manifest: PluginManifest,
}

/// 扫描 `plugins.directory` 下的所有子目录并解析 `plugin.toml`。
///
/// 解析通过的条目按 `manifest.plugin.name` 字典序返回。
///
/// # Errors
/// - I/O 错误（权限问题等）→ [`PluginManagerError::Discovery`]
/// - 单个 `plugin.toml` 解析失败 → [`PluginManagerError::InvalidManifest`]
pub fn scan_plugins_dir(root: &Path) -> Result<Vec<DiscoveredPlugin>, PluginManagerError> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let entries = fs::read_dir(root)
        .map_err(|e| PluginManagerError::Discovery(format!("read_dir {}: {e}", root.display())))?;

    let mut discovered = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| {
            PluginManagerError::Discovery(format!("read entry under {}: {e}", root.display()))
        })?;
        let file_type = entry
            .file_type()
            .map_err(|e| PluginManagerError::Discovery(format!("file_type: {e}")))?;
        if !file_type.is_dir() {
            continue;
        }
        let plugin_dir = entry.path();
        let manifest_path = plugin_dir.join("plugin.toml");
        if !manifest_path.exists() {
            continue;
        }
        let text = fs::read_to_string(&manifest_path).map_err(|e| {
            PluginManagerError::Discovery(format!("read {}: {e}", manifest_path.display()))
        })?;
        let manifest = PluginManifest::from_toml_str(&text)?;
        discovered.push(DiscoveredPlugin {
            directory: plugin_dir,
            manifest,
        });
    }

    discovered.sort_by(|a, b| a.manifest.plugin.name.cmp(&b.manifest.plugin.name));
    Ok(discovered)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    const MINIMAL_MANIFEST: &str = r#"
[plugin]
name = "%NAME%"
version = "0.1.0"
kind = "native"
entry = "entry.so"

[compatibility]
cycms = ">=0.1.0"
"#;

    fn write_plugin(root: &Path, name: &str) {
        let dir = root.join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("plugin.toml"),
            MINIMAL_MANIFEST.replace("%NAME%", name),
        )
        .unwrap();
    }

    #[test]
    fn missing_root_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("nope");
        assert!(scan_plugins_dir(&missing).unwrap().is_empty());
    }

    #[test]
    fn empty_root_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(scan_plugins_dir(tmp.path()).unwrap().is_empty());
    }

    #[test]
    fn scans_multiple_plugins_sorted() {
        let tmp = tempfile::tempdir().unwrap();
        write_plugin(tmp.path(), "zeta");
        write_plugin(tmp.path(), "alpha");
        write_plugin(tmp.path(), "mu");

        let found = scan_plugins_dir(tmp.path()).unwrap();
        let names: Vec<_> = found
            .iter()
            .map(|d| d.manifest.plugin.name.clone())
            .collect();
        assert_eq!(names, vec!["alpha", "mu", "zeta"]);
    }

    #[test]
    fn skips_non_directory_entries() {
        let tmp = tempfile::tempdir().unwrap();
        write_plugin(tmp.path(), "blog");
        fs::write(tmp.path().join("README.md"), "hi").unwrap();
        let found = scan_plugins_dir(tmp.path()).unwrap();
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn skips_dirs_without_manifest() {
        let tmp = tempfile::tempdir().unwrap();
        write_plugin(tmp.path(), "blog");
        fs::create_dir_all(tmp.path().join("scaffold")).unwrap();
        let found = scan_plugins_dir(tmp.path()).unwrap();
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn propagates_invalid_manifest_error() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("broken");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("plugin.toml"), "not = valid toml ===").unwrap();
        let err = scan_plugins_dir(tmp.path()).unwrap_err();
        assert!(matches!(err, PluginManagerError::InvalidManifest(_)));
    }
}
