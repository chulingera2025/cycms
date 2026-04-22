use std::collections::BTreeMap;
use std::path::Path;

use cycms_core::{Error, Result};
use cycms_host_types::{
    AdminPageMode, AdminPageRegistration, AssetBundleRegistration, CompatibilityKind,
    CompatibilityRegistration, CompiledExtensionRegistry, CompiledPluginDescriptor,
    EditorRegistration, HookRegistration, OwnershipMode, ParserRegistration,
    PublicPageRegistration, RegistrationOriginKind, RegistrationSource,
};

use crate::frontend_manifest::load_frontend_manifest;
use crate::frontend_snapshot::{build_frontend_runtime_state, plugin_admin_full_path};
use crate::manifest::{
    AdminPageSpec, CompatibilityBridgeSpec, HostManifestSpec, PluginKind, PluginManifest,
    PublicPageSpec,
};
use crate::{DiscoveredPlugin, scan_plugins_dir};

pub fn compile_extensions(plugins_root: &Path) -> Result<CompiledExtensionRegistry> {
    let discovered = scan_plugins_dir(plugins_root).map_err(Error::from)?;
    let mut compiler = RegistryCompiler::default();

    for plugin in discovered {
        compiler.ingest_plugin(plugin)?;
    }

    Ok(compiler.finish())
}

#[derive(Default)]
struct RegistryCompiler {
    registry: CompiledExtensionRegistry,
    declaration_order: usize,
}

impl RegistryCompiler {
    fn ingest_plugin(&mut self, plugin: DiscoveredPlugin) -> Result<()> {
        let manifest = &plugin.manifest;
        let has_host_manifest = manifest.host.as_ref().is_some_and(|host| !host.is_empty());
        let has_frontend_manifest = manifest.frontend.is_some();

        self.registry.plugins.push(CompiledPluginDescriptor {
            name: manifest.plugin.name.clone(),
            version: manifest.plugin.version.clone(),
            plugin_kind: manifest.plugin.kind.as_str().to_owned(),
            has_host_manifest,
            has_frontend_manifest,
        });

        if let Some(host) = &manifest.host {
            self.ingest_host_manifest(host, manifest);
        }

        self.ingest_runtime_compatibility(manifest);

        if let Some(spec) = &manifest.frontend {
            let frontend_manifest = load_frontend_manifest(&plugin.directory, spec)?;
            let runtime_state =
                build_frontend_runtime_state(&plugin.directory, manifest, frontend_manifest)?;
            self.ingest_frontend_compatibility(manifest, &runtime_state);
        }

        Ok(())
    }

    fn finish(mut self) -> CompiledExtensionRegistry {
        self.registry
            .plugins
            .sort_by(|left, right| left.name.cmp(&right.name));
        self.registry.public_pages.sort_by(compare_public_page);
        self.registry.admin_pages.sort_by(compare_admin_page);
        self.registry.parsers.sort_by(compare_parser);
        self.registry.hooks.sort_by(compare_hook);
        self.registry.assets.sort_by(compare_asset);
        self.registry.editors.sort_by(compare_editor);
        self.registry.compatibility.sort_by(compare_compatibility);
        self.registry
    }

    fn ingest_host_manifest(&mut self, host: &HostManifestSpec, manifest: &PluginManifest) {
        for asset in &host.assets {
            let registration = AssetBundleRegistration {
                id: asset.id.clone(),
                source: self.next_source(manifest, RegistrationOriginKind::HostManifest),
                apply_to: asset.apply_to.clone(),
                modules: asset.modules.clone(),
                scripts: asset.scripts.clone(),
                styles: asset.styles.clone(),
                inline_data_keys: asset.inline_data_keys.clone(),
            };
            self.registry.assets.push(registration);
        }

        for page in &host.public_pages {
            let registration = self.build_public_page(page, manifest);
            self.registry.public_pages.push(registration);
        }

        for page in &host.admin_pages {
            let registration = self.build_admin_page(page, manifest);
            self.registry.admin_pages.push(registration);
        }

        for parser in &host.parsers {
            let registration = ParserRegistration {
                id: parser.id.clone(),
                source: self.next_source(manifest, RegistrationOriginKind::HostManifest),
                priority: parser.priority,
                ownership: parser.ownership,
                parser: parser.parser.clone(),
                content_types: parser.content_types.clone(),
                field_names: parser.field_names.clone(),
                source_formats: parser.source_formats.clone(),
            };
            self.registry.parsers.push(registration);
        }

        for hook in &host.hooks {
            let registration = HookRegistration {
                id: hook.id.clone(),
                source: self.next_source(manifest, RegistrationOriginKind::HostManifest),
                priority: hook.priority,
                ownership: hook.ownership,
                phase: hook.phase.clone(),
                handler: hook.handler.clone(),
            };
            self.registry.hooks.push(registration);
        }

        for editor in &host.editors {
            let registration = EditorRegistration {
                id: editor.id.clone(),
                source: self.next_source(manifest, RegistrationOriginKind::HostManifest),
                priority: editor.priority,
                ownership: editor.ownership,
                editor: editor.editor.clone(),
                content_types: editor.content_types.clone(),
                field_types: editor.field_types.clone(),
                screen_targets: editor.screen_targets.clone(),
                asset_bundle_ids: editor.asset_bundle_ids.clone(),
            };
            self.registry.editors.push(registration);
        }

        for item in &host.compatibility {
            let registration = self.build_compatibility_bridge(item, manifest);
            self.registry.compatibility.push(registration);
        }
    }

