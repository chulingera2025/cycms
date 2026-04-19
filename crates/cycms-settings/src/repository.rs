//! `settings` 表 CRUD 与 `plugin_settings_schemas` 表 CRUD。
//!
//! 三方言 `value` / `schema` 列的 JSON 编解码统一通过 [`sqlx::types::Json`]：
//! - Postgres `JSONB` / `MySQL` `JSON` / `SQLite` `TEXT + json_valid` 都被视为 JSON，
//!   Encode 时 `serde_json::to_string` 落盘，Decode 时 `serde_json::from_str` 读回。

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

use crate::error::SettingsError;
use crate::model::{PluginSchema, SettingEntry};

/// `settings` 表的三方言 CRUD。
///
/// 读取路径（`find`/`list_by_namespace`）返回 [`SettingEntry`]；写入路径
/// （`upsert`）走 `ON CONFLICT (namespace, key) DO UPDATE` 语义保证幂等；
/// 删除返回 `true` 表示实际有行被清除。
pub struct SettingsRepository {
    db: Arc<DatabasePool>,
}

impl SettingsRepository {
    #[must_use]
    pub fn new(db: Arc<DatabasePool>) -> Self {
        Self { db }
    }

    /// 查找单条设置。
    ///
    /// # Errors
    /// - namespace / key 归一后为空 → [`cycms_core::Error::ValidationError`]
    /// - DB 故障 / JSON 解码失败 → [`cycms_core::Error::Internal`]
    pub async fn find(&self, namespace: &str, key: &str) -> Result<Option<SettingEntry>> {
        let (ns, k) = normalize_pair(namespace, key)?;
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(PG_SELECT_BY_NS_KEY)
                    .bind(&ns)
                    .bind(&k)
                    .fetch_optional(pool)
                    .await
                    .map_err(SettingsError::Database)?;
                row.map(|r| pg_row_to_entry(&r))
                    .transpose()
                    .map_err(Into::into)
            }
            DatabasePool::MySql(pool) => {
                let row = sqlx::query(MYSQL_SELECT_BY_NS_KEY)
                    .bind(&ns)
                    .bind(&k)
                    .fetch_optional(pool)
                    .await
                    .map_err(SettingsError::Database)?;
                row.map(|r| mysql_row_to_entry(&r))
                    .transpose()
                    .map_err(Into::into)
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(SQLITE_SELECT_BY_NS_KEY)
                    .bind(&ns)
                    .bind(&k)
                    .fetch_optional(pool)
                    .await
                    .map_err(SettingsError::Database)?;
                row.map(|r| sqlite_row_to_entry(&r))
                    .transpose()
                    .map_err(Into::into)
            }
        }
    }

    /// 列出某 namespace 下所有设置，按 `key` 升序。
    ///
    /// # Errors
    /// - namespace 归一后为空 → [`cycms_core::Error::ValidationError`]
    /// - DB 故障 → [`cycms_core::Error::Internal`]
    pub async fn list_by_namespace(&self, namespace: &str) -> Result<Vec<SettingEntry>> {
        let ns = normalize_namespace(namespace)?;
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(PG_SELECT_BY_NS)
                    .bind(&ns)
                    .fetch_all(pool)
                    .await
                    .map_err(SettingsError::Database)?;
                rows.iter()
                    .map(pg_row_to_entry)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
            DatabasePool::MySql(pool) => {
                let rows = sqlx::query(MYSQL_SELECT_BY_NS)
                    .bind(&ns)
                    .fetch_all(pool)
                    .await
                    .map_err(SettingsError::Database)?;
                rows.iter()
                    .map(mysql_row_to_entry)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(SQLITE_SELECT_BY_NS)
                    .bind(&ns)
                    .fetch_all(pool)
                    .await
                    .map_err(SettingsError::Database)?;
                rows.iter()
                    .map(sqlite_row_to_entry)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
        }
    }

    /// 插入或覆盖设置（以 `(namespace, key)` 为键），返回写入后的实体。
    ///
    /// 新增行时生成 v4 UUID；命中既有行时 `id` 保持不变，`value` / `updated_at` 被覆盖。
    ///
    /// # Errors
    /// - namespace / key 归一后为空 → [`cycms_core::Error::ValidationError`]
    /// - DB 故障 / JSON 解码失败 → [`cycms_core::Error::Internal`]
    pub async fn upsert(&self, namespace: &str, key: &str, value: Value) -> Result<SettingEntry> {
        let (ns, k) = normalize_pair(namespace, key)?;
        let new_id = Uuid::new_v4().to_string();

        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                sqlx::query(PG_UPSERT)
                    .bind(&new_id)
                    .bind(&ns)
                    .bind(&k)
                    .bind(Json(value))
                    .execute(pool)
                    .await
                    .map_err(SettingsError::Database)?;
            }
            DatabasePool::MySql(pool) => {
                sqlx::query(MYSQL_UPSERT)
                    .bind(&new_id)
                    .bind(&ns)
                    .bind(&k)
                    .bind(Json(value))
                    .execute(pool)
                    .await
                    .map_err(SettingsError::Database)?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(SQLITE_UPSERT)
                    .bind(&new_id)
                    .bind(&ns)
                    .bind(&k)
                    .bind(Json(value))
                    .execute(pool)
                    .await
                    .map_err(SettingsError::Database)?;
            }
        }

        self.find(&ns, &k)
            .await?
            .ok_or_else(|| cycms_core::Error::Internal {
                message: "upserted setting not found on read-back".to_owned(),
                source: None,
            })
    }

    /// 删除指定设置，返回是否有行被实际清除。
    ///
    /// # Errors
    /// - namespace / key 归一后为空 → [`cycms_core::Error::ValidationError`]
    /// - DB 故障 → [`cycms_core::Error::Internal`]
    pub async fn delete(&self, namespace: &str, key: &str) -> Result<bool> {
        let (ns, k) = normalize_pair(namespace, key)?;
        let affected = match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                sqlx::query("DELETE FROM settings WHERE namespace = $1 AND key = $2")
                    .bind(&ns)
                    .bind(&k)
                    .execute(pool)
                    .await
                    .map_err(SettingsError::Database)?
                    .rows_affected()
            }
            DatabasePool::MySql(pool) => {
                sqlx::query("DELETE FROM settings WHERE namespace = ? AND `key` = ?")
                    .bind(&ns)
                    .bind(&k)
                    .execute(pool)
                    .await
                    .map_err(SettingsError::Database)?
                    .rows_affected()
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query("DELETE FROM settings WHERE namespace = ? AND key = ?")
                    .bind(&ns)
                    .bind(&k)
                    .execute(pool)
                    .await
                    .map_err(SettingsError::Database)?
                    .rows_affected()
            }
        };
        Ok(affected > 0)
    }
}

