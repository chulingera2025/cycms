//! `event` host 组 —— 17.3 将 publish 直通 `EventBus`，subscribe 登记到 `HostState`。

use crate::bindings::cycms::plugin::event::Host;
use crate::host::HostState;

const NOT_IMPL: &str = "event host: not yet implemented (task 17.3)";

impl Host for HostState {
    async fn publish(
        &mut self,
        _kind: String,
        _payload_json: String,
    ) -> wasmtime::Result<Result<(), String>> {
        Ok(Err(NOT_IMPL.into()))
    }

    async fn subscribe(&mut self, _kind: String) -> wasmtime::Result<Result<(), String>> {
        Ok(Err(NOT_IMPL.into()))
    }
}
