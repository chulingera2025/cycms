//! `content_entries` 表三方言 CRUD（任务 11 Step 3）。
//!
//! 持久化层只关注 DB I/O，不做字段级校验 / 事件发布 / 发布状态机，这些职责由
//! service 层串联（Step 6/7）。
//!
//! `current_version_id` / `published_version_id` 任务 11 暂不维护，任务 12
//! (Revision) / 13 (Publish) 接入后由对应子系统更新。

use std::str::FromStr;
use std::sync::Arc;

use chrono::NaiveDateTime;
use cycms_core::Result;
use cycms_db::{DatabasePool, DatabaseType};
use serde_json::Value;
use sqlx::Row;
use sqlx::mysql::MySqlRow;
use sqlx::postgres::PgRow;
use sqlx::sqlite::SqliteRow;
use sqlx::types::Json;
use uuid::Uuid;

use crate::error::ContentEngineError;
use crate::model::{ContentEntry, ContentStatus};
use crate::query::{ContentQuery, QueryParam, compile_list_query};

/// 插入 `content_entries` 所需的行参数。
#[derive(Debug, Clone)]
pub struct NewContentEntryRow {
    pub id: String,
    pub content_type_id: String,
    pub slug: Option<String>,
    pub status: ContentStatus,
    pub fields: Value,
    pub created_by: String,
}

/// 全量更新 `content_entries` 所需的行参数。
///
/// 不涉及 `current_version_id` / `published_version_id`；它们由任务 12/13 独立维护。
#[derive(Debug, Clone)]
pub struct UpdateContentEntryRow {
    pub slug: Option<String>,
    pub status: ContentStatus,
    pub fields: Value,
    pub updated_by: String,
}

/// 列表查询的返回结构：包含 entries 与服务端汇总的分页信息。
#[derive(Debug, Clone)]
pub struct ListQueryResult {
    pub entries: Vec<ContentEntry>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 生成新的 content entry id（UUID v4 字符串）。
#[must_use]
pub fn new_content_entry_id() -> String {
    Uuid::new_v4().to_string()
}

/// `content_entries` CRUD 门面。
pub struct ContentEntryRepository {
    db: Arc<DatabasePool>,
}

impl ContentEntryRepository {
    #[must_use]
    pub fn new(db: Arc<DatabasePool>) -> Self {
        Self { db }
    }

    /// 插入一条 `content_entries` 记录。
    ///
    /// # Errors
    /// - FK 不满足（`content_type` 或 user 不存在）→ [`ContentEngineError::Database`]
    /// - DB 故障 → [`ContentEngineError::Database`]
    /// - 读回解码失败 → [`cycms_core::Error::Internal`]
    pub async fn insert(&self, row: NewContentEntryRow) -> Result<ContentEntry> {
        let fields_json = Json(row.fields);
        let status_str = row.status.as_str().to_owned();

        let result: std::result::Result<(), sqlx::Error> = match self.db.as_ref() {
            DatabasePool::Postgres(pool) => sqlx::query(PG_INSERT)
                .bind(&row.id)
                .bind(&row.content_type_id)
                .bind(row.slug.as_deref())
                .bind(&status_str)
                .bind(&fields_json)
                .bind(&row.created_by)
                .bind(&row.created_by)
                .execute(pool)
                .await
                .map(|_| ()),
            DatabasePool::MySql(pool) => sqlx::query(MYSQL_INSERT)
                .bind(&row.id)
                .bind(&row.content_type_id)
                .bind(row.slug.as_deref())
                .bind(&status_str)
                .bind(&fields_json)
                .bind(&row.created_by)
                .bind(&row.created_by)
                .execute(pool)
                .await
                .map(|_| ()),
            DatabasePool::Sqlite(pool) => sqlx::query(SQLITE_INSERT)
                .bind(&row.id)
                .bind(&row.content_type_id)
                .bind(row.slug.as_deref())
                .bind(&status_str)
                .bind(&fields_json)
                .bind(&row.created_by)
                .bind(&row.created_by)
                .execute(pool)
                .await
                .map(|_| ()),
        };

        result.map_err(ContentEngineError::Database)?;

        self.find_by_id(&row.id)
            .await?
            .ok_or_else(|| cycms_core::Error::Internal {
                message: "inserted content entry not found on read-back".to_owned(),
                source: None,
            })
    }

