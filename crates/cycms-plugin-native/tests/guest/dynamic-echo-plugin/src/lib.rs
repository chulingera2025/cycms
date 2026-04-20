use async_trait::async_trait;
use cycms_core::{Error, Result};
use cycms_plugin_api::{Plugin, PluginContext};
use std::fs;
use std::path::PathBuf;

pub struct DynamicEchoPlugin;

fn marker_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("dynamic-plugin.marker")
}

#[async_trait]
impl Plugin for DynamicEchoPlugin {
    fn name(&self) -> &str {
        "dynamic-echo"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    async fn on_enable(&self, ctx: &PluginContext) -> Result<()> {
        let _ = ctx;
        fs::write(marker_path(), "enabled").map_err(|source| Error::Internal {
            message: "failed to write dynamic plugin marker".to_owned(),
            source: Some(Box::new(source)),
        })?;
        Ok(())
    }

    async fn on_disable(&self, ctx: &PluginContext) -> Result<()> {
        let _ = ctx;
        fs::write(marker_path(), "disabled").map_err(|source| Error::Internal {
            message: "failed to write dynamic plugin marker".to_owned(),
            source: Some(Box::new(source)),
        })?;
        Ok(())
    }
}

cycms_plugin_api::export_plugin!(DynamicEchoPlugin);