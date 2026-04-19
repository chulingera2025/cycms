//! `content` host 组 —— 17.3 将代理到 `ContentEngine` 的 CRUD/查询。

use crate::bindings::cycms::plugin::content::Host;
use crate::host::HostState;

const NOT_IMPL: &str = "content host: not yet implemented (task 17.3)";

impl Host for HostState {
    async fn get(
        &mut self,
        _type_api_id: String,
        _entry_id: String,
    ) -> wasmtime::Result<Result<String, String>> {
        Ok(Err(NOT_IMPL.into()))
    }

    async fn find(
        &mut self,
        _type_api_id: String,
        _query_json: String,
    ) -> wasmtime::Result<Result<String, String>> {
        Ok(Err(NOT_IMPL.into()))
    }

    async fn create(
        &mut self,
        _type_api_id: String,
        _payload_json: String,
    ) -> wasmtime::Result<Result<String, String>> {
        Ok(Err(NOT_IMPL.into()))
    }

    async fn update(
        &mut self,
        _type_api_id: String,
        _entry_id: String,
        _patch_json: String,
    ) -> wasmtime::Result<Result<String, String>> {
        Ok(Err(NOT_IMPL.into()))
    }

    async fn delete(
        &mut self,
        _type_api_id: String,
        _entry_id: String,
    ) -> wasmtime::Result<Result<(), String>> {
        Ok(Err(NOT_IMPL.into()))
    }
}