    /// 全量更新内容实例。
    ///
    /// # Errors
    /// - `id` 不存在 → [`ContentEngineError::EntryNotFound`]
    /// - DB 故障 → [`ContentEngineError::Database`]
    pub async fn update(&self, id: &str, row: UpdateContentEntryRow) -> Result<ContentEntry> {
        let fields_json = Json(row.fields);
        let status_str = row.status.as_str().to_owned();

        let affected = match self.db.as_ref() {
            DatabasePool::Postgres(pool) => sqlx::query(PG_UPDATE)
                .bind(row.slug.as_deref())
                .bind(&status_str)
                .bind(&fields_json)
                .bind(&row.updated_by)
                .bind(id)
                .execute(pool)
                .await
                .map_err(ContentEngineError::Database)?
                .rows_affected(),
            DatabasePool::MySql(pool) => sqlx::query(MYSQL_UPDATE)
                .bind(row.slug.as_deref())
                .bind(&status_str)
                .bind(&fields_json)
                .bind(&row.updated_by)
                .bind(id)
                .execute(pool)
                .await
                .map_err(ContentEngineError::Database)?
                .rows_affected(),
            DatabasePool::Sqlite(pool) => sqlx::query(SQLITE_UPDATE)
                .bind(row.slug.as_deref())
                .bind(&status_str)
                .bind(&fields_json)
                .bind(&row.updated_by)
                .bind(id)
                .execute(pool)
                .await
                .map_err(ContentEngineError::Database)?
                .rows_affected(),
        };

        if affected == 0 {
            return Err(ContentEngineError::EntryNotFound(id.to_owned()).into());
        }

        self.find_by_id(id)
            .await?
            .ok_or_else(|| cycms_core::Error::Internal {
                message: "updated content entry not found on read-back".to_owned(),
                source: None,
            })
    }

    /// 标记指定实例为 `archived`（软删除）。返回更新后的实体。
    ///
    /// # Errors
    /// - `id` 不存在 → [`ContentEngineError::EntryNotFound`]
    /// - DB 故障 → [`ContentEngineError::Database`]
    pub async fn mark_archived(&self, id: &str, actor_id: &str) -> Result<ContentEntry> {
        let affected = match self.db.as_ref() {
            DatabasePool::Postgres(pool) => sqlx::query(PG_ARCHIVE)
                .bind(actor_id)
                .bind(id)
                .execute(pool)
                .await
                .map_err(ContentEngineError::Database)?
                .rows_affected(),
            DatabasePool::MySql(pool) => sqlx::query(MYSQL_ARCHIVE)
                .bind(actor_id)
                .bind(id)
                .execute(pool)
                .await
                .map_err(ContentEngineError::Database)?
                .rows_affected(),
            DatabasePool::Sqlite(pool) => sqlx::query(SQLITE_ARCHIVE)
                .bind(actor_id)
                .bind(id)
                .execute(pool)
                .await
                .map_err(ContentEngineError::Database)?
                .rows_affected(),
        };

        if affected == 0 {
            return Err(ContentEngineError::EntryNotFound(id.to_owned()).into());
        }

        self.find_by_id(id)
            .await?
            .ok_or_else(|| cycms_core::Error::Internal {
                message: "archived content entry not found on read-back".to_owned(),
                source: None,
            })
    }

