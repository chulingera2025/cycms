use cycms_core::Error;

/// `cycms-settings` 领域错误，跨 crate 边界统一映射到 [`cycms_core::Error`]。
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("input validation error: {0}")]
    InputValidation(String),

    #[error("setting not found")]
    NotFound,

    #[error("plugin schema not found")]
    SchemaNotFound,

    #[error("database error")]
    Database(#[source] sqlx::Error),

    #[error("json codec error")]
    Json(#[source] serde_json::Error),
}

impl From<SettingsError> for Error {
    fn from(value: SettingsError) -> Self {
        match value {
            SettingsError::InputValidation(message) => Self::ValidationError {
                message,
                details: None,
            },
            SettingsError::NotFound => Self::NotFound {
                message: "setting not found".to_owned(),
            },
            SettingsError::SchemaNotFound => Self::NotFound {
                message: "plugin schema not found".to_owned(),
            },
            SettingsError::Database(source) => Self::Internal {
                message: "database operation failed".to_owned(),
                source: Some(Box::new(source)),
            },
            SettingsError::Json(source) => Self::Internal {
                message: "json encode/decode failed".to_owned(),
                source: Some(Box::new(source)),
            },
        }
    }
}
