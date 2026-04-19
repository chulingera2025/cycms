use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::ContentEngineError;

/// 内容实例状态。序列化为 `snake_case`，与 `content_entries.status` 列存储的字符串一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentStatus {
    Draft,
    Published,
    Archived,
}

impl ContentStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Published => "published",
            Self::Archived => "archived",
        }
    }
}

impl std::fmt::Display for ContentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ContentStatus {
    type Err = ContentEngineError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(Self::Draft),
            "published" => Ok(Self::Published),
            "archived" => Ok(Self::Archived),
            other => Err(ContentEngineError::InvalidQuery(format!(
                "invalid content status `{other}`"
            ))),
        }
    }
}

/// 内容实例视图。`id` / `content_type_id` / `created_by` / `updated_by` 跨方言
/// 均以 UUID v4 字符串形式持有，对齐 `cycms-content-model` 的做法。
///
/// `content_type_api_id` 不落盘到 `content_entries`，由 service 层在读取后
/// 从关联的 `content_types.api_id` 反查补齐，便于上层路由与事件 payload 使用。
///
/// `populated` 仅在调用方传入 `populate` 列表后才被填充，按 `field_api_id` 分组
/// 给出对应 Relation 字段的目标 entries。v0.1 仅支持单层加载（嵌套 entries
/// 自身的 `populated` 不会再展开）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentEntry {
    pub id: String,
    pub content_type_id: String,
    pub content_type_api_id: String,
    pub slug: Option<String>,
    pub status: ContentStatus,
    pub current_version_id: Option<String>,
    pub published_version_id: Option<String>,
    pub fields: Value,
    pub created_by: String,
    pub updated_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub populated: Option<HashMap<String, Vec<ContentEntry>>>,
}

/// 创建内容实例的入参。
#[derive(Debug, Clone)]
pub struct CreateEntryInput {
    pub content_type_api_id: String,
    pub data: Value,
    pub slug: Option<String>,
    pub actor_id: String,
}

/// 更新内容实例的入参。
///
/// `slug` 使用 `Option<Option<String>>` 三态：
/// - `None`：保留原值；
/// - `Some(None)`：清空为 NULL；
/// - `Some(Some(s))`：替换为新值。
#[derive(Debug, Clone, Default)]
pub struct UpdateEntryInput {
    pub data: Option<Value>,
    pub slug: Option<Option<String>>,
    pub actor_id: String,
}

/// 分页元信息。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PaginationMeta {
    pub page: u64,
    pub page_size: u64,
    pub page_count: u64,
    pub total: u64,
}

/// 列表响应：`data` + `meta`。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub meta: PaginationMeta,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use serde_json::json;

    use super::{ContentEntry, ContentStatus, PaginatedResponse, PaginationMeta};

    #[test]
    fn content_status_serde_snake_case() {
        assert_eq!(
            serde_json::to_value(ContentStatus::Draft).unwrap(),
            json!("draft")
        );
        assert_eq!(
            serde_json::to_value(ContentStatus::Published).unwrap(),
            json!("published")
        );
        assert_eq!(
            serde_json::to_value(ContentStatus::Archived).unwrap(),
            json!("archived")
        );

        let parsed: ContentStatus = serde_json::from_value(json!("archived")).unwrap();
        assert_eq!(parsed, ContentStatus::Archived);
    }

    #[test]
    fn content_status_from_str_covers_all_variants() {
        assert_eq!(
            ContentStatus::from_str("draft").unwrap(),
            ContentStatus::Draft
        );
        assert_eq!(
            ContentStatus::from_str("published").unwrap(),
            ContentStatus::Published
        );
        assert_eq!(
            ContentStatus::from_str("archived").unwrap(),
            ContentStatus::Archived
        );
        assert!(ContentStatus::from_str("other").is_err());
    }

    #[test]
    fn content_status_display_matches_as_str() {
        assert_eq!(format!("{}", ContentStatus::Draft), "draft");
        assert_eq!(ContentStatus::Published.as_str(), "published");
    }

    #[test]
    fn content_entry_roundtrip() {
        let created_at = chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap();
        let updated_at = chrono::DateTime::from_timestamp(1_700_000_500, 0).unwrap();
        let entry = ContentEntry {
            id: "00000000-0000-0000-0000-000000000001".to_owned(),
            content_type_id: "00000000-0000-0000-0000-000000000010".to_owned(),
            content_type_api_id: "post".to_owned(),
            slug: Some("hello-world".to_owned()),
            status: ContentStatus::Draft,
            current_version_id: None,
            published_version_id: None,
            fields: json!({ "title": "Hello", "views": 3 }),
            created_by: "00000000-0000-0000-0000-000000000002".to_owned(),
            updated_by: "00000000-0000-0000-0000-000000000002".to_owned(),
            created_at,
            updated_at,
            published_at: None,
            populated: None,
        };
        let v = serde_json::to_value(&entry).unwrap();
        assert_eq!(v["status"], "draft");
        assert_eq!(v["fields"]["title"], "Hello");
        let back: ContentEntry = serde_json::from_value(v).unwrap();
        assert_eq!(back.id, entry.id);
        assert_eq!(back.fields, entry.fields);
        assert_eq!(back.status, entry.status);
        assert_eq!(back.content_type_api_id, "post");
    }

    #[test]
    fn paginated_response_roundtrip() {
        let resp = PaginatedResponse {
            data: vec![1_u32, 2, 3],
            meta: PaginationMeta {
                page: 1,
                page_size: 3,
                page_count: 1,
                total: 3,
            },
        };
        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["meta"]["total"], 3);
        assert_eq!(v["data"], json!([1, 2, 3]));
        let back: PaginatedResponse<u32> = serde_json::from_value(v).unwrap();
        assert_eq!(back.data, resp.data);
        assert_eq!(back.meta.total, 3);
    }
}
