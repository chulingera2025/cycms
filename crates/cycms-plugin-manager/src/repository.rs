//! `plugins` 表 CRUD，三方言一致入口。
//!
//! - `Postgres` `id UUID` 读取时 cast 为 `TEXT` 以保持跨方言 `String` 表示。
//! - `manifest` 列统一用 [`sqlx::types::Json`] 编解码（`Postgres` `JSONB` / `MySQL` `JSON` /
//!   `SQLite` `TEXT + json_valid`）。
//! - 时间戳统一归一化为 `DateTime<Utc>`；`MySQL` `DATETIME(6)` 读出 `NaiveDateTime` 后
//!   显式 `.and_utc()`。

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

use crate::error::PluginManagerError;
use crate::manifest::PluginKind;
use crate::model::{NewPluginRow, PluginRecord, PluginStatus};

/// `plugins` 表的三方言 CRUD 门面。
pub struct PluginRepository {
    db: Arc<DatabasePool>,
}

impl PluginRepository {
    #[must_use]
    pub fn new(db: Arc<DatabasePool>) -> Self {
        Self { db }
    }

    /// 插入新插件记录。`status` 初始固定为 [`PluginStatus::Disabled`]，
    /// service 层通过显式 [`Self::update_status`] 切换到 enabled。
    ///
    /// # Errors
    /// - `name` 冲突触发 UNIQUE violation → [`PluginManagerError::Database`]
    /// - DB 故障 / JSON 编解码失败 → [`cycms_core::Error::Internal`]
    pub async fn insert(&self, row: NewPluginRow) -> Result<PluginRecord> {
        let id = Uuid::new_v4().to_string();
        let kind = row.kind.as_str();
        let status = PluginStatus::Disabled.as_str();
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                sqlx::query(PG_INSERT)
                    .bind(&id)
                    .bind(&row.name)
                    .bind(&row.version)
                    .bind(kind)
                    .bind(status)
                    .bind(Json(&row.manifest))
                    .execute(pool)
                    .await
                    .map_err(PluginManagerError::Database)?;
            }
            DatabasePool::MySql(pool) => {
                sqlx::query(MYSQL_INSERT)
                    .bind(&id)
                    .bind(&row.name)
                    .bind(&row.version)
                    .bind(kind)
                    .bind(status)
                    .bind(Json(&row.manifest))
                    .execute(pool)
                    .await
                    .map_err(PluginManagerError::Database)?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(SQLITE_INSERT)
                    .bind(&id)
                    .bind(&row.name)
                    .bind(&row.version)
                    .bind(kind)
                    .bind(status)
                    .bind(Json(&row.manifest))
                    .execute(pool)
                    .await
                    .map_err(PluginManagerError::Database)?;
            }
        }

        self.find_by_name(&row.name)
            .await?
            .ok_or_else(|| cycms_core::Error::Internal {
                message: "inserted plugin not found on read-back".to_owned(),
                source: None,
            })
    }

    /// 按唯一 `name` 查找插件记录。
    ///
    /// # Errors
    /// DB 故障 / JSON 解码失败 → [`cycms_core::Error::Internal`]。
    pub async fn find_by_name(&self, name: &str) -> Result<Option<PluginRecord>> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(PG_SELECT_BY_NAME)
                    .bind(name)
                    .fetch_optional(pool)
                    .await
                    .map_err(PluginManagerError::Database)?;
                row.map(|r| pg_row_to_record(&r))
                    .transpose()
                    .map_err(Into::into)
            }
            DatabasePool::MySql(pool) => {
                let row = sqlx::query(MYSQL_SELECT_BY_NAME)
                    .bind(name)
                    .fetch_optional(pool)
                    .await
                    .map_err(PluginManagerError::Database)?;
                row.map(|r| mysql_row_to_record(&r))
                    .transpose()
                    .map_err(Into::into)
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(SQLITE_SELECT_BY_NAME)
                    .bind(name)
                    .fetch_optional(pool)
                    .await
                    .map_err(PluginManagerError::Database)?;
                row.map(|r| sqlite_row_to_record(&r))
                    .transpose()
                    .map_err(Into::into)
            }
        }
    }

    /// 列出所有插件记录，按 `name` 字典序。
    ///
    /// # Errors
    /// DB 故障 / JSON 解码失败 → [`cycms_core::Error::Internal`]。
    pub async fn list(&self) -> Result<Vec<PluginRecord>> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(PG_SELECT_ALL)
                    .fetch_all(pool)
                    .await
                    .map_err(PluginManagerError::Database)?;
                rows.iter()
                    .map(pg_row_to_record)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
            DatabasePool::MySql(pool) => {
                let rows = sqlx::query(MYSQL_SELECT_ALL)
                    .fetch_all(pool)
                    .await
                    .map_err(PluginManagerError::Database)?;
                rows.iter()
                    .map(mysql_row_to_record)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(SQLITE_SELECT_ALL)
                    .fetch_all(pool)
                    .await
                    .map_err(PluginManagerError::Database)?;
                rows.iter()
                    .map(sqlite_row_to_record)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
        }
    }

    /// 更新指定插件的状态，返回是否有行被实际修改。
    ///
    /// # Errors
    /// DB 故障 → [`cycms_core::Error::Internal`]。
    pub async fn update_status(&self, name: &str, status: PluginStatus) -> Result<bool> {
        let status_str = status.as_str();
        let affected = match self.db.as_ref() {
            DatabasePool::Postgres(pool) => sqlx::query(PG_UPDATE_STATUS)
                .bind(status_str)
                .bind(name)
                .execute(pool)
                .await
                .map_err(PluginManagerError::Database)?
                .rows_affected(),
            DatabasePool::MySql(pool) => sqlx::query(MYSQL_UPDATE_STATUS)
                .bind(status_str)
                .bind(name)
                .execute(pool)
                .await
                .map_err(PluginManagerError::Database)?
                .rows_affected(),
            DatabasePool::Sqlite(pool) => sqlx::query(SQLITE_UPDATE_STATUS)
                .bind(status_str)
                .bind(name)
                .execute(pool)
                .await
                .map_err(PluginManagerError::Database)?
                .rows_affected(),
        };
        Ok(affected > 0)
    }

    /// 删除指定插件记录，返回是否有行被实际清除。
    ///
    /// # Errors
    /// DB 故障 → [`cycms_core::Error::Internal`]。
    pub async fn delete(&self, name: &str) -> Result<bool> {
        let affected = match self.db.as_ref() {
            DatabasePool::Postgres(pool) => sqlx::query("DELETE FROM plugins WHERE name = $1")
                .bind(name)
                .execute(pool)
                .await
                .map_err(PluginManagerError::Database)?
                .rows_affected(),
            DatabasePool::MySql(pool) => sqlx::query("DELETE FROM plugins WHERE name = ?")
                .bind(name)
                .execute(pool)
                .await
                .map_err(PluginManagerError::Database)?
                .rows_affected(),
            DatabasePool::Sqlite(pool) => sqlx::query("DELETE FROM plugins WHERE name = ?")
                .bind(name)
                .execute(pool)
                .await
                .map_err(PluginManagerError::Database)?
                .rows_affected(),
        };
        Ok(affected > 0)
    }
}

