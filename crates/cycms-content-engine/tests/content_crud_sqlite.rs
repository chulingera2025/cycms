use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use cycms_config::{ContentConfig, DatabaseConfig, DatabaseDriver, DeleteMode};
use cycms_content_engine::{
    ColumnField, ContentEngine, ContentQuery, ContentStatus, CreateEntryInput, FieldRef,
    FilterOperator, FilterSpec, UpdateEntryInput, new_content_entry_id,
};
use cycms_content_model::{
    ContentModelRegistry, ContentTypeKind, CreateContentTypeInput, FieldDefinition, FieldType,
    RelationKind, ValidationRule,
};
use cycms_core::{Error, Result};
use cycms_db::DatabasePool;
use cycms_events::{Event, EventBus, EventHandler, EventKind};
use cycms_migrate::MigrationEngine;
use cycms_revision::RevisionManager;
use serde_json::{Value, json};
use sqlx::SqlitePool;

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
        .expect("sqlite"),
    );
    MigrationEngine::new(Arc::clone(&pool))
        .run_system_migrations(&system_migrations_root())
        .await
        .expect("migrations");
    pool
}

async fn seed_user(pool: &SqlitePool, id: &str) {
    sqlx::query("INSERT INTO users (id, username, email, password_hash) VALUES (?, ?, ?, 'hash')")
        .bind(id)
        .bind(id)
        .bind(format!("{id}@example.com"))
        .execute(pool)
        .await
        .expect("seed user");
}

async fn seed_relation(
    pool: &SqlitePool,
    source: &str,
    target: &str,
    field_api_id: &str,
    relation_kind: &str,
    position: i32,
) {
    sqlx::query(
        "INSERT INTO content_relations \
         (id, source_entry_id, target_entry_id, field_api_id, relation_kind, position) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(new_content_entry_id())
    .bind(source)
    .bind(target)
    .bind(field_api_id)
    .bind(relation_kind)
    .bind(position)
    .execute(pool)
    .await
    .expect("seed relation");
}

struct EventSink {
    events: Arc<Mutex<Vec<Arc<Event>>>>,
}

#[async_trait]
impl EventHandler for EventSink {
    fn name(&self) -> &'static str {
        "test.event-sink"
    }

    async fn handle(&self, event: Arc<Event>) -> Result<()> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}

