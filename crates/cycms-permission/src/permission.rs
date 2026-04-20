use std::sync::Arc;

use cycms_core::Result;
use cycms_db::DatabasePool;
use sqlx::Row;
use sqlx::mysql::MySqlRow;
use sqlx::postgres::PgRow;
use sqlx::sqlite::SqliteRow;
use uuid::Uuid;

use crate::error::PermissionError;
use crate::model::{Permission, PermissionDefinition, PermissionScope};

/// 权限表 CRUD + 幂等 upsert + 按 `source` 批量清理。
pub struct PermissionRepository {
    db: Arc<DatabasePool>,
}

impl PermissionRepository {
    #[must_use]
    pub fn new(db: Arc<DatabasePool>) -> Self {
        Self { db }
    }

    /// 创建单条权限。重复 `(domain, resource, action, scope)` → Conflict。
    ///
    /// # Errors
    /// - 唯一约束冲突 → [`cycms_core::Error::Conflict`]
    pub async fn create(
        &self,
        domain: &str,
        resource: &str,
        action: &str,
        scope: PermissionScope,
        source: &str,
    ) -> Result<Permission> {
        let id = Uuid::new_v4().to_string();
        let scope_str = scope.as_str();
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO permissions (id, domain, resource, action, scope, source) \
                     VALUES ($1::UUID, $2, $3, $4, $5, $6)",
                )
                .bind(&id)
                .bind(domain)
                .bind(resource)
                .bind(action)
                .bind(scope_str)
                .bind(source)
                .execute(pool)
                .await
                .map_err(map_unique_violation)?;
            }
            DatabasePool::MySql(pool) => {
                sqlx::query(
                    "INSERT INTO permissions (id, domain, resource, action, scope, source) \
                     VALUES (?, ?, ?, ?, ?, ?)",
                )
                .bind(&id)
                .bind(domain)
                .bind(resource)
                .bind(action)
                .bind(scope_str)
                .bind(source)
                .execute(pool)
                .await
                .map_err(map_unique_violation)?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO permissions (id, domain, resource, action, scope, source) \
                     VALUES (?, ?, ?, ?, ?, ?)",
                )
                .bind(&id)
                .bind(domain)
                .bind(resource)
                .bind(action)
                .bind(scope_str)
                .bind(source)
                .execute(pool)
                .await
                .map_err(map_unique_violation)?;
            }
        }

        self.find_by_id(&id)
            .await?
            .ok_or_else(|| cycms_core::Error::Internal {
                message: "inserted permission not found on read-back".to_owned(),
                source: None,
            })
    }

    /// 按主键查找。
    ///
    /// # Errors
    /// DB 错误 → [`cycms_core::Error::Internal`]。
    pub async fn find_by_id(&self, id: &str) -> Result<Option<Permission>> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(PG_SELECT_WHERE_ID)
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(PermissionError::Database)?;
                row.map(|r| pg_row_to_permission(&r))
                    .transpose()
                    .map_err(Into::into)
            }
            DatabasePool::MySql(pool) => {
                let row = sqlx::query(MYSQL_SELECT_WHERE_ID)
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(PermissionError::Database)?;
                row.map(|r| mysql_row_to_permission(&r))
                    .transpose()
                    .map_err(Into::into)
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(SQLITE_SELECT_WHERE_ID)
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(PermissionError::Database)?;
                row.map(|r| sqlite_row_to_permission(&r))
                    .transpose()
                    .map_err(Into::into)
            }
        }
    }

    /// 按 `(domain, resource, action, scope)` 唯一键查找。
    ///
    /// # Errors
    /// DB 错误 → [`cycms_core::Error::Internal`]。
    pub async fn find_by_code_and_scope(
        &self,
        domain: &str,
        resource: &str,
        action: &str,
        scope: PermissionScope,
    ) -> Result<Option<Permission>> {
        let scope_str = scope.as_str();
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(PG_FIND_BY_CODE)
                    .bind(domain)
                    .bind(resource)
                    .bind(action)
                    .bind(scope_str)
                    .fetch_optional(pool)
                    .await
                    .map_err(PermissionError::Database)?;
                row.map(|r| pg_row_to_permission(&r))
                    .transpose()
                    .map_err(Into::into)
            }
            DatabasePool::MySql(pool) => {
                let row = sqlx::query(MYSQL_FIND_BY_CODE)
                    .bind(domain)
                    .bind(resource)
                    .bind(action)
                    .bind(scope_str)
                    .fetch_optional(pool)
                    .await
                    .map_err(PermissionError::Database)?;
                row.map(|r| mysql_row_to_permission(&r))
                    .transpose()
                    .map_err(Into::into)
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(SQLITE_FIND_BY_CODE)
                    .bind(domain)
                    .bind(resource)
                    .bind(action)
                    .bind(scope_str)
                    .fetch_optional(pool)
                    .await
                    .map_err(PermissionError::Database)?;
                row.map(|r| sqlite_row_to_permission(&r))
                    .transpose()
                    .map_err(Into::into)
            }
        }
    }

    /// 列出指定 `source` 名下的全部权限，按 `(domain, resource, action, scope)` 升序。
    ///
    /// # Errors
    /// DB 错误 → [`cycms_core::Error::Internal`]。
    pub async fn list_by_source(&self, source: &str) -> Result<Vec<Permission>> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(PG_LIST_BY_SOURCE)
                    .bind(source)
                    .fetch_all(pool)
                    .await
                    .map_err(PermissionError::Database)?;
                rows.iter()
                    .map(pg_row_to_permission)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
            DatabasePool::MySql(pool) => {
                let rows = sqlx::query(MYSQL_LIST_BY_SOURCE)
                    .bind(source)
                    .fetch_all(pool)
                    .await
                    .map_err(PermissionError::Database)?;
                rows.iter()
                    .map(mysql_row_to_permission)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(SQLITE_LIST_BY_SOURCE)
                    .bind(source)
                    .fetch_all(pool)
                    .await
                    .map_err(PermissionError::Database)?;
                rows.iter()
                    .map(sqlite_row_to_permission)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
        }
    }

    /// 列出全部权限，按 `(domain, resource, action, scope)` 升序。
    ///
    /// # Errors
    /// DB 错误 → [`cycms_core::Error::Internal`]。
    pub async fn list_all(&self) -> Result<Vec<Permission>> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(PG_SELECT_ALL)
                    .fetch_all(pool)
                    .await
                    .map_err(PermissionError::Database)?;
                rows.iter()
                    .map(pg_row_to_permission)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
            DatabasePool::MySql(pool) => {
                let rows = sqlx::query(MYSQL_SELECT_ALL)
                    .fetch_all(pool)
                    .await
                    .map_err(PermissionError::Database)?;
                rows.iter()
                    .map(mysql_row_to_permission)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(SQLITE_SELECT_ALL)
                    .fetch_all(pool)
                    .await
                    .map_err(PermissionError::Database)?;
                rows.iter()
                    .map(sqlite_row_to_permission)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
        }
    }

    /// 列出角色已绑定的全部权限，按 `(domain, resource, action, scope)` 升序。
    ///
    /// # Errors
    /// DB 错误 → [`cycms_core::Error::Internal`]。
    pub async fn list_by_role_id(&self, role_id: &str) -> Result<Vec<Permission>> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(PG_LIST_BY_ROLE_ID)
                    .bind(role_id)
                    .fetch_all(pool)
                    .await
                    .map_err(PermissionError::Database)?;
                rows.iter()
                    .map(pg_row_to_permission)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
            DatabasePool::MySql(pool) => {
                let rows = sqlx::query(MYSQL_LIST_BY_ROLE_ID)
                    .bind(role_id)
                    .fetch_all(pool)
                    .await
                    .map_err(PermissionError::Database)?;
                rows.iter()
                    .map(mysql_row_to_permission)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(SQLITE_LIST_BY_ROLE_ID)
                    .bind(role_id)
                    .fetch_all(pool)
                    .await
                    .map_err(PermissionError::Database)?;
                rows.iter()
                    .map(sqlite_row_to_permission)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
        }
    }

    /// 幂等批量注册。对每条 [`PermissionDefinition`] 执行 `INSERT ON CONFLICT DO NOTHING`
    /// 语义，随后按 `(domain, resource, action, scope)` 读回，保证返回顺序与入参一致且每条
    /// 对应一行权限记录（新建或既存）。
    ///
    /// # Errors
    /// - DB 错误 → [`cycms_core::Error::Internal`]
    /// - upsert 后读回失败（通常意味着迁移或约束损坏）→ [`cycms_core::Error::Internal`]
    pub async fn upsert_many(
        &self,
        source: &str,
        defs: &[PermissionDefinition],
    ) -> Result<Vec<Permission>> {
        let mut out = Vec::with_capacity(defs.len());
        for def in defs {
            self.insert_or_ignore(&def.domain, &def.resource, &def.action, def.scope, source)
                .await?;
            let found = self
                .find_by_code_and_scope(&def.domain, &def.resource, &def.action, def.scope)
                .await?
                .ok_or_else(|| cycms_core::Error::Internal {
                    message: "permission not found after upsert".to_owned(),
                    source: None,
                })?;
            out.push(found);
        }
        Ok(out)
    }

    /// 按 `source` 批量删除；返回被删行数。用于插件卸载或种子重置。
    ///
    /// # Errors
    /// DB 错误 → [`cycms_core::Error::Internal`]。
    pub async fn delete_by_source(&self, source: &str) -> Result<u64> {
        let affected = match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                sqlx::query("DELETE FROM permissions WHERE source = $1")
                    .bind(source)
                    .execute(pool)
                    .await
                    .map_err(PermissionError::Database)?
                    .rows_affected()
            }
            DatabasePool::MySql(pool) => sqlx::query("DELETE FROM permissions WHERE source = ?")
                .bind(source)
                .execute(pool)
                .await
                .map_err(PermissionError::Database)?
                .rows_affected(),
            DatabasePool::Sqlite(pool) => sqlx::query("DELETE FROM permissions WHERE source = ?")
                .bind(source)
                .execute(pool)
                .await
                .map_err(PermissionError::Database)?
                .rows_affected(),
        };
        Ok(affected)
    }

    /// 按 `source` 删除 `role_permissions` 中所有指向该来源权限的关联行。
    ///
    /// # Errors
    /// DB 错误 → [`cycms_core::Error::Internal`]。
    pub async fn delete_role_links_by_source(&self, source: &str) -> Result<u64> {
        let affected = match self.db.as_ref() {
            DatabasePool::Postgres(pool) => sqlx::query(
                "DELETE FROM role_permissions rp \
                 USING permissions p \
                 WHERE rp.permission_id = p.id AND p.source = $1",
            )
            .bind(source)
            .execute(pool)
            .await
            .map_err(PermissionError::Database)?
            .rows_affected(),
            DatabasePool::MySql(pool) => sqlx::query(
                "DELETE rp FROM role_permissions rp \
                 INNER JOIN permissions p ON p.id = rp.permission_id \
                 WHERE p.source = ?",
            )
            .bind(source)
            .execute(pool)
            .await
            .map_err(PermissionError::Database)?
            .rows_affected(),
            DatabasePool::Sqlite(pool) => sqlx::query(
                "DELETE FROM role_permissions \
                 WHERE permission_id IN (SELECT id FROM permissions WHERE source = ?)",
            )
            .bind(source)
            .execute(pool)
            .await
            .map_err(PermissionError::Database)?
            .rows_affected(),
        };
        Ok(affected)
    }

    async fn insert_or_ignore(
        &self,
        domain: &str,
        resource: &str,
        action: &str,
        scope: PermissionScope,
        source: &str,
    ) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        let scope_str = scope.as_str();
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO permissions (id, domain, resource, action, scope, source) \
                     VALUES ($1::UUID, $2, $3, $4, $5, $6) \
                     ON CONFLICT (domain, resource, action, scope) DO NOTHING",
                )
                .bind(&id)
                .bind(domain)
                .bind(resource)
                .bind(action)
                .bind(scope_str)
                .bind(source)
                .execute(pool)
                .await
                .map_err(PermissionError::Database)?;
            }
            DatabasePool::MySql(pool) => {
                sqlx::query(
                    "INSERT IGNORE INTO permissions (id, domain, resource, action, scope, source) \
                     VALUES (?, ?, ?, ?, ?, ?)",
                )
                .bind(&id)
                .bind(domain)
                .bind(resource)
                .bind(action)
                .bind(scope_str)
                .bind(source)
                .execute(pool)
                .await
                .map_err(PermissionError::Database)?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT OR IGNORE INTO permissions (id, domain, resource, action, scope, source) \
                     VALUES (?, ?, ?, ?, ?, ?)",
                )
                .bind(&id)
                .bind(domain)
                .bind(resource)
                .bind(action)
                .bind(scope_str)
                .bind(source)
                .execute(pool)
                .await
                .map_err(PermissionError::Database)?;
            }
        }
        Ok(())
    }
}

