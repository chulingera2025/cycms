use cycms_core::Error;

/// 媒体资产操作错误类型。
#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[error("media asset not found: {0}")]
    AssetNotFound(String),

    #[error("file too large: {size} bytes exceeds limit of {limit} bytes")]
    FileTooLarge { size: u64, limit: u64 },

    #[error("disallowed MIME type: {0}")]
    DisallowedMimeType(String),

    /// 删除策略为 Block 时，资产仍被内容条目引用。
    #[error("asset '{0}' is referenced by {1} content entries; deletion blocked")]
    ReferencedAsset(String, u64),

    #[error("storage error: {0}")]
    Storage(String),

    #[error("database error: {0}")]
    Database(#[source] sqlx::Error),
}

impl From<MediaError> for Error {
    fn from(e: MediaError) -> Self {
        match e {
            MediaError::AssetNotFound(id) => Error::NotFound {
                message: format!("media asset not found: {id}"),
            },
            MediaError::FileTooLarge { size, limit } => Error::PayloadTooLarge {
                message: format!("file too large: {size} bytes exceeds limit of {limit} bytes"),
            },
            MediaError::DisallowedMimeType(mime) => Error::UnsupportedMediaType {
                message: format!("disallowed MIME type: {mime}"),
            },
            MediaError::ReferencedAsset(id, n) => Error::Conflict {
                message: format!(
                    "asset '{id}' is referenced by {n} content entries; deletion blocked"
                ),
            },
            MediaError::Storage(msg) => Error::Internal {
                message: msg,
                source: None,
            },
            MediaError::Database(e) => Error::Internal {
                message: format!("media db error: {e}"),
                source: None,
            },
        }
    }
}
