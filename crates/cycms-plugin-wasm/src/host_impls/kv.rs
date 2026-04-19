//! `kv` host 组 —— 17.3 将复用 `SettingsManager` 的 `plugin:<name>:kv` 命名空间实现。

use crate::bindings::cycms::plugin::kv::Host;
use crate::host::HostState;

const NOT_IMPL: &str = "kv host: not yet implemented (task 17.3)";

impl Host for HostState {
    async fn get(&mut self, _key: String) -> wasmtime::Result<Result<String, String>> {
        Ok(Err(NOT_IMPL.into()))
    }

    async fn set(
        &mut self,
        _key: String,
        _value: String,
    ) -> wasmtime::Result<Result<(), String>> {
        Ok(Err(NOT_IMPL.into()))
    }

    async fn delete(&mut self, _key: String) -> wasmtime::Result<Result<(), String>> {
        Ok(Err(NOT_IMPL.into()))
    }
}
