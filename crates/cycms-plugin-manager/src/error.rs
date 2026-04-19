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

    /// 扫描插件目录时发生 I/O 或结构错误。
    #[error("plugin discovery failed: {0}")]
    Discovery(String),

    /// 插件声明的 `compatibility.cycms` 范围未覆盖当前宿主版本（Req 20.2）。
    #[error("plugin {plugin} requires cycms {required}, but host is {actual}")]
    IncompatibleHost {
        plugin: String,
        required: String,
        actual: String,
    },

    /// 非 optional 依赖缺失（Req 20.3 / 10.2）。
    #[error("plugin {plugin} depends on missing plugin {dependency}")]
    MissingDependency {
        plugin: String,
        dependency: String,
    },

    /// 依赖版本不匹配（Req 20.3 / 10.2）。
    #[error(
        "plugin {plugin} requires {dependency} {required}, but installed version is {actual}"
    )]
    IncompatibleDependency {
        plugin: String,
        dependency: String,
        required: String,
        actual: String,
    },

    /// 依赖图存在循环，无法确定启用顺序。
    #[error("cyclic plugin dependency involving: {involved:?}")]
    CyclicDependency { involved: Vec<String> },

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
            PluginManagerError::Discovery(msg) => Self::Internal {
                message: format!("plugin discovery failed: {msg}"),
                source: None,
            },
            e @ (PluginManagerError::IncompatibleHost { .. }
            | PluginManagerError::MissingDependency { .. }
            | PluginManagerError::IncompatibleDependency { .. }
            | PluginManagerError::CyclicDependency { .. }) => Self::ValidationError {
                message: e.to_string(),
                details: None,
            },
            PluginManagerError::Database(e) => Self::Internal {
                message: format!("plugin db error: {e}"),
                source: None,
            },
        }
    }
}
