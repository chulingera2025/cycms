//! `content_types` 表三方言 CRUD。
//!
//! `fields` 列在三方言统一按 JSON 存取，通过 [`sqlx::types::Json`] 封装
//! `Vec<FieldDefinition>` 的 serde 编解码：PG `JSONB` / `MySQL` `JSON` / `SQLite`
//! `TEXT + json_valid` 的差异对 repository 调用方透明。
//!
//! `api_id` 为用户可见的 URL 友好标识符：`trim + lowercase` 归一后再通过
//! [`validate_api_id`] 做正则校验；入库唯一键由表级 `UNIQUE` 约束兜底。

use std::str::FromStr;
use std::sync::{Arc, LazyLock};

use chrono::NaiveDateTime;
use cycms_core::Result;
use cycms_db::DatabasePool;
use regex::Regex;
use sqlx::Row;
use sqlx::mysql::MySqlRow;
use sqlx::postgres::PgRow;
use sqlx::sqlite::SqliteRow;
use sqlx::types::Json;
use uuid::Uuid;

use crate::error::ContentModelError;
use crate::model::{ContentTypeDefinition, ContentTypeKind, FieldDefinition};

static API_ID_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9_-]{0,62}$").expect("api_id regex"));

/// 校验 `api_id` 是否符合 `^[a-z][a-z0-9_-]{0,62}$`。
///
/// 调用方应先 `trim + lowercase` 后再传入；此处只做格式断言。
///
/// # Errors
/// 格式不符返回 [`ContentModelError::InputValidation`]。
pub fn validate_api_id(api_id: &str) -> std::result::Result<(), ContentModelError> {
    if API_ID_PATTERN.is_match(api_id) {
        Ok(())
    } else {
        Err(ContentModelError::InputValidation(format!(
            "invalid api_id `{api_id}`: must match {}",
            API_ID_PATTERN.as_str()
        )))
    }
}

/// 归一 `api_id`：`trim + lowercase` 后再跑 [`validate_api_id`]。
///
/// # Errors
/// 经归一仍为空或格式非法返回 [`ContentModelError::InputValidation`]。
pub fn normalize_api_id(api_id: &str) -> std::result::Result<String, ContentModelError> {
    let n = api_id.trim().to_lowercase();
    validate_api_id(&n)?;
    Ok(n)
}

/// 归一 `name`：仅 `trim`。
///
/// # Errors
/// 经归一后为空返回 [`ContentModelError::InputValidation`]。
pub fn normalize_name(name: &str) -> std::result::Result<String, ContentModelError> {
    let n = name.trim();
    if n.is_empty() {
        return Err(ContentModelError::InputValidation(
            "name must not be empty".to_owned(),
        ));
    }
    Ok(n.to_owned())
}

/// 创建一条 Content Type 的 DB 行参数（service 层 validated 后交给 repository）。
#[derive(Debug, Clone)]
pub struct NewContentTypeRow {
    pub id: String,
    pub name: String,
    pub api_id: String,
    pub description: Option<String>,
    pub kind: ContentTypeKind,
    pub fields: Vec<FieldDefinition>,
}

/// 全量更新 Content Type 的 DB 行参数。
#[derive(Debug, Clone)]
pub struct UpdateContentTypeRow {
    pub name: String,
    pub description: Option<String>,
    pub kind: ContentTypeKind,
    pub fields: Vec<FieldDefinition>,
}

/// `content_types` 表 CRUD 门面。
pub struct ContentTypeRepository {
    db: Arc<DatabasePool>,
}

impl ContentTypeRepository {
    #[must_use]
    pub fn new(db: Arc<DatabasePool>) -> Self {
        Self { db }
    }

    /// 插入一条 Content Type。
    ///
    /// # Errors
    /// - `api_id` 唯一键冲突 → [`ContentModelError::DuplicateApiId`]
    /// - DB 故障 → [`ContentModelError::Database`]
    /// - 读回解码失败 → [`cycms_core::Error::Internal`]
    pub async fn insert(&self, row: NewContentTypeRow) -> Result<ContentTypeDefinition> {
        let fields_json = Json(row.fields.clone());
        let kind_str = row.kind.as_str().to_owned();

        let insert_result: std::result::Result<(), sqlx::Error> = match self.db.as_ref() {
            DatabasePool::Postgres(pool) => sqlx::query(PG_INSERT)
                .bind(&row.id)
                .bind(&row.name)
                .bind(&row.api_id)
                .bind(row.description.as_deref())
                .bind(&kind_str)
                .bind(&fields_json)
                .execute(pool)
                .await
                .map(|_| ()),
            DatabasePool::MySql(pool) => sqlx::query(MYSQL_INSERT)
                .bind(&row.id)
                .bind(&row.name)
                .bind(&row.api_id)
                .bind(row.description.as_deref())
                .bind(&kind_str)
                .bind(&fields_json)
                .execute(pool)
                .await
                .map(|_| ()),
            DatabasePool::Sqlite(pool) => sqlx::query(SQLITE_INSERT)
                .bind(&row.id)
                .bind(&row.name)
                .bind(&row.api_id)
                .bind(row.description.as_deref())
                .bind(&kind_str)
                .bind(&fields_json)
                .execute(pool)
                .await
                .map(|_| ()),
        };

        match insert_result {
            Ok(()) => {}
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                return Err(ContentModelError::DuplicateApiId(row.api_id).into());
            }
            Err(e) => return Err(ContentModelError::Database(e).into()),
        }

