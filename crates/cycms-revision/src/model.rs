use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 内容版本快照（不可变）。
///
/// 每次 `ContentEngine::create` / `update` 或 `RevisionManager::rollback`
/// 成功后自动追加一行，`version_number` 在同一 entry 内从 1 单调递增。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Revision {
    pub id: String,
    pub content_entry_id: String,
    pub version_number: i64,
    /// 创建时刻的完整 fields 快照。
    pub snapshot: Value,
    pub change_summary: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

/// 创建版本快照所需的输入参数。
#[derive(Debug, Clone)]
pub struct CreateRevisionInput {
    pub content_entry_id: String,
    pub snapshot: Value,
    pub actor_id: String,
    pub change_summary: Option<String>,
}

/// 分页版本历史结果。
#[derive(Debug, Clone)]
pub struct PaginatedRevisions {
    pub data: Vec<Revision>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn revision_roundtrip() {
        let r = Revision {
            id: "rev-1".to_owned(),
            content_entry_id: "entry-1".to_owned(),
            version_number: 1,
            snapshot: json!({"title": "hello"}),
            change_summary: Some("initial".to_owned()),
            created_by: "user-1".to_owned(),
            created_at: DateTime::from_timestamp(0, 0).unwrap(),
        };
        let json = serde_json::to_string(&r).unwrap();
        let r2: Revision = serde_json::from_str(&json).unwrap();
        assert_eq!(r.id, r2.id);
        assert_eq!(r.version_number, r2.version_number);
        assert_eq!(r.snapshot, r2.snapshot);
    }
}
