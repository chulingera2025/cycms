use std::sync::Arc;

use cycms_core::{Error, Result};
use cycms_db::DatabasePool;

use crate::error::PermissionError;
use crate::model::{Permission, PermissionDefinition, PermissionScope};
use crate::parser::parse_permission_code;
use crate::permission::PermissionRepository;
use crate::role::RoleRepository;

/// 默认系统超级管理员角色名称，短路整个检查流程。
pub const SUPER_ADMIN_ROLE: &str = "super_admin";

/// 权限引擎：编排角色/权限查询与 `domain.resource.action` 校验。
///
/// 本引擎不依赖 `cycms-auth`：`check_permission` 接受调用方解析好的
/// `user_id + user_roles` 字符串数组，便于 API 网关、插件运行时等复用。
pub struct PermissionEngine {
    db: Arc<DatabasePool>,
    roles: RoleRepository,
    permissions: PermissionRepository,
}

impl PermissionEngine {
    #[must_use]
    pub fn new(db: Arc<DatabasePool>) -> Self {
        let roles = RoleRepository::new(Arc::clone(&db));
        let permissions = PermissionRepository::new(Arc::clone(&db));
        Self {
            db,
            roles,
            permissions,
        }
    }

    #[must_use]
    pub fn roles(&self) -> &RoleRepository {
        &self.roles
    }

    #[must_use]
    pub fn permissions(&self) -> &PermissionRepository {
        &self.permissions
    }

    #[must_use]
    pub fn db(&self) -> &Arc<DatabasePool> {
        &self.db
    }

    /// 判定给定用户是否拥有指定权限。
    ///
    /// 匹配流程：
    /// 1. 解析 `code` 为 `(domain, resource, action)`；
    /// 2. `user_roles` 归一为 trim + lowercase，过滤空字符串；
    /// 3. 若含 [`SUPER_ADMIN_ROLE`] 直接放行；
    /// 4. 角色为空返回 false；
    /// 5. 对命中的每条 permission scope：`All` 直接放行，`Own` 需 `owner_id == user_id`。
    ///
    /// v0.1 未加缓存，每次都打 DB，单行索引查询 <1ms，v0.2 会引入本地 cache。
    ///
    /// # Errors
    /// - `code` 格式非法 → [`cycms_core::Error::ValidationError`]
    /// - DB 故障 → [`cycms_core::Error::Internal`]
    pub async fn check_permission(
        &self,
        user_id: &str,
        user_roles: &[String],
        code: &str,
        owner_id: Option<&str>,
    ) -> Result<bool> {
        let parsed = parse_permission_code(code)?;

        let normalized: Vec<String> = user_roles
            .iter()
            .filter_map(|r| {
                let n = r.trim().to_lowercase();
                if n.is_empty() { None } else { Some(n) }
            })
            .collect();

        if normalized.iter().any(|r| r == SUPER_ADMIN_ROLE) {
            return Ok(true);
        }
        if normalized.is_empty() {
            return Ok(false);
        }

        let scopes = self
            .fetch_scopes(&normalized, parsed.domain, parsed.resource, parsed.action)
            .await?;

        Ok(evaluate_scopes(&scopes, user_id, owner_id))
    }

    /// 断言权限：失败时返回 [`cycms_core::Error::Forbidden`]，供 handler / 中间件直接
    /// 透传到统一错误响应。
    ///
    /// # Errors
    /// - 权限拒绝 → [`cycms_core::Error::Forbidden`]
    /// - `code` 格式非法 → [`cycms_core::Error::ValidationError`]
    /// - DB 故障 → [`cycms_core::Error::Internal`]
    pub async fn require_permission(
        &self,
        user_id: &str,
        user_roles: &[String],
        code: &str,
        owner_id: Option<&str>,
    ) -> Result<()> {
        if self
            .check_permission(user_id, user_roles, code, owner_id)
            .await?
        {
            Ok(())
        } else {
            Err(PermissionError::Forbidden.into())
        }
    }

    /// 批量注册权限点（系统种子或插件启动时调用），幂等。
    ///
    /// 每条 [`PermissionDefinition`] 的 `(domain, resource, action)` 会被再次通过
    /// [`parse_permission_code`] 校验，保证不会把非法段写进 DB。`source` 统一覆盖，
    /// 无论 def 内部是否带 source 字段（这个结构目前不带，但未来若扩展可避免 drift）。
    ///
    /// # Errors
    /// - `source` 为空 → [`cycms_core::Error::ValidationError`]
    /// - 任意 def 字段非法 → [`cycms_core::Error::ValidationError`]
    /// - DB 故障 → [`cycms_core::Error::Internal`]
    pub async fn register_permissions(
        &self,
        source: &str,
        defs: Vec<PermissionDefinition>,
    ) -> Result<Vec<Permission>> {
        let source = source.trim();
        if source.is_empty() {
            return Err(
                PermissionError::InputValidation("source must not be empty".to_owned()).into(),
            );
        }
        for def in &defs {
            let code = format!("{}.{}.{}", def.domain, def.resource, def.action);
            parse_permission_code(&code)?;
        }
        self.permissions.upsert_many(source, &defs).await
    }

