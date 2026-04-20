use std::sync::Arc;

use chrono::{DateTime, NaiveDateTime, Utc};
use cycms_core::Result;
use cycms_db::DatabasePool;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use sqlx::mysql::MySqlRow;
use sqlx::postgres::PgRow;
use sqlx::sqlite::SqliteRow;
use uuid::Uuid;

use crate::error::AuthError;

/// 用户主数据对外视图。外部 API 统一以 `String` 持有 UUID，跨方言兼容。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// [`UserRepository::create`] 入参。
pub struct NewUserRow {
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub is_active: bool,
}

/// [`UserRepository::update`] 入参。`None` 表示保留原值。
#[derive(Debug, Clone, Default)]
pub struct UpdateUserRow {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password_hash: Option<String>,
    pub is_active: Option<bool>,
}

/// 用户表 CRUD 封装，屏蔽 PG/`MySQL`/`SQLite` 方言差异。
pub struct UserRepository {
    db: Arc<DatabasePool>,
}

impl UserRepository {
    #[must_use]
    pub fn new(db: Arc<DatabasePool>) -> Self {
        Self { db }
    }

    /// 返回当前用户总数，用于判定"系统无用户"以创建初始管理员。
    ///
    /// # Errors
    /// 查询失败时映射为 [`cycms_core::Error::Internal`]。
    pub async fn count(&self) -> Result<i64> {
        let sql = "SELECT COUNT(*) AS n FROM users";
        let count: i64 = match self.db.as_ref() {
            DatabasePool::Postgres(pool) => sqlx::query_scalar(sql)
                .fetch_one(pool)
                .await
                .map_err(AuthError::Database)?,
            DatabasePool::MySql(pool) => sqlx::query_scalar(sql)
                .fetch_one(pool)
                .await
                .map_err(AuthError::Database)?,
            DatabasePool::Sqlite(pool) => sqlx::query_scalar(sql)
                .fetch_one(pool)
                .await
                .map_err(AuthError::Database)?,
        };
        Ok(count)
    }