    fn ingest_runtime_compatibility(&mut self, manifest: &PluginManifest) {
        let kind = match manifest.plugin.kind {
            PluginKind::Native => CompatibilityKind::DynamicNativePlugin,
            PluginKind::Wasm => CompatibilityKind::DynamicWasmPlugin,
        };
        let mut metadata = BTreeMap::new();
        metadata.insert("entry".to_owned(), manifest.plugin.entry.clone());
        metadata.insert(
            "host_manifest_present".to_owned(),
            manifest
                .host
                .as_ref()
                .is_some_and(|host| !host.is_empty())
                .to_string(),
        );

        let registration = CompatibilityRegistration {
            id: format!("{}.runtime", manifest.plugin.name),
            source: self.next_source(manifest, RegistrationOriginKind::DynamicRuntime),
            kind,
            target: manifest.plugin.name.clone(),
            metadata,
        };
        self.registry.compatibility.push(registration);
    }

    fn ingest_frontend_compatibility(
        &mut self,
        manifest: &PluginManifest,
        runtime_state: &crate::FrontendRuntimeState,
    ) {
        let mut shared_metadata = BTreeMap::new();
        shared_metadata.insert(
            "frontend_compatible".to_owned(),
            runtime_state.compatibility.compatible.to_string(),
        );
        shared_metadata.insert(
            "frontend_required".to_owned(),
            runtime_state.required.to_string(),
        );

        for asset in &runtime_state.snapshot.assets {
            let mut metadata = shared_metadata.clone();
            metadata.insert("module_path".to_owned(), asset.module_path.clone());
            metadata.insert(
                "style_count".to_owned(),
                asset.style_paths.len().to_string(),
            );
            let compatibility = CompatibilityRegistration {
                id: format!("{}.frontend.asset.{}", manifest.plugin.name, asset.id),
                source: self.next_source(manifest, RegistrationOriginKind::FrontendCompatibility),
                kind: CompatibilityKind::AdminExtensionAssetBundle,
                target: asset.id.clone(),
                metadata,
            };
            self.registry.compatibility.push(compatibility);

            if runtime_state.compatibility.compatible {
                let registration = AssetBundleRegistration {
                    id: compat_asset_bundle_id(&manifest.plugin.name, &asset.id),
                    source: self
                        .next_source(manifest, RegistrationOriginKind::FrontendCompatibility),
                    apply_to: vec!["admin_extension".to_owned()],
                    modules: vec![asset.module_path.clone()],
                    scripts: Vec::new(),
                    styles: asset.style_paths.clone(),
                    inline_data_keys: Vec::new(),
                };
                self.registry.assets.push(registration);
            }
        }

        for menu in &runtime_state.snapshot.menus {
            let mut metadata = shared_metadata.clone();
            metadata.insert("zone".to_owned(), menu.zone.clone());
            metadata.insert("to".to_owned(), menu.to.clone());
            let compatibility = CompatibilityRegistration {
                id: format!("{}.frontend.menu.{}", manifest.plugin.name, menu.id),
                source: self.next_source(manifest, RegistrationOriginKind::FrontendCompatibility),
                kind: CompatibilityKind::AdminExtensionMenu,
                target: menu.to.clone(),
                metadata,
            };
            self.registry.compatibility.push(compatibility);
        }

        for route in &runtime_state.snapshot.routes {
            let full_path = plugin_admin_full_path(&manifest.plugin.name, &route.path);
            let mut metadata = shared_metadata.clone();
            metadata.insert("full_path".to_owned(), full_path.clone());
            metadata.insert("module_asset_id".to_owned(), route.module_asset_id.clone());
            let compatibility = CompatibilityRegistration {
                id: format!("{}.frontend.route.{}", manifest.plugin.name, route.id),
                source: self.next_source(manifest, RegistrationOriginKind::FrontendCompatibility),
                kind: CompatibilityKind::AdminExtensionRoute,
                target: full_path.clone(),
                metadata,
            };
            self.registry.compatibility.push(compatibility);

            if runtime_state.compatibility.compatible {
                let registration = AdminPageRegistration {
                    id: format!("compat.{}.route.{}", manifest.plugin.name, route.id),
                    source: self
                        .next_source(manifest, RegistrationOriginKind::FrontendCompatibility),
                    path: full_path,
                    title: route.title.clone(),
                    mode: AdminPageMode::Compatibility,
                    priority: -100,
                    ownership: OwnershipMode::Replace,
                    handler: format!("frontend.route:{}", route.id),
                    menu_label: None,
                    menu_zone: None,
                    asset_bundle_ids: vec![compat_asset_bundle_id(
                        &manifest.plugin.name,
                        &route.module_asset_id,
                    )],
                };
                self.registry.admin_pages.push(registration);
            }
        }

        for slot in &runtime_state.snapshot.slots {
            let mut metadata = shared_metadata.clone();
            metadata.insert("slot".to_owned(), slot.slot.clone());
            metadata.insert("module_asset_id".to_owned(), slot.module_asset_id.clone());
            let compatibility = CompatibilityRegistration {
                id: format!("{}.frontend.slot.{}", manifest.plugin.name, slot.id),
                source: self.next_source(manifest, RegistrationOriginKind::FrontendCompatibility),
                kind: CompatibilityKind::AdminExtensionSlot,
                target: slot.slot.clone(),
                metadata,
            };
            self.registry.compatibility.push(compatibility);
        }

        for renderer in &runtime_state.snapshot.field_renderers {
            let mut metadata = shared_metadata.clone();
            metadata.insert("field_type".to_owned(), renderer.type_name.clone());
            metadata.insert(
                "module_asset_id".to_owned(),
                renderer.module_asset_id.clone(),
            );
            let compatibility = CompatibilityRegistration {
                id: format!(
                    "{}.frontend.field_renderer.{}",
                    manifest.plugin.name, renderer.id
                ),
                source: self.next_source(manifest, RegistrationOriginKind::FrontendCompatibility),
                kind: CompatibilityKind::AdminExtensionFieldRenderer,
                target: renderer.type_name.clone(),
                metadata,
            };
            self.registry.compatibility.push(compatibility);

            if runtime_state.compatibility.compatible {
                let registration = EditorRegistration {
                    id: format!(
                        "compat.{}.field_renderer.{}",
                        manifest.plugin.name, renderer.id
                    ),
                    source: self
                        .next_source(manifest, RegistrationOriginKind::FrontendCompatibility),
                    priority: -100,
                    ownership: OwnershipMode::Replace,
                    editor: format!("frontend.field_renderer:{}", renderer.id),
                    content_types: Vec::new(),
                    field_types: vec![renderer.type_name.clone()],
                    screen_targets: vec!["admin:field_renderer".to_owned()],
                    asset_bundle_ids: vec![compat_asset_bundle_id(
                        &manifest.plugin.name,
                        &renderer.module_asset_id,
                    )],
                };
                self.registry.editors.push(registration);
            }
        }

        if let Some(settings) = &runtime_state.snapshot.settings {
            let mut metadata = shared_metadata.clone();
            metadata.insert("namespace".to_owned(), settings.namespace.clone());
            let compatibility = CompatibilityRegistration {
                id: format!(
                    "{}.frontend.settings.{}",
                    manifest.plugin.name, settings.namespace
                ),
                source: self.next_source(manifest, RegistrationOriginKind::FrontendCompatibility),
                kind: CompatibilityKind::AdminExtensionSettings,
                target: settings.namespace.clone(),
                metadata: metadata.clone(),
            };
            self.registry.compatibility.push(compatibility);

            if runtime_state.compatibility.compatible
                && let Some(custom_page) = &settings.custom_page
            {
                let registration = AdminPageRegistration {
                    id: format!(
                        "compat.{}.settings.{}",
                        manifest.plugin.name, settings.namespace
                    ),
                    source: self
                        .next_source(manifest, RegistrationOriginKind::FrontendCompatibility),
                    path: plugin_admin_full_path(&manifest.plugin.name, &custom_page.path),
                    title: format!("{} settings", settings.namespace),
                    mode: AdminPageMode::Compatibility,
                    priority: -100,
                    ownership: OwnershipMode::Replace,
                    handler: format!("frontend.settings:{}", settings.namespace),
                    menu_label: Some(format!("{} settings", settings.namespace)),
                    menu_zone: Some("settings".to_owned()),
                    asset_bundle_ids: vec![compat_asset_bundle_id(
                        &manifest.plugin.name,
                        &custom_page.module_asset_id,
                    )],
                };
                self.registry.admin_pages.push(registration);
            }
        }
    }

