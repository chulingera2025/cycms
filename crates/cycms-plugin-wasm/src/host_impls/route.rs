//! `route` host 组：把 guest 声明的 (path, method) 暂存到 `HostState.pending_routes`，
//! 17.4 的 `load` 会读取并合成 axum Router，命中时回调 guest 的 `handle-http`。

use crate::bindings::cycms::plugin::route::Host;
use crate::host::HostState;

impl Host for HostState {
    async fn register(
        &mut self,
        path: String,
        method: String,
    ) -> wasmtime::Result<Result<(), String>> {
        if path.trim().is_empty() {
            return Ok(Err("route.register: path must not be empty".into()));
        }
        let upper_method = method.to_ascii_uppercase();
        if !matches!(
            upper_method.as_str(),
            "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS"
        ) {
            return Ok(Err(format!("route.register: unsupported method {method}")));
        }
        if !self
            .pending_routes
            .iter()
            .any(|(p, m)| p == &path && m == &upper_method)
        {
            self.pending_routes.push((path, upper_method));
        }
        Ok(Ok(()))
    }
}
