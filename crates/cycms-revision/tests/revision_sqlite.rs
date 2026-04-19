use std::path::PathBuf;
use std::sync::Arc;

use cycms_config::{DatabaseConfig, DatabaseDriver};
use cycms_db::DatabasePool;
use cycms_migrate::MigrationEngine;
use cycms_revision::{CreateRevisionInput, RevisionManager};
use serde_json::json;
use sqlx::SqlitePool;

const USER_ID: &str = "00000000-0000-0000-0000-000000000001";
const ENTRY_ID: &str = "00000000-0000-0000-0000-00000000aa01";
const TYPE_ID: &str = "00000000-0000-0000-0000-000000000002";

fn system_migrations_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../cycms-migrate/migrations/system")
}

async fn fresh_pool() -> Arc<DatabasePool> {
    let pool = Arc::new(
        DatabasePool::connect(&DatabaseConfig {
            driver: DatabaseDriver::Sqlite,
            url: "sqlite::memory:".to_owned(),
            max_connections: 1,
            connect_timeout_secs: 5,
            idle_timeout_secs: 60,
        })
        .await
        .expect("pool connect"),
    );
    MigrationEngine::new(Arc::clone(&pool))
        .run_system_migrations(&system_migrations_root())
        .await
        .expect("run migrations");
    pool
}

/// 在 `content_entries` 表插入一条测试 entry，满足 FK 约束。
async fn seed_entry(pool: &SqlitePool) {
    sqlx::query("INSERT INTO users (id, username, email, password_hash) VALUES (?, ?, ?, 'h')")
        .bind(USER_ID)
        .bind("tester")
        .bind("t@t.com")
        .execute(pool)
        .await
        .expect("seed user");

    sqlx::query("INSERT INTO content_types (id, name, api_id, kind) VALUES (?, 'Post', 'post', 'Collection')")
        .bind(TYPE_ID)
        .execute(pool)
        .await
        .expect("seed content_type");

    sqlx::query(
        "INSERT INTO content_entries \
        (id, content_type_id, status, fields, created_by, updated_by) \
        VALUES (?, ?, 'draft', '{}', ?, ?)",
    )
    .bind(ENTRY_ID)
    .bind(TYPE_ID)
    .bind(USER_ID)
    .bind(USER_ID)
    .execute(pool)
    .await
    .expect("seed entry");
}

fn make_input(fields: serde_json::Value, summary: Option<&str>) -> CreateRevisionInput {
    CreateRevisionInput {
        content_entry_id: ENTRY_ID.to_owned(),
        snapshot: fields,
        actor_id: USER_ID.to_owned(),
        change_summary: summary.map(str::to_owned),
    }
}

// ── create_revision ──────────────────────────────────────────────────────────

#[tokio::test]
async fn create_revision_starts_at_version_1() {
    let pool = fresh_pool().await;
    let DatabasePool::Sqlite(sqlite) = pool.as_ref() else {
        panic!()
    };
    seed_entry(sqlite).await;

    let mgr = RevisionManager::new(Arc::clone(&pool));
    let rev = mgr
        .create_revision(make_input(json!({"title": "v1"}), None))
        .await
        .unwrap();

    assert_eq!(rev.version_number, 1);
    assert_eq!(rev.content_entry_id, ENTRY_ID);
    assert_eq!(rev.snapshot, json!({"title": "v1"}));
    assert!(rev.change_summary.is_none());
}

#[tokio::test]
async fn create_revision_increments_version_number() {
    let pool = fresh_pool().await;
    let DatabasePool::Sqlite(sqlite) = pool.as_ref() else {
        panic!()
    };
    seed_entry(sqlite).await;

    let mgr = RevisionManager::new(Arc::clone(&pool));
    let r1 = mgr
        .create_revision(make_input(json!({"title": "v1"}), None))
        .await
        .unwrap();
    let r2 = mgr
        .create_revision(make_input(json!({"title": "v2"}), Some("second")))
        .await
        .unwrap();
    let r3 = mgr
        .create_revision(make_input(json!({"title": "v3"}), None))
        .await
        .unwrap();

    assert_eq!(r1.version_number, 1);
    assert_eq!(r2.version_number, 2);
    assert_eq!(r3.version_number, 3);
    assert_eq!(r2.change_summary.as_deref(), Some("second"));
}

