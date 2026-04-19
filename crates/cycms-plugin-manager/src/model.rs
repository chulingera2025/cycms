use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::manifest::PluginKind;

/// 插件启用状态。以字符串形式落库到 `plugins.status` 列。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginStatus {
    Enabled,
    Disabled,
}

impl PluginStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
        }
    }
}

impl std::fmt::Display for PluginStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for PluginStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "enabled" => Ok(Self::Enabled),
            "disabled" => Ok(Self::Disabled),
            other => Err(format!("unknown plugin status: {other}")),
        }
    }
}

/// `plugins` 表行的完整映射（repository 层返回类型）。
///
/// 跨方言统一以 `String` 持有 UUID；时间戳统一为 UTC。`manifest` 列反序列化为
/// [`serde_json::Value`]，service 层按需再解析为 [`crate::PluginManifest`]。
#[derive(Debug, Clone)]
pub struct PluginRecord {
    pub id: String,
    pub name: String,
    pub version: String,
    pub kind: PluginKind,
    pub status: PluginStatus,
    pub manifest: Value,
    pub installed_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 新插件行入参。由 `PluginManager::install` 组装后交给 repository。
#[derive(Debug, Clone)]
pub struct NewPluginRow {
    pub name: String,
    pub version: String,
    pub kind: PluginKind,
    pub manifest: Value,
}