/// `plugin_settings_schemas` 表的三方言 CRUD。
///
/// 使用 `ON CONFLICT (plugin_name) DO UPDATE` 语义：重复注册同一个插件会覆盖
/// `schema` 字段，`created_at` 保留首次注册时间，便于追踪插件 schema 首次出现时点。
pub struct PluginSchemaRepository {
    db: Arc<DatabasePool>,
}

impl PluginSchemaRepository {
    #[must_use]
    pub fn new(db: Arc<DatabasePool>) -> Self {
        Self { db }
    }

    /// 注册或覆盖插件 schema，返回写入后的实体。
    ///
    /// # Errors
    /// - `plugin_name` 归一后为空 → [`cycms_core::Error::ValidationError`]
    /// - DB 故障 / JSON 解码失败 → [`cycms_core::Error::Internal`]
    pub async fn upsert(&self, plugin_name: &str, schema: Value) -> Result<PluginSchema> {
        let name = normalize_plugin_name(plugin_name)?;

        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                sqlx::query(PG_SCHEMA_UPSERT)
                    .bind(&name)
                    .bind(Json(schema))
                    .execute(pool)
                    .await
                    .map_err(SettingsError::Database)?;
            }
            DatabasePool::MySql(pool) => {
                sqlx::query(MYSQL_SCHEMA_UPSERT)
                    .bind(&name)
                    .bind(Json(schema))
                    .execute(pool)
                    .await
                    .map_err(SettingsError::Database)?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(SQLITE_SCHEMA_UPSERT)
                    .bind(&name)
                    .bind(Json(schema))
                    .execute(pool)
                    .await
                    .map_err(SettingsError::Database)?;
            }
        }

        self.find(&name)
            .await?
            .ok_or_else(|| cycms_core::Error::Internal {
                message: "upserted plugin schema not found on read-back".to_owned(),
                source: None,
            })
    }

    /// 查找插件 schema。
    ///
    /// # Errors
    /// - `plugin_name` 归一后为空 → [`cycms_core::Error::ValidationError`]
    /// - DB 故障 / JSON 解码失败 → [`cycms_core::Error::Internal`]
    pub async fn find(&self, plugin_name: &str) -> Result<Option<PluginSchema>> {
        let name = normalize_plugin_name(plugin_name)?;
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(PG_SCHEMA_SELECT)
                    .bind(&name)
                    .fetch_optional(pool)
                    .await
                    .map_err(SettingsError::Database)?;
                row.map(|r| pg_row_to_schema(&r))
                    .transpose()
                    .map_err(Into::into)
            }
            DatabasePool::MySql(pool) => {
                let row = sqlx::query(MYSQL_SCHEMA_SELECT)
                    .bind(&name)
                    .fetch_optional(pool)
                    .await
                    .map_err(SettingsError::Database)?;
                row.map(|r| mysql_row_to_schema(&r))
                    .transpose()
                    .map_err(Into::into)
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(SQLITE_SCHEMA_SELECT)
                    .bind(&name)
                    .fetch_optional(pool)
                    .await
                    .map_err(SettingsError::Database)?;
                row.map(|r| sqlite_row_to_schema(&r))
                    .transpose()
                    .map_err(Into::into)
            }
        }
    }

    /// 列出所有插件 schema，按 `plugin_name` 升序。
    ///
    /// # Errors
    /// DB 故障 / JSON 解码失败 → [`cycms_core::Error::Internal`]。
    pub async fn list(&self) -> Result<Vec<PluginSchema>> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(PG_SCHEMA_SELECT_ALL)
                    .fetch_all(pool)
                    .await
                    .map_err(SettingsError::Database)?;
                rows.iter()
                    .map(pg_row_to_schema)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
            DatabasePool::MySql(pool) => {
                let rows = sqlx::query(MYSQL_SCHEMA_SELECT_ALL)
                    .fetch_all(pool)
                    .await
                    .map_err(SettingsError::Database)?;
                rows.iter()
                    .map(mysql_row_to_schema)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(SQLITE_SCHEMA_SELECT_ALL)
                    .fetch_all(pool)
                    .await
                    .map_err(SettingsError::Database)?;
                rows.iter()
                    .map(sqlite_row_to_schema)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            }
        }
    }

    /// 删除插件 schema，返回是否有行被实际清除。
    ///
    /// # Errors
    /// - `plugin_name` 归一后为空 → [`cycms_core::Error::ValidationError`]
    /// - DB 故障 → [`cycms_core::Error::Internal`]
    pub async fn delete(&self, plugin_name: &str) -> Result<bool> {
        let name = normalize_plugin_name(plugin_name)?;
        let affected = match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                sqlx::query("DELETE FROM plugin_settings_schemas WHERE plugin_name = $1")
                    .bind(&name)
                    .execute(pool)
                    .await
                    .map_err(SettingsError::Database)?
                    .rows_affected()
            }
            DatabasePool::MySql(pool) => {
                sqlx::query("DELETE FROM plugin_settings_schemas WHERE plugin_name = ?")
                    .bind(&name)
                    .execute(pool)
                    .await
                    .map_err(SettingsError::Database)?
                    .rows_affected()
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query("DELETE FROM plugin_settings_schemas WHERE plugin_name = ?")
                    .bind(&name)
                    .execute(pool)
                    .await
                    .map_err(SettingsError::Database)?
                    .rows_affected()
            }
        };
        Ok(affected > 0)
    }
}

