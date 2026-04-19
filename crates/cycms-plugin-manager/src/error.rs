use cycms_core::Error;

/// `cycms-plugin-manager` 领域错误类型。跨 crate 边界统一映射到 [`cycms_core::Error`]。
#[derive(Debug, thiserror::Error)]
pub enum PluginManagerError {
    /// manifest 文件无法解析或关键字段校验失败（Req 20.1–20.4）。
    #[error("invalid plugin manifest: {0}")]
    InvalidManifest(String),
}

impl From<PluginManagerError> for Error {
    fn from(value: PluginManagerError) -> Self {
        match value {
            PluginManagerError::InvalidManifest(msg) => Self::ValidationError {
                message: msg,
                details: None,
            },
        }
    }
}