    /// 物理删除（硬删除）。
    ///
    /// # Errors
    /// DB 故障 → [`ContentEngineError::Database`]。
    pub async fn delete_hard(&self, id: &str) -> Result<bool> {
        let affected = match self.db.as_ref() {
            DatabasePool::Postgres(pool) => sqlx::query(PG_DELETE)
                .bind(id)
                .execute(pool)
                .await
                .map_err(ContentEngineError::Database)?
                .rows_affected(),
            DatabasePool::MySql(pool) => sqlx::query(MYSQL_DELETE)
                .bind(id)
                .execute(pool)
                .await
                .map_err(ContentEngineError::Database)?
                .rows_affected(),
            DatabasePool::Sqlite(pool) => sqlx::query(SQLITE_DELETE)
                .bind(id)
                .execute(pool)
                .await
                .map_err(ContentEngineError::Database)?
                .rows_affected(),
        };
        Ok(affected > 0)
    }

    /// 按 `id` 读取单条实例。
    ///
    /// # Errors
    /// DB 故障 / JSON 解码失败 → [`cycms_core::Error::Internal`]。
    pub async fn find_by_id(&self, id: &str) -> Result<Option<ContentEntry>> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => sqlx::query(PG_SELECT_BY_ID)
                .bind(id)
                .fetch_optional(pool)
                .await
                .map_err(ContentEngineError::Database)?
                .map(|r| pg_row_to_entry(&r))
                .transpose()
                .map_err(Into::into),
            DatabasePool::MySql(pool) => sqlx::query(MYSQL_SELECT_BY_ID)
                .bind(id)
                .fetch_optional(pool)
                .await
                .map_err(ContentEngineError::Database)?
                .map(|r| mysql_row_to_entry(&r))
                .transpose()
                .map_err(Into::into),
            DatabasePool::Sqlite(pool) => sqlx::query(SQLITE_SELECT_BY_ID)
                .bind(id)
                .fetch_optional(pool)
                .await
                .map_err(ContentEngineError::Database)?
                .map(|r| sqlite_row_to_entry(&r))
                .transpose()
                .map_err(Into::into),
        }
    }

    /// 按 `id` + `content_type_id` 精确读取（service 按 type 路由时使用）。
    ///
    /// # Errors
    /// DB 故障 / JSON 解码失败 → [`cycms_core::Error::Internal`]。
    pub async fn find_by_id_and_type(
        &self,
        id: &str,
        content_type_id: &str,
    ) -> Result<Option<ContentEntry>> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => sqlx::query(PG_SELECT_BY_ID_AND_TYPE)
                .bind(id)
                .bind(content_type_id)
                .fetch_optional(pool)
                .await
                .map_err(ContentEngineError::Database)?
                .map(|r| pg_row_to_entry(&r))
                .transpose()
                .map_err(Into::into),
            DatabasePool::MySql(pool) => sqlx::query(MYSQL_SELECT_BY_ID_AND_TYPE)
                .bind(id)
                .bind(content_type_id)
                .fetch_optional(pool)
                .await
                .map_err(ContentEngineError::Database)?
                .map(|r| mysql_row_to_entry(&r))
                .transpose()
                .map_err(Into::into),
            DatabasePool::Sqlite(pool) => sqlx::query(SQLITE_SELECT_BY_ID_AND_TYPE)
                .bind(id)
                .bind(content_type_id)
                .fetch_optional(pool)
                .await
                .map_err(ContentEngineError::Database)?
                .map(|r| sqlite_row_to_entry(&r))
                .transpose()
                .map_err(Into::into),
        }
    }

    /// 统计指定 `content_type_id` 下的实例数，用于判断 `Single` 类型是否已有条目。
    ///
    /// # Errors
    /// DB 故障 → [`ContentEngineError::Database`]。
    pub async fn count_by_type(&self, content_type_id: &str) -> Result<i64> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(PG_COUNT_BY_TYPE)
                    .bind(content_type_id)
                    .fetch_one(pool)
                    .await
                    .map_err(ContentEngineError::Database)?;
                row.try_get::<i64, _>(0)
                    .map_err(|e| ContentEngineError::Database(e).into())
            }
            DatabasePool::MySql(pool) => {
                let row = sqlx::query(MYSQL_COUNT_BY_TYPE)
                    .bind(content_type_id)
                    .fetch_one(pool)
                    .await
                    .map_err(ContentEngineError::Database)?;
                row.try_get::<i64, _>(0)
                    .map_err(|e| ContentEngineError::Database(e).into())
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(SQLITE_COUNT_BY_TYPE)
                    .bind(content_type_id)
                    .fetch_one(pool)
                    .await
                    .map_err(ContentEngineError::Database)?;
                row.try_get::<i64, _>(0)
                    .map_err(|e| ContentEngineError::Database(e).into())
            }
        }
    }

    /// 列出指定 content type 的实例，按 [`ContentQuery`] 的过滤 / 排序 / 分页执行。
    ///
    /// 同一 plan 内会做两次 SQL：一次 `COUNT(*)` 获取总数，一次按分页读取实体。
    ///
    /// # Errors
    /// - 查询编译失败 → [`ContentEngineError::InvalidQuery`]
    /// - DB 故障 / 解码失败 → [`ContentEngineError::Database`]
    pub async fn list(
        &self,
        content_type_id: &str,
        query: &ContentQuery,
        default_page_size: u64,
        max_page_size: u64,
    ) -> Result<ListQueryResult> {
        let db_type = self.db.db_type();
        let plan = compile_list_query(
            db_type,
            content_type_id,
            query,
            default_page_size,
            max_page_size,
        )?;
        let count_sql = format!(
            "SELECT COUNT(*) FROM content_entries WHERE {}",
            plan.where_sql
        );
        let prefix = match db_type {
            DatabaseType::Postgres => PG_SELECT_LIST_PREFIX,
            DatabaseType::MySql => MYSQL_SELECT_LIST_PREFIX,
            DatabaseType::Sqlite => SQLITE_SELECT_LIST_PREFIX,
        };
        let list_sql = format!(
            "{prefix} WHERE {} ORDER BY {} LIMIT {} OFFSET {}",
            plan.where_sql, plan.order_sql, plan.limit_placeholder, plan.offset_placeholder
        );

        let (entries, total) = match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                let mut count_q = sqlx::query(&count_sql);
                for p in &plan.params[..plan.content_param_count] {
                    count_q = bind_pg_param(count_q, p);
                }
                let total: i64 = count_q
                    .fetch_one(pool)
                    .await
                    .map_err(ContentEngineError::Database)?
                    .try_get(0)
                    .map_err(ContentEngineError::Database)?;

                let mut list_q = sqlx::query(&list_sql);
                for p in &plan.params {
                    list_q = bind_pg_param(list_q, p);
                }
                let rows = list_q
                    .fetch_all(pool)
                    .await
                    .map_err(ContentEngineError::Database)?;
                let entries = rows
                    .iter()
                    .map(pg_row_to_entry)
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                (entries, total)
            }
            DatabasePool::MySql(pool) => {
                let mut count_q = sqlx::query(&count_sql);
                for p in &plan.params[..plan.content_param_count] {
                    count_q = bind_mysql_param(count_q, p);
                }
                let total: i64 = count_q
                    .fetch_one(pool)
                    .await
                    .map_err(ContentEngineError::Database)?
                    .try_get(0)
                    .map_err(ContentEngineError::Database)?;

                let mut list_q = sqlx::query(&list_sql);
                for p in &plan.params {
                    list_q = bind_mysql_param(list_q, p);
                }
                let rows = list_q
                    .fetch_all(pool)
                    .await
                    .map_err(ContentEngineError::Database)?;
                let entries = rows
                    .iter()
                    .map(mysql_row_to_entry)
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                (entries, total)
            }
            DatabasePool::Sqlite(pool) => {
                let mut count_q = sqlx::query(&count_sql);
                for p in &plan.params[..plan.content_param_count] {
                    count_q = bind_sqlite_param(count_q, p);
                }
                let total: i64 = count_q
                    .fetch_one(pool)
                    .await
                    .map_err(ContentEngineError::Database)?
                    .try_get(0)
                    .map_err(ContentEngineError::Database)?;

                let mut list_q = sqlx::query(&list_sql);
                for p in &plan.params {
                    list_q = bind_sqlite_param(list_q, p);
                }
                let rows = list_q
                    .fetch_all(pool)
                    .await
                    .map_err(ContentEngineError::Database)?;
                let entries = rows
                    .iter()
                    .map(sqlite_row_to_entry)
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                (entries, total)
            }
        };

        Ok(ListQueryResult {
            entries,
            total: u64::try_from(total).unwrap_or(0),
            page: plan.page,
            page_size: plan.page_size,
        })
    }
}

