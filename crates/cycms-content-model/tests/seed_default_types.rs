use std::path::PathBuf;
use std::sync::Arc;

use cycms_config::{DatabaseConfig, DatabaseDriver};
use cycms_content_model::{
    ContentModelRegistry, ContentTypeKind, FieldTypeRegistry, seed_default_types,
};
use cycms_db::DatabasePool;
use cycms_migrate::MigrationEngine;

fn system_migrations_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../cycms-migrate/migrations/system")
}

async fn fresh_registry() -> ContentModelRegistry {
    let pool = Arc::new(
        DatabasePool::connect(&DatabaseConfig {
            driver: DatabaseDriver::Sqlite,
            url: "sqlite::memory:".to_owned(),
            max_connections: 1,
            connect_timeout_secs: 5,
            idle_timeout_secs: 60,
        })
        .await
        .expect("sqlite pool connect"),
    );
    MigrationEngine::new(Arc::clone(&pool))
        .run_system_migrations(&system_migrations_root())
        .await
        .expect("run system migrations");
    ContentModelRegistry::new(pool, Arc::new(FieldTypeRegistry::new()))
}

#[tokio::test]
async fn seed_creates_page_and_post_with_expected_shape() {
    let reg = fresh_registry().await;
    let seeded = seed_default_types(&reg).await.unwrap();
    assert_eq!(seeded.len(), 2);

    let page = reg.get_type("page").await.unwrap().unwrap();
    assert_eq!(page.kind, ContentTypeKind::Single);
    let page_field_ids: Vec<&str> = page.fields.iter().map(|f| f.api_id.as_str()).collect();
    assert_eq!(
        page_field_ids,
        vec!["title", "slug", "body", "published_at"]
    );

    let post = reg.get_type("post").await.unwrap().unwrap();
    assert_eq!(post.kind, ContentTypeKind::Collection);
    let post_field_ids: Vec<&str> = post.fields.iter().map(|f| f.api_id.as_str()).collect();
    assert_eq!(
        post_field_ids,
        vec!["title", "slug", "summary", "body", "published_at"]
    );
}

#[tokio::test]
async fn seed_is_idempotent() {
    let reg = fresh_registry().await;
    seed_default_types(&reg).await.unwrap();
    let first = reg.list_types().await.unwrap();
    let first_ids: Vec<String> = first.iter().map(|t| t.id.clone()).collect();

    // second call must not recreate or error
    seed_default_types(&reg).await.unwrap();
    let second = reg.list_types().await.unwrap();
    let second_ids: Vec<String> = second.iter().map(|t| t.id.clone()).collect();
    assert_eq!(first_ids, second_ids);
    assert_eq!(second.len(), 2);
}
