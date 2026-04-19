use std::path::PathBuf;
use std::sync::Arc;

use cycms_config::{DatabaseConfig, DatabaseDriver};
use cycms_content_engine::{
    ContentEntryRepository, ContentStatus, NewContentEntryRow, UpdateContentEntryRow,
    new_content_entry_id,
};
use cycms_core::Error;
use cycms_db::DatabasePool;
use cycms_migrate::MigrationEngine;
use serde_json::json;
use sqlx::SqlitePool;

const TYPE_ARTICLE: &str = "00000000-0000-0000-0000-0000000000aa";
const TYPE_PAGE: &str = "00000000-0000-0000-0000-0000000000bb";
const USER_AUTHOR: &str = "00000000-0000-0000-0000-000000000001";

fn system_migrations_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../cycms-migrate/migrations/system")
}

async fn fresh_sqlite_pool() -> Arc<DatabasePool> {
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

async fn seed_content_type(pool: &SqlitePool, id: &str, api_id: &str, kind: &str) {
    sqlx::query("INSERT INTO content_types (id, name, api_id, kind) VALUES (?, ?, ?, ?)")
        .bind(id)
        .bind(api_id)
        .bind(api_id)
        .bind(kind)
        .execute(pool)
        .await
        .expect("seed content type");
}

async fn prepare_fresh_repo() -> (Arc<DatabasePool>, ContentEntryRepository) {
    let pool = fresh_sqlite_pool().await;
    let DatabasePool::Sqlite(inner) = pool.as_ref() else {
        panic!("expected sqlite pool");
    };
    seed_user(inner, USER_AUTHOR).await;
    seed_content_type(inner, TYPE_ARTICLE, "article", "collection").await;
    seed_content_type(inner, TYPE_PAGE, "page", "single").await;
    let repo = ContentEntryRepository::new(Arc::clone(&pool));
    (pool, repo)
}

#[tokio::test]
async fn insert_then_find_by_id() {
    let (_pool, repo) = prepare_fresh_repo().await;
    let id = new_content_entry_id();
    let inserted = repo
        .insert(NewContentEntryRow {
            id: id.clone(),
            content_type_id: TYPE_ARTICLE.to_owned(),
            slug: Some("hello".to_owned()),
            status: ContentStatus::Draft,
            fields: json!({ "title": "Hello" }),
            created_by: USER_AUTHOR.to_owned(),
        })
        .await
        .unwrap();

    assert_eq!(inserted.id, id);
    assert_eq!(inserted.status, ContentStatus::Draft);
    assert_eq!(inserted.fields["title"], "Hello");
    assert_eq!(inserted.created_by, USER_AUTHOR);
    assert_eq!(inserted.updated_by, USER_AUTHOR);
    assert!(inserted.content_type_api_id.is_empty());
    assert!(inserted.current_version_id.is_none());
    assert!(inserted.published_version_id.is_none());
    assert!(inserted.published_at.is_none());

    let by_id = repo.find_by_id(&id).await.unwrap().unwrap();
    assert_eq!(by_id.id, id);
    assert_eq!(by_id.slug.as_deref(), Some("hello"));
}

#[tokio::test]
async fn update_changes_fields_and_bumps_updated_at() {
    let (_pool, repo) = prepare_fresh_repo().await;
    let id = new_content_entry_id();
    let inserted = repo
        .insert(NewContentEntryRow {
            id: id.clone(),
            content_type_id: TYPE_ARTICLE.to_owned(),
            slug: None,
            status: ContentStatus::Draft,
            fields: json!({ "title": "Hello" }),
            created_by: USER_AUTHOR.to_owned(),
        })
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    let updated = repo
        .update(
            &id,
            UpdateContentEntryRow {
                slug: Some("hello-2".to_owned()),
                status: ContentStatus::Draft,
                fields: json!({ "title": "Hello v2" }),
                updated_by: USER_AUTHOR.to_owned(),
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.slug.as_deref(), Some("hello-2"));
    assert_eq!(updated.fields["title"], "Hello v2");
    assert!(updated.updated_at >= inserted.updated_at);
}

#[tokio::test]
async fn update_missing_returns_not_found() {
    let (_pool, repo) = prepare_fresh_repo().await;
    let err = repo
        .update(
            &new_content_entry_id(),
            UpdateContentEntryRow {
                slug: None,
                status: ContentStatus::Draft,
                fields: json!({}),
                updated_by: USER_AUTHOR.to_owned(),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, Error::NotFound { .. }), "got: {err:?}");
}

#[tokio::test]
async fn mark_archived_flips_status() {
    let (_pool, repo) = prepare_fresh_repo().await;
    let id = new_content_entry_id();
    repo.insert(NewContentEntryRow {
        id: id.clone(),
        content_type_id: TYPE_ARTICLE.to_owned(),
        slug: None,
        status: ContentStatus::Draft,
        fields: json!({}),
        created_by: USER_AUTHOR.to_owned(),
    })
    .await
    .unwrap();

    let archived = repo.mark_archived(&id, USER_AUTHOR).await.unwrap();
    assert_eq!(archived.status, ContentStatus::Archived);

    let err = repo
        .mark_archived(&new_content_entry_id(), USER_AUTHOR)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::NotFound { .. }));
}

#[tokio::test]
async fn delete_hard_removes_row() {
    let (_pool, repo) = prepare_fresh_repo().await;
    let id = new_content_entry_id();
    repo.insert(NewContentEntryRow {
        id: id.clone(),
        content_type_id: TYPE_ARTICLE.to_owned(),
        slug: None,
        status: ContentStatus::Draft,
        fields: json!({}),
        created_by: USER_AUTHOR.to_owned(),
    })
    .await
    .unwrap();

    assert!(repo.delete_hard(&id).await.unwrap());
    assert!(!repo.delete_hard(&id).await.unwrap());
    assert!(repo.find_by_id(&id).await.unwrap().is_none());
}

#[tokio::test]
async fn count_by_type_tracks_inserts() {
    let (_pool, repo) = prepare_fresh_repo().await;
    assert_eq!(repo.count_by_type(TYPE_ARTICLE).await.unwrap(), 0);
    assert_eq!(repo.count_by_type(TYPE_PAGE).await.unwrap(), 0);

    for _ in 0..3 {
        repo.insert(NewContentEntryRow {
            id: new_content_entry_id(),
            content_type_id: TYPE_ARTICLE.to_owned(),
            slug: None,
            status: ContentStatus::Draft,
            fields: json!({}),
            created_by: USER_AUTHOR.to_owned(),
        })
        .await
        .unwrap();
    }

    assert_eq!(repo.count_by_type(TYPE_ARTICLE).await.unwrap(), 3);
    assert_eq!(repo.count_by_type(TYPE_PAGE).await.unwrap(), 0);
}

#[tokio::test]
async fn find_by_id_and_type_filters_mismatched_type() {
    let (_pool, repo) = prepare_fresh_repo().await;
    let id = new_content_entry_id();
    repo.insert(NewContentEntryRow {
        id: id.clone(),
        content_type_id: TYPE_ARTICLE.to_owned(),
        slug: None,
        status: ContentStatus::Draft,
        fields: json!({}),
        created_by: USER_AUTHOR.to_owned(),
    })
    .await
    .unwrap();

    assert!(
        repo.find_by_id_and_type(&id, TYPE_ARTICLE)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        repo.find_by_id_and_type(&id, TYPE_PAGE)
            .await
            .unwrap()
            .is_none()
    );
}
