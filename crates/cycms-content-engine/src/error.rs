use cycms_core::Error;

/// `cycms-content-engine` 领域错误，跨 crate 边界统一映射到 [`cycms_core::Error`]。
#[derive(Debug, thiserror::Error)]
pub enum ContentEngineError {
    #[error("content type `{0}` not found")]
    ContentTypeNotFound(String),

    #[error("content entry `{0}` not found")]
    EntryNotFound(String),

    #[error("content type `{0}` of kind `single` already has an entry")]
    SingleKindAlreadyExists(String),

    #[error("entry data must be a JSON object")]
    InvalidEntryShape,

    #[error("invalid query: {0}")]
    InvalidQuery(String),

    #[error("populate depth {depth} exceeds max {max}")]
    PopulateDepthExceeded { depth: u32, max: u32 },

    #[error("entry `{entry_id}` is still referenced by other entries")]
    ReferentialIntegrity {
        entry_id: String,
        violations: Vec<ReferenceViolation>,
    },

    #[error("database error")]
    Database(#[source] sqlx::Error),

    #[error("json codec error")]
    Json(#[source] serde_json::Error),
}

/// 单条反向引用明细（删除前由 [`ContentEngineError::ReferentialIntegrity`] 汇总返回）。
#[derive(Debug, Clone)]
pub struct ReferenceViolation {
    pub source_entry_id: String,
    pub field_api_id: String,
    pub relation_kind: String,
}

impl From<ContentEngineError> for Error {
    fn from(value: ContentEngineError) -> Self {
        match value {
            ContentEngineError::ContentTypeNotFound(api_id) => Self::NotFound {
                message: format!("content type `{api_id}` not found"),
            },
            ContentEngineError::EntryNotFound(id) => Self::NotFound {
                message: format!("content entry `{id}` not found"),
            },
            ContentEngineError::SingleKindAlreadyExists(api_id) => Self::Conflict {
                message: format!("content type `{api_id}` of kind `single` already has an entry"),
            },
            ContentEngineError::InvalidEntryShape => Self::ValidationError {
                message: "entry data must be a JSON object".to_owned(),
                details: None,
            },
            ContentEngineError::InvalidQuery(message) => Self::BadRequest {
                message,
                source: None,
            },
            ContentEngineError::PopulateDepthExceeded { depth, max } => Self::BadRequest {
                message: format!("populate depth {depth} exceeds max {max}"),
                source: None,
            },
            ContentEngineError::ReferentialIntegrity {
                entry_id,
                violations,
            } => Self::Conflict {
                message: format!(
                    "content entry `{entry_id}` is still referenced by {} relation(s)",
                    violations.len()
                ),
            },
            ContentEngineError::Database(source) => Self::Internal {
                message: "database operation failed".to_owned(),
                source: Some(Box::new(source)),
            },
            ContentEngineError::Json(source) => Self::Internal {
                message: "json encode/decode failed".to_owned(),
                source: Some(Box::new(source)),
            },
        }
    }
}