    /// 按 `source` 批量删除权限，返回被删行数。通常用于插件卸载。
    ///
    /// TODO!!!: 任务 15 `PluginManager` 卸载流程集成时，需要级联清理 `role_permissions`
    /// 中对应记录，v0.1 暂由 FK `ON DELETE CASCADE` 处理。
    ///
    /// # Errors
    /// DB 故障 → [`cycms_core::Error::Internal`]。
    pub async fn unregister_permissions_by_source(&self, source: &str) -> Result<u64> {
        self.permissions.delete_by_source(source).await
    }

    async fn fetch_scopes(
        &self,
        roles: &[String],
        domain: &str,
        resource: &str,
        action: &str,
    ) -> Result<Vec<PermissionScope>> {
        let raw: Vec<String> = match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                let role_ph = pg_placeholders(1, roles.len());
                let sql = format!(
                    "SELECT p.scope FROM permissions p \
                     INNER JOIN role_permissions rp ON rp.permission_id = p.id \
                     INNER JOIN roles r ON r.id = rp.role_id \
                     WHERE r.name IN ({role_ph}) AND p.domain = ${d} \
                       AND p.resource = ${r_} AND p.action = ${a}",
                    d = roles.len() + 1,
                    r_ = roles.len() + 2,
                    a = roles.len() + 3,
                );
                let mut q = sqlx::query_scalar::<_, String>(&sql);
                for r in roles {
                    q = q.bind(r);
                }
                q.bind(domain)
                    .bind(resource)
                    .bind(action)
                    .fetch_all(pool)
                    .await
                    .map_err(PermissionError::Database)?
            }
            DatabasePool::MySql(pool) => {
                let role_ph = qmark_placeholders(roles.len());
                let sql = format!(
                    "SELECT p.scope FROM permissions p \
                     INNER JOIN role_permissions rp ON rp.permission_id = p.id \
                     INNER JOIN roles r ON r.id = rp.role_id \
                     WHERE r.name IN ({role_ph}) AND p.domain = ? \
                       AND p.resource = ? AND p.action = ?"
                );
                let mut q = sqlx::query_scalar::<_, String>(&sql);
                for r in roles {
                    q = q.bind(r);
                }
                q.bind(domain)
                    .bind(resource)
                    .bind(action)
                    .fetch_all(pool)
                    .await
                    .map_err(PermissionError::Database)?
            }
            DatabasePool::Sqlite(pool) => {
                let role_ph = qmark_placeholders(roles.len());
                let sql = format!(
                    "SELECT p.scope FROM permissions p \
                     INNER JOIN role_permissions rp ON rp.permission_id = p.id \
                     INNER JOIN roles r ON r.id = rp.role_id \
                     WHERE r.name IN ({role_ph}) AND p.domain = ? \
                       AND p.resource = ? AND p.action = ?"
                );
                let mut q = sqlx::query_scalar::<_, String>(&sql);
                for r in roles {
                    q = q.bind(r);
                }
                q.bind(domain)
                    .bind(resource)
                    .bind(action)
                    .fetch_all(pool)
                    .await
                    .map_err(PermissionError::Database)?
            }
        };

        raw.iter()
            .map(|s| s.parse::<PermissionScope>())
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Error::from)
    }
}

fn evaluate_scopes(scopes: &[PermissionScope], user_id: &str, owner_id: Option<&str>) -> bool {
    if scopes.contains(&PermissionScope::All) {
        return true;
    }
    if scopes.contains(&PermissionScope::Own)
        && let Some(owner) = owner_id
        && owner == user_id
    {
        return true;
    }
    false
}

fn pg_placeholders(start: usize, n: usize) -> String {
    (0..n)
        .map(|i| format!("${}", start + i))
        .collect::<Vec<_>>()
        .join(", ")
}

fn qmark_placeholders(n: usize) -> String {
    std::iter::repeat_n("?", n).collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use super::{evaluate_scopes, pg_placeholders, qmark_placeholders};
    use crate::model::PermissionScope;

    #[test]
    fn evaluate_all_scope_wins() {
        assert!(evaluate_scopes(
            &[PermissionScope::All, PermissionScope::Own],
            "u1",
            Some("u2"),
        ));
    }

    #[test]
    fn evaluate_own_requires_match() {
        assert!(evaluate_scopes(&[PermissionScope::Own], "u1", Some("u1"),));
        assert!(!evaluate_scopes(&[PermissionScope::Own], "u1", Some("u2"),));
        assert!(!evaluate_scopes(&[PermissionScope::Own], "u1", None));
    }

    #[test]
    fn evaluate_empty_denies() {
        assert!(!evaluate_scopes(&[], "u1", Some("u1")));
    }

    #[test]
    fn placeholders_render_expected_sequence() {
        assert_eq!(pg_placeholders(1, 3), "$1, $2, $3");
        assert_eq!(pg_placeholders(4, 2), "$4, $5");
        assert_eq!(qmark_placeholders(3), "?, ?, ?");
    }
}
