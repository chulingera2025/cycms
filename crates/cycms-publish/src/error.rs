//! 发布状态机错误类型（任务 13）。

use cycms_content_engine::ContentStatus;

/// 发布子系统域内错误。
#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    #[error("content entry not found: {0}")]
    EntryNotFound(String),

    /// 状态转换不合法：如对非 Draft 实例执行 publish，或对非 Published 实例执行 unpublish。
    #[error("invalid publish transition for entry '{entry_id}': '{from}' → '{to}'")]
    InvalidTransition {
        entry_id: String,
        from: ContentStatus,
        to: ContentStatus,
    },

    #[error("database error: {0}")]
    Database(#[source] sqlx::Error),
}

impl From<PublishError> for cycms_core::Error {
    fn from(e: PublishError) -> Self {
        match e {
            PublishError::EntryNotFound(id) => cycms_core::Error::NotFound {
                message: format!("content entry not found: {id}"),
            },
            PublishError::InvalidTransition { entry_id, from, to } => cycms_core::Error::Conflict {
                message: format!("cannot transition entry '{entry_id}' from '{from}' to '{to}'"),
            },
            PublishError::Database(e) => cycms_core::Error::Internal {
                message: format!("publish db error: {e}"),
                source: None,
            },
        }
    }
}