    fn build_public_page(
        &mut self,
        page: &PublicPageSpec,
        manifest: &PluginManifest,
    ) -> PublicPageRegistration {
        PublicPageRegistration {
            id: page.id.clone(),
            source: self.next_source(manifest, RegistrationOriginKind::HostManifest),
            path: page.path.clone(),
            priority: page.priority,
            ownership: page.ownership,
            handler: page.handler.clone(),
            title: page.title.clone(),
            asset_bundle_ids: page.asset_bundle_ids.clone(),
        }
    }

    fn build_admin_page(
        &mut self,
        page: &AdminPageSpec,
        manifest: &PluginManifest,
    ) -> AdminPageRegistration {
        AdminPageRegistration {
            id: page.id.clone(),
            source: self.next_source(manifest, RegistrationOriginKind::HostManifest),
            path: page.path.clone(),
            title: page.title.clone(),
            mode: page.mode,
            priority: page.priority,
            ownership: page.ownership,
            handler: page.handler.clone(),
            menu_label: page.menu_label.clone(),
            menu_zone: page.menu_zone.clone(),
            asset_bundle_ids: page.asset_bundle_ids.clone(),
        }
    }

    fn build_compatibility_bridge(
        &mut self,
        item: &CompatibilityBridgeSpec,
        manifest: &PluginManifest,
    ) -> CompatibilityRegistration {
        CompatibilityRegistration {
            id: item.id.clone(),
            source: self.next_source(manifest, RegistrationOriginKind::CompatibilityBridge),
            kind: item.kind,
            target: item.target.clone(),
            metadata: item.metadata.clone().into_iter().collect(),
        }
    }