const PG_INSERT: &str = "INSERT INTO plugins (id, name, version, kind, status, manifest) \
     VALUES ($1::UUID, $2, $3, $4, $5, $6)";
const PG_SELECT_BY_NAME: &str =
    "SELECT id::TEXT AS id, name, version, kind, status, manifest, installed_at, updated_at \
     FROM plugins WHERE name = $1";
const PG_SELECT_ALL: &str =
    "SELECT id::TEXT AS id, name, version, kind, status, manifest, installed_at, updated_at \
     FROM plugins ORDER BY name";
const PG_UPDATE_STATUS: &str =
    "UPDATE plugins SET status = $1, updated_at = now() WHERE name = $2";

const MYSQL_INSERT: &str = "INSERT INTO plugins (id, name, version, kind, status, manifest) \
     VALUES (?, ?, ?, ?, ?, ?)";
const MYSQL_SELECT_BY_NAME: &str =
    "SELECT id, name, version, kind, status, manifest, installed_at, updated_at \
     FROM plugins WHERE name = ?";
const MYSQL_SELECT_ALL: &str =
    "SELECT id, name, version, kind, status, manifest, installed_at, updated_at \
     FROM plugins ORDER BY name";
const MYSQL_UPDATE_STATUS: &str =
    "UPDATE plugins SET status = ?, updated_at = CURRENT_TIMESTAMP(6) WHERE name = ?";

