//! `db` host 组 —— 17.3 将直接暴露当前 `DatabasePool` 的原始 SQL 执行能力。

use crate::bindings::cycms::plugin::db::Host;
use crate::host::HostState;

const NOT_IMPL: &str = "db host: not yet implemented (task 17.3)";

impl Host for HostState {
    async fn query(
        &mut self,
        _sql: String,
        _params_json: String,
    ) -> wasmtime::Result<Result<String, String>> {
        Ok(Err(NOT_IMPL.into()))
    }

    async fn execute(
        &mut self,
        _sql: String,
        _params_json: String,
    ) -> wasmtime::Result<Result<u64, String>> {
        Ok(Err(NOT_IMPL.into()))
    }
}