    /// 创建新用户，`id` 由应用层生成（v4 UUID 小写字符串）以对齐三方言。
    ///
    /// # Errors
    /// - `username` / `email` 冲突 → [`cycms_core::Error::Conflict`]
    /// - 其余 DB 错误 → [`cycms_core::Error::Internal`]
    pub async fn create(&self, input: NewUserRow) -> Result<User> {
        let id = Uuid::new_v4().to_string();
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO users (id, username, email, password_hash, is_active) \
                     VALUES ($1::UUID, $2, $3, $4, $5)",
                )
                .bind(&id)
                .bind(&input.username)
                .bind(&input.email)
                .bind(&input.password_hash)
                .bind(input.is_active)
                .execute(pool)
                .await
                .map_err(map_unique_violation)?;
            }
            DatabasePool::MySql(pool) => {
                sqlx::query(
                    "INSERT INTO users (id, username, email, password_hash, is_active) \
                     VALUES (?, ?, ?, ?, ?)",
                )
                .bind(&id)
                .bind(&input.username)
                .bind(&input.email)
                .bind(&input.password_hash)
                .bind(input.is_active)
                .execute(pool)
                .await
                .map_err(map_unique_violation)?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO users (id, username, email, password_hash, is_active) \
                     VALUES (?, ?, ?, ?, ?)",
                )
                .bind(&id)
                .bind(&input.username)
                .bind(&input.email)
                .bind(&input.password_hash)
                .bind(input.is_active)
                .execute(pool)
                .await
                .map_err(map_unique_violation)?;
            }
        }

        self.find_by_id(&id)
            .await?
            .ok_or_else(|| cycms_core::Error::Internal {
                message: "inserted user not found on read-back".to_owned(),
                source: None,
            })
    }

    /// 列出全部用户，按 `username` 升序。
    ///
    /// # Errors
    /// 查询失败时返回 [`cycms_core::Error::Internal`]。
    pub async fn list(&self) -> Result<Vec<User>> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(PG_SELECT_ALL)
                    .fetch_all(pool)
                    .await
                    .map_err(AuthError::Database)?;
                rows.iter()
                    .map(pg_row_to_user)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
            DatabasePool::MySql(pool) => {
                let rows = sqlx::query(MYSQL_SELECT_ALL)
                    .fetch_all(pool)
                    .await
                    .map_err(AuthError::Database)?;
                rows.iter()
                    .map(mysql_row_to_user)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(SQLITE_SELECT_ALL)
                    .fetch_all(pool)
                    .await
                    .map_err(AuthError::Database)?;
                rows.iter()
                    .map(sqlite_row_to_user)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
        }
    }

    /// 更新用户基础字段。未提供的字段保持原值不变。
    ///
    /// # Errors
    /// - 用户不存在 → [`cycms_core::Error::NotFound`]
    /// - `username` / `email` 冲突 → [`cycms_core::Error::Conflict`]
    /// - 其余 DB 错误 → [`cycms_core::Error::Internal`]
    pub async fn update(&self, id: &str, input: UpdateUserRow) -> Result<User> {
        let existing = self.find_by_id(id).await?.ok_or_else(|| cycms_core::Error::NotFound {
            message: format!("user not found: {id}"),
        })?;

        let username = input.username.or(Some(existing.username));
        let email = input.email.or(Some(existing.email));
        let password_hash = input.password_hash.or(Some(existing.password_hash));
        let is_active = input.is_active.or(Some(existing.is_active));

        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "UPDATE users SET username = $2, email = $3, password_hash = $4, \
                     is_active = $5, updated_at = CURRENT_TIMESTAMP WHERE id = $1::UUID",
                )
                .bind(id)
                .bind(&username)
                .bind(&email)
                .bind(&password_hash)
                .bind(is_active)
                .execute(pool)
                .await
                .map_err(map_unique_violation)?;
            }
            DatabasePool::MySql(pool) => {
                sqlx::query(
                    "UPDATE users SET username = ?, email = ?, password_hash = ?, \
                     is_active = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                )
                .bind(&username)
                .bind(&email)
                .bind(&password_hash)
                .bind(is_active)
                .bind(id)
                .execute(pool)
                .await
                .map_err(map_unique_violation)?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "UPDATE users SET username = ?, email = ?, password_hash = ?, \
                     is_active = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                )
                .bind(&username)
                .bind(&email)
                .bind(&password_hash)
                .bind(is_active)
                .bind(id)
                .execute(pool)
                .await
                .map_err(map_unique_violation)?;
            }
        }

        self.find_by_id(id)
            .await?
            .ok_or_else(|| cycms_core::Error::Internal {
                message: "updated user not found on read-back".to_owned(),
                source: None,
            })
    }

    /// 删除用户。
    ///
    /// # Errors
    /// - 用户不存在 → [`cycms_core::Error::NotFound`]
    /// - DB 错误 → [`cycms_core::Error::Internal`]
    pub async fn delete(&self, id: &str) -> Result<()> {
        let affected = match self.db.as_ref() {
            DatabasePool::Postgres(pool) => sqlx::query("DELETE FROM users WHERE id = $1::UUID")
                .bind(id)
                .execute(pool)
                .await
                .map_err(AuthError::Database)?
                .rows_affected(),
            DatabasePool::MySql(pool) => sqlx::query("DELETE FROM users WHERE id = ?")
                .bind(id)
                .execute(pool)
                .await
                .map_err(AuthError::Database)?
                .rows_affected(),
            DatabasePool::Sqlite(pool) => sqlx::query("DELETE FROM users WHERE id = ?")
                .bind(id)
                .execute(pool)
                .await
                .map_err(AuthError::Database)?
                .rows_affected(),
        };

        if affected == 0 {
            return Err(cycms_core::Error::NotFound {
                message: format!("user not found: {id}"),
            });
        }

        Ok(())
    }

    /// 按 username 精确查找。
    ///
    /// # Errors
    /// 查询失败时返回 [`cycms_core::Error::Internal`]。
    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(PG_SELECT_WHERE_USERNAME)
                    .bind(username)
                    .fetch_optional(pool)
                    .await
                    .map_err(AuthError::Database)?;
                row.map(|r| pg_row_to_user(&r))
                    .transpose()
                    .map_err(Into::into)
            }
            DatabasePool::MySql(pool) => {
                let row = sqlx::query(MYSQL_SELECT_WHERE_USERNAME)
                    .bind(username)
                    .fetch_optional(pool)
                    .await
                    .map_err(AuthError::Database)?;
                row.map(|r| mysql_row_to_user(&r))
                    .transpose()
                    .map_err(Into::into)
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(SQLITE_SELECT_WHERE_USERNAME)
                    .bind(username)
                    .fetch_optional(pool)
                    .await
                    .map_err(AuthError::Database)?;
                row.map(|r| sqlite_row_to_user(&r))
                    .transpose()
                    .map_err(Into::into)
            }
        }
    }

    /// 按主键查找。
    ///
    /// # Errors
    /// 查询失败时返回 [`cycms_core::Error::Internal`]。
    pub async fn find_by_id(&self, id: &str) -> Result<Option<User>> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(PG_SELECT_WHERE_ID)
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(AuthError::Database)?;
                row.map(|r| pg_row_to_user(&r))
                    .transpose()
                    .map_err(Into::into)
            }
            DatabasePool::MySql(pool) => {
                let row = sqlx::query(MYSQL_SELECT_WHERE_ID)
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(AuthError::Database)?;
                row.map(|r| mysql_row_to_user(&r))
                    .transpose()
                    .map_err(Into::into)
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(SQLITE_SELECT_WHERE_ID)
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(AuthError::Database)?;
                row.map(|r| sqlite_row_to_user(&r))
                    .transpose()
                    .map_err(Into::into)
            }
        }
    }

    /// 拉取指定用户关联的所有角色名（role.name）。
    ///
    /// # Errors
    /// 查询失败时返回 [`cycms_core::Error::Internal`]。
    pub async fn fetch_roles(&self, user_id: &str) -> Result<Vec<String>> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => sqlx::query_scalar(
                "SELECT r.name FROM roles r \
                 INNER JOIN user_roles ur ON ur.role_id = r.id \
                 WHERE ur.user_id = $1::UUID \
                 ORDER BY r.name",
            )
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(|err| AuthError::Database(err).into()),
            DatabasePool::MySql(pool) => sqlx::query_scalar(
                "SELECT r.name FROM roles r \
                 INNER JOIN user_roles ur ON ur.role_id = r.id \
                 WHERE ur.user_id = ? \
                 ORDER BY r.name",
            )
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(|err| AuthError::Database(err).into()),
            DatabasePool::Sqlite(pool) => sqlx::query_scalar(
                "SELECT r.name FROM roles r \
                 INNER JOIN user_roles ur ON ur.role_id = r.id \
                 WHERE ur.user_id = ? \
                 ORDER BY r.name",
            )
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(|err| AuthError::Database(err).into()),
        }
    }
}

