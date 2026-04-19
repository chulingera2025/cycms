use cycms_core::Error;

/// `cycms-plugin-api` 领域错误，跨 crate 边界统一映射到 [`cycms_core::Error`]。
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    /// 指定 key 未注册。
    #[error("service not registered: {key}")]
    ServiceNotFound { key: String },

    /// 服务已注册但被标记为不可用（对应 Req 13.3：被依赖插件未启用）。
    #[error("service unavailable: {key}")]
    ServiceUnavailable { key: String },

    /// 注册时的具体类型与 `get<T>()` 请求类型不一致。
    #[error("service type mismatch for key {key}: expected {expected}")]
    TypeMismatch { key: String, expected: &'static str },

    /// key 不符合 `{plugin_name}.{service_name}` 两段式格式（对应 Req 13.1）。
    #[error("invalid service key: {0}")]
    InvalidKey(String),
}

impl From<RegistryError> for Error {
    fn from(value: RegistryError) -> Self {
        match value {
            RegistryError::ServiceNotFound { key } => Self::NotFound {
                message: format!("service not registered: {key}"),
            },
            // 插件未启用导致的不可用走 PluginError，语义最贴合 Req 13.3
            RegistryError::ServiceUnavailable { key } => Self::PluginError {
                message: format!("service unavailable: {key}"),
                source: None,
            },
            RegistryError::TypeMismatch { key, expected } => Self::Internal {
                message: format!("service type mismatch for key {key}: expected {expected}"),
                source: None,
            },
            RegistryError::InvalidKey(msg) => Self::ValidationError {
                message: msg,
                details: None,
            },
        }
    }
}