#[tokio::test]
async fn create_revision_updates_entry_current_version_id() {
    let pool = fresh_pool().await;
    let DatabasePool::Sqlite(sqlite) = pool.as_ref() else {
        panic!()
    };
    seed_entry(sqlite).await;

    let mgr = RevisionManager::new(Arc::clone(&pool));
    let rev = mgr
        .create_revision(make_input(json!({"x": 1}), None))
        .await
        .unwrap();

    let (current_version_id,): (Option<String>,) =
        sqlx::query_as("SELECT current_version_id FROM content_entries WHERE id = ?")
            .bind(ENTRY_ID)
            .fetch_one(sqlite)
            .await
            .unwrap();

    assert_eq!(current_version_id.as_deref(), Some(rev.id.as_str()));
}

// ── list_revisions ───────────────────────────────────────────────────────────

#[tokio::test]
async fn list_revisions_returns_newest_first() {
    let pool = fresh_pool().await;
    let DatabasePool::Sqlite(sqlite) = pool.as_ref() else {
        panic!()
    };
    seed_entry(sqlite).await;

    let mgr = RevisionManager::new(Arc::clone(&pool));
    for i in 1_u32..=4 {
        mgr.create_revision(make_input(json!({"i": i}), None))
            .await
            .unwrap();
    }

    let result = mgr.list_revisions(ENTRY_ID, 1, 10).await.unwrap();
    assert_eq!(result.total, 4);
    assert_eq!(result.data.len(), 4);
    assert_eq!(result.data[0].version_number, 4);
    assert_eq!(result.data[3].version_number, 1);
}

#[tokio::test]
async fn list_revisions_pagination() {
    let pool = fresh_pool().await;
    let DatabasePool::Sqlite(sqlite) = pool.as_ref() else {
        panic!()
    };
    seed_entry(sqlite).await;

    let mgr = RevisionManager::new(Arc::clone(&pool));
    for i in 1_u32..=5 {
        mgr.create_revision(make_input(json!({"i": i}), None))
            .await
            .unwrap();
    }

    let page1 = mgr.list_revisions(ENTRY_ID, 1, 2).await.unwrap();
    let page2 = mgr.list_revisions(ENTRY_ID, 2, 2).await.unwrap();

    assert_eq!(page1.total, 5);
    assert_eq!(page1.data.len(), 2);
    assert_eq!(page1.data[0].version_number, 5);

    assert_eq!(page2.data.len(), 2);
    assert_eq!(page2.data[0].version_number, 3);
}

// ── get_revision ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_revision_returns_correct_snapshot() {
    let pool = fresh_pool().await;
    let DatabasePool::Sqlite(sqlite) = pool.as_ref() else {
        panic!()
    };
    seed_entry(sqlite).await;

    let mgr = RevisionManager::new(Arc::clone(&pool));
    mgr.create_revision(make_input(json!({"title": "first"}), None))
        .await
        .unwrap();
    mgr.create_revision(make_input(json!({"title": "second"}), None))
        .await
        .unwrap();

    let rev = mgr.get_revision(ENTRY_ID, 1).await.unwrap();
    assert_eq!(rev.snapshot, json!({"title": "first"}));

    let rev2 = mgr.get_revision(ENTRY_ID, 2).await.unwrap();
    assert_eq!(rev2.snapshot, json!({"title": "second"}));
}