const PG_SELECT_ALL: &str = "SELECT id::TEXT AS id, username, email, password_hash, is_active, created_at, updated_at \
    FROM users ORDER BY username";
const PG_SELECT_WHERE_USERNAME: &str = "SELECT id::TEXT AS id, username, email, password_hash, is_active, created_at, updated_at \
     FROM users WHERE username = $1";
const PG_SELECT_WHERE_ID: &str = "SELECT id::TEXT AS id, username, email, password_hash, is_active, created_at, updated_at \
     FROM users WHERE id = $1::UUID";

const MYSQL_SELECT_ALL: &str = "SELECT id, username, email, password_hash, is_active, created_at, updated_at \
    FROM users ORDER BY username";
const MYSQL_SELECT_WHERE_USERNAME: &str = "SELECT id, username, email, password_hash, is_active, created_at, updated_at \
     FROM users WHERE username = ?";
const MYSQL_SELECT_WHERE_ID: &str = "SELECT id, username, email, password_hash, is_active, created_at, updated_at \
     FROM users WHERE id = ?";

const SQLITE_SELECT_ALL: &str = "SELECT id, username, email, password_hash, is_active, created_at, updated_at \
    FROM users ORDER BY username";
const SQLITE_SELECT_WHERE_USERNAME: &str = "SELECT id, username, email, password_hash, is_active, created_at, updated_at \
     FROM users WHERE username = ?";
const SQLITE_SELECT_WHERE_ID: &str = "SELECT id, username, email, password_hash, is_active, created_at, updated_at \
     FROM users WHERE id = ?";

fn pg_row_to_user(row: &PgRow) -> std::result::Result<User, AuthError> {
    Ok(User {
        id: row.try_get("id").map_err(AuthError::Database)?,
        username: row.try_get("username").map_err(AuthError::Database)?,
        email: row.try_get("email").map_err(AuthError::Database)?,
        password_hash: row.try_get("password_hash").map_err(AuthError::Database)?,
        is_active: row.try_get("is_active").map_err(AuthError::Database)?,
        created_at: row.try_get("created_at").map_err(AuthError::Database)?,
        updated_at: row.try_get("updated_at").map_err(AuthError::Database)?,
    })
}

fn mysql_row_to_user(row: &MySqlRow) -> std::result::Result<User, AuthError> {
    let created_at: NaiveDateTime = row.try_get("created_at").map_err(AuthError::Database)?;
    let updated_at: NaiveDateTime = row.try_get("updated_at").map_err(AuthError::Database)?;
    Ok(User {
        id: row.try_get("id").map_err(AuthError::Database)?,
        username: row.try_get("username").map_err(AuthError::Database)?,
        email: row.try_get("email").map_err(AuthError::Database)?,
        password_hash: row.try_get("password_hash").map_err(AuthError::Database)?,
        is_active: row.try_get("is_active").map_err(AuthError::Database)?,
        created_at: created_at.and_utc(),
        updated_at: updated_at.and_utc(),
    })
}

fn sqlite_row_to_user(row: &SqliteRow) -> std::result::Result<User, AuthError> {
    Ok(User {
        id: row.try_get("id").map_err(AuthError::Database)?,
        username: row.try_get("username").map_err(AuthError::Database)?,
        email: row.try_get("email").map_err(AuthError::Database)?,
        password_hash: row.try_get("password_hash").map_err(AuthError::Database)?,
        is_active: row.try_get("is_active").map_err(AuthError::Database)?,
        created_at: row.try_get("created_at").map_err(AuthError::Database)?,
        updated_at: row.try_get("updated_at").map_err(AuthError::Database)?,
    })
}

fn map_unique_violation(err: sqlx::Error) -> cycms_core::Error {
    if err
        .as_database_error()
        .is_some_and(sqlx::error::DatabaseError::is_unique_violation)
    {
        AuthError::UserAlreadyExists.into()
    } else {
        AuthError::Database(err).into()
    }
}
