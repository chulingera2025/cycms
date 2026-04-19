//! `http` host 组 —— 17.3 将用 reqwest 做无白名单出站 HTTP。

use crate::bindings::cycms::plugin::http::Host;
use crate::host::HostState;

const NOT_IMPL: &str = "http host: not yet implemented (task 17.3)";

impl Host for HostState {
    async fn fetch(&mut self, _request_json: String) -> wasmtime::Result<Result<String, String>> {
        Ok(Err(NOT_IMPL.into()))
    }
}
