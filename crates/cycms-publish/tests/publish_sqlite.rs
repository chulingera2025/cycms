use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use cycms_config::{ContentConfig, DatabaseConfig, DatabaseDriver, DeleteMode};
use cycms_content_engine::{
    ContentEngine, ContentEntryRepository, ContentQuery, ContentStatus, NewContentEntryRow,
    new_content_entry_id,
};
use cycms_content_model::{ContentModelRegistry, ContentTypeKind, CreateContentTypeInput};
use cycms_core::{Error, Result};
use cycms_db::DatabasePool;
use cycms_events::{Event, EventBus, EventHandler, EventKind};
use cycms_migrate::MigrationEngine;
use cycms_publish::PublishManager;
use cycms_revision::RevisionManager;
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

struct EventSink {
    events: Arc<Mutex<Vec<Arc<Event>>>>,
}

#[async_trait]
impl EventHandler for EventSink {
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
    publish: PublishManager,
    bus: Arc<EventBus>,
    repo: ContentEntryRepository,
}

async fn make_setup() -> Setup {
    let pool = fresh_pool().await;
    let DatabasePool::Sqlite(inner) = pool.as_ref() else {
        panic!("expected sqlite");
    };
    let inner = inner.clone();
    seed_user(&inner, USER_AUTHOR).await;
    seed_type(&inner, TYPE_ARTICLE, "article", "collection").await;

    let bus = Arc::new(EventBus::new());
    let publish = PublishManager::new(&pool, Arc::clone(&bus));
    let repo = ContentEntryRepository::new(Arc::clone(&pool));
    Setup {
        pool,
        publish,
        bus,
        repo,
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

// ── 直接通过 repo 插入 Draft 实例（不依赖 ContentEngine 的 create，避免类型注册）
async fn insert_draft(repo: &ContentEntryRepository) -> String {
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
    .expect("insert draft");
    id
}

#[tokio::test]
async fn publish_draft_sets_status_and_version() {
    let s = make_setup().await;
    let id = insert_draft(&s.repo).await;

    // 插入一个 revision 使 current_version_id 有值
    let rev_id = new_content_entry_id();
    let DatabasePool::Sqlite(inner) = s.pool.as_ref() else {
        panic!("sqlite");
    };
    sqlx::query(
        "INSERT INTO content_revisions \
         (id, content_entry_id, version_number, snapshot, created_by) VALUES (?, ?, 1, '{}', ?)",
    )
    .bind(&rev_id)
    .bind(&id)
    .bind(USER_AUTHOR)
    .execute(inner)
    .await
    .expect("insert revision");
    sqlx::query("UPDATE content_entries SET current_version_id = ? WHERE id = ?")
        .bind(&rev_id)
        .bind(&id)
        .execute(inner)
        .await
        .expect("set current_version_id");

    let entry = s
        .publish
        .publish(&id, "article", USER_AUTHOR)
        .await
        .unwrap();

    assert_eq!(entry.status, ContentStatus::Published);
    assert_eq!(entry.published_version_id.as_deref(), Some(rev_id.as_str()));
    assert!(entry.published_at.is_some());
    assert_eq!(entry.content_type_api_id, "article");
}

#[tokio::test]
async fn publish_emits_content_published_event() {
    let s = make_setup().await;
    let id = insert_draft(&s.repo).await;

    let collected = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::new(EventSink {
        events: Arc::clone(&collected),
    });
    let _h = s.bus.subscribe(EventKind::ContentPublished, sink);

    s.publish
        .publish(&id, "article", USER_AUTHOR)
        .await
        .unwrap();

    wait_for(&collected, 1).await;
    let events = collected.lock().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::ContentPublished);
    assert_eq!(events[0].payload["id"], id);
    assert_eq!(events[0].payload["content_type_api_id"], "article");
    assert_eq!(events[0].actor_id.as_deref(), Some(USER_AUTHOR));
}

#[tokio::test]
async fn publish_already_published_returns_conflict() {
    let s = make_setup().await;
    let id = insert_draft(&s.repo).await;

    // 先 publish 一次
    s.publish
        .publish(&id, "article", USER_AUTHOR)
        .await
        .unwrap();

    // 再次 publish → Conflict
    let err = s
        .publish
        .publish(&id, "article", USER_AUTHOR)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Conflict { .. }), "got: {err:?}");
}