const PG_INSERT: &str = "INSERT INTO content_entries \
    (id, content_type_id, slug, status, fields, created_by, updated_by) \
    VALUES ($1::UUID, $2::UUID, $3, $4, $5, $6::UUID, $7::UUID)";
const MYSQL_INSERT: &str = "INSERT INTO content_entries \
    (id, content_type_id, slug, status, fields, created_by, updated_by) \
    VALUES (?, ?, ?, ?, ?, ?, ?)";
const SQLITE_INSERT: &str = "INSERT INTO content_entries \
    (id, content_type_id, slug, status, fields, created_by, updated_by) \
    VALUES (?, ?, ?, ?, ?, ?, ?)";

const PG_UPDATE: &str = "UPDATE content_entries \
    SET slug = $1, status = $2, fields = $3, updated_by = $4::UUID, updated_at = now() \
    WHERE id = $5::UUID";
const MYSQL_UPDATE: &str = "UPDATE content_entries \
    SET slug = ?, status = ?, fields = ?, updated_by = ?, updated_at = CURRENT_TIMESTAMP(6) \
    WHERE id = ?";
const SQLITE_UPDATE: &str = "UPDATE content_entries \
    SET slug = ?, status = ?, fields = ?, updated_by = ?, \
    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
    WHERE id = ?";