const PG_SELECT_WHERE_ID: &str = "SELECT id::TEXT AS id, domain, resource, action, scope, source FROM permissions WHERE id = $1::UUID";
const PG_FIND_BY_CODE: &str = "SELECT id::TEXT AS id, domain, resource, action, scope, source FROM permissions \
     WHERE domain = $1 AND resource = $2 AND action = $3 AND scope = $4";
const PG_SELECT_ALL: &str = "SELECT id::TEXT AS id, domain, resource, action, scope, source FROM permissions \
    ORDER BY domain, resource, action, scope";
const PG_LIST_BY_SOURCE: &str = "SELECT id::TEXT AS id, domain, resource, action, scope, source FROM permissions \
     WHERE source = $1 ORDER BY domain, resource, action, scope";
const PG_LIST_BY_ROLE_ID: &str = "SELECT p.id::TEXT AS id, p.domain, p.resource, p.action, p.scope, p.source \
    FROM permissions p INNER JOIN role_permissions rp ON rp.permission_id = p.id \
    WHERE rp.role_id = $1::UUID ORDER BY p.domain, p.resource, p.action, p.scope";

const MYSQL_SELECT_WHERE_ID: &str =
    "SELECT id, domain, resource, action, scope, source FROM permissions WHERE id = ?";