    fn next_source(
        &mut self,
        manifest: &PluginManifest,
        origin: RegistrationOriginKind,
    ) -> RegistrationSource {
        let source = RegistrationSource {
            plugin_name: manifest.plugin.name.clone(),
            plugin_version: manifest.plugin.version.clone(),
            origin,
            declaration_order: self.declaration_order,
        };
        self.declaration_order += 1;
        source
    }
}

fn compat_asset_bundle_id(plugin_name: &str, asset_id: &str) -> String {
    format!("compat.{plugin_name}.asset.{asset_id}")
}

fn compare_public_page(
    left: &PublicPageRegistration,
    right: &PublicPageRegistration,
) -> std::cmp::Ordering {
    left.path
        .cmp(&right.path)
        .then(left.id.cmp(&right.id))
        .then(left.source.plugin_name.cmp(&right.source.plugin_name))
}

fn compare_admin_page(
    left: &AdminPageRegistration,
    right: &AdminPageRegistration,
) -> std::cmp::Ordering {
    left.path
        .cmp(&right.path)
        .then(left.id.cmp(&right.id))
        .then(left.source.plugin_name.cmp(&right.source.plugin_name))
}

fn compare_parser(left: &ParserRegistration, right: &ParserRegistration) -> std::cmp::Ordering {
    left.id
        .cmp(&right.id)
        .then(left.source.plugin_name.cmp(&right.source.plugin_name))
}

fn compare_hook(left: &HookRegistration, right: &HookRegistration) -> std::cmp::Ordering {
    left.phase
        .cmp(&right.phase)
        .then(left.id.cmp(&right.id))
        .then(left.source.plugin_name.cmp(&right.source.plugin_name))
}

fn compare_asset(
    left: &AssetBundleRegistration,
    right: &AssetBundleRegistration,
) -> std::cmp::Ordering {
    left.id
        .cmp(&right.id)
        .then(left.source.plugin_name.cmp(&right.source.plugin_name))
}

fn compare_editor(left: &EditorRegistration, right: &EditorRegistration) -> std::cmp::Ordering {
    left.id
        .cmp(&right.id)
        .then(left.source.plugin_name.cmp(&right.source.plugin_name))
}

