//! `permission` host 组 —— 17.3 将代理到 `PermissionEngine::check`。

use crate::bindings::cycms::plugin::permission::Host;
use crate::host::HostState;

const NOT_IMPL: &str = "permission host: not yet implemented (task 17.3)";

impl Host for HostState {
    async fn check(
        &mut self,
        _user_id: String,
        _domain: String,
        _resource_name: String,
        _action: String,
    ) -> wasmtime::Result<Result<bool, String>> {
        Ok(Err(NOT_IMPL.into()))
    }
}
