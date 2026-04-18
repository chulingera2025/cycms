use std::sync::Arc;

use chrono::NaiveDateTime;
use cycms_core::Result;
use cycms_db::DatabasePool;
use sqlx::Row;
use sqlx::mysql::MySqlRow;
use sqlx::postgres::PgRow;
use sqlx::sqlite::SqliteRow;
use uuid::Uuid;

use crate::error::PermissionError;
use crate::model::{NewRoleRow, Role};

/// 角色表 CRUD + 角色-权限绑定 + 用户-角色绑定，屏蔽 PG/`MySQL`/`SQLite` 方言差异。
pub struct RoleRepository {
    db: Arc<DatabasePool>,
}

impl RoleRepository {
    #[must_use]
    pub fn new(db: Arc<DatabasePool>) -> Self {
        Self { db }
    }

    /// 创建角色。`id` 由应用层生成（v4 UUID 小写字符串），`name` 会被 trim + lowercase
    /// 归一，避免大小写漂移形成 `"admin"` / `"Admin"` 这种"同角色双身份"。
    ///
    /// # Errors
    /// - `name` 为空 → [`cycms_core::Error::ValidationError`]
    /// - `name` 冲突 → [`cycms_core::Error::Conflict`]
    pub async fn create(&self, input: NewRoleRow) -> Result<Role> {
        let normalized = input.name.trim().to_lowercase();
        if normalized.is_empty() {
            return Err(
                PermissionError::InputValidation("role name must not be empty".to_owned()).into(),
            );
        }

        let id = Uuid::new_v4().to_string();
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO roles (id, name, description, is_system) \
                     VALUES ($1::UUID, $2, $3, $4)",
                )
                .bind(&id)
                .bind(&normalized)
                .bind(&input.description)
                .bind(input.is_system)
                .execute(pool)
                .await
                .map_err(map_unique_violation)?;
            }
            DatabasePool::MySql(pool) => {
                sqlx::query(
                    "INSERT INTO roles (id, name, description, is_system) VALUES (?, ?, ?, ?)",
                )
                .bind(&id)
                .bind(&normalized)
                .bind(&input.description)
                .bind(input.is_system)
                .execute(pool)
                .await
                .map_err(map_unique_violation)?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO roles (id, name, description, is_system) VALUES (?, ?, ?, ?)",
                )
                .bind(&id)
                .bind(&normalized)
                .bind(&input.description)
                .bind(input.is_system)
                .execute(pool)
                .await
                .map_err(map_unique_violation)?;
            }
        }

        self.find_by_id(&id)
            .await?
            .ok_or_else(|| cycms_core::Error::Internal {
                message: "inserted role not found on read-back".to_owned(),
                source: None,
            })
    }

    /// 按主键查找。
    ///
    /// # Errors
    /// DB 错误 → [`cycms_core::Error::Internal`]。
    pub async fn find_by_id(&self, id: &str) -> Result<Option<Role>> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(PG_SELECT_WHERE_ID)
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(PermissionError::Database)?;
                row.map(|r| pg_row_to_role(&r))
                    .transpose()
                    .map_err(Into::into)
            }
            DatabasePool::MySql(pool) => {
                let row = sqlx::query(MYSQL_SELECT_WHERE_ID)
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(PermissionError::Database)?;
                row.map(|r| mysql_row_to_role(&r))
                    .transpose()
                    .map_err(Into::into)
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(SQLITE_SELECT_WHERE_ID)
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(PermissionError::Database)?;
                row.map(|r| sqlite_row_to_role(&r))
                    .transpose()
                    .map_err(Into::into)
            }
        }
    }

    /// 按 `name` 查找；`name` 会归一为 trim+lowercase 后比较。
    ///
    /// # Errors
    /// DB 错误 → [`cycms_core::Error::Internal`]。
    pub async fn find_by_name(&self, name: &str) -> Result<Option<Role>> {
        let normalized = name.trim().to_lowercase();
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(PG_SELECT_WHERE_NAME)
                    .bind(&normalized)
                    .fetch_optional(pool)
                    .await
                    .map_err(PermissionError::Database)?;
                row.map(|r| pg_row_to_role(&r))
                    .transpose()
                    .map_err(Into::into)
            }
            DatabasePool::MySql(pool) => {
                let row = sqlx::query(MYSQL_SELECT_WHERE_NAME)
                    .bind(&normalized)
                    .fetch_optional(pool)
                    .await
                    .map_err(PermissionError::Database)?;
                row.map(|r| mysql_row_to_role(&r))
                    .transpose()
                    .map_err(Into::into)
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(SQLITE_SELECT_WHERE_NAME)
                    .bind(&normalized)
                    .fetch_optional(pool)
                    .await
                    .map_err(PermissionError::Database)?;
                row.map(|r| sqlite_row_to_role(&r))
                    .transpose()
                    .map_err(Into::into)
            }
        }
    }

    /// 列表，按 `name` 升序。
    ///
    /// # Errors
    /// DB 错误 → [`cycms_core::Error::Internal`]。
    pub async fn list(&self) -> Result<Vec<Role>> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(PG_SELECT_ALL)
                    .fetch_all(pool)
                    .await
                    .map_err(PermissionError::Database)?;
                rows.iter()
                    .map(pg_row_to_role)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
            DatabasePool::MySql(pool) => {
                let rows = sqlx::query(MYSQL_SELECT_ALL)
                    .fetch_all(pool)
                    .await
                    .map_err(PermissionError::Database)?;
                rows.iter()
                    .map(mysql_row_to_role)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(SQLITE_SELECT_ALL)
                    .fetch_all(pool)
                    .await
                    .map_err(PermissionError::Database)?;
                rows.iter()
                    .map(sqlite_row_to_role)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
        }
    }

    /// 按主键删除。系统内置角色（`is_system=true`）拒绝删除。
    ///
    /// # Errors
    /// - 角色不存在 → [`cycms_core::Error::NotFound`]
    /// - 系统角色 → [`cycms_core::Error::Conflict`]
    pub async fn delete(&self, id: &str) -> Result<()> {
        let role = self
            .find_by_id(id)
            .await?
            .ok_or(PermissionError::RoleNotFound)?;
        if role.is_system {
            return Err(PermissionError::SystemRoleUndeletable.into());
        }

        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                sqlx::query("DELETE FROM roles WHERE id = $1::UUID")
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(PermissionError::Database)?;
            }
            DatabasePool::MySql(pool) => {
                sqlx::query("DELETE FROM roles WHERE id = ?")
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(PermissionError::Database)?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query("DELETE FROM roles WHERE id = ?")
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(PermissionError::Database)?;
            }
        }
        Ok(())
    }

    /// 角色绑定权限，幂等；重复调用不报错也不产生多行。
    ///
    /// # Errors
    /// DB 错误 → [`cycms_core::Error::Internal`]。
    pub async fn attach_permission(&self, role_id: &str, permission_id: &str) -> Result<()> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO role_permissions (role_id, permission_id) \
                     VALUES ($1::UUID, $2::UUID) ON CONFLICT DO NOTHING",
                )
                .bind(role_id)
                .bind(permission_id)
                .execute(pool)
                .await
                .map_err(PermissionError::Database)?;
            }
            DatabasePool::MySql(pool) => {
                sqlx::query(
                    "INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES (?, ?)",
                )
                .bind(role_id)
                .bind(permission_id)
                .execute(pool)
                .await
                .map_err(PermissionError::Database)?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT OR IGNORE INTO role_permissions (role_id, permission_id) VALUES (?, ?)",
                )
                .bind(role_id)
                .bind(permission_id)
                .execute(pool)
                .await
                .map_err(PermissionError::Database)?;
            }
        }
        Ok(())
    }

    /// 角色解绑权限，幂等；不存在的关联不会报错。
    ///
    /// # Errors
    /// DB 错误 → [`cycms_core::Error::Internal`]。
    pub async fn detach_permission(&self, role_id: &str, permission_id: &str) -> Result<()> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "DELETE FROM role_permissions WHERE role_id = $1::UUID AND permission_id = $2::UUID",
                )
                .bind(role_id)
                .bind(permission_id)
                .execute(pool)
                .await
                .map_err(PermissionError::Database)?;
            }
            DatabasePool::MySql(pool) => {
                sqlx::query("DELETE FROM role_permissions WHERE role_id = ? AND permission_id = ?")
                    .bind(role_id)
                    .bind(permission_id)
                    .execute(pool)
                    .await
                    .map_err(PermissionError::Database)?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query("DELETE FROM role_permissions WHERE role_id = ? AND permission_id = ?")
                    .bind(role_id)
                    .bind(permission_id)
                    .execute(pool)
                    .await
                    .map_err(PermissionError::Database)?;
            }
        }
        Ok(())
    }

    /// 绑定用户与角色，幂等。
    ///
    /// # Errors
    /// DB 错误 → [`cycms_core::Error::Internal`]。
    pub async fn bind_user(&self, user_id: &str, role_id: &str) -> Result<()> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO user_roles (user_id, role_id) \
                     VALUES ($1::UUID, $2::UUID) ON CONFLICT DO NOTHING",
                )
                .bind(user_id)
                .bind(role_id)
                .execute(pool)
                .await
                .map_err(PermissionError::Database)?;
            }
            DatabasePool::MySql(pool) => {
                sqlx::query("INSERT IGNORE INTO user_roles (user_id, role_id) VALUES (?, ?)")
                    .bind(user_id)
                    .bind(role_id)
                    .execute(pool)
                    .await
                    .map_err(PermissionError::Database)?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query("INSERT OR IGNORE INTO user_roles (user_id, role_id) VALUES (?, ?)")
                    .bind(user_id)
                    .bind(role_id)
                    .execute(pool)
                    .await
                    .map_err(PermissionError::Database)?;
            }
        }
        Ok(())
    }

    /// 解除用户与角色的绑定，幂等。
    ///
    /// # Errors
    /// DB 错误 → [`cycms_core::Error::Internal`]。
    pub async fn unbind_user(&self, user_id: &str, role_id: &str) -> Result<()> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "DELETE FROM user_roles WHERE user_id = $1::UUID AND role_id = $2::UUID",
                )
                .bind(user_id)
                .bind(role_id)
                .execute(pool)
                .await
                .map_err(PermissionError::Database)?;
            }
            DatabasePool::MySql(pool) => {
                sqlx::query("DELETE FROM user_roles WHERE user_id = ? AND role_id = ?")
                    .bind(user_id)
                    .bind(role_id)
                    .execute(pool)
                    .await
                    .map_err(PermissionError::Database)?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query("DELETE FROM user_roles WHERE user_id = ? AND role_id = ?")
                    .bind(user_id)
                    .bind(role_id)
                    .execute(pool)
                    .await
                    .map_err(PermissionError::Database)?;
            }
        }
        Ok(())
    }
}