fn normalize_pair(namespace: &str, key: &str) -> Result<(String, String)> {
    Ok((normalize_namespace(namespace)?, normalize_key(key)?))
}

fn normalize_namespace(namespace: &str) -> Result<String> {
    let trimmed = namespace.trim();
    if trimmed.is_empty() {
        return Err(
            SettingsError::InputValidation("namespace must not be empty".to_owned()).into(),
        );
    }
    Ok(trimmed.to_owned())
}

fn normalize_key(key: &str) -> Result<String> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err(SettingsError::InputValidation("key must not be empty".to_owned()).into());
    }
    Ok(trimmed.to_owned())
}

fn normalize_plugin_name(plugin_name: &str) -> Result<String> {
    let trimmed = plugin_name.trim();
    if trimmed.is_empty() {
        return Err(
            SettingsError::InputValidation("plugin_name must not be empty".to_owned()).into(),
        );
    }
    Ok(trimmed.to_owned())
}

const PG_SELECT_BY_NS_KEY: &str = "SELECT id::TEXT AS id, namespace, key, value, updated_at \
     FROM settings WHERE namespace = $1 AND key = $2";
const PG_SELECT_BY_NS: &str = "SELECT id::TEXT AS id, namespace, key, value, updated_at \
     FROM settings WHERE namespace = $1 ORDER BY key";