const PG_ARCHIVE: &str = "UPDATE content_entries \
    SET status = 'archived', updated_by = $1::UUID, updated_at = now() \
    WHERE id = $2::UUID";
const MYSQL_ARCHIVE: &str = "UPDATE content_entries \
    SET status = 'archived', updated_by = ?, updated_at = CURRENT_TIMESTAMP(6) \
    WHERE id = ?";
const SQLITE_ARCHIVE: &str = "UPDATE content_entries \
    SET status = 'archived', updated_by = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
    WHERE id = ?";

const PG_DELETE: &str = "DELETE FROM content_entries WHERE id = $1::UUID";
const MYSQL_DELETE: &str = "DELETE FROM content_entries WHERE id = ?";
const SQLITE_DELETE: &str = "DELETE FROM content_entries WHERE id = ?";

const PG_SELECT_BY_ID: &str = "SELECT id::TEXT AS id, content_type_id::TEXT AS content_type_id, \
    slug, status, current_version_id::TEXT AS current_version_id, \
    published_version_id::TEXT AS published_version_id, fields, \
    created_by::TEXT AS created_by, updated_by::TEXT AS updated_by, \
    created_at, updated_at, published_at FROM content_entries WHERE id = $1::UUID";
const MYSQL_SELECT_BY_ID: &str = "SELECT id, content_type_id, slug, status, \
    current_version_id, published_version_id, fields, created_by, updated_by, \
    created_at, updated_at, published_at FROM content_entries WHERE id = ?";
const SQLITE_SELECT_BY_ID: &str = "SELECT id, content_type_id, slug, status, \
    current_version_id, published_version_id, fields, created_by, updated_by, \
    created_at, updated_at, published_at FROM content_entries WHERE id = ?";

