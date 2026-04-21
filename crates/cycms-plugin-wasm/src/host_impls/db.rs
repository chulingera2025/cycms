//! `db` host 组：原始 SQL 入口，完全信任模型下直接作用于当前 [`DatabasePool`]。
//!
//! 协议（JSON 字符串穿透）：
//!
//! - `params-json` 为 JSON 数组，每个元素按类型绑定：`null` / `bool` / `number` /
//!   `string` / `array|object`（对象与数组优先按 JSON 参数绑定；不支持原生 JSON 参数
//!   的方言退化为序列化字符串）。
//! - `query` 返回 JSON 数组，每行一个对象：键为列名，值按 `SQLite` 动态类型还原
//!   `NULL` / `INTEGER` → number / `REAL` → number / `TEXT` → string / `BLOB` → base64。
//! - `execute` 返回影响行数。

use cycms_db::DatabasePool;
use serde_json::{Map, Number, Value};
use sqlx::Column;
use sqlx::Row;
use sqlx::TypeInfo;
use sqlx::mysql::MySqlRow;
use sqlx::postgres::PgRow;
use sqlx::sqlite::SqliteRow;

use crate::bindings::cycms::plugin::db::Host;
use crate::host::HostState;

fn parse_params(params_json: &str) -> Result<Vec<Value>, String> {
    if params_json.trim().is_empty() {
        return Ok(Vec::new());
    }
    match serde_json::from_str::<Value>(params_json) {
        Ok(Value::Array(a)) => Ok(a),
        Ok(_) => Err("db: params-json must be a JSON array".into()),
        Err(e) => Err(format!("db: invalid params-json: {e}")),
    }
}

fn sqlite_column_to_json(row: &SqliteRow, idx: usize) -> Value {
    // 针对 SQLite 动态类型分别尝试；未知类型降级到 TEXT。
    let type_name = row.columns()[idx].type_info().name();
    match type_name {
        "NULL" => Value::Null,
        "INTEGER" => row
            .try_get::<Option<i64>, _>(idx)
            .ok()
            .flatten()
            .map_or(Value::Null, |v| Value::Number(Number::from(v))),
        "REAL" => row
            .try_get::<Option<f64>, _>(idx)
            .ok()
            .flatten()
            .and_then(|v| Number::from_f64(v).map(Value::Number))
            .unwrap_or(Value::Null),
        "BLOB" => row
            .try_get::<Option<Vec<u8>>, _>(idx)
            .ok()
            .flatten()
            .map_or(Value::Null, |bytes| {
                use base64::Engine;
                Value::String(base64::engine::general_purpose::STANDARD.encode(&bytes))
            }),
        _ => row
            .try_get::<Option<String>, _>(idx)
            .ok()
            .flatten()
            .map_or(Value::Null, Value::String),
    }
}

fn postgres_column_to_json(row: &PgRow, idx: usize) -> Value {
    let type_name = row.columns()[idx].type_info().name().to_ascii_uppercase();
    match type_name.as_str() {
        "BOOL" => row
            .try_get::<Option<bool>, _>(idx)
            .ok()
            .flatten()
            .map_or(Value::Null, Value::Bool),
        "INT2" | "INT4" | "INT8" | "OID" => row
            .try_get::<Option<i64>, _>(idx)
            .ok()
            .flatten()
            .map_or(Value::Null, |v| Value::Number(Number::from(v))),
        "FLOAT4" | "FLOAT8" | "NUMERIC" => row
            .try_get::<Option<f64>, _>(idx)
            .ok()
            .flatten()
            .and_then(|v| Number::from_f64(v).map(Value::Number))
            .unwrap_or(Value::Null),
        "JSON" | "JSONB" => row
            .try_get::<Option<Value>, _>(idx)
            .ok()
            .flatten()
            .unwrap_or(Value::Null),
        "BYTEA" => row
            .try_get::<Option<Vec<u8>>, _>(idx)
            .ok()
            .flatten()
            .map_or(Value::Null, bytes_to_base64),
        "UUID" => row
            .try_get::<Option<uuid::Uuid>, _>(idx)
            .ok()
            .flatten()
            .map_or(Value::Null, |v| Value::String(v.to_string())),
        "TIMESTAMPTZ" => row
            .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(idx)
            .ok()
            .flatten()
            .map_or(Value::Null, |v| Value::String(v.to_rfc3339())),
        "TIMESTAMP" => row
            .try_get::<Option<chrono::NaiveDateTime>, _>(idx)
            .ok()
            .flatten()
            .map_or(Value::Null, |v| Value::String(v.to_string())),
        "DATE" => row
            .try_get::<Option<chrono::NaiveDate>, _>(idx)
            .ok()
            .flatten()
            .map_or(Value::Null, |v| Value::String(v.to_string())),
        _ => row
            .try_get::<Option<String>, _>(idx)
            .ok()
            .flatten()
            .map_or(Value::Null, Value::String),
    }
}

