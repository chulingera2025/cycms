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
async fn seed_creates_blog_preset_with_expected_shape() {
    let reg = fresh_registry().await;
    let seeded = seed_default_types(&reg).await.unwrap();
    assert_eq!(seeded.len(), 5);

    let category = reg.get_type("category").await.unwrap().unwrap();
    assert_eq!(category.kind, ContentTypeKind::Collection);
    let category_field_ids: Vec<&str> = category.fields.iter().map(|f| f.api_id.as_str()).collect();
    assert_eq!(
        category_field_ids,
        vec!["name", "description", "cover_image", "seo_title", "seo_description"]
    );

    let tag = reg.get_type("tag").await.unwrap().unwrap();
    assert_eq!(tag.kind, ContentTypeKind::Collection);
    let tag_field_ids: Vec<&str> = tag.fields.iter().map(|f| f.api_id.as_str()).collect();
    assert_eq!(tag_field_ids, vec!["name", "description"]);

    let page = reg.get_type("page").await.unwrap().unwrap();
    assert_eq!(page.kind, ContentTypeKind::Collection);
    let page_field_ids: Vec<&str> = page.fields.iter().map(|f| f.api_id.as_str()).collect();
    assert_eq!(
        page_field_ids,
        vec!["title", "cover_image", "body", "seo_title", "seo_description"]
    );

    let post = reg.get_type("post").await.unwrap().unwrap();
    assert_eq!(post.kind, ContentTypeKind::Collection);
    let post_field_ids: Vec<&str> = post.fields.iter().map(|f| f.api_id.as_str()).collect();
    assert_eq!(
        post_field_ids,
        vec![
            "title",
            "excerpt",
            "cover_image",
            "body",
            "categories",
            "tags",
            "featured",
            "seo_title",
            "seo_description"
        ]
    );

    let site_settings = reg.get_type("site_settings").await.unwrap().unwrap();
    assert_eq!(site_settings.kind, ContentTypeKind::Single);
    let settings_field_ids: Vec<&str> = site_settings.fields.iter().map(|f| f.api_id.as_str()).collect();
    assert_eq!(
        settings_field_ids,
        vec![
            "site_name",
            "tagline",
            "logo",
            "hero_title",
            "hero_subtitle",
            "footer_text",
            "featured_posts"
        ]
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
    assert_eq!(second.len(), 5);
}
