use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 一条系统 / 插件设置的对外视图。
///
/// `id` 跨方言以 UUID 字符串形式持有；`value` 始终解码为 `serde_json::Value`，
/// 三方言存储形态差异（PG `JSONB` / `MySQL` `JSON` / `SQLite` `TEXT + json_valid`）
/// 由 [`crate::repository`] 层内部透明处理。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingEntry {
    pub id: String,
    pub namespace: String,
    pub key: String,
    pub value: serde_json::Value,
    pub updated_at: DateTime<Utc>,
}

/// 一份插件设置 schema 的对外视图，`plugin_name` 作为主键。
///
/// 任务 8 阶段仅持久化 schema JSON；真正的 JSON Schema 校验将在 v0.2 接入
/// `jsonschema` crate 后补齐（见 [`crate::schema`] 中的 TODO）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSchema {
    pub plugin_name: String,
    pub schema: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
