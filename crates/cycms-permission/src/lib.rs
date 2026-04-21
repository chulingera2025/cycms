//! `cycms-permission` —— 权限引擎 crate。
//!
//! 覆盖 Requirements 2.1–2.5：角色/权限数据模型、`domain.resource.action` 解析、
//! `scope=own` 判断、插件权限点注册、默认角色种子、axum 权限中间件。

mod error;
mod middleware;
mod model;
mod parser;
mod permission;
mod role;
mod seed;
mod service;

pub use error::PermissionError;
pub use middleware::{PermissionMiddlewareState, require_permission_middleware};
pub use model::{
    NewRoleRow, Permission, PermissionDefinition, PermissionScope, Role, UpdateRoleRow,
};
pub use parser::{ParsedCode, parse_permission_code};
pub use permission::PermissionRepository;
pub use role::RoleRepository;
pub use seed::{AUTHOR_ROLE, EDITOR_ROLE, seed_defaults};
pub use service::{PermissionEngine, SUPER_ADMIN_ROLE};