        self.find_by_id(&row.id)
            .await?
            .ok_or_else(|| cycms_core::Error::Internal {
                message: "inserted content type not found on read-back".to_owned(),
                source: None,
            })
    }

    /// 按 `id` 更新全量字段。
    ///
    /// # Errors
    /// - DB 故障 → [`ContentModelError::Database`]
    /// - `id` 不存在 → [`ContentModelError::NotFound`]
    /// - 读回解码失败 → [`cycms_core::Error::Internal`]
    pub async fn update(
        &self,
        id: &str,
        row: UpdateContentTypeRow,
    ) -> Result<ContentTypeDefinition> {
        let fields_json = Json(row.fields.clone());
        let kind_str = row.kind.as_str().to_owned();

        let affected = match self.db.as_ref() {
            DatabasePool::Postgres(pool) => sqlx::query(PG_UPDATE)
                .bind(id)
                .bind(&row.name)
                .bind(row.description.as_deref())
                .bind(&kind_str)
                .bind(&fields_json)
                .execute(pool)
                .await
                .map_err(ContentModelError::Database)?
                .rows_affected(),
            DatabasePool::MySql(pool) => sqlx::query(MYSQL_UPDATE)
                .bind(&row.name)
                .bind(row.description.as_deref())
                .bind(&kind_str)
                .bind(&fields_json)
                .bind(id)
                .execute(pool)
                .await
                .map_err(ContentModelError::Database)?
                .rows_affected(),
            DatabasePool::Sqlite(pool) => sqlx::query(SQLITE_UPDATE)
                .bind(&row.name)
                .bind(row.description.as_deref())
                .bind(&kind_str)
                .bind(&fields_json)
                .bind(id)
                .execute(pool)
                .await
                .map_err(ContentModelError::Database)?
                .rows_affected(),
        };

        if affected == 0 {
            return Err(ContentModelError::NotFound(id.to_owned()).into());
        }

        self.find_by_id(id)
            .await?
            .ok_or_else(|| cycms_core::Error::Internal {
                message: "updated content type not found on read-back".to_owned(),
                source: None,
            })
    }

    /// 按 `id` 删除。
    ///
    /// # Errors
    /// DB 故障 → [`ContentModelError::Database`]。
    pub async fn delete_by_id(&self, id: &str) -> Result<bool> {
        let affected = match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                sqlx::query("DELETE FROM content_types WHERE id = $1::UUID")
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(ContentModelError::Database)?
                    .rows_affected()
            }
            DatabasePool::MySql(pool) => sqlx::query("DELETE FROM content_types WHERE id = ?")
                .bind(id)
                .execute(pool)
                .await
                .map_err(ContentModelError::Database)?
                .rows_affected(),
            DatabasePool::Sqlite(pool) => sqlx::query("DELETE FROM content_types WHERE id = ?")
                .bind(id)
                .execute(pool)
                .await
                .map_err(ContentModelError::Database)?
                .rows_affected(),
        };
        Ok(affected > 0)
    }

    /// 按 `id` 查询。
    ///
    /// # Errors
    /// DB 故障 / JSON 解码失败 → [`cycms_core::Error::Internal`]。
    pub async fn find_by_id(&self, id: &str) -> Result<Option<ContentTypeDefinition>> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(PG_SELECT_BY_ID)
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(ContentModelError::Database)?;
                row.map(|r| pg_row_to_def(&r))
                    .transpose()
                    .map_err(Into::into)
            }
            DatabasePool::MySql(pool) => {
                let row = sqlx::query(MYSQL_SELECT_BY_ID)
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(ContentModelError::Database)?;
                row.map(|r| mysql_row_to_def(&r))
                    .transpose()
                    .map_err(Into::into)
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(SQLITE_SELECT_BY_ID)
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(ContentModelError::Database)?;
                row.map(|r| sqlite_row_to_def(&r))
                    .transpose()
                    .map_err(Into::into)
            }
        }
    }

    /// 按 `api_id` 查询。
    ///
    /// # Errors
    /// DB 故障 / JSON 解码失败 → [`cycms_core::Error::Internal`]。
    pub async fn find_by_api_id(&self, api_id: &str) -> Result<Option<ContentTypeDefinition>> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(PG_SELECT_BY_API_ID)
                    .bind(api_id)
                    .fetch_optional(pool)
                    .await
                    .map_err(ContentModelError::Database)?;
                row.map(|r| pg_row_to_def(&r))
                    .transpose()
                    .map_err(Into::into)
            }
            DatabasePool::MySql(pool) => {
                let row = sqlx::query(MYSQL_SELECT_BY_API_ID)
                    .bind(api_id)
                    .fetch_optional(pool)
                    .await
                    .map_err(ContentModelError::Database)?;
                row.map(|r| mysql_row_to_def(&r))
                    .transpose()
                    .map_err(Into::into)
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(SQLITE_SELECT_BY_API_ID)
                    .bind(api_id)
                    .fetch_optional(pool)
                    .await
                    .map_err(ContentModelError::Database)?;
                row.map(|r| sqlite_row_to_def(&r))
                    .transpose()
                    .map_err(Into::into)
            }
        }
    }

    /// 列出全部 Content Type，按 `api_id` 升序。
    ///
    /// # Errors
    /// DB 故障 / JSON 解码失败 → [`cycms_core::Error::Internal`]。
    pub async fn list(&self) -> Result<Vec<ContentTypeDefinition>> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(PG_SELECT_ALL)
                    .fetch_all(pool)
                    .await
                    .map_err(ContentModelError::Database)?;
                rows.iter()
                    .map(pg_row_to_def)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
            DatabasePool::MySql(pool) => {
                let rows = sqlx::query(MYSQL_SELECT_ALL)
                    .fetch_all(pool)
                    .await
                    .map_err(ContentModelError::Database)?;
                rows.iter()
                    .map(mysql_row_to_def)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(SQLITE_SELECT_ALL)
                    .fetch_all(pool)
                    .await
                    .map_err(ContentModelError::Database)?;
                rows.iter()
                    .map(sqlite_row_to_def)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
        }
    }
}