const MYSQL_FIND_BY_CODE: &str = "SELECT id, domain, resource, action, scope, source FROM permissions \
     WHERE domain = ? AND resource = ? AND action = ? AND scope = ?";
const MYSQL_SELECT_ALL: &str = "SELECT id, domain, resource, action, scope, source FROM permissions \
    ORDER BY domain, resource, action, scope";
const MYSQL_LIST_BY_SOURCE: &str = "SELECT id, domain, resource, action, scope, source FROM permissions \
     WHERE source = ? ORDER BY domain, resource, action, scope";
const MYSQL_LIST_BY_ROLE_ID: &str = "SELECT p.id, p.domain, p.resource, p.action, p.scope, p.source \
    FROM permissions p INNER JOIN role_permissions rp ON rp.permission_id = p.id \
    WHERE rp.role_id = ? ORDER BY p.domain, p.resource, p.action, p.scope";

const SQLITE_SELECT_WHERE_ID: &str =
    "SELECT id, domain, resource, action, scope, source FROM permissions WHERE id = ?";
const SQLITE_FIND_BY_CODE: &str = "SELECT id, domain, resource, action, scope, source FROM permissions \
     WHERE domain = ? AND resource = ? AND action = ? AND scope = ?";
const SQLITE_SELECT_ALL: &str = "SELECT id, domain, resource, action, scope, source FROM permissions \
    ORDER BY domain, resource, action, scope";