fn mysql_column_to_json(row: &MySqlRow, idx: usize) -> Value {
    let type_name = row.columns()[idx].type_info().name().to_ascii_uppercase();
    match type_name.as_str() {
        "BOOL" | "BOOLEAN" => row
            .try_get::<Option<bool>, _>(idx)
            .ok()
            .flatten()
            .map_or(Value::Null, Value::Bool),
        "TINYINT" | "SMALLINT" | "MEDIUMINT" | "INT" | "INTEGER" | "BIGINT" => row
            .try_get::<Option<i64>, _>(idx)
            .ok()
            .flatten()
            .map_or(Value::Null, |v| Value::Number(Number::from(v))),
        "FLOAT" | "DOUBLE" | "DECIMAL" => row
            .try_get::<Option<f64>, _>(idx)
            .ok()
            .flatten()
            .and_then(|v| Number::from_f64(v).map(Value::Number))
            .unwrap_or(Value::Null),
        "JSON" => row
            .try_get::<Option<Value>, _>(idx)
            .ok()
            .flatten()
            .unwrap_or(Value::Null),
        "BINARY" | "VARBINARY" | "BIT" | "BLOB" | "TINYBLOB" | "MEDIUMBLOB" | "LONGBLOB" => row
            .try_get::<Option<Vec<u8>>, _>(idx)
            .ok()
            .flatten()
            .map_or(Value::Null, bytes_to_base64),
        "TIMESTAMP" | "DATETIME" => row
            .try_get::<Option<chrono::NaiveDateTime>, _>(idx)
            .ok()
            .flatten()
            .map_or(Value::Null, |v| Value::String(v.to_string())),
        "DATE" => row
            .try_get::<Option<chrono::NaiveDate>, _>(idx)
            .ok()
            .flatten()
            .map_or(Value::Null, |v| Value::String(v.to_string())),
        "TIME" => row
            .try_get::<Option<chrono::NaiveTime>, _>(idx)
            .ok()
            .flatten()
            .map_or(Value::Null, |v| Value::String(v.to_string())),
        _ => row
            .try_get::<Option<String>, _>(idx)
            .ok()
            .flatten()
            .map_or(Value::Null, Value::String),
    }
}

fn bytes_to_base64(bytes: Vec<u8>) -> Value {
    use base64::Engine;
    Value::String(base64::engine::general_purpose::STANDARD.encode(bytes))
}

async fn sqlite_query(
    pool: &sqlx::SqlitePool,
    sql: &str,
    params: &[Value],
) -> Result<Vec<Value>, String> {
    let mut q = sqlx::query(sql);
    for v in params {
        q = bind_sqlite(q, v);
    }
    let rows = q
        .fetch_all(pool)
        .await
        .map_err(|e| format!("db.query: {e}"))?;
    let mut out = Vec::with_capacity(rows.len());
    for row in &rows {
        let mut obj = Map::new();
        for (i, col) in row.columns().iter().enumerate() {
            obj.insert(col.name().to_owned(), sqlite_column_to_json(row, i));
        }
        out.push(Value::Object(obj));
    }
    Ok(out)
}

async fn sqlite_execute(
    pool: &sqlx::SqlitePool,
    sql: &str,
    params: &[Value],
) -> Result<u64, String> {
    let mut q = sqlx::query(sql);
    for v in params {
        q = bind_sqlite(q, v);
    }
    q.execute(pool)
        .await
        .map(|r| r.rows_affected())
        .map_err(|e| format!("db.execute: {e}"))
}

async fn postgres_query(
    pool: &sqlx::PgPool,
    sql: &str,
    params: &[Value],
) -> Result<Vec<Value>, String> {
    let mut q = sqlx::query(sql);
    for v in params {
        q = bind_postgres(q, v);
    }
    let rows = q
        .fetch_all(pool)
        .await
        .map_err(|e| format!("db.query: {e}"))?;
    let mut out = Vec::with_capacity(rows.len());
    for row in &rows {
        let mut obj = Map::new();
        for (i, col) in row.columns().iter().enumerate() {
            obj.insert(col.name().to_owned(), postgres_column_to_json(row, i));
        }
        out.push(Value::Object(obj));
    }
    Ok(out)
}

