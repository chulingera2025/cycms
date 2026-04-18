//! 迁移文件校验和。

use sha2::{Digest, Sha256};

/// 计算给定内容的 SHA-256 校验和，返回 32 字节数组。
pub fn sha256_bytes(content: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(content);
    hasher.finalize().into()
}