/// 便于上层生成 UUID v4 字符串：repository 不强制 `id` 形态，仅对列宽敏感。
#[must_use]
pub fn new_content_type_id() -> String {
    Uuid::new_v4().to_string()
}

const PG_INSERT: &str = "INSERT INTO content_types (id, name, api_id, description, kind, fields) \
     VALUES ($1::UUID, $2, $3, $4, $5, $6)";
const PG_UPDATE: &str = "UPDATE content_types SET name = $2, description = $3, kind = $4, \
     fields = $5, updated_at = now() WHERE id = $1::UUID";
const PG_SELECT_BY_ID: &str = "SELECT id::TEXT AS id, name, api_id, description, kind, fields, created_at, updated_at \
     FROM content_types WHERE id = $1::UUID";
const PG_SELECT_BY_API_ID: &str = "SELECT id::TEXT AS id, name, api_id, description, kind, fields, created_at, updated_at \
     FROM content_types WHERE api_id = $1";
const PG_SELECT_ALL: &str = "SELECT id::TEXT AS id, name, api_id, description, kind, fields, created_at, updated_at \
     FROM content_types ORDER BY api_id";

const MYSQL_INSERT: &str = "INSERT INTO content_types (id, name, api_id, description, kind, fields) \
     VALUES (?, ?, ?, ?, ?, ?)";
const MYSQL_UPDATE: &str = "UPDATE content_types SET name = ?, description = ?, kind = ?, \
     fields = ?, updated_at = CURRENT_TIMESTAMP(6) WHERE id = ?";
const MYSQL_SELECT_BY_ID: &str = "SELECT id, name, api_id, description, kind, fields, created_at, updated_at \
     FROM content_types WHERE id = ?";
const MYSQL_SELECT_BY_API_ID: &str = "SELECT id, name, api_id, description, kind, fields, created_at, updated_at \
     FROM content_types WHERE api_id = ?";
const MYSQL_SELECT_ALL: &str = "SELECT id, name, api_id, description, kind, fields, created_at, updated_at \
     FROM content_types ORDER BY api_id";

const SQLITE_INSERT: &str = "INSERT INTO content_types (id, name, api_id, description, kind, fields) \
     VALUES (?, ?, ?, ?, ?, ?)";
const SQLITE_UPDATE: &str = "UPDATE content_types SET name = ?, description = ?, kind = ?, \
     fields = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?";
const SQLITE_SELECT_BY_ID: &str = "SELECT id, name, api_id, description, kind, fields, created_at, updated_at \
     FROM content_types WHERE id = ?";