const PG_SELECT_ALL: &str =
    "SELECT id::TEXT AS id, name, description, is_system, created_at FROM roles ORDER BY name";
const PG_SELECT_WHERE_ID: &str = "SELECT id::TEXT AS id, name, description, is_system, created_at FROM roles WHERE id = $1::UUID";
const PG_SELECT_WHERE_NAME: &str =
    "SELECT id::TEXT AS id, name, description, is_system, created_at FROM roles WHERE name = $1";

const MYSQL_SELECT_ALL: &str =
    "SELECT id, name, description, is_system, created_at FROM roles ORDER BY name";
const MYSQL_SELECT_WHERE_ID: &str =
    "SELECT id, name, description, is_system, created_at FROM roles WHERE id = ?";
const MYSQL_SELECT_WHERE_NAME: &str =
    "SELECT id, name, description, is_system, created_at FROM roles WHERE name = ?";

const SQLITE_SELECT_ALL: &str =
    "SELECT id, name, description, is_system, created_at FROM roles ORDER BY name";
const SQLITE_SELECT_WHERE_ID: &str =
    "SELECT id, name, description, is_system, created_at FROM roles WHERE id = ?";
const SQLITE_SELECT_WHERE_NAME: &str =
    "SELECT id, name, description, is_system, created_at FROM roles WHERE name = ?";