#[tokio::test]
async fn get_revision_nonexistent_returns_error() {
    let pool = fresh_pool().await;
    let DatabasePool::Sqlite(sqlite) = pool.as_ref() else {
        panic!()
    };
    seed_entry(sqlite).await;

    let mgr = RevisionManager::new(Arc::clone(&pool));
    mgr.create_revision(make_input(json!({}), None))
        .await
        .unwrap();

    let err = mgr.get_revision(ENTRY_ID, 99).await.unwrap_err();
    assert!(
        err.to_string().contains("not_found"),
        "expected not_found, got: {err}"
    );
}

// ── rollback ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn rollback_creates_new_version_with_target_snapshot() {
    let pool = fresh_pool().await;
    let DatabasePool::Sqlite(sqlite) = pool.as_ref() else {
        panic!()
    };
    seed_entry(sqlite).await;

    let mgr = RevisionManager::new(Arc::clone(&pool));
    mgr.create_revision(make_input(json!({"title": "v1"}), None))
        .await
        .unwrap();
    mgr.create_revision(make_input(json!({"title": "v2"}), None))
        .await
        .unwrap();
    mgr.create_revision(make_input(json!({"title": "v3"}), None))
        .await
        .unwrap();

    // 回滚到 v1
    let rollback_rev = mgr.rollback(ENTRY_ID, 1, USER_ID).await.unwrap();

    assert_eq!(rollback_rev.version_number, 4);
    assert_eq!(rollback_rev.snapshot, json!({"title": "v1"}));
    assert_eq!(
        rollback_rev.change_summary.as_deref(),
        Some("Rollback to v1")
    );
}

#[tokio::test]
async fn rollback_updates_entry_fields() {
    let pool = fresh_pool().await;
    let DatabasePool::Sqlite(sqlite) = pool.as_ref() else {
        panic!()
    };
    seed_entry(sqlite).await;

    let mgr = RevisionManager::new(Arc::clone(&pool));
    mgr.create_revision(make_input(json!({"title": "original"}), None))
        .await
        .unwrap();
    mgr.create_revision(make_input(json!({"title": "modified"}), None))
        .await
        .unwrap();

    mgr.rollback(ENTRY_ID, 1, USER_ID).await.unwrap();

    let (fields_raw,): (String,) =
        sqlx::query_as("SELECT fields FROM content_entries WHERE id = ?")
            .bind(ENTRY_ID)
            .fetch_one(sqlite)
            .await
            .unwrap();
    let fields: serde_json::Value = serde_json::from_str(&fields_raw).unwrap();
    assert_eq!(fields, json!({"title": "original"}));
}

#[tokio::test]
async fn rollback_preserves_intermediate_versions() {
    let pool = fresh_pool().await;
    let DatabasePool::Sqlite(sqlite) = pool.as_ref() else {
        panic!()
    };
    seed_entry(sqlite).await;

    let mgr = RevisionManager::new(Arc::clone(&pool));
    for i in 1_u32..=3 {
        mgr.create_revision(make_input(json!({"i": i}), None))
            .await
            .unwrap();
    }
    mgr.rollback(ENTRY_ID, 1, USER_ID).await.unwrap();

    // 版本总数应为 4（v1 v2 v3 + rollback 新版本）
    let result = mgr.list_revisions(ENTRY_ID, 1, 20).await.unwrap();
    assert_eq!(result.total, 4);
}

#[tokio::test]
async fn rollback_nonexistent_version_returns_error() {
    let pool = fresh_pool().await;
    let DatabasePool::Sqlite(sqlite) = pool.as_ref() else {
        panic!()
    };
    seed_entry(sqlite).await;

    let mgr = RevisionManager::new(Arc::clone(&pool));
    mgr.create_revision(make_input(json!({}), None))
        .await
        .unwrap();

    let err = mgr.rollback(ENTRY_ID, 99, USER_ID).await.unwrap_err();
    assert!(
        err.to_string().contains("not_found"),
        "expected not_found, got: {err}"
    );
}