const PG_UPSERT: &str = "INSERT INTO settings (id, namespace, key, value) \
     VALUES ($1::UUID, $2, $3, $4) \
     ON CONFLICT (namespace, key) DO UPDATE \
       SET value = EXCLUDED.value, updated_at = now()";

const MYSQL_SELECT_BY_NS_KEY: &str = "SELECT id, namespace, `key` AS `key`, value, updated_at \
     FROM settings WHERE namespace = ? AND `key` = ?";
const MYSQL_SELECT_BY_NS: &str = "SELECT id, namespace, `key` AS `key`, value, updated_at \
     FROM settings WHERE namespace = ? ORDER BY `key`";
const MYSQL_UPSERT: &str = "INSERT INTO settings (id, namespace, `key`, value) \
     VALUES (?, ?, ?, ?) \
     ON DUPLICATE KEY UPDATE \
       value = VALUES(value), updated_at = CURRENT_TIMESTAMP(6)";

const SQLITE_SELECT_BY_NS_KEY: &str = "SELECT id, namespace, key, value, updated_at \
     FROM settings WHERE namespace = ? AND key = ?";
const SQLITE_SELECT_BY_NS: &str = "SELECT id, namespace, key, value, updated_at \
     FROM settings WHERE namespace = ? ORDER BY key";
const SQLITE_UPSERT: &str = "INSERT INTO settings (id, namespace, key, value) \
     VALUES (?, ?, ?, ?) \
     ON CONFLICT (namespace, key) DO UPDATE \
       SET value = excluded.value, \
           updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')";

const PG_SCHEMA_SELECT: &str = "SELECT plugin_name, schema, created_at \
     FROM plugin_settings_schemas WHERE plugin_name = $1";
const PG_SCHEMA_SELECT_ALL: &str = "SELECT plugin_name, schema, created_at \
     FROM plugin_settings_schemas ORDER BY plugin_name";
const PG_SCHEMA_UPSERT: &str = "INSERT INTO plugin_settings_schemas (plugin_name, schema) \
     VALUES ($1, $2) \
     ON CONFLICT (plugin_name) DO UPDATE SET schema = EXCLUDED.schema";

const MYSQL_SCHEMA_SELECT: &str = "SELECT plugin_name, `schema` AS `schema`, created_at \
     FROM plugin_settings_schemas WHERE plugin_name = ?";
const MYSQL_SCHEMA_SELECT_ALL: &str = "SELECT plugin_name, `schema` AS `schema`, created_at \
     FROM plugin_settings_schemas ORDER BY plugin_name";
const MYSQL_SCHEMA_UPSERT: &str = "INSERT INTO plugin_settings_schemas (plugin_name, `schema`) \
     VALUES (?, ?) \
     ON DUPLICATE KEY UPDATE `schema` = VALUES(`schema`)";

const SQLITE_SCHEMA_SELECT: &str = "SELECT plugin_name, schema, created_at \
     FROM plugin_settings_schemas WHERE plugin_name = ?";