fn pg_row_to_role(row: &PgRow) -> std::result::Result<Role, PermissionError> {
    Ok(Role {
        id: row.try_get("id").map_err(PermissionError::Database)?,
        name: row.try_get("name").map_err(PermissionError::Database)?,
        description: row
            .try_get("description")
            .map_err(PermissionError::Database)?,
        is_system: row
            .try_get("is_system")
            .map_err(PermissionError::Database)?,
        created_at: row
            .try_get("created_at")
            .map_err(PermissionError::Database)?,
    })
}

fn mysql_row_to_role(row: &MySqlRow) -> std::result::Result<Role, PermissionError> {
    let created_at: NaiveDateTime = row
        .try_get("created_at")
        .map_err(PermissionError::Database)?;
    Ok(Role {
        id: row.try_get("id").map_err(PermissionError::Database)?,
        name: row.try_get("name").map_err(PermissionError::Database)?,
        description: row
            .try_get("description")
            .map_err(PermissionError::Database)?,
        is_system: row
            .try_get("is_system")
            .map_err(PermissionError::Database)?,
        created_at: created_at.and_utc(),
    })
}

fn sqlite_row_to_role(row: &SqliteRow) -> std::result::Result<Role, PermissionError> {
    Ok(Role {
        id: row.try_get("id").map_err(PermissionError::Database)?,
        name: row.try_get("name").map_err(PermissionError::Database)?,
        description: row
            .try_get("description")
            .map_err(PermissionError::Database)?,
        is_system: row
            .try_get("is_system")
            .map_err(PermissionError::Database)?,
        created_at: row
            .try_get("created_at")
            .map_err(PermissionError::Database)?,
    })
}

fn map_unique_violation(err: sqlx::Error) -> cycms_core::Error {
    if err
        .as_database_error()
        .is_some_and(sqlx::error::DatabaseError::is_unique_violation)
    {
        PermissionError::RoleAlreadyExists.into()
    } else {
        PermissionError::Database(err).into()
    }
}