const PG_SELECT_BY_ID_AND_TYPE: &str = "SELECT id::TEXT AS id, content_type_id::TEXT AS content_type_id, \
    slug, status, current_version_id::TEXT AS current_version_id, \
    published_version_id::TEXT AS published_version_id, fields, \
    created_by::TEXT AS created_by, updated_by::TEXT AS updated_by, \
    created_at, updated_at, published_at FROM content_entries \
    WHERE id = $1::UUID AND content_type_id = $2::UUID";
const MYSQL_SELECT_BY_ID_AND_TYPE: &str = "SELECT id, content_type_id, slug, status, \
    current_version_id, published_version_id, fields, created_by, updated_by, \
    created_at, updated_at, published_at FROM content_entries \
    WHERE id = ? AND content_type_id = ?";
const SQLITE_SELECT_BY_ID_AND_TYPE: &str = "SELECT id, content_type_id, slug, status, \
    current_version_id, published_version_id, fields, created_by, updated_by, \
    created_at, updated_at, published_at FROM content_entries \
    WHERE id = ? AND content_type_id = ?";

const PG_COUNT_BY_TYPE: &str =
    "SELECT COUNT(*) FROM content_entries WHERE content_type_id = $1::UUID";
const MYSQL_COUNT_BY_TYPE: &str = "SELECT COUNT(*) FROM content_entries WHERE content_type_id = ?";
const SQLITE_COUNT_BY_TYPE: &str = "SELECT COUNT(*) FROM content_entries WHERE content_type_id = ?";

const PG_SELECT_LIST_PREFIX: &str = "SELECT id::TEXT AS id, content_type_id::TEXT AS content_type_id, \
    slug, status, current_version_id::TEXT AS current_version_id, \
    published_version_id::TEXT AS published_version_id, fields, \
    created_by::TEXT AS created_by, updated_by::TEXT AS updated_by, \
    created_at, updated_at, published_at FROM content_entries";
const MYSQL_SELECT_LIST_PREFIX: &str = "SELECT id, content_type_id, slug, status, \
    current_version_id, published_version_id, fields, created_by, updated_by, \
    created_at, updated_at, published_at FROM content_entries";
const SQLITE_SELECT_LIST_PREFIX: &str = MYSQL_SELECT_LIST_PREFIX;

fn bind_pg_param<'q>(
    q: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    p: &QueryParam,
) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
    match p {
        QueryParam::Text(s) => q.bind(s.clone()),
        QueryParam::Int(i) => q.bind(*i),
        QueryParam::Float(f) => q.bind(*f),
        QueryParam::Bool(b) => q.bind(*b),
    }
}

fn bind_mysql_param<'q>(
    q: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
    p: &QueryParam,
) -> sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments> {
    match p {
        QueryParam::Text(s) => q.bind(s.clone()),
        QueryParam::Int(i) => q.bind(*i),
        QueryParam::Float(f) => q.bind(*f),
        QueryParam::Bool(b) => q.bind(*b),
    }
}

fn bind_sqlite_param<'q>(
    q: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
    p: &QueryParam,
) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
    match p {
        QueryParam::Text(s) => q.bind(s.clone()),
        QueryParam::Int(i) => q.bind(*i),
        QueryParam::Float(f) => q.bind(*f),
        QueryParam::Bool(b) => q.bind(*b),
    }
}

fn pg_row_to_entry(row: &PgRow) -> std::result::Result<ContentEntry, ContentEngineError> {
    let status_raw: String = row
        .try_get("status")
        .map_err(ContentEngineError::Database)?;
    let status = ContentStatus::from_str(&status_raw)?;
    let fields: Json<Value> = row
        .try_get("fields")
        .map_err(ContentEngineError::Database)?;
    Ok(ContentEntry {
        id: row.try_get("id").map_err(ContentEngineError::Database)?,
        content_type_id: row
            .try_get("content_type_id")
            .map_err(ContentEngineError::Database)?,
        content_type_api_id: String::new(),
        slug: row.try_get("slug").map_err(ContentEngineError::Database)?,
        status,
        current_version_id: row
            .try_get("current_version_id")
            .map_err(ContentEngineError::Database)?,
        published_version_id: row
            .try_get("published_version_id")
            .map_err(ContentEngineError::Database)?,
        fields: fields.0,
        created_by: row
            .try_get("created_by")
            .map_err(ContentEngineError::Database)?,
        updated_by: row
            .try_get("updated_by")
            .map_err(ContentEngineError::Database)?,
        created_at: row
            .try_get("created_at")
            .map_err(ContentEngineError::Database)?,
        updated_at: row
            .try_get("updated_at")
            .map_err(ContentEngineError::Database)?,
        published_at: row
            .try_get("published_at")
            .map_err(ContentEngineError::Database)?,
        populated: None,
    })
}

