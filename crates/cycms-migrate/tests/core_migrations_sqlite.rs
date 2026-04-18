use std::path::PathBuf;
use std::sync::Arc;

use cycms_config::{DatabaseConfig, DatabaseDriver};
use cycms_db::DatabasePool;
use cycms_migrate::MigrationEngine;

fn system_migrations_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations/system")
}

#[tokio::test]
async fn core_migrations_apply_on_sqlite_and_create_all_tables() {
    let pool = Arc::new(
        DatabasePool::connect(&DatabaseConfig {
            driver: DatabaseDriver::Sqlite,
            url: "sqlite::memory:".to_owned(),
            max_connections: 1,
            connect_timeout_secs: 5,
            idle_timeout_secs: 60,
        })
        .await
        .unwrap(),
    );
    let engine = MigrationEngine::new(Arc::clone(&pool));

    let applied = engine
        .run_system_migrations(&system_migrations_root())
        .await
        .unwrap();
    assert_eq!(applied.len(), 5, "five initial migrations should apply");

    let DatabasePool::Sqlite(inner) = pool.as_ref() else {
        panic!("expected sqlite pool");
    };

    for expected in [
        "users",
        "roles",
        "user_roles",
        "permissions",
        "role_permissions",
        "revoked_tokens",
        "content_types",
        "content_entries",
        "content_revisions",
        "content_relations",
        "media_assets",
        "settings",
        "plugin_settings_schemas",
        "plugin_kv",
        "plugins",
        "audit_logs",
    ] {
        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?")
                .bind(expected)
                .fetch_one(inner)
                .await
                .unwrap();
        assert_eq!(count, 1, "expected table `{expected}` to exist");
    }

    // 回滚应倒序删除所有表。
    let rolled = engine
        .rollback("system", &system_migrations_root(), 5)
        .await
        .unwrap();
    assert_eq!(rolled.len(), 5);

    for expected in ["users", "content_entries", "plugins", "audit_logs"] {
        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?")
                .bind(expected)
                .fetch_one(inner)
                .await
                .unwrap();
        assert_eq!(count, 0, "table `{expected}` should be dropped after rollback");
    }
}