#[tokio::test]
async fn publish_archived_returns_conflict() {
    let s = make_setup().await;
    let id = insert_draft(&s.repo).await;
    // 标记为 archived
    s.repo.mark_archived(&id, USER_AUTHOR).await.unwrap();

    let err = s
        .publish
        .publish(&id, "article", USER_AUTHOR)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Conflict { .. }), "got: {err:?}");
}

#[tokio::test]
async fn publish_missing_entry_returns_not_found() {
    let s = make_setup().await;
    let err = s
        .publish
        .publish(&new_content_entry_id(), "article", USER_AUTHOR)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::NotFound { .. }), "got: {err:?}");
}

#[tokio::test]
async fn unpublish_published_clears_version_and_sets_draft() {
    let s = make_setup().await;
    let id = insert_draft(&s.repo).await;

    // publish first
    s.publish
        .publish(&id, "article", USER_AUTHOR)
        .await
        .unwrap();

    let entry = s
        .publish
        .unpublish(&id, "article", USER_AUTHOR)
        .await
        .unwrap();

    assert_eq!(entry.status, ContentStatus::Draft);
    assert!(entry.published_version_id.is_none());
    // published_at 保留
    assert!(entry.published_at.is_some());
    assert_eq!(entry.content_type_api_id, "article");
}

#[tokio::test]
async fn unpublish_emits_content_unpublished_event() {
    let s = make_setup().await;
    let id = insert_draft(&s.repo).await;
    s.publish
        .publish(&id, "article", USER_AUTHOR)
        .await
        .unwrap();

    let collected = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::new(EventSink {
        events: Arc::clone(&collected),
    });
    let _h = s.bus.subscribe(EventKind::ContentUnpublished, sink);

    s.publish
        .unpublish(&id, "article", USER_AUTHOR)
        .await
        .unwrap();

    wait_for(&collected, 1).await;
    let events = collected.lock().unwrap();
    assert_eq!(events[0].kind, EventKind::ContentUnpublished);
    assert_eq!(events[0].payload["id"], id);
    assert_eq!(events[0].actor_id.as_deref(), Some(USER_AUTHOR));
}

#[tokio::test]
async fn unpublish_draft_returns_conflict() {
    let s = make_setup().await;
    let id = insert_draft(&s.repo).await;

    let err = s
        .publish
        .unpublish(&id, "article", USER_AUTHOR)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Conflict { .. }), "got: {err:?}");
}

#[tokio::test]
async fn unpublish_missing_entry_returns_not_found() {
    let s = make_setup().await;
    let err = s
        .publish
        .unpublish(&new_content_entry_id(), "article", USER_AUTHOR)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::NotFound { .. }), "got: {err:?}");
}

#[tokio::test]
async fn list_published_only_via_content_query() {
    let s = make_setup().await;

    let id_a = insert_draft(&s.repo).await;
    let id_b = insert_draft(&s.repo).await;

    // 只发布 id_a
    s.publish
        .publish(&id_a, "article", USER_AUTHOR)
        .await
        .unwrap();

    // ContentQuery 按 status 过滤，仅返回 Published
    let model = Arc::new(ContentModelRegistry::with_fresh_registry(Arc::clone(
        &s.pool,
    )));
    model
        .create_type(CreateContentTypeInput {
            name: "Article".into(),
            api_id: "article".into(),
            description: None,
            kind: ContentTypeKind::Collection,
            fields: vec![],
        })
        .await
        .ok();
    let cfg = ContentConfig {
        default_delete_mode: DeleteMode::Soft,
        default_page_size: 20,
        max_page_size: 100,
    };
    let revision_manager = Arc::new(RevisionManager::new(Arc::clone(&s.pool)));
    let engine = ContentEngine::new(
        Arc::clone(&s.pool),
        model,
        Arc::clone(&s.bus),
        cfg,
        revision_manager,
    );
    let q = ContentQuery {
        status: Some(ContentStatus::Published),
        ..ContentQuery::default()
    };
    let res = engine.list("article", &q).await.unwrap();
    assert_eq!(res.meta.total, 1);
    assert_eq!(res.data[0].id, id_a);
    let _ = id_b; // id_b 仍是 draft，不在结果中
}
