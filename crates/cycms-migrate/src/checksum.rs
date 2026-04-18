//! 迁移文件校验和（占位，MIG-3 实现）。

#![allow(dead_code)]

use sha2::{Digest, Sha256};

/// 计算给定内容的 SHA-256 校验和，返回 32 字节小端数组。
///
/// TODO!!! MIG-3 在发现阶段调用此函数并持久化到 `migration_records.checksum`。
pub fn sha256_bytes(content: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(content);
    hasher.finalize().into()
}
