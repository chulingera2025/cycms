use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use cycms_config::{ContentConfig, DatabaseConfig, DatabaseDriver, DeleteMode};
use cycms_content_engine::{
    ContentEngine, ContentEntryRepository, ContentStatus, NewContentEntryRow, new_content_entry_id,
};
use cycms_content_model::ContentModelRegistry;
use cycms_core::{Error, Result};
use cycms_db::DatabasePool;
use cycms_events::{Event, EventBus, EventHandler, EventKind};
use cycms_migrate::MigrationEngine;
use serde_json::json;
use sqlx::SqlitePool;

const TYPE_ARTICLE: &str = "00000000-0000-0000-0000-0000000000aa";
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

async fn seed_relation(pool: &SqlitePool, source: &str, target: &str, field: &str) {
    sqlx::query(
        "INSERT INTO content_relations \
         (id, source_entry_id, target_entry_id, field_api_id, relation_kind, position) \
         VALUES (?, ?, ?, ?, 'one_to_many', 0)",
    )
    .bind(new_content_entry_id())
    .bind(source)
    .bind(target)
    .bind(field)
    .execute(pool)
    .await
    .expect("seed relation");
}

struct DeletedSink {
    events: Arc<Mutex<Vec<Arc<Event>>>>,
}

#[async_trait]
impl EventHandler for DeletedSink {
    fn name(&self) -> &'static str {
        "test.sink"
    }

    async fn handle(&self, event: Arc<Event>) -> Result<()> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}

struct Setup {
    pool: Arc<DatabasePool>,
    engine: ContentEngine,
    bus: Arc<EventBus>,
    repo: ContentEntryRepository,
}

async fn make_setup(default_mode: DeleteMode) -> Setup {
    let pool = fresh_pool().await;
    let DatabasePool::Sqlite(inner) = pool.as_ref() else {
        panic!("expected sqlite");
    };
    let inner = inner.clone();
    seed_user(&inner, USER_AUTHOR).await;
    seed_type(&inner, TYPE_ARTICLE, "article", "collection").await;

    let model = Arc::new(ContentModelRegistry::with_fresh_registry(Arc::clone(&pool)));
    let bus = Arc::new(EventBus::new());
    let cfg = ContentConfig {
        default_delete_mode: default_mode,
        default_page_size: 20,
        max_page_size: 100,
    };
    let engine = ContentEngine::new(Arc::clone(&pool), model, Arc::clone(&bus), cfg);
    let repo = ContentEntryRepository::new(Arc::clone(&pool));
    Setup {
        pool,
        engine,
        bus,
        repo,
    }
}

async fn insert_article(repo: &ContentEntryRepository, slug: Option<&str>) -> String {
    let id = new_content_entry_id();
    repo.insert(NewContentEntryRow {
        id: id.clone(),
        content_type_id: TYPE_ARTICLE.to_owned(),
        slug: slug.map(str::to_owned),
        status: ContentStatus::Draft,
        fields: json!({}),
        created_by: USER_AUTHOR.to_owned(),
    })
    .await
    .expect("insert");
    id
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

#[tokio::test]
async fn delete_soft_marks_archived_and_emits_event() {
    let setup = make_setup(DeleteMode::Soft).await;
    let id = insert_article(&setup.repo, None).await;

    let collected = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::new(DeletedSink {
        events: Arc::clone(&collected),
    });
    let _h = setup.bus.subscribe(EventKind::ContentDeleted, sink);

    setup
        .engine
        .delete("article", &id, None, USER_AUTHOR)
        .await
        .unwrap();

    let entry = setup.repo.find_by_id(&id).await.unwrap().unwrap();
    assert_eq!(entry.status, ContentStatus::Archived);

    wait_for(&collected, 1).await;
    let events = collected.lock().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::ContentDeleted);
    assert_eq!(events[0].payload["mode"], "soft");
    assert_eq!(events[0].payload["content_type_api_id"], "article");
    assert_eq!(events[0].payload["id"], id);
    assert_eq!(events[0].actor_id.as_deref(), Some(USER_AUTHOR));
}

#[tokio::test]
async fn delete_hard_removes_entry_and_emits_event() {
    let setup = make_setup(DeleteMode::Soft).await;
    let id = insert_article(&setup.repo, None).await;

    let collected = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::new(DeletedSink {
        events: Arc::clone(&collected),
    });
    let _h = setup.bus.subscribe(EventKind::ContentDeleted, sink);

    setup
        .engine
        .delete("article", &id, Some(DeleteMode::Hard), USER_AUTHOR)
        .await
        .unwrap();

    assert!(setup.repo.find_by_id(&id).await.unwrap().is_none());

    wait_for(&collected, 1).await;
    let events = collected.lock().unwrap();
    assert_eq!(events[0].payload["mode"], "hard");
}

#[tokio::test]
async fn delete_hard_with_inbound_refs_returns_conflict() {
    let setup = make_setup(DeleteMode::Hard).await;
    let DatabasePool::Sqlite(inner) = setup.pool.as_ref() else {
        panic!("sqlite expected");
    };

    let target_id = insert_article(&setup.repo, Some("target")).await;
    let source_id = insert_article(&setup.repo, Some("source")).await;
    seed_relation(inner, &source_id, &target_id, "related").await;

    let collected = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::new(DeletedSink {
        events: Arc::clone(&collected),
    });
    let _h = setup.bus.subscribe(EventKind::ContentDeleted, sink);

    let err = setup
        .engine
        .delete("article", &target_id, Some(DeleteMode::Hard), USER_AUTHOR)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Conflict { .. }), "got: {err:?}");

    // Entry still exists
    assert!(setup.repo.find_by_id(&target_id).await.unwrap().is_some());

    // No event published when delete fails
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(collected.lock().unwrap().is_empty());
}

#[tokio::test]
async fn delete_missing_entry_returns_not_found() {
    let setup = make_setup(DeleteMode::Soft).await;
    let err = setup
        .engine
        .delete("article", &new_content_entry_id(), None, USER_AUTHOR)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::NotFound { .. }), "got: {err:?}");
}

#[tokio::test]
async fn delete_unknown_content_type_returns_not_found() {
    let setup = make_setup(DeleteMode::Soft).await;
    let err = setup
        .engine
        .delete(
            "phantom",
            "00000000-0000-0000-0000-000000000099",
            None,
            USER_AUTHOR,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, Error::NotFound { .. }), "got: {err:?}");
}

#[tokio::test]
async fn delete_uses_default_mode_from_content_config() {
    let setup = make_setup(DeleteMode::Hard).await;
    let id = insert_article(&setup.repo, None).await;

    setup
        .engine
        .delete("article", &id, None, USER_AUTHOR)
        .await
        .unwrap();
    assert!(setup.repo.find_by_id(&id).await.unwrap().is_none());
}
