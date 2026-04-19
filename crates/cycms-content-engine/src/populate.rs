//! 单层关联加载（任务 11 Step 5）。
//!
//! v0.1 仅支持深度 1，深度 > 1 触发 [`ContentEngineError::PopulateDepthExceeded`]。
//! 服务层调用 [`populate_entries`] 把 `content_relations` 中的目标 entries 按
//! `field_api_id` 分组注入到 source `ContentEntry::populated`。
//!
//! Relation 字段的 `relation_kind` 不参与 SQL 选择（v0.1 一律按 (`source_id`,
//! `field_api_id`, position) 抓取），由调用方根据 [`crate::FieldRef`] 自行决定取首
//! 条（`OneToOne`）还是整组（`OneToMany` / `ManyToMany`）。

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Arc;

use cycms_core::Result;
use cycms_db::{DatabasePool, DatabaseType};
use sqlx::Row;

use crate::error::ContentEngineError;
use crate::model::ContentEntry;
use crate::repository::ContentEntryRepository;

/// 单层 populate 深度上限（v0.1 固定 1）。
pub const SINGLE_LEVEL_DEPTH: u32 = 1;

/// 校验 populate 深度是否在允许范围内。`depth=0` 视作不展开。
///
/// # Errors
/// `depth>SINGLE_LEVEL_DEPTH` 时返回 [`ContentEngineError::PopulateDepthExceeded`]。
pub fn check_depth(depth: u32) -> std::result::Result<(), ContentEngineError> {
    if depth > SINGLE_LEVEL_DEPTH {
        Err(ContentEngineError::PopulateDepthExceeded {
            depth,
            max: SINGLE_LEVEL_DEPTH,
        })
    } else {
        Ok(())
    }
}

/// 加载 `content_relations` 中按 `populate` 列表展开的 target entries，按
/// `field_api_id` 分组写入 source entries 的 `populated` 字段。
///
/// 空 `populate` 或空 `entries` 视作 no-op。被引用但已不存在的 target id
/// （脏数据）会被跳过且不报错。
///
/// # Errors
/// - DB 故障 → [`ContentEngineError::Database`]
pub async fn populate_entries(
    db: &Arc<DatabasePool>,
    repo: &ContentEntryRepository,
    mut entries: Vec<ContentEntry>,
    populate: &[String],
) -> Result<Vec<ContentEntry>> {
    if populate.is_empty() || entries.is_empty() {
        return Ok(entries);
    }

    let source_ids: Vec<String> = entries.iter().map(|e| e.id.clone()).collect();
    let relations = fetch_relations(db, &source_ids, populate).await?;
    if relations.is_empty() {
        return Ok(entries);
    }

    let mut target_ids: Vec<String> = relations
        .iter()
        .map(|r| r.target_entry_id.clone())
        .collect();
    target_ids.sort();
    target_ids.dedup();

    let mut targets_by_id: HashMap<String, ContentEntry> = HashMap::new();
    for tid in &target_ids {
        if let Some(t) = repo.find_by_id(tid).await? {
            targets_by_id.insert(tid.clone(), t);
        }
    }

    let mut grouped: HashMap<String, HashMap<String, Vec<ContentEntry>>> = HashMap::new();
    for r in &relations {
        if let Some(target) = targets_by_id.get(&r.target_entry_id) {
            grouped
                .entry(r.source_entry_id.clone())
                .or_default()
                .entry(r.field_api_id.clone())
                .or_default()
                .push(target.clone());
        }
    }

    for entry in &mut entries {
        if let Some(field_map) = grouped.remove(&entry.id) {
            entry.populated = Some(field_map);
        }
    }

    Ok(entries)
}

#[derive(Debug, Clone)]
#[allow(clippy::struct_field_names)]
struct RelationRow {
    source_entry_id: String,
    target_entry_id: String,
    field_api_id: String,
}