async fn wait_for(events: &Arc<Mutex<Vec<Arc<Event>>>>, target: usize) {
    for _ in 0..200 {
        if events.lock().unwrap().len() >= target {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("timed out waiting for {target} events");
}

struct Setup {
    engine: ContentEngine,
    bus: Arc<EventBus>,
    model: Arc<ContentModelRegistry>,
    pool: Arc<DatabasePool>,
}

impl Setup {
    fn sqlite(&self) -> SqlitePool {
        let DatabasePool::Sqlite(p) = self.pool.as_ref() else {
            panic!("sqlite expected");
        };
        p.clone()
    }
}

async fn setup() -> Setup {
    let pool = fresh_pool().await;
    let DatabasePool::Sqlite(inner) = pool.as_ref() else {
        panic!("sqlite expected");
    };
    seed_user(&inner.clone(), USER_AUTHOR).await;

    let model = Arc::new(ContentModelRegistry::with_fresh_registry(Arc::clone(&pool)));
    let bus = Arc::new(EventBus::new());
    let cfg = ContentConfig {
        default_delete_mode: DeleteMode::Soft,
        default_page_size: 20,
        max_page_size: 100,
    };
    let engine = ContentEngine::new(
        Arc::clone(&pool),
        Arc::clone(&model),
        Arc::clone(&bus),
        cfg,
        Arc::new(RevisionManager::new(Arc::clone(&pool))),
    );
    Setup {
        engine,
        bus,
        model,
        pool,
    }
}

async fn create_article_type(model: &ContentModelRegistry) {
    model
        .create_type(CreateContentTypeInput {
            name: "Article".into(),
            api_id: "article".into(),
            description: None,
            kind: ContentTypeKind::Collection,
            fields: vec![FieldDefinition {
                name: "Title".into(),
                api_id: "title".into(),
                field_type: FieldType::Text,
                required: true,
                unique: false,
                default_value: None,
                validations: vec![ValidationRule::MaxLength { value: 255 }],
                position: 0,
            }],
        })
        .await
        .expect("create article type");
}

async fn create_homepage_type(model: &ContentModelRegistry) {
    model
        .create_type(CreateContentTypeInput {
            name: "Homepage".into(),
            api_id: "homepage".into(),
            description: None,
            kind: ContentTypeKind::Single,
            fields: vec![FieldDefinition {
                name: "Title".into(),
                api_id: "title".into(),
                field_type: FieldType::Text,
                required: true,
                unique: false,
                default_value: None,
                validations: vec![],
                position: 0,
            }],
        })
        .await
        .expect("create homepage type");
}

async fn create_relation_types(model: &ContentModelRegistry) {
    model
        .create_type(CreateContentTypeInput {
            name: "Tag".into(),
            api_id: "tag".into(),
            description: None,
            kind: ContentTypeKind::Collection,
            fields: vec![FieldDefinition {
                name: "Name".into(),
                api_id: "name".into(),
                field_type: FieldType::Text,
                required: true,
                unique: false,
                default_value: None,
                validations: vec![],
                position: 0,
            }],
        })
        .await
        .expect("create tag type");
    model
        .create_type(CreateContentTypeInput {
            name: "Story".into(),
            api_id: "story".into(),
            description: None,
            kind: ContentTypeKind::Collection,
            fields: vec![
                FieldDefinition {
                    name: "Title".into(),
                    api_id: "title".into(),
                    field_type: FieldType::Text,
                    required: true,
                    unique: false,
                    default_value: None,
                    validations: vec![],
                    position: 0,
                },
                FieldDefinition {
                    name: "Tags".into(),
                    api_id: "tags".into(),
                    field_type: FieldType::Relation {
                        target_type: "tag".into(),
                        relation_kind: RelationKind::OneToMany,
                    },
                    required: false,
                    unique: false,
                    default_value: None,
                    validations: vec![],
                    position: 1,
                },
            ],
        })
        .await
        .expect("create story type");
}

async fn make_article(engine: &ContentEngine, fields: Value, slug: Option<&str>) -> String {
    let entry = engine
        .create(CreateEntryInput {
            content_type_api_id: "article".into(),
            data: fields,
            slug: slug.map(str::to_owned),
            actor_id: USER_AUTHOR.into(),
        })
        .await
        .expect("create article");
    entry.id
}

#[tokio::test]
async fn create_collection_entry_persists_and_emits_event() {
    let s = setup().await;
    create_article_type(&s.model).await;

    let collected = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::new(EventSink {
        events: Arc::clone(&collected),
    });
    let _h = s.bus.subscribe(EventKind::ContentCreated, sink);

    let entry = s
        .engine
        .create(CreateEntryInput {
            content_type_api_id: "article".into(),
            data: json!({ "title": "Hello" }),
            slug: Some("hello".into()),
            actor_id: USER_AUTHOR.into(),
        })
        .await
        .unwrap();

    assert_eq!(entry.content_type_api_id, "article");
    assert_eq!(entry.slug.as_deref(), Some("hello"));
    assert_eq!(entry.fields["title"], "Hello");
    assert_eq!(entry.status, ContentStatus::Draft);

    wait_for(&collected, 1).await;
    let events = collected.lock().unwrap();
    assert_eq!(events[0].kind, EventKind::ContentCreated);
    assert_eq!(events[0].payload["id"], entry.id);
    assert_eq!(events[0].payload["content_type_api_id"], "article");
    assert_eq!(events[0].actor_id.as_deref(), Some(USER_AUTHOR));
}

#[tokio::test]
async fn create_with_missing_required_field_returns_validation_error() {
    let s = setup().await;
    create_article_type(&s.model).await;

    let err = s
        .engine
        .create(CreateEntryInput {
            content_type_api_id: "article".into(),
            data: json!({}),
            slug: None,
            actor_id: USER_AUTHOR.into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, Error::ValidationError { .. }), "got: {err:?}");
}

#[tokio::test]
async fn create_with_non_object_data_returns_validation_error() {
    let s = setup().await;
    create_article_type(&s.model).await;

    let err = s
        .engine
        .create(CreateEntryInput {
            content_type_api_id: "article".into(),
            data: json!("not an object"),
            slug: None,
            actor_id: USER_AUTHOR.into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, Error::ValidationError { .. }), "got: {err:?}");
}

#[tokio::test]
async fn create_unknown_type_returns_not_found() {
    let s = setup().await;
    let err = s
        .engine
        .create(CreateEntryInput {
            content_type_api_id: "phantom".into(),
            data: json!({}),
            slug: None,
            actor_id: USER_AUTHOR.into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, Error::NotFound { .. }), "got: {err:?}");
}

#[tokio::test]
async fn create_single_kind_twice_returns_conflict() {
    let s = setup().await;
    create_homepage_type(&s.model).await;

    s.engine
        .create(CreateEntryInput {
            content_type_api_id: "homepage".into(),
            data: json!({ "title": "Welcome" }),
            slug: None,
            actor_id: USER_AUTHOR.into(),
        })
        .await
        .unwrap();

    let err = s
        .engine
        .create(CreateEntryInput {
            content_type_api_id: "homepage".into(),
            data: json!({ "title": "Second" }),
            slug: None,
            actor_id: USER_AUTHOR.into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Conflict { .. }), "got: {err:?}");
}

#[tokio::test]
async fn update_overwrites_fields_and_emits_event() {
    let s = setup().await;
    create_article_type(&s.model).await;
    let id = make_article(&s.engine, json!({ "title": "v1" }), None).await;

    let collected = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::new(EventSink {
        events: Arc::clone(&collected),
    });
    let _h = s.bus.subscribe(EventKind::ContentUpdated, sink);

    let updated = s
        .engine
        .update(
            "article",
            &id,
            UpdateEntryInput {
                data: Some(json!({ "title": "v2" })),
                slug: Some(Some("v2-slug".into())),
                actor_id: USER_AUTHOR.into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.fields["title"], "v2");
    assert_eq!(updated.slug.as_deref(), Some("v2-slug"));
    assert_eq!(updated.content_type_api_id, "article");

    wait_for(&collected, 1).await;
    let events = collected.lock().unwrap();
    assert_eq!(events[0].kind, EventKind::ContentUpdated);
    assert_eq!(events[0].payload["id"], id);
}

#[tokio::test]
async fn update_slug_three_state_handles_keep_clear_replace() {
    let s = setup().await;
    create_article_type(&s.model).await;
    let id = make_article(&s.engine, json!({ "title": "x" }), Some("initial")).await;

    let kept = s
        .engine
        .update(
            "article",
            &id,
            UpdateEntryInput {
                data: None,
                slug: None,
                actor_id: USER_AUTHOR.into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(kept.slug.as_deref(), Some("initial"));

    let replaced = s
        .engine
        .update(
            "article",
            &id,
            UpdateEntryInput {
                data: None,
                slug: Some(Some("replaced".into())),
                actor_id: USER_AUTHOR.into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(replaced.slug.as_deref(), Some("replaced"));

    let cleared = s
        .engine
        .update(
            "article",
            &id,
            UpdateEntryInput {
                data: None,
                slug: Some(None),
                actor_id: USER_AUTHOR.into(),
            },
        )
        .await
        .unwrap();
    assert!(cleared.slug.is_none());
}

#[tokio::test]
async fn update_with_invalid_data_returns_validation_error() {
    let s = setup().await;
    create_article_type(&s.model).await;
    let id = make_article(&s.engine, json!({ "title": "x" }), None).await;

    let err = s
        .engine
        .update(
            "article",
            &id,
            UpdateEntryInput {
                data: Some(json!({})),
                slug: None,
                actor_id: USER_AUTHOR.into(),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, Error::ValidationError { .. }));
}

#[tokio::test]
async fn update_unknown_entry_returns_not_found() {
    let s = setup().await;
    create_article_type(&s.model).await;
    let err = s
        .engine
        .update(
            "article",
            "00000000-0000-0000-0000-000000000099",
            UpdateEntryInput {
                data: None,
                slug: None,
                actor_id: USER_AUTHOR.into(),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, Error::NotFound { .. }));
}

#[tokio::test]
async fn get_returns_entry_with_content_type_api_id() {
    let s = setup().await;
    create_article_type(&s.model).await;
    let id = make_article(&s.engine, json!({ "title": "x" }), None).await;

    let fetched = s
        .engine
        .get("article", &id, &[])
        .await
        .unwrap()
        .expect("entry exists");
    assert_eq!(fetched.id, id);
    assert_eq!(fetched.content_type_api_id, "article");
}

#[tokio::test]
async fn get_missing_entry_returns_none() {
    let s = setup().await;
    create_article_type(&s.model).await;
    let fetched = s
        .engine
        .get("article", "00000000-0000-0000-0000-000000000099", &[])
        .await
        .unwrap();
    assert!(fetched.is_none());
}

#[tokio::test]
async fn get_with_populate_loads_relations() {
    let s = setup().await;
    create_relation_types(&s.model).await;

    let tag_id = s
        .engine
        .create(CreateEntryInput {
            content_type_api_id: "tag".into(),
            data: json!({ "name": "rust" }),
            slug: None,
            actor_id: USER_AUTHOR.into(),
        })
        .await
        .unwrap()
        .id;
    let story_id = s
        .engine
        .create(CreateEntryInput {
            content_type_api_id: "story".into(),
            data: json!({ "title": "a story" }),
            slug: None,
            actor_id: USER_AUTHOR.into(),
        })
        .await
        .unwrap()
        .id;
    seed_relation(&s.sqlite(), &story_id, &tag_id, "tags", "one_to_many", 0).await;

    let fetched = s
        .engine
        .get("story", &story_id, &["tags".to_owned()])
        .await
        .unwrap()
        .unwrap();
    let populated = fetched.populated.expect("populated set");
    let tags = populated.get("tags").expect("tags loaded");
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].fields["name"], "rust");
}

#[tokio::test]
async fn list_paginates_and_filters_by_status() {
    let s = setup().await;
    create_article_type(&s.model).await;
    for i in 0..5 {
        make_article(
            &s.engine,
            json!({ "title": format!("t-{i}") }),
            Some(&format!("s-{i}")),
        )
        .await;
    }

    let q = ContentQuery {
        page: Some(1),
        page_size: Some(2),
        status: Some(ContentStatus::Draft),
        sort: vec![],
        filters: vec![FilterSpec {
            field: FieldRef::Column(ColumnField::Slug),
            op: FilterOperator::Contains,
            value: json!("s-"),
        }],
        populate: vec![],
    };
    let res = s.engine.list("article", &q).await.unwrap();
    assert_eq!(res.meta.total, 5);
    assert_eq!(res.meta.page, 1);
    assert_eq!(res.meta.page_size, 2);
    assert_eq!(res.meta.page_count, 3);
    assert_eq!(res.data.len(), 2);
    for entry in &res.data {
        assert_eq!(entry.content_type_api_id, "article");
    }
}

#[tokio::test]
async fn list_with_populate_loads_relations_per_entry() {
    let s = setup().await;
    create_relation_types(&s.model).await;

    let tag_a = s
        .engine
        .create(CreateEntryInput {
            content_type_api_id: "tag".into(),
            data: json!({ "name": "rust" }),
            slug: None,
            actor_id: USER_AUTHOR.into(),
        })
        .await
        .unwrap()
        .id;
    let tag_b = s
        .engine
        .create(CreateEntryInput {
            content_type_api_id: "tag".into(),
            data: json!({ "name": "wasm" }),
            slug: None,
            actor_id: USER_AUTHOR.into(),
        })
        .await
        .unwrap()
        .id;

    let story_one = s
        .engine
        .create(CreateEntryInput {
            content_type_api_id: "story".into(),
            data: json!({ "title": "one" }),
            slug: None,
            actor_id: USER_AUTHOR.into(),
        })
        .await
        .unwrap()
        .id;
    let story_two = s
        .engine
        .create(CreateEntryInput {
            content_type_api_id: "story".into(),
            data: json!({ "title": "two" }),
            slug: None,
            actor_id: USER_AUTHOR.into(),
        })
        .await
        .unwrap()
        .id;
    seed_relation(&s.sqlite(), &story_one, &tag_a, "tags", "one_to_many", 0).await;
    seed_relation(&s.sqlite(), &story_two, &tag_b, "tags", "one_to_many", 0).await;

    let q = ContentQuery {
        populate: vec!["tags".to_owned()],
        ..ContentQuery::default()
    };
    let res = s.engine.list("story", &q).await.unwrap();
    assert_eq!(res.meta.total, 2);
    for entry in &res.data {
        let populated = entry.populated.as_ref().expect("populated set");
        let tags = populated.get("tags").expect("tags");
        assert_eq!(tags.len(), 1);
    }
}
