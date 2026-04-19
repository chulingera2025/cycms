//! `auth` host 组 —— 17.3 将代理到 `AuthEngine::verify_access_token` 与用户查询。

use crate::bindings::cycms::plugin::auth::Host;
use crate::host::HostState;

const NOT_IMPL: &str = "auth host: not yet implemented (task 17.3)";

impl Host for HostState {
    async fn verify_access_token(
        &mut self,
        _token: String,
    ) -> wasmtime::Result<Result<String, String>> {
        Ok(Err(NOT_IMPL.into()))
    }

    async fn get_user(&mut self, _user_id: String) -> wasmtime::Result<Result<String, String>> {
        Ok(Err(NOT_IMPL.into()))
    }
}