fn mysql_row_to_entry(row: &MySqlRow) -> std::result::Result<ContentEntry, ContentEngineError> {
    let status_raw: String = row
        .try_get("status")
        .map_err(ContentEngineError::Database)?;
    let status = ContentStatus::from_str(&status_raw)?;
    let fields: Json<Value> = row
        .try_get("fields")
        .map_err(ContentEngineError::Database)?;
    let created_at: NaiveDateTime = row
        .try_get("created_at")
        .map_err(ContentEngineError::Database)?;
    let updated_at: NaiveDateTime = row
        .try_get("updated_at")
        .map_err(ContentEngineError::Database)?;
    let published_at: Option<NaiveDateTime> = row
        .try_get("published_at")
        .map_err(ContentEngineError::Database)?;
    Ok(ContentEntry {
        id: row.try_get("id").map_err(ContentEngineError::Database)?,
        content_type_id: row
            .try_get("content_type_id")
            .map_err(ContentEngineError::Database)?,
        content_type_api_id: String::new(),
        slug: row.try_get("slug").map_err(ContentEngineError::Database)?,
        status,
        current_version_id: row
            .try_get("current_version_id")
            .map_err(ContentEngineError::Database)?,
        published_version_id: row
            .try_get("published_version_id")
            .map_err(ContentEngineError::Database)?,
        fields: fields.0,
        created_by: row
            .try_get("created_by")
            .map_err(ContentEngineError::Database)?,
        updated_by: row
            .try_get("updated_by")
            .map_err(ContentEngineError::Database)?,
        created_at: created_at.and_utc(),
        updated_at: updated_at.and_utc(),
        published_at: published_at.map(|d| d.and_utc()),
        populated: None,
    })
}

fn sqlite_row_to_entry(row: &SqliteRow) -> std::result::Result<ContentEntry, ContentEngineError> {
    let status_raw: String = row
        .try_get("status")
        .map_err(ContentEngineError::Database)?;
    let status = ContentStatus::from_str(&status_raw)?;
    let fields: Json<Value> = row
        .try_get("fields")
        .map_err(ContentEngineError::Database)?;
    Ok(ContentEntry {
        id: row.try_get("id").map_err(ContentEngineError::Database)?,
        content_type_id: row
            .try_get("content_type_id")
            .map_err(ContentEngineError::Database)?,
        content_type_api_id: String::new(),
        slug: row.try_get("slug").map_err(ContentEngineError::Database)?,
        status,
        current_version_id: row
            .try_get("current_version_id")
            .map_err(ContentEngineError::Database)?,
        published_version_id: row
            .try_get("published_version_id")
            .map_err(ContentEngineError::Database)?,
        fields: fields.0,
        created_by: row
            .try_get("created_by")
            .map_err(ContentEngineError::Database)?,
        updated_by: row
            .try_get("updated_by")
            .map_err(ContentEngineError::Database)?,
        created_at: row
            .try_get("created_at")
            .map_err(ContentEngineError::Database)?,
        updated_at: row
            .try_get("updated_at")
            .map_err(ContentEngineError::Database)?,
        published_at: row
            .try_get("published_at")
            .map_err(ContentEngineError::Database)?,
        populated: None,
    })
}
