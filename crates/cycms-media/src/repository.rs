use std::fmt::Write as _;
use std::sync::Arc;

use cycms_db::DatabasePool;
use serde_json::Value;
use sqlx::Row;
use sqlx::types::Json;

use crate::error::MediaError;
use crate::model::{MediaAsset, MediaOrderDir, MediaQuery, PaginatedMedia};

// ─── INSERT ────────────────────────────────────────────────────────────────

const PG_INSERT: &str = "INSERT INTO media_assets \
    (id, filename, original_filename, mime_type, size, storage_path, metadata, uploaded_by) \
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)";

const MYSQL_INSERT: &str = "INSERT INTO media_assets \
    (id, filename, original_filename, mime_type, size, storage_path, metadata, uploaded_by) \
    VALUES (?, ?, ?, ?, ?, ?, ?, ?)";

const SQLITE_INSERT: &str = "INSERT INTO media_assets \
    (id, filename, original_filename, mime_type, size, storage_path, metadata, uploaded_by) \
    VALUES (?, ?, ?, ?, ?, ?, ?, ?)";

// ─── SELECT BY ID ──────────────────────────────────────────────────────────

const PG_SELECT_BY_ID: &str = "SELECT id, filename, original_filename, mime_type, size, \
    storage_path, metadata, uploaded_by, created_at \
    FROM media_assets WHERE id = $1";

const MYSQL_SELECT_BY_ID: &str = "SELECT id, filename, original_filename, mime_type, size, \
    storage_path, metadata, uploaded_by, created_at \
    FROM media_assets WHERE id = ?";

const SQLITE_SELECT_BY_ID: &str = "SELECT id, filename, original_filename, mime_type, size, \
    storage_path, metadata, uploaded_by, created_at \
    FROM media_assets WHERE id = ?";

// ─── DELETE ────────────────────────────────────────────────────────────────

const PG_DELETE: &str = "DELETE FROM media_assets WHERE id = $1";
const MYSQL_DELETE: &str = "DELETE FROM media_assets WHERE id = ?";
const SQLITE_DELETE: &str = "DELETE FROM media_assets WHERE id = ?";

// ─── COUNT REFERENCES ─────────────────────────────────────────────────────

const PG_COUNT_REFS: &str = "SELECT COUNT(*) FROM content_entries \
    WHERE jsonb_path_exists( \
        fields, \
        '$.** ? (@ == $asset_id)', \
        jsonb_build_object('asset_id', to_jsonb($1::text)) \
    )";

const MYSQL_COUNT_REFS: &str =
    "SELECT COUNT(*) FROM content_entries WHERE JSON_SEARCH(fields, 'one', ?) IS NOT NULL";

const SQLITE_COUNT_REFS: &str = "SELECT COUNT(DISTINCT ce.id) \
    FROM content_entries ce, json_tree(ce.fields) jt \
    WHERE jt.type = 'text' AND jt.value = ?";

// ─── LIST SELECT PREFIXES ─────────────────────────────────────────────────

const SELECT_LIST_COLS: &str = "SELECT id, filename, original_filename, mime_type, size, \
    storage_path, metadata, uploaded_by, created_at FROM media_assets";

// ─────────────────────────────────────────────────────────────────────────

/// 将含 `?` 占位符的 SQL 转换为 Postgres `$N` 格式。
fn to_pg_sql(sql: &str) -> String {
    let mut n: usize = 0;
    let mut out = String::with_capacity(sql.len() + 16);
    for c in sql.chars() {
        if c == '?' {
            n += 1;
            write!(out, "${n}").expect("write to String is infallible");
        } else {
            out.push(c);
        }
    }
    out
}

// ─── Row-to-model converters ──────────────────────────────────────────────

fn pg_row_to_asset(row: &sqlx::postgres::PgRow) -> Result<MediaAsset, MediaError> {
    let meta: Option<Json<Value>> = row.try_get("metadata").map_err(MediaError::Database)?;
    Ok(MediaAsset {
        id: row.try_get("id").map_err(MediaError::Database)?,
        filename: row.try_get("filename").map_err(MediaError::Database)?,
        original_filename: row
            .try_get("original_filename")
            .map_err(MediaError::Database)?,
        mime_type: row.try_get("mime_type").map_err(MediaError::Database)?,
        size: row.try_get("size").map_err(MediaError::Database)?,
        storage_path: row.try_get("storage_path").map_err(MediaError::Database)?,
        metadata: meta.map(|j| j.0),
        uploaded_by: row.try_get("uploaded_by").map_err(MediaError::Database)?,
        created_at: row.try_get("created_at").map_err(MediaError::Database)?,
    })
}

