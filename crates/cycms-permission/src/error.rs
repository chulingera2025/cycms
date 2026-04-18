use cycms_core::Error;

/// `cycms-permission` 领域错误，跨 crate 边界统一映射到 [`cycms_core::Error`]。
#[derive(Debug, thiserror::Error)]
pub enum PermissionError {
    #[error("input validation error: {0}")]
    InputValidation(String),

    #[error("role already exists")]
    RoleAlreadyExists,

    #[error("role not found")]
    RoleNotFound,

    #[error("permission not found")]
    PermissionNotFound,

    #[error("system role cannot be deleted")]
    SystemRoleUndeletable,

    #[error("permission denied")]
    Forbidden,

    #[error("database error")]
    Database(#[source] sqlx::Error),
}

impl From<PermissionError> for Error {
    fn from(value: PermissionError) -> Self {
        match value {
            PermissionError::InputValidation(message) => Self::ValidationError {
                message,
                details: None,
            },
            PermissionError::RoleAlreadyExists => Self::Conflict {
                message: "role already exists".to_owned(),
            },
            PermissionError::RoleNotFound => Self::NotFound {
                message: "role not found".to_owned(),
            },
            PermissionError::PermissionNotFound => Self::NotFound {
                message: "permission not found".to_owned(),
            },
            PermissionError::SystemRoleUndeletable => Self::Conflict {
                message: "system role cannot be deleted".to_owned(),
            },
            PermissionError::Forbidden => Self::Forbidden {
                message: "permission denied".to_owned(),
            },
            PermissionError::Database(source) => Self::Internal {
                message: "database operation failed".to_owned(),
                source: Some(Box::new(source)),
            },
        }
    }
}
