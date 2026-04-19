use std::path::PathBuf;
use std::sync::Arc;

use cycms_config::{DatabaseConfig, DatabaseDriver};
use cycms_content_model::{
    ContentModelRegistry, ContentModelError, ContentTypeKind, CreateContentTypeInput,
    FieldDefinition, FieldType, FieldTypeHandler, FieldTypeRegistry, UpdateContentTypeInput,
    ValidationRule,
};
use cycms_core::Error;
use cycms_db::DatabasePool;
use cycms_migrate::MigrationEngine;
use serde_json::{Value, json};

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

fn article_input() -> CreateContentTypeInput {
    CreateContentTypeInput {
        name: "Article".into(),
        api_id: "article".into(),
        description: Some("Blog articles".into()),
        kind: ContentTypeKind::Collection,
        fields: vec![
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
                name: "Slug".into(),
                api_id: "slug".into(),
                field_type: FieldType::Text,
                required: true,
                unique: true,
                default_value: None,
                validations: vec![ValidationRule::Regex {
                    pattern: "^[a-z0-9-]+$".into(),
                }],
                position: 1,
            },
            FieldDefinition {
                name: "Views".into(),
                api_id: "views".into(),
                field_type: FieldType::Number { decimal: false },
                required: false,
                unique: false,
                default_value: Some(json!(0)),
                validations: vec![ValidationRule::Min { value: 0.0 }],
                position: 2,
            },
        ],
    }
}

#[tokio::test]
async fn create_type_persists_and_listable() {
    let reg = fresh_registry().await;
    let created = reg.create_type(article_input()).await.unwrap();
    assert_eq!(created.api_id, "article");
    assert_eq!(created.fields.len(), 3);

    let list = reg.list_types().await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, created.id);

    let fetched = reg.get_type("article").await.unwrap().unwrap();
    assert_eq!(fetched.id, created.id);
}

#[tokio::test]
async fn create_type_normalizes_api_id_case_and_space() {
    let reg = fresh_registry().await;
    let mut input = article_input();
    input.api_id = "  Article  ".into();
    let created = reg.create_type(input).await.unwrap();
    assert_eq!(created.api_id, "article");
}

#[tokio::test]
async fn create_type_rejects_duplicate_api_id() {
    let reg = fresh_registry().await;
    reg.create_type(article_input()).await.unwrap();
    let err = reg.create_type(article_input()).await.unwrap_err();
    assert!(matches!(err, Error::Conflict { .. }), "got: {err:?}");
}

#[tokio::test]
async fn create_type_rejects_duplicate_field_api_id() {
    let reg = fresh_registry().await;
    let mut input = article_input();
    input.fields[1].api_id = "title".into();
    let err = reg.create_type(input).await.unwrap_err();
    assert!(matches!(err, Error::ValidationError { .. }), "got: {err:?}");
}

