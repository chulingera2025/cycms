use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use serde_json::Value;

/// 统一错误类型，对应 design.md §Error Code 规范的 11 个顶层类别。
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("bad_request: {message}")]
    BadRequest {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("validation_error: {message}")]
    ValidationError {
        message: String,
        /// 字段级错误数组，形如 [{field, rule, message}]
        details: Option<Value>,
    },

    #[error("unauthorized: {message}")]
    Unauthorized { message: String },

    #[error("forbidden: {message}")]
    Forbidden { message: String },

    #[error("not_found: {message}")]
    NotFound { message: String },

    #[error("conflict: {message}")]
    Conflict { message: String },

    #[error("rate_limited: {message}")]
    RateLimited { message: String },

    #[error("payload_too_large: {message}")]
    PayloadTooLarge { message: String },

    #[error("unsupported_media_type: {message}")]
    UnsupportedMediaType { message: String },

    #[error("plugin_error: {message}")]
    PluginError {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("internal_error: {message}")]
    Internal {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorBody {
    pub status: u16,
    pub name: &'static str,
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

/// 全局 Result 别名，默认错误类型为 [`Error`]。
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// [`Error`] 在 axum 中间件等场景下的语义别名。
pub use Error as AppError;

impl Error {
    #[must_use]
    pub const fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest { .. } => StatusCode::BAD_REQUEST,
            Self::ValidationError { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
            Self::Forbidden { .. } => StatusCode::FORBIDDEN,
            Self::NotFound { .. } => StatusCode::NOT_FOUND,
            Self::Conflict { .. } => StatusCode::CONFLICT,
            Self::RateLimited { .. } => StatusCode::TOO_MANY_REQUESTS,
            Self::PayloadTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            Self::UnsupportedMediaType { .. } => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::PluginError { .. } => StatusCode::BAD_GATEWAY,
            Self::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    #[must_use]
    pub const fn error_name(&self) -> &'static str {
        match self {
            Self::BadRequest { .. } => "bad_request",
            Self::ValidationError { .. } => "validation_error",
            Self::Unauthorized { .. } => "unauthorized",
            Self::Forbidden { .. } => "forbidden",
            Self::NotFound { .. } => "not_found",
            Self::Conflict { .. } => "conflict",
            Self::RateLimited { .. } => "rate_limited",
            Self::PayloadTooLarge { .. } => "payload_too_large",
            Self::UnsupportedMediaType { .. } => "unsupported_media_type",
            Self::PluginError { .. } => "plugin_error",
            Self::Internal { .. } => "internal_error",
        }
    }

    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            Self::BadRequest { message, .. }
            | Self::ValidationError { message, .. }
            | Self::Unauthorized { message }
            | Self::Forbidden { message }
            | Self::NotFound { message }
            | Self::Conflict { message }
            | Self::RateLimited { message }
            | Self::PayloadTooLarge { message }
            | Self::UnsupportedMediaType { message }
            | Self::PluginError { message, .. }
            | Self::Internal { message, .. } => message,
        }
    }

    #[must_use]
    pub const fn details(&self) -> Option<&Value> {
        match self {
            Self::ValidationError { details, .. } => details.as_ref(),
            _ => None,
        }
    }

    #[must_use]
    pub fn error_body(&self) -> ErrorBody {
        ErrorBody {
            status: self.status_code().as_u16(),
            name: self.error_name(),
            code: self.error_name(),
            message: self.message().to_owned(),
            details: self.details().cloned(),
        }
    }

    #[must_use]
    pub fn error_response(&self) -> ErrorResponse {
        ErrorResponse {
            error: self.error_body(),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = Json(self.error_response());
        (status, body).into_response()
    }
}

/// 具名组件接口，插件与核心服务均可实现。
pub trait Named {
    fn name(&self) -> &str;
}

/// 版本信息接口。
pub trait Versioned {
    fn version(&self) -> &str;
}
