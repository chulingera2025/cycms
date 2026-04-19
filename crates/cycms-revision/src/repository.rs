//! `content_revisions` 表三方言 CRUD。

use std::sync::Arc;

use chrono::NaiveDateTime;
use cycms_core::Result;
use cycms_db::DatabasePool;
use serde_json::Value;
use sqlx::Row;
use sqlx::mysql::MySqlRow;
use sqlx::postgres::PgRow;
use sqlx::sqlite::SqliteRow;
use sqlx::types::Json;
use uuid::Uuid;

use crate::error::RevisionError;
use crate::model::Revision;

/// 生成新的 revision id（UUID v4 字符串）。
#[must_use]
pub fn new_revision_id() -> String {
    Uuid::new_v4().to_string()
}

pub struct RevisionRepository {
    db: Arc<DatabasePool>,
}

impl RevisionRepository {
    #[must_use]
    pub fn new(db: Arc<DatabasePool>) -> Self {
        Self { db }
    }

    /// 获取指定 entry 当前最大 `version_number`，不存在时返回 0。
    ///
    /// # Errors
    /// DB 故障 → [`RevisionError::Database`]
    pub async fn max_version(&self, entry_id: &str) -> Result<i64> {
        let n: i64 = match self.db.as_ref() {
            DatabasePool::Postgres(pool) => sqlx::query_scalar(PG_MAX_VERSION)
                .bind(entry_id)
                .fetch_one(pool)
                .await
                .map_err(RevisionError::Database)?,
            DatabasePool::MySql(pool) => sqlx::query_scalar(MYSQL_MAX_VERSION)
                .bind(entry_id)
                .fetch_one(pool)
                .await
                .map_err(RevisionError::Database)?,
            DatabasePool::Sqlite(pool) => sqlx::query_scalar(SQLITE_MAX_VERSION)
                .bind(entry_id)
                .fetch_one(pool)
                .await
                .map_err(RevisionError::Database)?,
        };
        Ok(n)
    }

    /// 插入一条 `content_revisions` 行，返回插入的版本。
    ///
    /// # Errors
    /// - FK 不满足 → [`RevisionError::Database`]
    /// - DB 故障 → [`RevisionError::Database`]
    pub async fn insert(
        &self,
        id: &str,
        entry_id: &str,
        version_number: i64,
        snapshot: &Value,
        change_summary: Option<&str>,
        created_by: &str,
    ) -> Result<Revision> {
        let snapshot_json = Json(snapshot);
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => sqlx::query(PG_INSERT)
                .bind(id)
                .bind(entry_id)
                .bind(version_number)
                .bind(snapshot_json)
                .bind(change_summary)
                .bind(created_by)
                .execute(pool)
                .await
                .map_err(RevisionError::Database)
                .map(|_| ()),
            DatabasePool::MySql(pool) => sqlx::query(MYSQL_INSERT)
                .bind(id)
                .bind(entry_id)
                .bind(version_number)
                .bind(snapshot_json)
                .bind(change_summary)
                .bind(created_by)
                .execute(pool)
                .await
                .map_err(RevisionError::Database)
                .map(|_| ()),
            DatabasePool::Sqlite(pool) => sqlx::query(SQLITE_INSERT)
                .bind(id)
                .bind(entry_id)
                .bind(version_number)
                .bind(snapshot_json)
                .bind(change_summary)
                .bind(created_by)
                .execute(pool)
                .await
                .map_err(RevisionError::Database)
                .map(|_| ()),
        }?;