fn mysql_row_to_asset(row: &sqlx::mysql::MySqlRow) -> Result<MediaAsset, MediaError> {
    let meta: Option<Json<Value>> = row.try_get("metadata").map_err(MediaError::Database)?;
    Ok(MediaAsset {
        id: row.try_get("id").map_err(MediaError::Database)?,
        filename: row.try_get("filename").map_err(MediaError::Database)?,
        original_filename: row
            .try_get("original_filename")
            .map_err(MediaError::Database)?,
        mime_type: row.try_get("mime_type").map_err(MediaError::Database)?,
        size: row.try_get("size").map_err(MediaError::Database)?,
        storage_path: row.try_get("storage_path").map_err(MediaError::Database)?,
        metadata: meta.map(|j| j.0),
        uploaded_by: row.try_get("uploaded_by").map_err(MediaError::Database)?,
        created_at: row.try_get("created_at").map_err(MediaError::Database)?,
    })
}

fn sqlite_row_to_asset(row: &sqlx::sqlite::SqliteRow) -> Result<MediaAsset, MediaError> {
    let meta: Option<Json<Value>> = row.try_get("metadata").map_err(MediaError::Database)?;
    Ok(MediaAsset {
        id: row.try_get("id").map_err(MediaError::Database)?,
        filename: row.try_get("filename").map_err(MediaError::Database)?,
        original_filename: row
            .try_get("original_filename")
            .map_err(MediaError::Database)?,
        mime_type: row.try_get("mime_type").map_err(MediaError::Database)?,
        size: row.try_get("size").map_err(MediaError::Database)?,
        storage_path: row.try_get("storage_path").map_err(MediaError::Database)?,
        metadata: meta.map(|j| j.0),
        uploaded_by: row.try_get("uploaded_by").map_err(MediaError::Database)?,
        created_at: row.try_get("created_at").map_err(MediaError::Database)?,
    })
}

// ─── Bind helpers ─────────────────────────────────────────────────────────

fn bind_pg(
    q: sqlx::query::Query<'_, sqlx::Postgres, sqlx::postgres::PgArguments>,
    s: String,
) -> sqlx::query::Query<'_, sqlx::Postgres, sqlx::postgres::PgArguments> {
    q.bind(s)
}

fn bind_mysql(
    q: sqlx::query::Query<'_, sqlx::MySql, sqlx::mysql::MySqlArguments>,
    s: String,
) -> sqlx::query::Query<'_, sqlx::MySql, sqlx::mysql::MySqlArguments> {
    q.bind(s)
}

fn bind_sqlite<'q>(
    q: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
    s: String,
) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
    q.bind(s)
}