async fn postgres_execute(pool: &sqlx::PgPool, sql: &str, params: &[Value]) -> Result<u64, String> {
    let mut q = sqlx::query(sql);
    for v in params {
        q = bind_postgres(q, v);
    }
    q.execute(pool)
        .await
        .map(|r| r.rows_affected())
        .map_err(|e| format!("db.execute: {e}"))
}

async fn mysql_query(
    pool: &sqlx::MySqlPool,
    sql: &str,
    params: &[Value],
) -> Result<Vec<Value>, String> {
    let mut q = sqlx::query(sql);
    for v in params {
        q = bind_mysql(q, v);
    }
    let rows = q
        .fetch_all(pool)
        .await
        .map_err(|e| format!("db.query: {e}"))?;
    let mut out = Vec::with_capacity(rows.len());
    for row in &rows {
        let mut obj = Map::new();
        for (i, col) in row.columns().iter().enumerate() {
            obj.insert(col.name().to_owned(), mysql_column_to_json(row, i));
        }
        out.push(Value::Object(obj));
    }
    Ok(out)
}

async fn mysql_execute(pool: &sqlx::MySqlPool, sql: &str, params: &[Value]) -> Result<u64, String> {
    let mut q = sqlx::query(sql);
    for v in params {
        q = bind_mysql(q, v);
    }
    q.execute(pool)
        .await
        .map(|r| r.rows_affected())
        .map_err(|e| format!("db.execute: {e}"))
}

fn bind_sqlite<'q>(
    q: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
    v: &Value,
) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
    match v {
        Value::Null => q.bind(None::<String>),
        Value::Bool(b) => q.bind(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                q.bind(i)
            } else if let Some(f) = n.as_f64() {
                q.bind(f)
            } else {
                q.bind(n.to_string())
            }
        }
        Value::String(s) => q.bind(s.clone()),
        other => q.bind(other.to_string()),
    }
}

fn bind_postgres<'q>(
    q: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    v: &Value,
) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
    match v {
        Value::Null => q.bind(None::<String>),
        Value::Bool(b) => q.bind(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                q.bind(i)
            } else if let Some(f) = n.as_f64() {
                q.bind(f)
            } else {
                q.bind(n.to_string())
            }
        }
        Value::String(s) => q.bind(s.clone()),
        other => q.bind(other.clone()),
    }
}

fn bind_mysql<'q>(
    q: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
    v: &Value,
) -> sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments> {
    match v {
        Value::Null => q.bind(None::<String>),
        Value::Bool(b) => q.bind(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                q.bind(i)
            } else if let Some(f) = n.as_f64() {
                q.bind(f)
            } else {
                q.bind(n.to_string())
            }
        }
        Value::String(s) => q.bind(s.clone()),
        other => q.bind(other.clone()),
    }
}

impl Host for HostState {
    async fn query(
        &mut self,
        sql: String,
        params_json: String,
    ) -> wasmtime::Result<Result<String, String>> {
        let params = match parse_params(&params_json) {
            Ok(p) => p,
            Err(e) => return Ok(Err(e)),
        };
        let rows_result = match &*self.db {
            DatabasePool::Sqlite(p) => sqlite_query(p, &sql, &params).await,
            DatabasePool::Postgres(p) => postgres_query(p, &sql, &params).await,
            DatabasePool::MySql(p) => mysql_query(p, &sql, &params).await,
        };
        match rows_result {
            Ok(rows) => match serde_json::to_string(&rows) {
                Ok(s) => Ok(Ok(s)),
                Err(e) => Ok(Err(format!("db.query: serialize rows: {e}"))),
            },
            Err(msg) => Ok(Err(msg)),
        }
    }

    async fn execute(
        &mut self,
        sql: String,
        params_json: String,
    ) -> wasmtime::Result<Result<u64, String>> {
        let params = match parse_params(&params_json) {
            Ok(p) => p,
            Err(e) => return Ok(Err(e)),
        };
        match &*self.db {
            DatabasePool::Sqlite(p) => match sqlite_execute(p, &sql, &params).await {
                Ok(n) => Ok(Ok(n)),
                Err(msg) => Ok(Err(msg)),
            },
            DatabasePool::Postgres(p) => match postgres_execute(p, &sql, &params).await {
                Ok(n) => Ok(Ok(n)),
                Err(msg) => Ok(Err(msg)),
            },
            DatabasePool::MySql(p) => match mysql_execute(p, &sql, &params).await {
                Ok(n) => Ok(Ok(n)),
                Err(msg) => Ok(Err(msg)),
            },
        }
    }
}