const SQLITE_LIST_BY_SOURCE: &str = "SELECT id, domain, resource, action, scope, source FROM permissions \
     WHERE source = ? ORDER BY domain, resource, action, scope";
const SQLITE_LIST_BY_ROLE_ID: &str = "SELECT p.id, p.domain, p.resource, p.action, p.scope, p.source \
    FROM permissions p INNER JOIN role_permissions rp ON rp.permission_id = p.id \
    WHERE rp.role_id = ? ORDER BY p.domain, p.resource, p.action, p.scope";

fn pg_row_to_permission(row: &PgRow) -> std::result::Result<Permission, PermissionError> {
    let scope_str: String = row.try_get("scope").map_err(PermissionError::Database)?;
    let scope: PermissionScope = scope_str.parse()?;
    Ok(Permission {
        id: row.try_get("id").map_err(PermissionError::Database)?,
        domain: row.try_get("domain").map_err(PermissionError::Database)?,
        resource: row.try_get("resource").map_err(PermissionError::Database)?,
        action: row.try_get("action").map_err(PermissionError::Database)?,
        scope,
        source: row.try_get("source").map_err(PermissionError::Database)?,
    })
}

fn mysql_row_to_permission(row: &MySqlRow) -> std::result::Result<Permission, PermissionError> {
    let scope_str: String = row.try_get("scope").map_err(PermissionError::Database)?;
    let scope: PermissionScope = scope_str.parse()?;
    Ok(Permission {
        id: row.try_get("id").map_err(PermissionError::Database)?,
        domain: row.try_get("domain").map_err(PermissionError::Database)?,
        resource: row.try_get("resource").map_err(PermissionError::Database)?,
        action: row.try_get("action").map_err(PermissionError::Database)?,
        scope,
        source: row.try_get("source").map_err(PermissionError::Database)?,
    })
}

fn sqlite_row_to_permission(row: &SqliteRow) -> std::result::Result<Permission, PermissionError> {
    let scope_str: String = row.try_get("scope").map_err(PermissionError::Database)?;
    let scope: PermissionScope = scope_str.parse()?;
    Ok(Permission {
        id: row.try_get("id").map_err(PermissionError::Database)?,
        domain: row.try_get("domain").map_err(PermissionError::Database)?,
        resource: row.try_get("resource").map_err(PermissionError::Database)?,
        action: row.try_get("action").map_err(PermissionError::Database)?,
        scope,
        source: row.try_get("source").map_err(PermissionError::Database)?,
    })
}

fn map_unique_violation(err: sqlx::Error) -> cycms_core::Error {
    if err
        .as_database_error()
        .is_some_and(sqlx::error::DatabaseError::is_unique_violation)
    {
        cycms_core::Error::Conflict {
            message: "permission already exists".to_owned(),
        }
    } else {
        PermissionError::Database(err).into()
    }
}