const SQLITE_SELECT_BY_API_ID: &str = "SELECT id, name, api_id, description, kind, fields, created_at, updated_at \
     FROM content_types WHERE api_id = ?";
const SQLITE_SELECT_ALL: &str = "SELECT id, name, api_id, description, kind, fields, created_at, updated_at \
     FROM content_types ORDER BY api_id";

fn pg_row_to_def(row: &PgRow) -> std::result::Result<ContentTypeDefinition, ContentModelError> {
    let kind_raw: String = row.try_get("kind").map_err(ContentModelError::Database)?;
    let kind = ContentTypeKind::from_str(&kind_raw)?;
    let fields: Json<Vec<FieldDefinition>> =
        row.try_get("fields").map_err(ContentModelError::Database)?;
    Ok(ContentTypeDefinition {
        id: row.try_get("id").map_err(ContentModelError::Database)?,
        name: row.try_get("name").map_err(ContentModelError::Database)?,
        api_id: row.try_get("api_id").map_err(ContentModelError::Database)?,
        description: row
            .try_get("description")
            .map_err(ContentModelError::Database)?,
        kind,
        fields: fields.0,
        created_at: row
            .try_get("created_at")
            .map_err(ContentModelError::Database)?,
        updated_at: row
            .try_get("updated_at")
            .map_err(ContentModelError::Database)?,
    })
}

fn mysql_row_to_def(
    row: &MySqlRow,
) -> std::result::Result<ContentTypeDefinition, ContentModelError> {
    let kind_raw: String = row.try_get("kind").map_err(ContentModelError::Database)?;
    let kind = ContentTypeKind::from_str(&kind_raw)?;
    let fields: Json<Vec<FieldDefinition>> =
        row.try_get("fields").map_err(ContentModelError::Database)?;
    let created_at: NaiveDateTime = row
        .try_get("created_at")
        .map_err(ContentModelError::Database)?;
    let updated_at: NaiveDateTime = row
        .try_get("updated_at")
        .map_err(ContentModelError::Database)?;
    Ok(ContentTypeDefinition {
        id: row.try_get("id").map_err(ContentModelError::Database)?,
        name: row.try_get("name").map_err(ContentModelError::Database)?,
        api_id: row.try_get("api_id").map_err(ContentModelError::Database)?,
        description: row
            .try_get("description")
            .map_err(ContentModelError::Database)?,
        kind,
        fields: fields.0,
        created_at: created_at.and_utc(),
        updated_at: updated_at.and_utc(),
    })
}

fn sqlite_row_to_def(
    row: &SqliteRow,
) -> std::result::Result<ContentTypeDefinition, ContentModelError> {
    let kind_raw: String = row.try_get("kind").map_err(ContentModelError::Database)?;
    let kind = ContentTypeKind::from_str(&kind_raw)?;
    let fields: Json<Vec<FieldDefinition>> =
        row.try_get("fields").map_err(ContentModelError::Database)?;
    Ok(ContentTypeDefinition {
        id: row.try_get("id").map_err(ContentModelError::Database)?,
        name: row.try_get("name").map_err(ContentModelError::Database)?,
        api_id: row.try_get("api_id").map_err(ContentModelError::Database)?,
        description: row
            .try_get("description")
            .map_err(ContentModelError::Database)?,
        kind,
        fields: fields.0,
        created_at: row
            .try_get("created_at")
            .map_err(ContentModelError::Database)?,
        updated_at: row
            .try_get("updated_at")
            .map_err(ContentModelError::Database)?,
    })
}

#[cfg(test)]
mod tests {
    use super::{normalize_api_id, normalize_name, validate_api_id};

    #[test]
    fn validate_api_id_accepts_legal_values() {
        for ok in ["page", "blog-post", "my_type", "t1", "a-b_c-123"] {
            assert!(validate_api_id(ok).is_ok(), "{ok} should be ok");
        }
    }

    #[test]
    fn validate_api_id_rejects_illegal_values() {
        for bad in [
            "",
            "Page",
            "0starts-digit",
            "-leading-dash",
            "has space",
            "dot.inside",
            "slash/inside",
            &"a".repeat(64),
        ] {
            assert!(validate_api_id(bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn normalize_api_id_trims_and_lowercases() {
        assert_eq!(normalize_api_id("  Page  ").unwrap(), "page");
        assert_eq!(normalize_api_id("\tBlog-Post\n").unwrap(), "blog-post");
    }

    #[test]
    fn normalize_name_rejects_empty() {
        assert_eq!(normalize_name("  hi  ").unwrap(), "hi");
        assert!(normalize_name("").is_err());
        assert!(normalize_name("   ").is_err());
    }
}
