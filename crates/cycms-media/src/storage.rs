use std::path::PathBuf;

use async_trait::async_trait;

use crate::error::MediaError;

/// 文件存储后端抽象。v0.1 提供 [`LocalStorageBackend`] 实现。
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// 以 `key` 为路径存储文件内容。`key` 由调用方生成，格式如 `2026/04/{uuid}.jpg`。
    async fn store(&self, key: &str, data: &[u8]) -> Result<(), MediaError>;
    /// 删除指定 key 对应的文件；若文件已不存在则静默成功。
    async fn delete(&self, key: &str) -> Result<(), MediaError>;
    /// 返回可对外访问的 URL，如 `/uploads/2026/04/{uuid}.jpg`。
    fn public_url(&self, key: &str) -> String;
}

/// 本地文件系统存储后端。
pub struct LocalStorageBackend {
    base_dir: PathBuf,
    /// 对外暴露的 URL 前缀，如 `/uploads`（默认值）。
    url_prefix: String,
}

impl LocalStorageBackend {
    pub fn new(base_dir: impl Into<PathBuf>, url_prefix: impl Into<String>) -> Self {
        Self {
            base_dir: base_dir.into(),
            url_prefix: url_prefix.into(),
        }
    }
}

#[async_trait]
impl StorageBackend for LocalStorageBackend {
    async fn store(&self, key: &str, data: &[u8]) -> Result<(), MediaError> {
        let path = self.base_dir.join(key);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| MediaError::Storage(format!("create_dir_all failed: {e}")))?;
        }
        tokio::fs::write(&path, data)
            .await
            .map_err(|e| MediaError::Storage(format!("write failed for {key}: {e}")))?;
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), MediaError> {
        let path = self.base_dir.join(key);
        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(MediaError::Storage(format!(
                "remove_file failed for {key}: {e}"
            ))),
        }
    }

    fn public_url(&self, key: &str) -> String {
        format!("{}/{key}", self.url_prefix.trim_end_matches('/'))
    }
}
