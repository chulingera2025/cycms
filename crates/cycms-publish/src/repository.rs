//! `content_entries` 发布状态列操作。
//!
//! 只负责 `publish` / `unpublish` 两条 UPDATE，其余读取通过
//! `ContentEntryRepository::find_by_id` 完成，避免重复行映射代码。

use std::sync::Arc;

use cycms_core::Result;
use cycms_db::DatabasePool;

use crate::error::PublishError;

pub struct PublishRepository {
    db: Arc<DatabasePool>,
}

impl PublishRepository {
    #[must_use]
    pub fn new(db: Arc<DatabasePool>) -> Self {
        Self { db }
    }

    /// 将指定 Draft 实例升级为 Published：
    /// - `status = 'published'`
    /// - `published_version_id = current_version_id`
    /// - `published_at = now()`
    ///
    /// WHERE 子句含 `status = 'draft'` 保证幂等安全；返回实际影响行数。
    ///
    /// # Errors
    /// DB 故障 → [`PublishError::Database`]
    pub async fn publish_entry(&self, id: &str, actor_id: &str) -> Result<u64> {
        let affected = match self.db.as_ref() {
            cycms_db::DatabasePool::Postgres(pool) => sqlx::query(PG_PUBLISH)
                .bind(actor_id)
                .bind(id)
                .execute(pool)
                .await
                .map_err(PublishError::Database)?
                .rows_affected(),
            cycms_db::DatabasePool::MySql(pool) => sqlx::query(MYSQL_PUBLISH)
                .bind(actor_id)
                .bind(id)
                .execute(pool)
                .await
                .map_err(PublishError::Database)?
                .rows_affected(),
            cycms_db::DatabasePool::Sqlite(pool) => sqlx::query(SQLITE_PUBLISH)
                .bind(actor_id)
                .bind(id)
                .execute(pool)
                .await
                .map_err(PublishError::Database)?
                .rows_affected(),
        };
        Ok(affected)
    }

    /// 将指定 Published 实例撤回为 Draft：
    /// - `status = 'draft'`
    /// - `published_version_id = NULL`
    ///
    /// `published_at` 保留（记录最后发布时间），不清零。
    /// WHERE 子句含 `status = 'published'` 保证幂等安全。
    ///
    /// # Errors
    /// DB 故障 → [`PublishError::Database`]
    pub async fn unpublish_entry(&self, id: &str, actor_id: &str) -> Result<u64> {
        let affected = match self.db.as_ref() {
            cycms_db::DatabasePool::Postgres(pool) => sqlx::query(PG_UNPUBLISH)
                .bind(actor_id)
                .bind(id)
                .execute(pool)
                .await
                .map_err(PublishError::Database)?
                .rows_affected(),
            cycms_db::DatabasePool::MySql(pool) => sqlx::query(MYSQL_UNPUBLISH)
                .bind(actor_id)
                .bind(id)
                .execute(pool)
                .await
                .map_err(PublishError::Database)?
                .rows_affected(),
            cycms_db::DatabasePool::Sqlite(pool) => sqlx::query(SQLITE_UNPUBLISH)
                .bind(actor_id)
                .bind(id)
                .execute(pool)
                .await
                .map_err(PublishError::Database)?
                .rows_affected(),
        };
        Ok(affected)
    }
}

const PG_PUBLISH: &str = "UPDATE content_entries \
    SET status = 'published', \
        published_version_id = current_version_id, \
        published_at = now(), \
        updated_by = $1::UUID, \
        updated_at = now() \
    WHERE id = $2::UUID AND status = 'draft'";

const MYSQL_PUBLISH: &str = "UPDATE content_entries \
    SET status = 'published', \
        published_version_id = current_version_id, \
        published_at = CURRENT_TIMESTAMP(6), \
        updated_by = ?, \
        updated_at = CURRENT_TIMESTAMP(6) \
    WHERE id = ? AND status = 'draft'";

const SQLITE_PUBLISH: &str = "UPDATE content_entries \
    SET status = 'published', \
        published_version_id = current_version_id, \
        published_at = strftime('%Y-%m-%dT%H:%M:%fZ','now'), \
        updated_by = ?, \
        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
    WHERE id = ? AND status = 'draft'";

const PG_UNPUBLISH: &str = "UPDATE content_entries \
    SET status = 'draft', \
        published_version_id = NULL, \
        updated_by = $1::UUID, \
        updated_at = now() \
    WHERE id = $2::UUID AND status = 'published'";

const MYSQL_UNPUBLISH: &str = "UPDATE content_entries \
    SET status = 'draft', \
        published_version_id = NULL, \
        updated_by = ?, \
        updated_at = CURRENT_TIMESTAMP(6) \
    WHERE id = ? AND status = 'published'";

const SQLITE_UNPUBLISH: &str = "UPDATE content_entries \
    SET status = 'draft', \
        published_version_id = NULL, \
        updated_by = ?, \
        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
    WHERE id = ? AND status = 'published'";
