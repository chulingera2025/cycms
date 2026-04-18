use cycms_core::Result;

use crate::error::PermissionError;
use crate::model::{NewRoleRow, PermissionDefinition, PermissionScope};
use crate::service::{PermissionEngine, SUPER_ADMIN_ROLE};

/// 默认编辑者角色名称：拥有全部内容/媒体/内容类型只读之上的管理能力。
pub const EDITOR_ROLE: &str = "editor";

/// 默认作者角色名称：拥有内容创建权与"仅自己内容"的更新/删除/发布能力。
pub const AUTHOR_ROLE: &str = "author";

/// 系统权限点：编译期拆为 `(domain, resource, action, scope)` 四元组，
/// 避免运行时解析，也让权限矩阵一目了然。新增系统权限在此统一登记。
const SYSTEM_PERMISSIONS: &[(&str, &str, &str, PermissionScope)] = &[
    // 用户/角色管理（super_admin 专享）
    ("system", "user", "read", PermissionScope::All),
    ("system", "user", "manage", PermissionScope::All),
    ("system", "role", "read", PermissionScope::All),
    ("system", "role", "manage", PermissionScope::All),
    // 内容类型定义
    ("content", "type", "create", PermissionScope::All),
    ("content", "type", "read", PermissionScope::All),
    ("content", "type", "update", PermissionScope::All),
    ("content", "type", "delete", PermissionScope::All),
    // 内容条目（all + own 双 scope，支持作者"仅自己"）
    ("content", "entry", "create", PermissionScope::All),
    ("content", "entry", "read", PermissionScope::All),
    ("content", "entry", "update", PermissionScope::All),
    ("content", "entry", "update", PermissionScope::Own),
    ("content", "entry", "delete", PermissionScope::All),
    ("content", "entry", "delete", PermissionScope::Own),
    ("content", "entry", "publish", PermissionScope::All),
    ("content", "entry", "publish", PermissionScope::Own),
    // 媒体
    ("media", "asset", "upload", PermissionScope::All),
    ("media", "asset", "read", PermissionScope::All),
    ("media", "asset", "delete", PermissionScope::All),
    ("media", "asset", "delete", PermissionScope::Own),
    // 系统设置
    ("settings", "namespace", "read", PermissionScope::All),
    ("settings", "namespace", "manage", PermissionScope::All),
    // 插件生命周期
    ("plugin", "lifecycle", "read", PermissionScope::All),
    ("plugin", "lifecycle", "manage", PermissionScope::All),
];

/// 编辑角色的权限集合：内容/媒体全面管理（all scope），但不管用户/角色/设置/插件。
const EDITOR_GRANTS: &[(&str, &str, &str, PermissionScope)] = &[
    ("content", "type", "read", PermissionScope::All),
    ("content", "entry", "create", PermissionScope::All),
    ("content", "entry", "read", PermissionScope::All),
    ("content", "entry", "update", PermissionScope::All),
    ("content", "entry", "delete", PermissionScope::All),
    ("content", "entry", "publish", PermissionScope::All),
    ("media", "asset", "upload", PermissionScope::All),
    ("media", "asset", "read", PermissionScope::All),
    ("media", "asset", "delete", PermissionScope::All),
];

/// 作者角色的权限集合：可创建内容，但只能更新/删除/发布自己的内容。
const AUTHOR_GRANTS: &[(&str, &str, &str, PermissionScope)] = &[
    ("content", "type", "read", PermissionScope::All),
    ("content", "entry", "create", PermissionScope::All),
    ("content", "entry", "read", PermissionScope::All),
    ("content", "entry", "update", PermissionScope::Own),
    ("content", "entry", "delete", PermissionScope::Own),
    ("content", "entry", "publish", PermissionScope::Own),
    ("media", "asset", "upload", PermissionScope::All),
    ("media", "asset", "read", PermissionScope::All),
    ("media", "asset", "delete", PermissionScope::Own),
];

/// 写入所有默认权限 + 三个系统角色（`super_admin` / `editor` / `author`）并建立关联。
///
/// 全幂等：重复调用不产生额外行、不报错。适用于 CLI `cycms seed`
/// 或测试初始化。bootstrap 不自动调用，避免多进程启动互相写入。
///
/// # Errors
/// DB 故障或权限读取失败 → [`cycms_core::Error::Internal`]。
pub async fn seed_defaults(engine: &PermissionEngine) -> Result<()> {
    let defs: Vec<PermissionDefinition> = SYSTEM_PERMISSIONS
        .iter()
        .map(|(d, r, a, s)| PermissionDefinition {
            domain: (*d).to_owned(),
            resource: (*r).to_owned(),
            action: (*a).to_owned(),
            scope: *s,
        })
        .collect();
    engine.register_permissions("system", defs).await?;

    // super_admin 即便被短路也显式授予所有系统权限，防止短路逻辑被关闭后失能
    seed_role(
        engine,
        SUPER_ADMIN_ROLE,
        "System super administrator",
        SYSTEM_PERMISSIONS,
    )
    .await?;
    seed_role(
        engine,
        EDITOR_ROLE,
        "Editorial content manager",
        EDITOR_GRANTS,
    )
    .await?;
    seed_role(
        engine,
        AUTHOR_ROLE,
        "Content author with scope=own write permissions",
        AUTHOR_GRANTS,
    )
    .await?;
    Ok(())
}

async fn seed_role(
    engine: &PermissionEngine,
    name: &str,
    description: &str,
    grants: &[(&str, &str, &str, PermissionScope)],
) -> Result<()> {
    let role = if let Some(existing) = engine.roles().find_by_name(name).await? {
        existing
    } else {
        engine
            .roles()
            .create(NewRoleRow {
                name: name.to_owned(),
                description: Some(description.to_owned()),
                is_system: true,
            })
            .await?
    };

    for (domain, resource, action, scope) in grants {
        let perm = engine
            .permissions()
            .find_by_code_and_scope(domain, resource, action, *scope)
            .await?
            .ok_or(PermissionError::PermissionNotFound)?;
        engine.roles().attach_permission(&role.id, &perm.id).await?;
    }
    Ok(())
}