async fn fetch_relations(
    db: &Arc<DatabasePool>,
    source_ids: &[String],
    field_api_ids: &[String],
) -> std::result::Result<Vec<RelationRow>, ContentEngineError> {
    if source_ids.is_empty() || field_api_ids.is_empty() {
        return Ok(Vec::new());
    }
    let db_type = db.db_type();
    let sql = build_relations_sql(db_type, source_ids.len(), field_api_ids.len());

    match db.as_ref() {
        DatabasePool::Postgres(pool) => {
            let mut q = sqlx::query(&sql);
            for s in source_ids {
                q = q.bind(s.clone());
            }
            for f in field_api_ids {
                q = q.bind(f.clone());
            }
            let rows = q
                .fetch_all(pool)
                .await
                .map_err(ContentEngineError::Database)?;
            rows.iter()
                .map(|r| {
                    Ok(RelationRow {
                        source_entry_id: r
                            .try_get("source_entry_id")
                            .map_err(ContentEngineError::Database)?,
                        target_entry_id: r
                            .try_get("target_entry_id")
                            .map_err(ContentEngineError::Database)?,
                        field_api_id: r
                            .try_get("field_api_id")
                            .map_err(ContentEngineError::Database)?,
                    })
                })
                .collect()
        }
        DatabasePool::MySql(pool) => {
            let mut q = sqlx::query(&sql);
            for s in source_ids {
                q = q.bind(s.clone());
            }
            for f in field_api_ids {
                q = q.bind(f.clone());
            }
            let rows = q
                .fetch_all(pool)
                .await
                .map_err(ContentEngineError::Database)?;
            rows.iter()
                .map(|r| {
                    Ok(RelationRow {
                        source_entry_id: r
                            .try_get("source_entry_id")
                            .map_err(ContentEngineError::Database)?,
                        target_entry_id: r
                            .try_get("target_entry_id")
                            .map_err(ContentEngineError::Database)?,
                        field_api_id: r
                            .try_get("field_api_id")
                            .map_err(ContentEngineError::Database)?,
                    })
                })
                .collect()
        }
        DatabasePool::Sqlite(pool) => {
            let mut q = sqlx::query(&sql);
            for s in source_ids {
                q = q.bind(s.clone());
            }
            for f in field_api_ids {
                q = q.bind(f.clone());
            }
            let rows = q
                .fetch_all(pool)
                .await
                .map_err(ContentEngineError::Database)?;
            rows.iter()
                .map(|r| {
                    Ok(RelationRow {
                        source_entry_id: r
                            .try_get("source_entry_id")
                            .map_err(ContentEngineError::Database)?,
                        target_entry_id: r
                            .try_get("target_entry_id")
                            .map_err(ContentEngineError::Database)?,
                        field_api_id: r
                            .try_get("field_api_id")
                            .map_err(ContentEngineError::Database)?,
                    })
                })
                .collect()
        }
    }
}

fn build_relations_sql(db_type: DatabaseType, source_count: usize, field_count: usize) -> String {
    let select = match db_type {
        DatabaseType::Postgres => {
            "source_entry_id::TEXT AS source_entry_id, \
             target_entry_id::TEXT AS target_entry_id, \
             field_api_id"
        }
        DatabaseType::MySql | DatabaseType::Sqlite => {
            "source_entry_id, target_entry_id, field_api_id"
        }
    };
    let mut sql = format!("SELECT {select} FROM content_relations WHERE source_entry_id IN (");
    let mut idx = 1usize;
    for i in 0..source_count {
        if i > 0 {
            sql.push_str(", ");
        }
        match db_type {
            DatabaseType::Postgres => {
                let _ = write!(sql, "${idx}::UUID");
            }
            DatabaseType::MySql | DatabaseType::Sqlite => sql.push('?'),
        }
        idx += 1;
    }
    sql.push_str(") AND field_api_id IN (");
    for i in 0..field_count {
        if i > 0 {
            sql.push_str(", ");
        }
        match db_type {
            DatabaseType::Postgres => {
                let _ = write!(sql, "${idx}");
            }
            DatabaseType::MySql | DatabaseType::Sqlite => sql.push('?'),
        }
        idx += 1;
    }
    sql.push_str(") ORDER BY source_entry_id, field_api_id, position");
    sql
}

#[cfg(test)]
mod tests {
    use super::{ContentEngineError, SINGLE_LEVEL_DEPTH, build_relations_sql, check_depth};
    use cycms_db::DatabaseType;

    #[test]
    fn check_depth_allows_zero_and_one() {
        assert!(check_depth(0).is_ok());
        assert!(check_depth(SINGLE_LEVEL_DEPTH).is_ok());
    }

    #[test]
    fn check_depth_rejects_above_single_level() {
        let err = check_depth(2).unwrap_err();
        assert!(matches!(
            err,
            ContentEngineError::PopulateDepthExceeded { depth: 2, max: 1 }
        ));
    }

    #[test]
    fn relations_sql_uses_uuid_cast_in_postgres() {
        let sql = build_relations_sql(DatabaseType::Postgres, 2, 1);
        assert!(sql.contains("$1::UUID"));
        assert!(sql.contains("$2::UUID"));
        assert!(sql.contains("$3"));
        assert!(!sql.contains("$3::UUID"));
    }

    #[test]
    fn relations_sql_uses_question_marks_in_sqlite() {
        let sql = build_relations_sql(DatabaseType::Sqlite, 2, 2);
        assert_eq!(sql.matches('?').count(), 4);
    }
}
