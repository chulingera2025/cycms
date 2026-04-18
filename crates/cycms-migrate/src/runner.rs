//! 单条迁移的事务化执行器。
//!
//! 三方言实现独立，以避免 sqlx 的 `Transaction` 跨方言类型差异。
//! 共享的职责：在一个事务内执行 `up` SQL，再写入元表行，整条语义失败即回滚。

use std::time::Instant;

use chrono::Utc;
use cycms_core::{Error, Result};
use cycms_db::DatabasePool;
use sqlx::{MySqlPool, PgPool, Row, SqlitePool};

use crate::discovery::DiscoveredMigration;
use crate::record::{MigrationRecord, MigrationStatus};

pub(crate) async fn list_applied_versions(db: &DatabasePool, source: &str) -> Result<Vec<i64>> {
    match db {
        DatabasePool::Postgres(pool) => sqlx::query_scalar::<_, i64>(
            "SELECT version FROM migration_records WHERE source = $1 AND status = 'applied'",
        )
        .bind(source)
        .fetch_all(pool)
        .await
        .map_err(map_db_error("failed to list applied versions")),
        DatabasePool::MySql(pool) => sqlx::query_scalar::<_, i64>(
            "SELECT version FROM migration_records WHERE source = ? AND status = 'applied'",
        )
        .bind(source)
        .fetch_all(pool)
        .await
        .map_err(map_db_error("failed to list applied versions")),
        DatabasePool::Sqlite(pool) => sqlx::query_scalar::<_, i64>(
            "SELECT version FROM migration_records WHERE source = ? AND status = 'applied'",
        )
        .bind(source)
        .fetch_all(pool)
        .await
        .map_err(map_db_error("failed to list applied versions")),
    }
}

pub(crate) async fn apply_one(
    db: &DatabasePool,
    source: &str,
    migration: &DiscoveredMigration,
) -> Result<MigrationRecord> {
    let started = Instant::now();
    match db {
        DatabasePool::Postgres(pool) => {
            apply_postgres(pool, source, migration, started).await
        }
        DatabasePool::MySql(pool) => apply_mysql(pool, source, migration, started).await,
        DatabasePool::Sqlite(pool) => apply_sqlite(pool, source, migration, started).await,
    }
}

async fn apply_postgres(
    pool: &PgPool,
    source: &str,
    migration: &DiscoveredMigration,
    started: Instant,
) -> Result<MigrationRecord> {
    let mut tx = pool
        .begin()
        .await
        .map_err(map_db_error("failed to begin transaction"))?;

    sqlx::raw_sql(&migration.up_sql)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error("failed to execute up migration"))?;

    let elapsed_ms = elapsed_millis_i64(started);
    let row = sqlx::query(
        "INSERT INTO migration_records \
         (version, name, source, checksum, execution_time_ms, status) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING id, applied_at",
    )
    .bind(migration.version)
    .bind(&migration.name)
    .bind(source)
    .bind(migration.checksum.as_slice())
    .bind(elapsed_ms)
    .bind(MigrationStatus::Applied.as_str())
    .fetch_one(&mut *tx)
    .await
    .map_err(map_db_error("failed to insert migration record"))?;

    let id: i64 = row.try_get("id").map_err(map_db_error("missing id"))?;
    let applied_at: chrono::DateTime<Utc> = row
        .try_get("applied_at")
        .map_err(map_db_error("missing applied_at"))?;

    tx.commit()
        .await
        .map_err(map_db_error("failed to commit migration"))?;

    Ok(MigrationRecord {
        id,
        version: migration.version,
        name: migration.name.clone(),
        source: source.to_owned(),
        applied_at,
        execution_time_ms: elapsed_ms,
        status: MigrationStatus::Applied,
    })
}

async fn apply_mysql(
    pool: &MySqlPool,
    source: &str,
    migration: &DiscoveredMigration,
    started: Instant,
) -> Result<MigrationRecord> {
    let mut tx = pool
        .begin()
        .await
        .map_err(map_db_error("failed to begin transaction"))?;

    sqlx::raw_sql(&migration.up_sql)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error("failed to execute up migration"))?;

    let elapsed_ms = elapsed_millis_i64(started);
    let result = sqlx::query(
        "INSERT INTO migration_records \
         (version, name, source, checksum, execution_time_ms, status) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(migration.version)
    .bind(&migration.name)
    .bind(source)
    .bind(migration.checksum.as_slice())
    .bind(elapsed_ms)
    .bind(MigrationStatus::Applied.as_str())
    .execute(&mut *tx)
    .await
    .map_err(map_db_error("failed to insert migration record"))?;

    let id = i64::try_from(result.last_insert_id()).map_err(|_| Error::Internal {
        message: "migration record id exceeds i64 range".to_owned(),
        source: None,
    })?;

    tx.commit()
        .await
        .map_err(map_db_error("failed to commit migration"))?;

    // MySQL 不支持跨事务的 RETURNING；applied_at 读取其默认生成值。
    Ok(MigrationRecord {
        id,
        version: migration.version,
        name: migration.name.clone(),
        source: source.to_owned(),
        applied_at: Utc::now(),
        execution_time_ms: elapsed_ms,
        status: MigrationStatus::Applied,
    })
}

async fn apply_sqlite(
    pool: &SqlitePool,
    source: &str,
    migration: &DiscoveredMigration,
    started: Instant,
) -> Result<MigrationRecord> {
    let mut tx = pool
        .begin()
        .await
        .map_err(map_db_error("failed to begin transaction"))?;

    sqlx::raw_sql(&migration.up_sql)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error("failed to execute up migration"))?;

    let elapsed_ms = elapsed_millis_i64(started);
    let row = sqlx::query(
        "INSERT INTO migration_records \
         (version, name, source, checksum, execution_time_ms, status) \
         VALUES (?, ?, ?, ?, ?, ?) \
         RETURNING id, applied_at",
    )
    .bind(migration.version)
    .bind(&migration.name)
    .bind(source)
    .bind(migration.checksum.as_slice())
    .bind(elapsed_ms)
    .bind(MigrationStatus::Applied.as_str())
    .fetch_one(&mut *tx)
    .await
    .map_err(map_db_error("failed to insert migration record"))?;

    let id: i64 = row.try_get("id").map_err(map_db_error("missing id"))?;
    // SQLite 的 applied_at 以 ISO8601 文本存储，客户端解析为 DateTime<Utc>。
    let applied_at_raw: String = row
        .try_get("applied_at")
        .map_err(map_db_error("missing applied_at"))?;
    let applied_at = parse_sqlite_timestamp(&applied_at_raw)?;

    tx.commit()
        .await
        .map_err(map_db_error("failed to commit migration"))?;

    Ok(MigrationRecord {
        id,
        version: migration.version,
        name: migration.name.clone(),
        source: source.to_owned(),
        applied_at,
        execution_time_ms: elapsed_ms,
        status: MigrationStatus::Applied,
    })
}

fn elapsed_millis_i64(started: Instant) -> i64 {
    i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX)
}

fn map_db_error(
    message: &'static str,
) -> impl FnOnce(sqlx::Error) -> Error + 'static {
    move |source| Error::Internal {
        message: message.to_owned(),
        source: Some(Box::new(source)),
    }
}

fn parse_sqlite_timestamp(raw: &str) -> Result<chrono::DateTime<Utc>> {
    // 元表 DDL 生成的默认值形如 `2026-04-19T10:20:30.123Z`。
    chrono::DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|source| Error::Internal {
            message: format!("invalid applied_at timestamp in sqlite: {raw}"),
            source: Some(Box::new(source)),
        })
}
