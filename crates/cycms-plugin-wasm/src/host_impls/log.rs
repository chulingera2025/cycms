use crate::bindings::cycms::plugin::log::Host;
use crate::host::HostState;

impl Host for HostState {
    async fn info(&mut self, message: String) -> wasmtime::Result<()> {
        tracing::info!(plugin = %self.plugin_name, "{}", message);
        Ok(())
    }

    async fn warn(&mut self, message: String) -> wasmtime::Result<()> {
        tracing::warn!(plugin = %self.plugin_name, "{}", message);
        Ok(())
    }

    async fn error(&mut self, message: String) -> wasmtime::Result<()> {
        tracing::error!(plugin = %self.plugin_name, "{}", message);
        Ok(())
    }
}