        self.find_by_entry_and_version(entry_id, version_number)
            .await?
            .ok_or_else(|| cycms_core::Error::Internal {
                message: "inserted revision not found on read-back".to_owned(),
                source: None,
            })
    }

    /// 按 `entry_id` + `version_number` 精确查找版本。
    ///
    /// # Errors
    /// DB 故障 → [`RevisionError::Database`]
    pub async fn find_by_entry_and_version(
        &self,
        entry_id: &str,
        version_number: i64,
    ) -> Result<Option<Revision>> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => sqlx::query(PG_SELECT_BY_ENTRY_VERSION)
                .bind(entry_id)
                .bind(version_number)
                .fetch_optional(pool)
                .await
                .map_err(RevisionError::Database)?
                .map(|r| pg_row_to_revision(&r))
                .transpose()
                .map_err(Into::into),
            DatabasePool::MySql(pool) => sqlx::query(MYSQL_SELECT_BY_ENTRY_VERSION)
                .bind(entry_id)
                .bind(version_number)
                .fetch_optional(pool)
                .await
                .map_err(RevisionError::Database)?
                .map(|r| mysql_row_to_revision(&r))
                .transpose()
                .map_err(Into::into),
            DatabasePool::Sqlite(pool) => sqlx::query(SQLITE_SELECT_BY_ENTRY_VERSION)
                .bind(entry_id)
                .bind(version_number)
                .fetch_optional(pool)
                .await
                .map_err(RevisionError::Database)?
                .map(|r| sqlite_row_to_revision(&r))
                .transpose()
                .map_err(Into::into),
        }
    }

    /// 按 `entry_id` 分页列出版本（按 `version_number DESC`）。
    ///
    /// # Errors
    /// DB 故障 → [`RevisionError::Database`]
    pub async fn list_by_entry(
        &self,
        entry_id: &str,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<Revision>, u64)> {
        let offset = page.saturating_sub(1).saturating_mul(page_size);
        let (revisions, total) = match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                let total: i64 = sqlx::query_scalar(PG_COUNT_BY_ENTRY)
                    .bind(entry_id)
                    .fetch_one(pool)
                    .await
                    .map_err(RevisionError::Database)?;
                let rows = sqlx::query(PG_LIST_BY_ENTRY)
                    .bind(entry_id)
                    .bind(i64::try_from(page_size).unwrap_or(i64::MAX))
                    .bind(i64::try_from(offset).unwrap_or(0))
                    .fetch_all(pool)
                    .await
                    .map_err(RevisionError::Database)?;
                let revisions = rows
                    .iter()
                    .map(pg_row_to_revision)
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                (revisions, total)
            }
            DatabasePool::MySql(pool) => {
                let total: i64 = sqlx::query_scalar(MYSQL_COUNT_BY_ENTRY)
                    .bind(entry_id)
                    .fetch_one(pool)
                    .await
                    .map_err(RevisionError::Database)?;
                let rows = sqlx::query(MYSQL_LIST_BY_ENTRY)
                    .bind(entry_id)
                    .bind(i64::try_from(page_size).unwrap_or(i64::MAX))
                    .bind(i64::try_from(offset).unwrap_or(0))
                    .fetch_all(pool)
                    .await
                    .map_err(RevisionError::Database)?;
                let revisions = rows
                    .iter()
                    .map(mysql_row_to_revision)
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                (revisions, total)
            }
            DatabasePool::Sqlite(pool) => {
                let total: i64 = sqlx::query_scalar(SQLITE_COUNT_BY_ENTRY)
                    .bind(entry_id)
                    .fetch_one(pool)
                    .await
                    .map_err(RevisionError::Database)?;
                let rows = sqlx::query(SQLITE_LIST_BY_ENTRY)
                    .bind(entry_id)
                    .bind(i64::try_from(page_size).unwrap_or(i64::MAX))
                    .bind(i64::try_from(offset).unwrap_or(0))
                    .fetch_all(pool)
                    .await
                    .map_err(RevisionError::Database)?;
                let revisions = rows
                    .iter()
                    .map(sqlite_row_to_revision)
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                (revisions, total)
            }
        };
        Ok((revisions, u64::try_from(total).unwrap_or(0)))
    }

    /// 更新 `content_entries.current_version_id`，在创建 revision 后同步调用。
    ///
    /// # Errors
    /// DB 故障 → [`RevisionError::Database`]
    pub async fn update_entry_current_version(
        &self,
        entry_id: &str,
        revision_id: &str,
    ) -> Result<()> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => sqlx::query(PG_UPDATE_ENTRY_CURRENT_VERSION)
                .bind(revision_id)
                .bind(entry_id)
                .execute(pool)
                .await
                .map_err(RevisionError::Database)
                .map(|_| ()),
            DatabasePool::MySql(pool) => sqlx::query(MYSQL_UPDATE_ENTRY_CURRENT_VERSION)
                .bind(revision_id)
                .bind(entry_id)
                .execute(pool)
                .await
                .map_err(RevisionError::Database)
                .map(|_| ()),
            DatabasePool::Sqlite(pool) => sqlx::query(SQLITE_UPDATE_ENTRY_CURRENT_VERSION)
                .bind(revision_id)
                .bind(entry_id)
                .execute(pool)
                .await
                .map_err(RevisionError::Database)
                .map(|_| ()),
        }?;
        Ok(())
    }

    /// 更新 `content_entries.fields`（rollback 时使用）。
    ///
    /// # Errors
    /// DB 故障 → [`RevisionError::Database`]
    pub async fn update_entry_fields(&self, entry_id: &str, fields: &Value) -> Result<()> {
        let json = Json(fields);
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => sqlx::query(PG_UPDATE_ENTRY_FIELDS)
                .bind(json)
                .bind(entry_id)
                .execute(pool)
                .await
                .map_err(RevisionError::Database)
                .map(|_| ()),
            DatabasePool::MySql(pool) => sqlx::query(MYSQL_UPDATE_ENTRY_FIELDS)
                .bind(json)
                .bind(entry_id)
                .execute(pool)
                .await
                .map_err(RevisionError::Database)
                .map(|_| ()),
            DatabasePool::Sqlite(pool) => sqlx::query(SQLITE_UPDATE_ENTRY_FIELDS)
                .bind(json)
                .bind(entry_id)
                .execute(pool)
                .await
                .map_err(RevisionError::Database)
                .map(|_| ()),
        }?;
        Ok(())
    }
}

