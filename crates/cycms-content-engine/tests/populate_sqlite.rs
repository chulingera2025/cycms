use std::path::PathBuf;
use std::sync::Arc;

use cycms_config::{DatabaseConfig, DatabaseDriver};
use cycms_content_engine::{
    ContentEntryRepository, ContentStatus, NewContentEntryRow, new_content_entry_id,
    populate_entries,
};
use cycms_db::DatabasePool;
use cycms_migrate::MigrationEngine;
use serde_json::json;
use sqlx::SqlitePool;

const TYPE_ARTICLE: &str = "00000000-0000-0000-0000-0000000000aa";
const TYPE_TAG: &str = "00000000-0000-0000-0000-0000000000bb";
const USER_AUTHOR: &str = "00000000-0000-0000-0000-000000000001";

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
        .expect("sqlite pool"),
    );
    MigrationEngine::new(Arc::clone(&pool))
        .run_system_migrations(&system_migrations_root())
        .await
        .expect("migrations");
    pool
}

async fn seed_user(pool: &SqlitePool, id: &str) {
    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash) VALUES (?, ?, ?, 'hashed')",
    )
    .bind(id)
    .bind(id)
    .bind(format!("{id}@example.com"))
    .execute(pool)
    .await
    .expect("seed user");
}

async fn seed_type(pool: &SqlitePool, id: &str, api_id: &str, kind: &str) {
    sqlx::query("INSERT INTO content_types (id, name, api_id, kind) VALUES (?, ?, ?, ?)")
        .bind(id)
        .bind(api_id)
        .bind(api_id)
        .bind(kind)
        .execute(pool)
        .await
        .expect("seed type");
}

