use cycms_core::Error;

/// `cycms-auth` 领域错误，跨 crate 边界统一映射到 [`cycms_core::Error`]。
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("account is disabled")]
    AccountDisabled,

    #[error("token expired")]
    TokenExpired,

    #[error("token is invalid")]
    TokenInvalid,

    #[error("token type mismatch")]
    TokenTypeMismatch,

    #[error("token has been revoked")]
    TokenRevoked,

    #[error("password policy violation: {0}")]
    PasswordPolicy(String),

    #[error("input validation error: {0}")]
    InputValidation(String),

    #[error("username or email already exists")]
    UserAlreadyExists,

    #[error("initial admin already exists")]
    AdminAlreadyExists,

    #[error("password hash error: {0}")]
    PasswordHash(String),

    #[error("jwt error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("database error")]
    Database(#[source] sqlx::Error),
}

impl From<AuthError> for Error {
    fn from(value: AuthError) -> Self {
        match value {
            AuthError::InvalidCredentials
            | AuthError::AccountDisabled
            | AuthError::TokenExpired
            | AuthError::TokenInvalid
            | AuthError::TokenTypeMismatch
            | AuthError::TokenRevoked => Self::Unauthorized {
                message: "invalid credentials".to_owned(),
            },
            AuthError::PasswordPolicy(message) | AuthError::InputValidation(message) => {
                Self::ValidationError {
                    message,
                    details: None,
                }
            }
            AuthError::UserAlreadyExists => Self::Conflict {
                message: "username or email already exists".to_owned(),
            },
            AuthError::AdminAlreadyExists => Self::Conflict {
                message: "initial admin already exists".to_owned(),
            },
            AuthError::PasswordHash(message) => Self::Internal {
                message,
                source: None,
            },
            AuthError::Jwt(source) => Self::Internal {
                message: "jwt codec failure".to_owned(),
                source: Some(Box::new(source)),
            },
            AuthError::Database(source) => Self::Internal {
                message: "database operation failed".to_owned(),
                source: Some(Box::new(source)),
            },
        }
    }
}