// ── SQL 常量 ────────────────────────────────────────────────────────────────

const PG_MAX_VERSION: &str = "SELECT COALESCE(MAX(version_number), 0) \
    FROM content_revisions WHERE content_entry_id = $1::UUID";
const MYSQL_MAX_VERSION: &str = "SELECT COALESCE(MAX(version_number), 0) \
    FROM content_revisions WHERE content_entry_id = ?";
const SQLITE_MAX_VERSION: &str = "SELECT COALESCE(MAX(version_number), 0) \
    FROM content_revisions WHERE content_entry_id = ?";

const PG_INSERT: &str = "INSERT INTO content_revisions \
    (id, content_entry_id, version_number, snapshot, change_summary, created_by) \
    VALUES ($1::UUID, $2::UUID, $3, $4, $5, $6::UUID)";
const MYSQL_INSERT: &str = "INSERT INTO content_revisions \
    (id, content_entry_id, version_number, snapshot, change_summary, created_by) \
    VALUES (?, ?, ?, ?, ?, ?)";
const SQLITE_INSERT: &str = MYSQL_INSERT;

const PG_SELECT_BY_ENTRY_VERSION: &str = "SELECT id::TEXT AS id, \
    content_entry_id::TEXT AS content_entry_id, version_number, snapshot, \
    change_summary, created_by::TEXT AS created_by, created_at \
    FROM content_revisions WHERE content_entry_id = $1::UUID AND version_number = $2";
const MYSQL_SELECT_BY_ENTRY_VERSION: &str = "SELECT id, content_entry_id, version_number, \
    snapshot, change_summary, created_by, created_at \
    FROM content_revisions WHERE content_entry_id = ? AND version_number = ?";
const SQLITE_SELECT_BY_ENTRY_VERSION: &str = MYSQL_SELECT_BY_ENTRY_VERSION;

const PG_COUNT_BY_ENTRY: &str =
    "SELECT COUNT(*) FROM content_revisions WHERE content_entry_id = $1::UUID";
const MYSQL_COUNT_BY_ENTRY: &str =
    "SELECT COUNT(*) FROM content_revisions WHERE content_entry_id = ?";
const SQLITE_COUNT_BY_ENTRY: &str = MYSQL_COUNT_BY_ENTRY;

