//! `kv` host 组：复用 `SettingsManager` 存储，namespace 为 `plugin:<name>:kv`。
//!
//! 值以 `serde_json::Value::String` 形式持久化，对 guest 表现为普通字符串，不做 JSON
//! 解析。完全信任模型下对插件私有 kv 完全开放读写。

use serde_json::Value;

use crate::bindings::cycms::plugin::kv::Host;
use crate::host::HostState;

impl HostState {
    fn kv_namespace(&self) -> String {
        format!("plugin:{}:kv", self.plugin_name)
    }
}

impl Host for HostState {
    async fn get(&mut self, key: String) -> wasmtime::Result<Result<String, String>> {
        let namespace = self.kv_namespace();
        match self.settings.get(&namespace, &key).await {
            Ok(Some(Value::String(s))) => Ok(Ok(s)),
            Ok(Some(v)) => Ok(Ok(v.to_string())),
            Ok(None) => Ok(Ok(String::new())),
            Err(e) => Ok(Err(format!("kv.get: {e}"))),
        }
    }

    async fn set(&mut self, key: String, value: String) -> wasmtime::Result<Result<(), String>> {
        let namespace = self.kv_namespace();
        match self
            .settings
            .set(&namespace, &key, Value::String(value))
            .await
        {
            Ok(_) => Ok(Ok(())),
            Err(e) => Ok(Err(format!("kv.set: {e}"))),
        }
    }

    async fn delete(&mut self, key: String) -> wasmtime::Result<Result<(), String>> {
        let namespace = self.kv_namespace();
        match self.settings.delete(&namespace, &key).await {
            Ok(_) => Ok(Ok(())),
            Err(e) => Ok(Err(format!("kv.delete: {e}"))),
        }
    }
}
