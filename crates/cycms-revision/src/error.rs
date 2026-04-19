use thiserror::Error;

#[derive(Debug, Error)]
pub enum RevisionError {
    /// 目标 content entry 不存在。
    #[error("content entry not found: {0}")]
    EntryNotFound(String),

    /// 指定版本号不存在。
    #[error("revision not found: entry={entry_id} version={version_number}")]
    RevisionNotFound {
        entry_id: String,
        version_number: i64,
    },

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<RevisionError> for cycms_core::Error {
    fn from(e: RevisionError) -> Self {
        match e {
            RevisionError::EntryNotFound(id) => cycms_core::Error::NotFound {
                message: format!("content entry not found: {id}"),
            },
            RevisionError::RevisionNotFound {
                entry_id,
                version_number,
            } => cycms_core::Error::NotFound {
                message: format!("revision not found: entry={entry_id} version={version_number}"),
            },
            RevisionError::Database(e) => cycms_core::Error::Internal {
                message: e.to_string(),
                source: Some(Box::new(e)),
            },
        }
    }
}
