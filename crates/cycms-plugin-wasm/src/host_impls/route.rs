//! `route` host 组 —— 17.3 将记录到 `HostState.pending_routes`，供 17.4 合成 Router。

use crate::bindings::cycms::plugin::route::Host;
use crate::host::HostState;

const NOT_IMPL: &str = "route host: not yet implemented (task 17.3)";

impl Host for HostState {
    async fn register(
        &mut self,
        _path: String,
        _method: String,
    ) -> wasmtime::Result<Result<(), String>> {
        Ok(Err(NOT_IMPL.into()))
    }
}
