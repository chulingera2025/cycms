use std::sync::Arc;

use chrono::{DateTime, Utc};
use cycms_core::Result;
use cycms_db::DatabasePool;

use crate::error::AuthError;

/// 撤销 token 黑名单仓储。由于 v0.1 不引入 Redis，这里直接命中 DB；
/// `revoked_tokens` 表仅有主键 jti 查询路径，开销可接受。
pub struct RevokedTokenRepository {
    db: Arc<DatabasePool>,
}

impl RevokedTokenRepository {
    #[must_use]
    pub fn new(db: Arc<DatabasePool>) -> Self {
        Self { db }
    }

    /// 判定 jti 是否已吊销。
    ///
    /// # Errors
    /// 查询失败时返回 [`cycms_core::Error::Internal`]。
    pub async fn is_revoked(&self, jti: &str) -> Result<bool> {
        let sql = "SELECT 1 FROM revoked_tokens WHERE jti = ?";
        let pg_sql = "SELECT 1 FROM revoked_tokens WHERE jti = $1";
        let found: Option<i32> = match self.db.as_ref() {
            DatabasePool::Postgres(pool) => sqlx::query_scalar(pg_sql)
                .bind(jti)
                .fetch_optional(pool)
                .await
                .map_err(AuthError::Database)?,
            DatabasePool::MySql(pool) => sqlx::query_scalar(sql)
                .bind(jti)
                .fetch_optional(pool)
                .await
                .map_err(AuthError::Database)?,
            DatabasePool::Sqlite(pool) => sqlx::query_scalar(sql)
                .bind(jti)
                .fetch_optional(pool)
                .await
                .map_err(AuthError::Database)?,
        };
        Ok(found.is_some())
    }

    /// 写入一条吊销记录。若 jti 已存在则返回错误（不做 upsert；重入视为异常）。
    ///
    /// # Errors
    /// 插入失败时返回 [`cycms_core::Error::Internal`]。
    pub async fn revoke(
        &self,
        jti: &str,
        expires_at: DateTime<Utc>,
        reason: &str,
    ) -> Result<()> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO revoked_tokens (jti, expires_at, reason) VALUES ($1, $2, $3)",
                )
                .bind(jti)
                .bind(expires_at)
                .bind(reason)
                .execute(pool)
                .await
                .map_err(AuthError::Database)?;
            }
            DatabasePool::MySql(pool) => {
                sqlx::query(
                    "INSERT INTO revoked_tokens (jti, expires_at, reason) VALUES (?, ?, ?)",
                )
                .bind(jti)
                .bind(expires_at.naive_utc())
                .bind(reason)
                .execute(pool)
                .await
                .map_err(AuthError::Database)?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO revoked_tokens (jti, expires_at, reason) VALUES (?, ?, ?)",
                )
                .bind(jti)
                .bind(expires_at.to_rfc3339())
                .bind(reason)
                .execute(pool)
                .await
                .map_err(AuthError::Database)?;
            }
        }
        Ok(())
    }
}
