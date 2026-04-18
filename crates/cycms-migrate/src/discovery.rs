//! 迁移文件发现与解析。
//!
//! 约定与 sqlx-cli 的 `migrate add -r` 保持兼容：
//!   - `{version}_{name}.up.sql` 必须存在（版本号为单调递增的整数字符串）。
//!   - `{version}_{name}.down.sql` 可选，缺失时 [`MigrationEngine::rollback`] 将拒绝回滚该条。
//!
//! 扫描结果按 `version` 升序返回，重复版本号视为致命错误。

use std::fs;
use std::path::{Path, PathBuf};

use cycms_core::{Error, Result};

use crate::checksum::sha256_bytes;

const UP_EXT: &str = ".up.sql";
const DOWN_EXT: &str = ".down.sql";

/// 发现阶段解析出的单条迁移描述。
#[derive(Debug, Clone)]
pub struct DiscoveredMigration {
    pub version: i64,
    pub name: String,
    pub up_sql: String,
    pub down_sql: Option<String>,
    pub checksum: [u8; 32],
}

/// 扫描目录并解析出排序后的迁移列表。
///
/// # Errors
/// - 目录不存在 → `NotFound`
/// - 文件名不符合 `{version}_{name}.up.sql` 约定 → `BadRequest`
/// - 存在重复版本 → `Conflict`
/// - I/O 失败 → `Internal`
pub fn scan(dir: &Path) -> Result<Vec<DiscoveredMigration>> {
    if !dir.exists() {
        return Err(Error::NotFound {
            message: format!("migration directory not found: {}", dir.display()),
        });
    }

    let mut entries: Vec<DiscoveredMigration> = Vec::new();

    for entry in fs::read_dir(dir).map_err(|source| Error::Internal {
        message: format!("failed to read migration directory: {}", dir.display()),
        source: Some(Box::new(source)),
    })? {
        let entry = entry.map_err(|source| Error::Internal {
            message: "failed to iterate migration directory entry".to_owned(),
            source: Some(Box::new(source)),
        })?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(stem) = file_name.strip_suffix(UP_EXT) else {
            // `.down.sql` 与其他文件由 up 扫描时按名称反查，避免重复处理。
            continue;
        };

        let (version, name) = parse_stem(stem).ok_or_else(|| Error::BadRequest {
            message: format!("invalid migration file name: {file_name}"),
            source: None,
        })?;

        let up_sql = fs::read_to_string(&path).map_err(|source| Error::Internal {
            message: format!("failed to read migration up file: {}", path.display()),
            source: Some(Box::new(source)),
        })?;
        let checksum = sha256_bytes(up_sql.as_bytes());

        let down_path = down_companion(&path, stem);
        let down_sql = if down_path.exists() {
            Some(
                fs::read_to_string(&down_path).map_err(|source| Error::Internal {
                    message: format!(
                        "failed to read migration down file: {}",
                        down_path.display()
                    ),
                    source: Some(Box::new(source)),
                })?,
            )
        } else {
            None
        };

        entries.push(DiscoveredMigration {
            version,
            name: name.to_owned(),
            up_sql,
            down_sql,
            checksum,
        });
    }

    entries.sort_by_key(|migration| migration.version);
    check_no_duplicates(&entries)?;

    Ok(entries)
}

fn parse_stem(stem: &str) -> Option<(i64, &str)> {
    let (version_part, name_part) = stem.split_once('_')?;
    if version_part.is_empty() || name_part.is_empty() {
        return None;
    }
    if !version_part.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if !name_part
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return None;
    }
    let version: i64 = version_part.parse().ok()?;
    Some((version, name_part))
}

fn down_companion(up_path: &Path, stem: &str) -> PathBuf {
    let mut down = up_path.to_path_buf();
    down.set_file_name(format!("{stem}{DOWN_EXT}"));
    down
}

fn check_no_duplicates(entries: &[DiscoveredMigration]) -> Result<()> {
    for window in entries.windows(2) {
        if window[0].version == window[1].version {
            return Err(Error::Conflict {
                message: format!(
                    "duplicate migration version {}: {} vs {}",
                    window[0].version, window[0].name, window[1].name
                ),
            });
        }
    }
    Ok(())
}
