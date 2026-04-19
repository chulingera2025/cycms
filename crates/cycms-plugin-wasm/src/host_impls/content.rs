//! `content` host 组：代理到 `ContentEngine` 的 CRUD / 查询。
//!
//! 参数与返回均为 JSON 字符串：
//! - create: `{ "data": {...}, "slug": "...", "actor_id": "..." }`；`actor_id` 省略时
//!   默认为 `plugin:<name>`。返回 `ContentEntry` JSON。
//! - update: `{ "data": {...}, "slug": null | "...", "actor_id": "..." }`；`slug`
//!   采用三态：缺省字段保留原值，`null` 清空，字符串替换。
//! - delete: v0.1 始终走 `actor_id = plugin:<name>`，`mode` 使用 `ContentConfig` 默认。
//! - find: 简化版 [`ContentQuery`]（`page / page_size / status / populate`），filters
//!   与 sort 由后续任务扩展。

#![allow(clippy::option_option)]

use cycms_content_engine::{
    ContentQuery, ContentStatus, CreateEntryInput, UpdateEntryInput,
};
use serde::Deserialize;
use serde_json::Value;

use crate::bindings::cycms::plugin::content::Host;
use crate::host::HostState;

#[derive(Deserialize)]
struct CreatePayload {
    data: Value,
    #[serde(default)]
    slug: Option<String>,
    #[serde(default)]
    actor_id: Option<String>,
}

#[derive(Deserialize)]
struct UpdatePayload {
    #[serde(default)]
    data: Option<Value>,
    #[serde(default, deserialize_with = "deserialize_slug")]
    slug: Option<Option<String>>,
    #[serde(default)]
    actor_id: Option<String>,
}

fn deserialize_slug<'de, D>(d: D) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = Option::<Option<String>>::deserialize(d)?;
    Ok(v)
}

#[derive(Deserialize, Default)]
struct QueryDto {
    #[serde(default)]
    page: Option<u64>,
    #[serde(default)]
    page_size: Option<u64>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    populate: Vec<String>,
}

fn to_content_query(dto: QueryDto) -> Result<ContentQuery, String> {
    let status = match dto.status.as_deref() {
        None | Some("") => None,
        Some(s) => Some(s.parse::<ContentStatus>().map_err(|e| e.to_string())?),
    };
    Ok(ContentQuery {
        page: dto.page,
        page_size: dto.page_size,
        sort: Vec::new(),
        filters: Vec::new(),
        status,
        populate: dto.populate,
    })
}

impl HostState {
    fn default_actor(&self) -> String {
        format!("plugin:{}", self.plugin_name)
    }
}

impl Host for HostState {
    async fn get(
        &mut self,
        type_api_id: String,
        entry_id: String,
    ) -> wasmtime::Result<Result<String, String>> {
        match self.content.get(&type_api_id, &entry_id, &[]).await {
            Ok(Some(entry)) => match serde_json::to_string(&entry) {
                Ok(s) => Ok(Ok(s)),
                Err(e) => Ok(Err(format!("content.get: serialize: {e}"))),
            },
            Ok(None) => Ok(Err(format!("content.get: entry {entry_id} not found"))),
            Err(e) => Ok(Err(format!("content.get: {e}"))),
        }
    }

    async fn find(
        &mut self,
        type_api_id: String,
        query_json: String,
    ) -> wasmtime::Result<Result<String, String>> {
        let dto: QueryDto = if query_json.trim().is_empty() {
            QueryDto::default()
        } else {
            match serde_json::from_str(&query_json) {
                Ok(d) => d,
                Err(e) => return Ok(Err(format!("content.find: invalid query json: {e}"))),
            }
        };
        let query = match to_content_query(dto) {
            Ok(q) => q,
            Err(e) => return Ok(Err(format!("content.find: {e}"))),
        };
        match self.content.list(&type_api_id, &query).await {
            Ok(page) => match serde_json::to_string(&page) {
                Ok(s) => Ok(Ok(s)),
                Err(e) => Ok(Err(format!("content.find: serialize: {e}"))),
            },
            Err(e) => Ok(Err(format!("content.find: {e}"))),
        }
    }

    async fn create(
        &mut self,
        type_api_id: String,
        payload_json: String,
    ) -> wasmtime::Result<Result<String, String>> {
        let payload: CreatePayload = match serde_json::from_str(&payload_json) {
            Ok(p) => p,
            Err(e) => return Ok(Err(format!("content.create: invalid payload json: {e}"))),
        };
        let actor_id = payload.actor_id.unwrap_or_else(|| self.default_actor());
        let input = CreateEntryInput {
            content_type_api_id: type_api_id,
            data: payload.data,
            slug: payload.slug,
            actor_id,
        };
        match self.content.create(input).await {
            Ok(entry) => match serde_json::to_string(&entry) {
                Ok(s) => Ok(Ok(s)),
                Err(e) => Ok(Err(format!("content.create: serialize: {e}"))),
            },
            Err(e) => Ok(Err(format!("content.create: {e}"))),
        }
    }

    async fn update(
        &mut self,
        type_api_id: String,
        entry_id: String,
        patch_json: String,
    ) -> wasmtime::Result<Result<String, String>> {
        let patch: UpdatePayload = match serde_json::from_str(&patch_json) {
            Ok(p) => p,
            Err(e) => return Ok(Err(format!("content.update: invalid patch json: {e}"))),
        };
        let actor_id = patch.actor_id.unwrap_or_else(|| self.default_actor());
        let input = UpdateEntryInput {
            data: patch.data,
            slug: patch.slug,
            actor_id,
        };
        match self.content.update(&type_api_id, &entry_id, input).await {
            Ok(entry) => match serde_json::to_string(&entry) {
                Ok(s) => Ok(Ok(s)),
                Err(e) => Ok(Err(format!("content.update: serialize: {e}"))),
            },
            Err(e) => Ok(Err(format!("content.update: {e}"))),
        }
    }

    async fn delete(
        &mut self,
        type_api_id: String,
        entry_id: String,
    ) -> wasmtime::Result<Result<(), String>> {
        let actor = self.default_actor();
        match self.content.delete(&type_api_id, &entry_id, None, &actor).await {
            Ok(()) => Ok(Ok(())),
            Err(e) => Ok(Err(format!("content.delete: {e}"))),
        }
    }
}
