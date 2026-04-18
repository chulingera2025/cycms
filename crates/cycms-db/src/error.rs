use cycms_core::Error;

/// 将 [`sqlx::Error`] 统一映射为 [`cycms_core::Error`]。
///
/// 配置类错误映射为 `BadRequest`，其余视为 `Internal`，避免把数据库层异常原样抛给上层。
pub fn map_sqlx_error(source: sqlx::Error) -> Error {
    match source {
        sqlx::Error::Configuration(_) | sqlx::Error::Io(_) | sqlx::Error::Tls(_) => {
            Error::BadRequest {
                message: "database configuration error".to_owned(),
                source: Some(Box::new(source)),
            }
        }
        other => Error::Internal {
            message: "database operation failed".to_owned(),
            source: Some(Box::new(other)),
        },
    }
}
