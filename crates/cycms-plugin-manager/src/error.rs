use cycms_core::Error;

/// `cycms-plugin-manager` 领域错误类型。跨 crate 边界统一映射到 [`cycms_core::Error`]。
#[derive(Debug, thiserror::Error)]
pub enum PluginManagerError {
    /// manifest 文件无法解析或关键字段校验失败（Req 20.1–20.4）。
    #[error("invalid plugin manifest: {0}")]
    InvalidManifest(String),

    /// 数据库中 `plugins` 行的列值无法反序列化（如非法 kind / status 字符串）。
    #[error("invalid plugin record: {0}")]
    InvalidRecord(String),

    /// 底层数据库错误。
    #[error("plugin database error")]
    Database(#[source] sqlx::Error),
}

impl From<PluginManagerError> for Error {
    fn from(value: PluginManagerError) -> Self {
        match value {
            PluginManagerError::InvalidManifest(msg) => Self::ValidationError {
                message: msg,
                details: None,
            },
            PluginManagerError::InvalidRecord(msg) => Self::Internal {
                message: format!("invalid plugin record: {msg}"),
                source: None,
            },
            PluginManagerError::Database(e) => Self::Internal {
                message: format!("plugin db error: {e}"),
                source: None,
            },
        }
    }
}
