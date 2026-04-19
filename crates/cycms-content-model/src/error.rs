use cycms_core::Error;
use serde_json::json;

/// `cycms-content-model` 领域错误，跨 crate 边界统一映射到 [`cycms_core::Error`]。
#[derive(Debug, thiserror::Error)]
pub enum ContentModelError {
    #[error("input validation error: {0}")]
    InputValidation(String),

    #[error("content type not found: {0}")]
    NotFound(String),

    #[error("content type with api_id `{0}` already exists")]
    DuplicateApiId(String),

    #[error("invalid field definition: {0}")]
    InvalidField(String),

    #[error("schema violation")]
    SchemaViolation { errors: Vec<FieldViolation> },

    #[error("database error")]
    Database(#[source] sqlx::Error),

    #[error("json codec error")]
    Json(#[source] serde_json::Error),
}

/// 单条字段校验失败明细，`SchemaViolation.errors` 的成员。
#[derive(Debug, Clone)]
pub struct FieldViolation {
    pub field: String,
    pub rule: &'static str,
    pub message: String,
}

impl From<ContentModelError> for Error {
    fn from(value: ContentModelError) -> Self {
        match value {
            ContentModelError::InputValidation(message)
            | ContentModelError::InvalidField(message) => Self::ValidationError {
                message,
                details: None,
            },
            ContentModelError::DuplicateApiId(api_id) => Self::Conflict {
                message: format!("content type with api_id `{api_id}` already exists"),
            },
            ContentModelError::NotFound(api_id) => Self::NotFound {
                message: format!("content type `{api_id}` not found"),
            },
            ContentModelError::SchemaViolation { errors } => {
                let details = errors
                    .into_iter()
                    .map(|e| {
                        json!({
                            "field": e.field,
                            "rule": e.rule,
                            "message": e.message,
                        })
                    })
                    .collect::<Vec<_>>();
                Self::ValidationError {
                    message: "content field validation failed".to_owned(),
                    details: Some(json!(details)),
                }
            }
            ContentModelError::Database(source) => Self::Internal {
                message: "database operation failed".to_owned(),
                source: Some(Box::new(source)),
            },
            ContentModelError::Json(source) => Self::Internal {
                message: "json encode/decode failed".to_owned(),
                source: Some(Box::new(source)),
            },
        }
    }
}