fn escape_like_pattern(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '%' | '_' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

// ─── List plan builder ────────────────────────────────────────────────────

struct MediaListPlan {
    where_sql: String,
    params: Vec<String>,
    page_size: u64,
    page: u64,
}

fn build_list_plan(
    query: &MediaQuery,
    default_page_size: u64,
    max_page_size: u64,
) -> MediaListPlan {
    let mut conditions = vec!["1=1".to_owned()];
    let mut params: Vec<String> = Vec::new();

    if let Some(mime) = &query.mime_type {
        conditions.push("mime_type = ?".to_owned());
        params.push(mime.clone());
    }
    if let Some(fname) = &query.filename_contains {
        conditions.push(
            "(filename LIKE ? ESCAPE '\\' OR original_filename LIKE ? ESCAPE '\\')".to_owned(),
        );
        let pattern = format!("%{}%", escape_like_pattern(fname));
        params.push(pattern.clone());
        params.push(pattern);
    }
    if let Some(uid) = &query.uploaded_by {
        conditions.push("uploaded_by = ?".to_owned());
        params.push(uid.clone());
    }
    if let Some(after) = &query.created_after {
        conditions.push("created_at >= ?".to_owned());
        params.push(after.to_rfc3339());
    }
    if let Some(before) = &query.created_before {
        conditions.push("created_at <= ?".to_owned());
        params.push(before.to_rfc3339());
    }

    let page_size = query
        .page_size
        .unwrap_or(default_page_size)
        .min(max_page_size);
    let page = query.page.unwrap_or(1).max(1);

    MediaListPlan {
        where_sql: conditions.join(" AND "),
        params,
        page_size,
        page,
    }
}

// ─── Repository ───────────────────────────────────────────────────────────

pub struct MediaAssetRepository {
    pool: Arc<DatabasePool>,
}

impl MediaAssetRepository {
    pub fn new(pool: Arc<DatabasePool>) -> Self {
        Self { pool }
    }

    /// 插入媒体资产记录；`created_at` 由数据库 DEFAULT 设置，调用方随后通过 `find_by_id` 读回。
    pub async fn insert(&self, asset: &MediaAsset) -> Result<(), MediaError> {
        let meta: Option<Json<&Value>> = asset.metadata.as_ref().map(Json);

        match self.pool.as_ref() {
            DatabasePool::Postgres(pool) => sqlx::query(PG_INSERT)
                .bind(&asset.id)
                .bind(&asset.filename)
                .bind(&asset.original_filename)
                .bind(&asset.mime_type)
                .bind(asset.size)
                .bind(&asset.storage_path)
                .bind(meta.as_ref())
                .bind(&asset.uploaded_by)
                .execute(pool)
                .await
                .map_err(MediaError::Database)
                .map(|_| ()),
            DatabasePool::MySql(pool) => sqlx::query(MYSQL_INSERT)
                .bind(&asset.id)
                .bind(&asset.filename)
                .bind(&asset.original_filename)
                .bind(&asset.mime_type)
                .bind(asset.size)
                .bind(&asset.storage_path)
                .bind(meta.as_ref())
                .bind(&asset.uploaded_by)
                .execute(pool)
                .await
                .map_err(MediaError::Database)
                .map(|_| ()),
            DatabasePool::Sqlite(pool) => sqlx::query(SQLITE_INSERT)
                .bind(&asset.id)
                .bind(&asset.filename)
                .bind(&asset.original_filename)
                .bind(&asset.mime_type)
                .bind(asset.size)
                .bind(&asset.storage_path)
                .bind(meta.as_ref())
                .bind(&asset.uploaded_by)
                .execute(pool)
                .await
                .map_err(MediaError::Database)
                .map(|_| ()),
        }
    }

    /// 按 ID 查找媒体资产；返回 `None` 表示不存在。
    pub async fn find_by_id(&self, id: &str) -> Result<Option<MediaAsset>, MediaError> {
        match self.pool.as_ref() {
            DatabasePool::Postgres(pool) => sqlx::query(PG_SELECT_BY_ID)
                .bind(id)
                .fetch_optional(pool)
                .await
                .map_err(MediaError::Database)?
                .map(|r| pg_row_to_asset(&r))
                .transpose(),
            DatabasePool::MySql(pool) => sqlx::query(MYSQL_SELECT_BY_ID)
                .bind(id)
                .fetch_optional(pool)
                .await
                .map_err(MediaError::Database)?
                .map(|r| mysql_row_to_asset(&r))
                .transpose(),
            DatabasePool::Sqlite(pool) => sqlx::query(SQLITE_SELECT_BY_ID)
                .bind(id)
                .fetch_optional(pool)
                .await
                .map_err(MediaError::Database)?
                .map(|r| sqlite_row_to_asset(&r))
                .transpose(),
        }
    }

    /// 删除媒体资产记录；返回 `true` 表示实际删除了一行。
    pub async fn delete(&self, id: &str) -> Result<bool, MediaError> {
        let affected = match self.pool.as_ref() {
            DatabasePool::Postgres(pool) => sqlx::query(PG_DELETE)
                .bind(id)
                .execute(pool)
                .await
                .map_err(MediaError::Database)?
                .rows_affected(),
            DatabasePool::MySql(pool) => sqlx::query(MYSQL_DELETE)
                .bind(id)
                .execute(pool)
                .await
                .map_err(MediaError::Database)?
                .rows_affected(),
            DatabasePool::Sqlite(pool) => sqlx::query(SQLITE_DELETE)
                .bind(id)
                .execute(pool)
                .await
                .map_err(MediaError::Database)?
                .rows_affected(),
        };
        Ok(affected > 0)
    }

    /// 统计在 `content_entries.fields` 中引用该资产 ID 的条目数。
    pub async fn count_references(&self, asset_id: &str) -> Result<u64, MediaError> {
        let count: i64 = match self.pool.as_ref() {
            DatabasePool::Postgres(pool) => sqlx::query(PG_COUNT_REFS)
                .bind(asset_id)
                .fetch_one(pool)
                .await
                .map_err(MediaError::Database)?
                .try_get(0)
                .map_err(MediaError::Database)?,
            DatabasePool::MySql(pool) => sqlx::query(MYSQL_COUNT_REFS)
                .bind(asset_id)
                .fetch_one(pool)
                .await
                .map_err(MediaError::Database)?
                .try_get(0)
                .map_err(MediaError::Database)?,
            DatabasePool::Sqlite(pool) => sqlx::query(SQLITE_COUNT_REFS)
                .bind(asset_id)
                .fetch_one(pool)
                .await
                .map_err(MediaError::Database)?
                .try_get(0)
                .map_err(MediaError::Database)?,
        };
        Ok(u64::try_from(count).unwrap_or(0))
    }

    /// 带过滤与分页的列表查询。
    #[allow(clippy::too_many_lines)]
    pub async fn list(
        &self,
        query: &MediaQuery,
        default_page_size: u64,
        max_page_size: u64,
    ) -> Result<PaginatedMedia, MediaError> {
        let plan = build_list_plan(query, default_page_size, max_page_size);
        let order_dir = match query.order_dir {
            MediaOrderDir::Asc => "ASC",
            MediaOrderDir::Desc => "DESC",
        };
        let offset = (plan.page - 1) * plan.page_size;

        let (data, total) = match self.pool.as_ref() {
            DatabasePool::Postgres(pool) => {
                let pg_where = to_pg_sql(&plan.where_sql);
                let count_sql = format!("SELECT COUNT(*) FROM media_assets WHERE {pg_where}");
                let list_sql = format!(
                    "{SELECT_LIST_COLS} WHERE {pg_where} \
                     ORDER BY created_at {order_dir} LIMIT {ps} OFFSET {off}",
                    ps = plan.page_size,
                    off = offset,
                );

                let mut count_q = sqlx::query(&count_sql);
                for p in &plan.params {
                    count_q = bind_pg(count_q, p.clone());
                }
                let total: i64 = count_q
                    .fetch_one(pool)
                    .await
                    .map_err(MediaError::Database)?
                    .try_get(0)
                    .map_err(MediaError::Database)?;

                let mut list_q = sqlx::query(&list_sql);
                for p in &plan.params {
                    list_q = bind_pg(list_q, p.clone());
                }
                let rows = list_q.fetch_all(pool).await.map_err(MediaError::Database)?;
                let data = rows
                    .iter()
                    .map(pg_row_to_asset)
                    .collect::<Result<Vec<_>, _>>()?;
                (data, total)
            }
            DatabasePool::MySql(pool) => {
                let count_sql =
                    format!("SELECT COUNT(*) FROM media_assets WHERE {}", plan.where_sql);
                let list_sql = format!(
                    "{SELECT_LIST_COLS} WHERE {} \
                     ORDER BY created_at {order_dir} LIMIT {ps} OFFSET {off}",
                    plan.where_sql,
                    ps = plan.page_size,
                    off = offset,
                );

                let mut count_q = sqlx::query(&count_sql);
                for p in &plan.params {
                    count_q = bind_mysql(count_q, p.clone());
                }
                let total: i64 = count_q
                    .fetch_one(pool)
                    .await
                    .map_err(MediaError::Database)?
                    .try_get(0)
                    .map_err(MediaError::Database)?;

                let mut list_q = sqlx::query(&list_sql);
                for p in &plan.params {
                    list_q = bind_mysql(list_q, p.clone());
                }
                let rows = list_q.fetch_all(pool).await.map_err(MediaError::Database)?;
                let data = rows
                    .iter()
                    .map(mysql_row_to_asset)
                    .collect::<Result<Vec<_>, _>>()?;
                (data, total)
            }
            DatabasePool::Sqlite(pool) => {
                let count_sql =
                    format!("SELECT COUNT(*) FROM media_assets WHERE {}", plan.where_sql);
                let list_sql = format!(
                    "{SELECT_LIST_COLS} WHERE {} \
                     ORDER BY created_at {order_dir} LIMIT {ps} OFFSET {off}",
                    plan.where_sql,
                    ps = plan.page_size,
                    off = offset,
                );

                let mut count_q = sqlx::query(&count_sql);
                for p in &plan.params {
                    count_q = bind_sqlite(count_q, p.clone());
                }
                let total: i64 = count_q
                    .fetch_one(pool)
                    .await
                    .map_err(MediaError::Database)?
                    .try_get(0)
                    .map_err(MediaError::Database)?;

                let mut list_q = sqlx::query(&list_sql);
                for p in &plan.params {
                    list_q = bind_sqlite(list_q, p.clone());
                }
                let rows = list_q.fetch_all(pool).await.map_err(MediaError::Database)?;
                let data = rows
                    .iter()
                    .map(sqlite_row_to_asset)
                    .collect::<Result<Vec<_>, _>>()?;
                (data, total)
            }
        };

        let total = u64::try_from(total).unwrap_or(0);
        let page_count = total.div_ceil(plan.page_size).max(1);
        Ok(PaginatedMedia {
            data,
            total,
            page: plan.page,
            page_size: plan.page_size,
            page_count,
        })
    }
}
