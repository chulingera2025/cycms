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

/// 全局 Result 别名，默认错误类型为 [`Error`]。
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// [`Error`] 在 axum 中间件等场景下的语义别名。
pub use Error as AppError;

/// 具名组件接口，插件与核心服务均可实现。
pub trait Named {
    fn name(&self) -> &str;
}

/// 版本信息接口。
pub trait Versioned {
    fn version(&self) -> &str;
}
