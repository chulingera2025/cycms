use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::PermissionError;

/// 角色对外视图，跨 crate 以 `String` 持有 UUID 以对齐三方言。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub created_at: DateTime<Utc>,
}

/// 权限对外视图。`scope` 与 `source` 作为独立字段，避免编码到字符串里。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub id: String,
    pub domain: String,
    pub resource: String,
    pub action: String,
    pub scope: PermissionScope,
    pub source: String,
}

/// 权限作用范围。v0.1 只有两档：`All`（忽略 owner）/ `Own`（仅当 `owner_id == user_id`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionScope {
    All,
    Own,
}

impl PermissionScope {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Own => "own",
        }
    }
}

impl std::fmt::Display for PermissionScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for PermissionScope {
    type Err = PermissionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "all" => Ok(Self::All),
            "own" => Ok(Self::Own),
            other => Err(PermissionError::InputValidation(format!(
                "invalid scope: {other}"
            ))),
        }
    }
}

/// [`crate::role::RoleRepository::create`] 入参。
#[derive(Debug, Clone)]
pub struct NewRoleRow {
    pub name: String,
    pub description: Option<String>,
    pub is_system: bool,
}

/// [`crate::role::RoleRepository::update`] 入参。
#[derive(Debug, Clone, Default)]
pub struct UpdateRoleRow {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
}

/// 用于批量注册（种子 / 插件权限点）的权限定义。`source` 由 upsert 调用方统一指定。
#[derive(Debug, Clone)]
pub struct PermissionDefinition {
    pub domain: String,
    pub resource: String,
    pub action: String,
    pub scope: PermissionScope,
}