const PG_LIST_BY_ENTRY: &str = "SELECT id::TEXT AS id, \
    content_entry_id::TEXT AS content_entry_id, version_number, snapshot, \
    change_summary, created_by::TEXT AS created_by, created_at \
    FROM content_revisions WHERE content_entry_id = $1::UUID \
    ORDER BY version_number DESC LIMIT $2 OFFSET $3";
const MYSQL_LIST_BY_ENTRY: &str = "SELECT id, content_entry_id, version_number, \
    snapshot, change_summary, created_by, created_at \
    FROM content_revisions WHERE content_entry_id = ? \
    ORDER BY version_number DESC LIMIT ? OFFSET ?";
const SQLITE_LIST_BY_ENTRY: &str = MYSQL_LIST_BY_ENTRY;

const PG_UPDATE_ENTRY_CURRENT_VERSION: &str =
    "UPDATE content_entries SET current_version_id = $1::UUID WHERE id = $2::UUID";
const MYSQL_UPDATE_ENTRY_CURRENT_VERSION: &str =
    "UPDATE content_entries SET current_version_id = ? WHERE id = ?";
const SQLITE_UPDATE_ENTRY_CURRENT_VERSION: &str = MYSQL_UPDATE_ENTRY_CURRENT_VERSION;

const PG_UPDATE_ENTRY_FIELDS: &str =
    "UPDATE content_entries SET fields = $1, updated_at = now() WHERE id = $2::UUID";
const MYSQL_UPDATE_ENTRY_FIELDS: &str =
    "UPDATE content_entries SET fields = ?, updated_at = CURRENT_TIMESTAMP(6) WHERE id = ?";
const SQLITE_UPDATE_ENTRY_FIELDS: &str = "UPDATE content_entries SET fields = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?";

// ── 行转换辅助函数 ──────────────────────────────────────────────────────────

fn pg_row_to_revision(row: &PgRow) -> std::result::Result<Revision, RevisionError> {
    let snapshot: Json<Value> = row.try_get("snapshot").map_err(RevisionError::Database)?;
    Ok(Revision {
        id: row.try_get("id").map_err(RevisionError::Database)?,
        content_entry_id: row
            .try_get("content_entry_id")
            .map_err(RevisionError::Database)?,
        version_number: row
            .try_get("version_number")
            .map_err(RevisionError::Database)?,
        snapshot: snapshot.0,
        change_summary: row
            .try_get("change_summary")
            .map_err(RevisionError::Database)?,
        created_by: row.try_get("created_by").map_err(RevisionError::Database)?,
        created_at: row.try_get("created_at").map_err(RevisionError::Database)?,
    })
}

fn mysql_row_to_revision(row: &MySqlRow) -> std::result::Result<Revision, RevisionError> {
    let snapshot: Json<Value> = row.try_get("snapshot").map_err(RevisionError::Database)?;
    let created_at: NaiveDateTime = row.try_get("created_at").map_err(RevisionError::Database)?;
    Ok(Revision {
        id: row.try_get("id").map_err(RevisionError::Database)?,
        content_entry_id: row
            .try_get("content_entry_id")
            .map_err(RevisionError::Database)?,
        version_number: row
            .try_get("version_number")
            .map_err(RevisionError::Database)?,
        snapshot: snapshot.0,
        change_summary: row
            .try_get("change_summary")
            .map_err(RevisionError::Database)?,
        created_by: row.try_get("created_by").map_err(RevisionError::Database)?,
        created_at: created_at.and_utc(),
    })
}

fn sqlite_row_to_revision(row: &SqliteRow) -> std::result::Result<Revision, RevisionError> {
    let snapshot: Json<Value> = row.try_get("snapshot").map_err(RevisionError::Database)?;
    Ok(Revision {
        id: row.try_get("id").map_err(RevisionError::Database)?,
        content_entry_id: row
            .try_get("content_entry_id")
            .map_err(RevisionError::Database)?,
        version_number: row
            .try_get("version_number")
            .map_err(RevisionError::Database)?,
        snapshot: snapshot.0,
        change_summary: row
            .try_get("change_summary")
            .map_err(RevisionError::Database)?,
        created_by: row.try_get("created_by").map_err(RevisionError::Database)?,
        created_at: row.try_get("created_at").map_err(RevisionError::Database)?,
    })
}