fn compare_compatibility(
    left: &CompatibilityRegistration,
    right: &CompatibilityRegistration,
) -> std::cmp::Ordering {
    left.id
        .cmp(&right.id)
        .then(left.source.plugin_name.cmp(&right.source.plugin_name))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use base64::Engine as _;
    use sha2::Digest as _;

    use super::*;

    fn write_plugin(root: &Path, name: &str, plugin_toml: &str) {
        let plugin_dir = root.join(name);
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(plugin_dir.join("plugin.toml"), plugin_toml).unwrap();
    }

    fn write_frontend_files(root: &Path, name: &str) {
        let admin_dir = root.join(name).join("admin");
        fs::create_dir_all(&admin_dir).unwrap();
        fs::write(admin_dir.join("main.js"), "console.log('blog');").unwrap();
        fs::write(admin_dir.join("main.css"), ".blog-admin{}\n").unwrap();
        let sha384 = sha2::Sha384::digest("console.log('blog');".as_bytes());
        let manifest = format!(
            r#"{{
  "schemaVersion": 1,
  "sdkVersion": "^1.0.0",
  "pluginName": "{name}",
  "pluginVersion": "0.1.0",
  "assets": [
    {{
      "id": "admin-main",
      "path": "admin/main.js",
      "sha384": "{}",
      "contentType": "text/javascript",
      "styles": ["admin/main.css"]
    }}
  ],
  "routes": [
    {{
      "id": "root",
      "path": "/dashboard",
      "moduleAssetId": "admin-main",
      "kind": "island",
      "title": "Plugin Dashboard"
    }}
  ],
  "fieldRenderers": [
    {{
      "id": "custom-richtext",
      "typeName": "custom.richtext",
      "moduleAssetId": "admin-main"
    }}
  ]
}}"#,
            format!(
                "sha384-{}",
                base64::engine::general_purpose::STANDARD.encode(sha384)
            )
        );
        fs::write(admin_dir.join("manifest.json"), manifest).unwrap();
    }

    #[test]
    fn compile_extensions_is_deterministic_and_ingests_compatibility() {
        let temp = tempfile::tempdir().unwrap();
        write_plugin(
            temp.path(),
            "blog",
            r#"
[plugin]
name = "blog"
version = "0.1.0"
kind = "native"
entry = "blog.so"

[compatibility]
cycms = ">=0.1.0"

[frontend]
manifest = "admin/manifest.json"

[host]

[[host.assets]]
id = "blog-css"
styles = ["admin/blog.css"]

[[host.public_pages]]
id = "blog-home"
path = "/blog"
handler = "blog::public::home"
asset_bundle_ids = ["blog-css"]

[[host.admin_pages]]
id = "blog-admin"
path = "/admin/blog"
title = "Blog"
handler = "blog::admin::page"

[[host.parsers]]
id = "markdown"
parser = "blog::parse_markdown"
source_formats = ["markdown"]

[[host.hooks]]
id = "before-send"
phase = "before_send"
handler = "blog::before_send"

[[host.editors]]
id = "post-editor"
editor = "blog::editor"
content_types = ["post"]

[[host.compatibility]]
id = "legacy-public-api"
kind = "manifest_compatibility_bridge"
target = "/api/v1/public/blog"
"#,
        );
        write_frontend_files(temp.path(), "blog");
        fs::write(temp.path().join("blog").join("admin/blog.css"), ".css{}\n").unwrap();

        write_plugin(
            temp.path(),
            "shop",
            r#"
[plugin]
name = "shop"
version = "0.1.0"
kind = "wasm"
entry = "shop.wasm"

[compatibility]
cycms = ">=0.1.0"
"#,
        );

        let first = compile_extensions(temp.path()).unwrap();
        let second = compile_extensions(temp.path()).unwrap();

        assert_eq!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        );
        assert_eq!(first.plugins.len(), 2);
        assert_eq!(first.public_pages.len(), 1);
        assert_eq!(first.admin_pages.len(), 2);
        assert_eq!(first.parsers.len(), 1);
        assert_eq!(first.hooks.len(), 1);
        assert_eq!(first.assets.len(), 2);
        assert_eq!(first.editors.len(), 2);
        assert!(
            first
                .compatibility
                .iter()
                .any(|entry| entry.kind == CompatibilityKind::DynamicNativePlugin)
        );
        assert!(
            first
                .compatibility
                .iter()
                .any(|entry| entry.kind == CompatibilityKind::AdminExtensionRoute)
        );
        assert!(
            first
                .admin_pages
                .iter()
                .any(|page| page.path == "/admin/x/blog/dashboard"
                    && page.mode == AdminPageMode::Compatibility)
        );
    }
}
