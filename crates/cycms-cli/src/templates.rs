use cycms_core::Result;
use cycms_plugin_manager::PluginManifest;

pub(crate) const DEFAULT_CONFIG_TEMPLATE: &str = include_str!("../../../cycms.toml");
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) struct RenderedPluginScaffold {
    pub cargo_toml: String,
    pub plugin_toml: String,
    pub lib_rs: String,
}

pub(crate) enum DependencyStyle {
    Workspace,
    Direct,
}

pub(crate) fn render_project_workspace(example_plugin_name: &str) -> String {
    format!(
        r#"[workspace]
resolver = "3"
members = [
    "plugins/{example_plugin_name}",
]

[workspace.package]
edition = "2024"
version = "{CURRENT_VERSION}"

[workspace.dependencies]
async-trait = "0.1"
cycms-core = "{CURRENT_VERSION}"
cycms-plugin-api = "{CURRENT_VERSION}"
"#
    )
}

pub(crate) fn render_native_plugin_scaffold(
    plugin_name: &str,
    dependency_style: &DependencyStyle,
) -> Result<RenderedPluginScaffold> {
    let plugin_toml = render_plugin_manifest(plugin_name);
    PluginManifest::from_toml_str(&plugin_toml).map_err(cycms_core::Error::from)?;

    Ok(RenderedPluginScaffold {
        cargo_toml: render_plugin_cargo_toml(plugin_name, dependency_style),
        plugin_toml,
        lib_rs: render_plugin_lib_rs(plugin_name),
    })
}

fn render_plugin_manifest(plugin_name: &str) -> String {
    format!(
        r#"migrations = ["migrations"]

[plugin]
name = "{plugin_name}"
version = "0.1.0"
kind = "native"
entry = "{plugin_name}.so"
description = "{plugin_name} plugin for cycms"
license = "Apache-2.0"

[compatibility]
cycms = ">={CURRENT_VERSION}, <0.2.0"
"#
    )
}

fn render_plugin_cargo_toml(plugin_name: &str, dependency_style: &DependencyStyle) -> String {
    let dependency_block = match dependency_style {
        DependencyStyle::Workspace => {
            "async-trait = { workspace = true }\ncycms-core = { workspace = true }\ncycms-plugin-api = { workspace = true }".to_owned()
        }
        DependencyStyle::Direct => format!(
            "async-trait = \"0.1\"\ncycms-core = \"{CURRENT_VERSION}\"\ncycms-plugin-api = \"{CURRENT_VERSION}\""
        ),
    };

    format!(
        r#"[package]
name = "{plugin_name}"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
{dependency_block}
"#
    )
}

fn render_plugin_lib_rs(plugin_name: &str) -> String {
    let struct_name = pascal_case(plugin_name);
    format!(
        r#"use async_trait::async_trait;
use cycms_core::Result;
use cycms_plugin_api::{{Plugin, PluginContext}};

pub struct {struct_name};

#[async_trait]
impl Plugin for {struct_name} {{
    fn name(&self) -> &str {{
        "{plugin_name}"
    }}

    fn version(&self) -> &str {{
        "0.1.0"
    }}

    async fn on_enable(&self, _ctx: &PluginContext) -> Result<()> {{
        Ok(())
    }}

    async fn on_disable(&self, _ctx: &PluginContext) -> Result<()> {{
        Ok(())
    }}
}}

cycms_plugin_api::export_plugin!({struct_name});
"#
    )
}

fn pascal_case(value: &str) -> String {
    let mut out = String::new();
    for segment in value.split(['-', '_']) {
        if segment.is_empty() {
            continue;
        }
        let mut chars = segment.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            out.push_str(chars.as_str());
        }
    }

    if out.is_empty() {
        return "GeneratedPlugin".to_owned();
    }

    if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        return format!("Plugin{out}");
    }

    out
}