const SQLITE_SCHEMA_SELECT_ALL: &str = "SELECT plugin_name, schema, created_at \
     FROM plugin_settings_schemas ORDER BY plugin_name";
const SQLITE_SCHEMA_UPSERT: &str = "INSERT INTO plugin_settings_schemas (plugin_name, schema) \
     VALUES (?, ?) \
     ON CONFLICT (plugin_name) DO UPDATE SET schema = excluded.schema";

fn pg_row_to_entry(row: &PgRow) -> std::result::Result<SettingEntry, SettingsError> {
    let value: Json<Value> = row.try_get("value").map_err(SettingsError::Database)?;
    Ok(SettingEntry {
        id: row.try_get("id").map_err(SettingsError::Database)?,
        namespace: row.try_get("namespace").map_err(SettingsError::Database)?,
        key: row.try_get("key").map_err(SettingsError::Database)?,
        value: value.0,
        updated_at: row.try_get("updated_at").map_err(SettingsError::Database)?,
    })
}

fn mysql_row_to_entry(row: &MySqlRow) -> std::result::Result<SettingEntry, SettingsError> {
    let value: Json<Value> = row.try_get("value").map_err(SettingsError::Database)?;
    let updated_at: NaiveDateTime = row.try_get("updated_at").map_err(SettingsError::Database)?;
    Ok(SettingEntry {
        id: row.try_get("id").map_err(SettingsError::Database)?,
        namespace: row.try_get("namespace").map_err(SettingsError::Database)?,
        key: row.try_get("key").map_err(SettingsError::Database)?,
        value: value.0,
        updated_at: updated_at.and_utc(),
    })
}

fn sqlite_row_to_entry(row: &SqliteRow) -> std::result::Result<SettingEntry, SettingsError> {
    let value: Json<Value> = row.try_get("value").map_err(SettingsError::Database)?;
    Ok(SettingEntry {
        id: row.try_get("id").map_err(SettingsError::Database)?,
        namespace: row.try_get("namespace").map_err(SettingsError::Database)?,
        key: row.try_get("key").map_err(SettingsError::Database)?,
        value: value.0,
        updated_at: row.try_get("updated_at").map_err(SettingsError::Database)?,
    })
}

fn pg_row_to_schema(row: &PgRow) -> std::result::Result<PluginSchema, SettingsError> {
    let schema: Json<Value> = row.try_get("schema").map_err(SettingsError::Database)?;
    Ok(PluginSchema {
        plugin_name: row
            .try_get("plugin_name")
            .map_err(SettingsError::Database)?,
        schema: schema.0,
        created_at: row.try_get("created_at").map_err(SettingsError::Database)?,
    })
}

fn mysql_row_to_schema(row: &MySqlRow) -> std::result::Result<PluginSchema, SettingsError> {
    let schema: Json<Value> = row.try_get("schema").map_err(SettingsError::Database)?;
    let created_at: NaiveDateTime = row.try_get("created_at").map_err(SettingsError::Database)?;
    Ok(PluginSchema {
        plugin_name: row
            .try_get("plugin_name")
            .map_err(SettingsError::Database)?,
        schema: schema.0,
        created_at: created_at.and_utc(),
    })
}

fn sqlite_row_to_schema(row: &SqliteRow) -> std::result::Result<PluginSchema, SettingsError> {
    let schema: Json<Value> = row.try_get("schema").map_err(SettingsError::Database)?;
    Ok(PluginSchema {
        plugin_name: row
            .try_get("plugin_name")
            .map_err(SettingsError::Database)?,
        schema: schema.0,
        created_at: row.try_get("created_at").map_err(SettingsError::Database)?,
    })
}

#[cfg(test)]
mod tests {
    use super::{normalize_key, normalize_namespace};

    #[test]
    fn normalize_trims_whitespace() {
        assert_eq!(normalize_namespace("  ui.theme  ").unwrap(), "ui.theme");
        assert_eq!(normalize_key("\tdark\n").unwrap(), "dark");
    }

    #[test]
    fn normalize_rejects_empty_after_trim() {
        assert!(normalize_namespace("   ").is_err());
        assert!(normalize_key("").is_err());
    }
}