async fn seed_relation(
    pool: &SqlitePool,
    source_id: &str,
    target_id: &str,
    field_api_id: &str,
    relation_kind: &str,
    position: i32,
) {
    sqlx::query(
        "INSERT INTO content_relations (id, source_entry_id, target_entry_id, field_api_id, relation_kind, position) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(new_content_entry_id())
    .bind(source_id)
    .bind(target_id)
    .bind(field_api_id)
    .bind(relation_kind)
    .bind(position)
    .execute(pool)
    .await
    .expect("seed relation");
}

async fn prepare() -> (Arc<DatabasePool>, ContentEntryRepository, SqlitePool) {
    let pool = fresh_pool().await;
    let DatabasePool::Sqlite(inner) = pool.as_ref() else {
        panic!("expected sqlite");
    };
    let inner = inner.clone();
    seed_user(&inner, USER_AUTHOR).await;
    seed_type(&inner, TYPE_ARTICLE, "article", "collection").await;
    seed_type(&inner, TYPE_TAG, "tag", "collection").await;
    let repo = ContentEntryRepository::new(Arc::clone(&pool));
    (pool, repo, inner)
}

async fn insert_entry(
    repo: &ContentEntryRepository,
    type_id: &str,
    fields: serde_json::Value,
) -> String {
    let id = new_content_entry_id();
    repo.insert(NewContentEntryRow {
        id: id.clone(),
        content_type_id: type_id.to_owned(),
        slug: None,
        status: ContentStatus::Draft,
        fields,
        created_by: USER_AUTHOR.to_owned(),
    })
    .await
    .expect("insert entry");
    id
}

#[tokio::test]
async fn empty_populate_is_noop() {
    let (pool, repo, _) = prepare().await;
    let id = insert_entry(&repo, TYPE_ARTICLE, json!({})).await;
    let entries = vec![repo.find_by_id(&id).await.unwrap().unwrap()];

    let result = populate_entries(&pool, &repo, entries, &[]).await.unwrap();
    assert_eq!(result.len(), 1);
    assert!(result[0].populated.is_none());
}

#[tokio::test]
async fn populate_with_no_relations_returns_unchanged() {
    let (pool, repo, _) = prepare().await;
    let id = insert_entry(&repo, TYPE_ARTICLE, json!({})).await;
    let entries = vec![repo.find_by_id(&id).await.unwrap().unwrap()];

    let result = populate_entries(&pool, &repo, entries, &["tags".to_owned()])
        .await
        .unwrap();
    assert!(result[0].populated.is_none());
}

#[tokio::test]
async fn populate_loads_one_to_many_in_order() {
    let (pool, repo, inner) = prepare().await;
    let article_id = insert_entry(&repo, TYPE_ARTICLE, json!({})).await;
    let tag1 = insert_entry(&repo, TYPE_TAG, json!({"name": "rust"})).await;
    let tag2 = insert_entry(&repo, TYPE_TAG, json!({"name": "cms"})).await;

    seed_relation(&inner, &article_id, &tag2, "tags", "one_to_many", 1).await;
    seed_relation(&inner, &article_id, &tag1, "tags", "one_to_many", 0).await;

    let entries = vec![repo.find_by_id(&article_id).await.unwrap().unwrap()];
    let result = populate_entries(&pool, &repo, entries, &["tags".to_owned()])
        .await
        .unwrap();

    let populated = result[0].populated.as_ref().expect("populated populated");
    let tag_list = populated.get("tags").expect("tags field present");
    assert_eq!(tag_list.len(), 2);
    assert_eq!(tag_list[0].fields["name"], "rust");
    assert_eq!(tag_list[1].fields["name"], "cms");
}

#[tokio::test]
async fn populate_skips_missing_targets() {
    let (pool, repo, inner) = prepare().await;
    let article_id = insert_entry(&repo, TYPE_ARTICLE, json!({})).await;
    let tag_id = insert_entry(&repo, TYPE_TAG, json!({"name": "rust"})).await;
    let phantom_id = new_content_entry_id();

    seed_relation(&inner, &article_id, &tag_id, "tags", "one_to_many", 0).await;
    // 直接造一条 dangling 关联 — 现实中不允许，这里跳过 FK 校验（sqlite 默认开启 FK，因此插入会失败）。
    // 改为移除 tag 后的 dangling 验证：先建 relation，然后硬删 target。
    let extra_tag = insert_entry(&repo, TYPE_TAG, json!({"name": "doomed"})).await;
    seed_relation(&inner, &article_id, &extra_tag, "tags", "one_to_many", 1).await;
    repo.delete_hard(&extra_tag).await.unwrap();
    let _ = phantom_id; // 保留以避免 unused 警告

    let entries = vec![repo.find_by_id(&article_id).await.unwrap().unwrap()];
    let result = populate_entries(&pool, &repo, entries, &["tags".to_owned()])
        .await
        .unwrap();
    let tags = result[0]
        .populated
        .as_ref()
        .and_then(|m| m.get("tags"))
        .cloned()
        .unwrap_or_default();
    assert_eq!(tags.len(), 1, "dangling relation must be skipped");
    assert_eq!(tags[0].fields["name"], "rust");
}

#[tokio::test]
async fn populate_groups_multiple_fields_per_entry() {
    let (pool, repo, inner) = prepare().await;
    let article_id = insert_entry(&repo, TYPE_ARTICLE, json!({})).await;
    let tag_id = insert_entry(&repo, TYPE_TAG, json!({"name": "rust"})).await;
    let related_id = insert_entry(&repo, TYPE_TAG, json!({"name": "wasm"})).await;

    seed_relation(&inner, &article_id, &tag_id, "tags", "one_to_many", 0).await;
    seed_relation(
        &inner,
        &article_id,
        &related_id,
        "related",
        "many_to_many",
        0,
    )
    .await;

    let entries = vec![repo.find_by_id(&article_id).await.unwrap().unwrap()];
    let result = populate_entries(
        &pool,
        &repo,
        entries,
        &["tags".to_owned(), "related".to_owned()],
    )
    .await
    .unwrap();
    let populated = result[0].populated.as_ref().unwrap();
    assert_eq!(populated.get("tags").unwrap().len(), 1);
    assert_eq!(populated.get("related").unwrap().len(), 1);
}
