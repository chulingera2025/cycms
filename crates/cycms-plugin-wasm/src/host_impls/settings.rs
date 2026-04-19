//! `settings` host 组：代理到 `SettingsManager`，namespace 强制绑定当前插件名。
//!
//! 完全信任模型下不再做 namespace 隔离之外的额外约束——插件有完整的读写自身配置的
//! 能力。值以 `serde_json::Value` 持久化，`value-json` 入参必须为合法 JSON。

use serde_json::Value;

use crate::bindings::cycms::plugin::settings::Host;
use crate::host::HostState;

impl Host for HostState {
    async fn get(&mut self, key: String) -> wasmtime::Result<Result<String, String>> {
        match self.settings.get(&self.plugin_name, &key).await {
            Ok(Some(v)) => Ok(Ok(v.to_string())),
            Ok(None) => Ok(Ok(String::new())),
            Err(e) => Ok(Err(format!("settings.get: {e}"))),
        }
    }

    async fn set(
        &mut self,
        key: String,
        value_json: String,
    ) -> wasmtime::Result<Result<(), String>> {
        let value: Value = match serde_json::from_str(&value_json) {
            Ok(v) => v,
            Err(e) => return Ok(Err(format!("settings.set: invalid json: {e}"))),
        };
        match self.settings.set(&self.plugin_name, &key, value).await {
            Ok(_) => Ok(Ok(())),
            Err(e) => Ok(Err(format!("settings.set: {e}"))),
        }
    }

    async fn delete(&mut self, key: String) -> wasmtime::Result<Result<(), String>> {
        match self.settings.delete(&self.plugin_name, &key).await {
            Ok(_) => Ok(Ok(())),
            Err(e) => Ok(Err(format!("settings.delete: {e}"))),
        }
    }
}