const SQLITE_INSERT: &str = "INSERT INTO plugins (id, name, version, kind, status, manifest) \
     VALUES (?, ?, ?, ?, ?, ?)";
const SQLITE_SELECT_BY_NAME: &str =
    "SELECT id, name, version, kind, status, manifest, installed_at, updated_at \
     FROM plugins WHERE name = ?";
const SQLITE_SELECT_ALL: &str =
    "SELECT id, name, version, kind, status, manifest, installed_at, updated_at \
     FROM plugins ORDER BY name";
const SQLITE_UPDATE_STATUS: &str =
    "UPDATE plugins SET status = ?, \
       updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
     WHERE name = ?";

fn pg_row_to_record(row: &PgRow) -> std::result::Result<PluginRecord, PluginManagerError> {
    let manifest: Json<Value> = row
        .try_get("manifest")
        .map_err(PluginManagerError::Database)?;
    let kind_str: String = row.try_get("kind").map_err(PluginManagerError::Database)?;
    let status_str: String = row
        .try_get("status")
        .map_err(PluginManagerError::Database)?;
    Ok(PluginRecord {
        id: row.try_get("id").map_err(PluginManagerError::Database)?,
        name: row.try_get("name").map_err(PluginManagerError::Database)?,
        version: row
            .try_get("version")
            .map_err(PluginManagerError::Database)?,
        kind: kind_str.parse::<PluginKind>().map_err(PluginManagerError::InvalidRecord)?,
        status: status_str
            .parse::<PluginStatus>()
            .map_err(PluginManagerError::InvalidRecord)?,
        manifest: manifest.0,
        installed_at: row
            .try_get("installed_at")
            .map_err(PluginManagerError::Database)?,
        updated_at: row
            .try_get("updated_at")
            .map_err(PluginManagerError::Database)?,
    })
}

fn mysql_row_to_record(row: &MySqlRow) -> std::result::Result<PluginRecord, PluginManagerError> {
    let manifest: Json<Value> = row
        .try_get("manifest")
        .map_err(PluginManagerError::Database)?;
    let kind_str: String = row.try_get("kind").map_err(PluginManagerError::Database)?;
    let status_str: String = row
        .try_get("status")
        .map_err(PluginManagerError::Database)?;
    let installed_at: NaiveDateTime = row
        .try_get("installed_at")
        .map_err(PluginManagerError::Database)?;
    let updated_at: NaiveDateTime = row
        .try_get("updated_at")
        .map_err(PluginManagerError::Database)?;
    Ok(PluginRecord {
        id: row.try_get("id").map_err(PluginManagerError::Database)?,
        name: row.try_get("name").map_err(PluginManagerError::Database)?,
        version: row
            .try_get("version")
            .map_err(PluginManagerError::Database)?,
        kind: kind_str.parse::<PluginKind>().map_err(PluginManagerError::InvalidRecord)?,
        status: status_str
            .parse::<PluginStatus>()
            .map_err(PluginManagerError::InvalidRecord)?,
        manifest: manifest.0,
        installed_at: installed_at.and_utc(),
        updated_at: updated_at.and_utc(),
    })
}

fn sqlite_row_to_record(row: &SqliteRow) -> std::result::Result<PluginRecord, PluginManagerError> {
    let manifest: Json<Value> = row
        .try_get("manifest")
        .map_err(PluginManagerError::Database)?;
    let kind_str: String = row.try_get("kind").map_err(PluginManagerError::Database)?;
    let status_str: String = row
        .try_get("status")
        .map_err(PluginManagerError::Database)?;
    Ok(PluginRecord {
        id: row.try_get("id").map_err(PluginManagerError::Database)?,
        name: row.try_get("name").map_err(PluginManagerError::Database)?,
        version: row
            .try_get("version")
            .map_err(PluginManagerError::Database)?,
        kind: kind_str.parse::<PluginKind>().map_err(PluginManagerError::InvalidRecord)?,
        status: status_str
            .parse::<PluginStatus>()
            .map_err(PluginManagerError::InvalidRecord)?,
        manifest: manifest.0,
        installed_at: row
            .try_get("installed_at")
            .map_err(PluginManagerError::Database)?,
        updated_at: row
            .try_get("updated_at")
            .map_err(PluginManagerError::Database)?,
    })
}
