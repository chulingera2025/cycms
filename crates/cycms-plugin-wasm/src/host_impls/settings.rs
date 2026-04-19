//! `settings` host 组 —— 17.3 将用 `SettingsManager` 实现。

use crate::bindings::cycms::plugin::settings::Host;
use crate::host::HostState;

const NOT_IMPL: &str = "settings host: not yet implemented (task 17.3)";

impl Host for HostState {
    async fn get(&mut self, _key: String) -> wasmtime::Result<Result<String, String>> {
        Ok(Err(NOT_IMPL.into()))
    }

    async fn set(
        &mut self,
        _key: String,
        _value_json: String,
    ) -> wasmtime::Result<Result<(), String>> {
        Ok(Err(NOT_IMPL.into()))
    }

    async fn delete(&mut self, _key: String) -> wasmtime::Result<Result<(), String>> {
        Ok(Err(NOT_IMPL.into()))
    }
}