#[tokio::test]
async fn update_type_replaces_fields_and_name() {
    let reg = fresh_registry().await;
    reg.create_type(article_input()).await.unwrap();

    let updated = reg
        .update_type(
            "article",
            UpdateContentTypeInput {
                name: Some("Post".into()),
                description: Some(None),
                kind: Some(ContentTypeKind::Collection),
                fields: Some(vec![FieldDefinition {
                    name: "Title".into(),
                    api_id: "title".into(),
                    field_type: FieldType::Text,
                    required: true,
                    unique: false,
                    default_value: None,
                    validations: vec![],
                    position: 0,
                }]),
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.name, "Post");
    assert!(updated.description.is_none());
    assert_eq!(updated.fields.len(), 1);
}

#[tokio::test]
async fn update_type_unknown_api_id_returns_not_found() {
    let reg = fresh_registry().await;
    let err = reg
        .update_type(
            "ghost",
            UpdateContentTypeInput {
                name: Some("X".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, Error::NotFound { .. }), "got: {err:?}");
}

#[tokio::test]
async fn delete_type_removes_row() {
    let reg = fresh_registry().await;
    reg.create_type(article_input()).await.unwrap();
    assert!(reg.delete_type("article").await.unwrap());
    assert!(!reg.delete_type("article").await.unwrap());
    assert!(reg.get_type("article").await.unwrap().is_none());
}

#[tokio::test]
async fn validate_entry_accepts_conforming_payload() {
    let reg = fresh_registry().await;
    reg.create_type(article_input()).await.unwrap();

    reg.validate_entry(
        "article",
        &json!({
            "title": "Hello",
            "slug": "hello-world",
            "views": 10
        }),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn validate_entry_reports_schema_violations() {
    let reg = fresh_registry().await;
    reg.create_type(article_input()).await.unwrap();

    let err = reg
        .validate_entry(
            "article",
            &json!({
                "slug": "BAD SLUG",
                "views": -1
            }),
        )
        .await
        .unwrap_err();

    match err {
        Error::ValidationError { details, .. } => {
            let arr = details.as_ref().and_then(Value::as_array).unwrap();
            let rules: Vec<&str> = arr
                .iter()
                .filter_map(|v| v.get("rule").and_then(Value::as_str))
                .collect();
            assert!(rules.contains(&"required"), "missing required: {rules:?}");
            assert!(rules.contains(&"regex"), "missing regex: {rules:?}");
            assert!(rules.contains(&"min"), "missing min: {rules:?}");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn to_json_schema_returns_expected_keys() {
    let reg = fresh_registry().await;
    reg.create_type(article_input()).await.unwrap();

    let schema = reg.to_json_schema("article").await.unwrap();
    assert_eq!(schema.get("type"), Some(&json!("object")));
    let props = schema.get("properties").unwrap();
    assert!(props.get("title").is_some());
    assert!(props.get("slug").is_some());
    let required = schema.get("required").and_then(Value::as_array).unwrap();
    let required_names: Vec<&str> = required.iter().filter_map(Value::as_str).collect();
    assert!(required_names.contains(&"title"));
    assert!(required_names.contains(&"slug"));
}

#[tokio::test]
async fn custom_field_type_requires_registration() {
    struct LatLng;
    impl FieldTypeHandler for LatLng {
        fn validate(
            &self,
            value: &Value,
            _rules: &[ValidationRule],
        ) -> std::result::Result<(), ContentModelError> {
            if value.get("lat").is_some() && value.get("lng").is_some() {
                Ok(())
            } else {
                Err(ContentModelError::InvalidField(
                    "lat/lng required".to_owned(),
                ))
            }
        }
        fn to_openapi_schema(&self) -> Value {
            json!({ "type": "object" })
        }
    }

    let reg = fresh_registry().await;

    let input = CreateContentTypeInput {
        name: "Geo".into(),
        api_id: "geo".into(),
        description: None,
        kind: ContentTypeKind::Collection,
        fields: vec![FieldDefinition {
            name: "Point".into(),
            api_id: "point".into(),
            field_type: FieldType::Custom {
                type_name: "plugin.latlng".into(),
            },
            required: true,
            unique: false,
            default_value: None,
            validations: vec![],
            position: 0,
        }],
    };

    let err = reg.create_type(input.clone()).await.unwrap_err();
    assert!(matches!(err, Error::ValidationError { .. }), "got: {err:?}");

    reg.register_field_type("plugin.latlng", Arc::new(LatLng));
    reg.create_type(input).await.unwrap();

    reg.validate_entry("geo", &json!({ "point": { "lat": 1, "lng": 2 } }))
        .await
        .unwrap();

    let err = reg
        .validate_entry("geo", &json!({ "point": { "lat": 1 } }))
        .await
        .unwrap_err();
    assert!(matches!(err, Error::ValidationError { .. }), "got: {err:?}");
}

#[tokio::test]
async fn unregister_field_types_by_prefix_scoped() {
    struct Dummy;
    impl FieldTypeHandler for Dummy {
        fn validate(
            &self,
            _value: &Value,
            _rules: &[ValidationRule],
        ) -> std::result::Result<(), ContentModelError> {
            Ok(())
        }
        fn to_openapi_schema(&self) -> Value {
            json!({})
        }
    }

    let reg = fresh_registry().await;

    reg.register_field_type("blog.md", Arc::new(Dummy));
    reg.register_field_type("blog.code", Arc::new(Dummy));
    reg.register_field_type("auth.totp", Arc::new(Dummy));

    assert_eq!(reg.unregister_field_types_by_prefix("blog"), 2);
    assert!(reg.field_types().contains("auth.totp"));
    assert!(!reg.field_types().contains("blog.md"));
}
