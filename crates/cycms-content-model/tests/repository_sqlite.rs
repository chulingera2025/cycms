use std::path::PathBuf;
use std::sync::Arc;

use cycms_config::{DatabaseConfig, DatabaseDriver};
use cycms_content_model::{
    ContentTypeKind, ContentTypeRepository, FieldDefinition, FieldType, NewContentTypeRow,
    UpdateContentTypeRow, ValidationRule, new_content_type_id,
};
use cycms_core::Error;
use cycms_db::DatabasePool;
use cycms_migrate::MigrationEngine;

fn system_migrations_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../cycms-migrate/migrations/system")
}

async fn fresh_sqlite_repo() -> ContentTypeRepository {
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
    ContentTypeRepository::new(pool)
}

fn sample_fields() -> Vec<FieldDefinition> {
    vec![
        FieldDefinition {
            name: "Title".into(),
            api_id: "title".into(),
            field_type: FieldType::Text,
            required: true,
            unique: false,
            default_value: None,
            validations: vec![ValidationRule::MaxLength { value: 255 }],
            position: 0,
        },
        FieldDefinition {
            name: "Body".into(),
            api_id: "body".into(),
            field_type: FieldType::RichText,
            required: false,
            unique: false,
            default_value: None,
            validations: vec![],
            position: 1,
        },
    ]
}

fn sample_row(api_id: &str) -> NewContentTypeRow {
    NewContentTypeRow {
        id: new_content_type_id(),
        name: "Article".into(),
        api_id: api_id.to_owned(),
        description: Some("Blog articles".into()),
        kind: ContentTypeKind::Collection,
        fields: sample_fields(),
    }
}

#[tokio::test]
async fn insert_then_find_by_id_and_api_id() {
    let repo = fresh_sqlite_repo().await;
    let inserted = repo.insert(sample_row("article")).await.unwrap();
    assert_eq!(inserted.api_id, "article");
    assert_eq!(inserted.kind, ContentTypeKind::Collection);
    assert_eq!(inserted.fields.len(), 2);
    assert_eq!(inserted.fields[0].api_id, "title");

    let by_id = repo.find_by_id(&inserted.id).await.unwrap().unwrap();
    assert_eq!(by_id.id, inserted.id);

    let by_api = repo.find_by_api_id("article").await.unwrap().unwrap();
    assert_eq!(by_api.id, inserted.id);
}

#[tokio::test]
async fn insert_duplicate_api_id_conflicts() {
    let repo = fresh_sqlite_repo().await;
    repo.insert(sample_row("article")).await.unwrap();
    let err = repo.insert(sample_row("article")).await.unwrap_err();
    assert!(matches!(err, Error::Conflict { .. }), "got: {err:?}");
}

#[tokio::test]
async fn update_changes_fields_and_bumps_updated_at() {
    let repo = fresh_sqlite_repo().await;
    let inserted = repo.insert(sample_row("article")).await.unwrap();

    let updated = repo
        .update(
            &inserted.id,
            UpdateContentTypeRow {
                name: "Post".into(),
                description: None,
                kind: ContentTypeKind::Single,
                fields: vec![FieldDefinition {
                    name: "Title".into(),
                    api_id: "title".into(),
                    field_type: FieldType::Text,
                    required: true,
                    unique: true,
                    default_value: None,
                    validations: vec![],
                    position: 0,
                }],
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.name, "Post");
    assert!(updated.description.is_none());
    assert_eq!(updated.kind, ContentTypeKind::Single);
    assert_eq!(updated.fields.len(), 1);
    assert!(updated.fields[0].unique);
    assert!(updated.updated_at >= inserted.updated_at);
}

#[tokio::test]
async fn update_unknown_id_returns_not_found() {
    let repo = fresh_sqlite_repo().await;
    let err = repo
        .update(
            &new_content_type_id(),
            UpdateContentTypeRow {
                name: "X".into(),
                description: None,
                kind: ContentTypeKind::Collection,
                fields: vec![],
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, Error::NotFound { .. }), "got: {err:?}");
}

#[tokio::test]
async fn delete_and_list_roundtrip() {
    let repo = fresh_sqlite_repo().await;
    let a = repo.insert(sample_row("article")).await.unwrap();
    let b = repo
        .insert(NewContentTypeRow {
            api_id: "page".into(),
            kind: ContentTypeKind::Single,
            ..sample_row("page")
        })
        .await
        .unwrap();

    let list = repo.list().await.unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].api_id, "article");
    assert_eq!(list[1].api_id, "page");

    assert!(repo.delete_by_id(&a.id).await.unwrap());
    assert!(!repo.delete_by_id(&a.id).await.unwrap());

    let list = repo.list().await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, b.id);
}
