use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use cycms_core::{Error, Result};
use cycms_db::DatabasePool;
use cycms_events::{Event, EventBus, EventHandler, EventKind, SubscriptionHandle};
use serde_json::Value;
use sqlx::types::Json;
use uuid::Uuid;

pub const SYSTEM_ACTOR_ID: &str = "00000000-0000-0000-0000-000000000000";

pub struct AuditLogger {
    db: Arc<DatabasePool>,
}

struct AuditEntry {
    id: Uuid,
    actor_id: Uuid,
    action: String,
    resource_type: &'static str,
    resource_id: Option<String>,
    details: Option<Value>,
    result: String,
    created_at: DateTime<Utc>,
}

impl AuditLogger {
    #[must_use]
    pub fn new(db: Arc<DatabasePool>) -> Self {
        Self { db }
    }

    pub fn subscribe_all(self: &Arc<Self>, event_bus: &EventBus) -> Vec<SubscriptionHandle> {
        audit_event_kinds()
            .into_iter()
            .map(|kind| event_bus.subscribe(kind, Arc::clone(self) as Arc<dyn EventHandler>))
            .collect()
    }

    async fn insert_entry(&self, entry: AuditEntry) -> Result<()> {
        match self.db.as_ref() {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO audit_logs \
                     (id, actor_id, action, resource_type, resource_id, details, result, created_at) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                )
                .bind(entry.id)
                .bind(entry.actor_id)
                .bind(entry.action)
                .bind(entry.resource_type)
                .bind(entry.resource_id)
                .bind(entry.details.map(Json))
                .bind(entry.result)
                .bind(entry.created_at)
                .execute(pool)
                .await
                .map_err(|source| Error::Internal {
                    message: format!("insert audit log failed: {source}"),
                    source: Some(Box::new(source)),
                })?;
            }
            DatabasePool::MySql(pool) => {
                sqlx::query(
                    "INSERT INTO audit_logs \
                     (id, actor_id, action, resource_type, resource_id, details, result, created_at) \
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(entry.id.to_string())
                .bind(entry.actor_id.to_string())
                .bind(entry.action)
                .bind(entry.resource_type)
                .bind(entry.resource_id)
                .bind(entry.details.map(Json))
                .bind(entry.result)
                .bind(entry.created_at.naive_utc())
                .execute(pool)
                .await
                .map_err(|source| Error::Internal {
                    message: format!("insert audit log failed: {source}"),
                    source: Some(Box::new(source)),
                })?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO audit_logs \
                     (id, actor_id, action, resource_type, resource_id, details, result, created_at) \
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(entry.id.to_string())
                .bind(entry.actor_id.to_string())
                .bind(entry.action)
                .bind(entry.resource_type)
                .bind(entry.resource_id)
                .bind(entry.details.as_ref().map(Value::to_string))
                .bind(entry.result)
                .bind(entry.created_at.to_rfc3339())
                .execute(pool)
                .await
                .map_err(|source| Error::Internal {
                    message: format!("insert audit log failed: {source}"),
                    source: Some(Box::new(source)),
                })?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl EventHandler for AuditLogger {
    fn name(&self) -> &str {
        "audit_logger"
    }

    async fn handle(&self, event: Arc<Event>) -> Result<()> {
        self.insert_entry(AuditEntry::from_event(event.as_ref()))
            .await
    }
}

impl AuditEntry {
    fn from_event(event: &Event) -> Self {
        let (resource_type, resource_id) = resource_target(event);
        Self {
            id: event.id,
            actor_id: normalize_actor_id(event.actor_id.as_deref()),
            action: event.kind.as_str().to_owned(),
            resource_type,
            resource_id,
            details: (!event.payload.is_null()).then(|| event.payload.clone()),
            result: event
                .payload
                .get("result")
                .and_then(Value::as_str)
                .unwrap_or("success")
                .to_owned(),
            created_at: event.timestamp,
        }
    }
}

fn normalize_actor_id(actor_id: Option<&str>) -> Uuid {
    actor_id
        .and_then(|value| Uuid::parse_str(value).ok())
        .unwrap_or_else(Uuid::nil)
}

fn resource_target(event: &Event) -> (&'static str, Option<String>) {
    match &event.kind {
        EventKind::ContentCreated
        | EventKind::ContentUpdated
        | EventKind::ContentDeleted
        | EventKind::ContentPublished
        | EventKind::ContentUnpublished => ("content", payload_string(&event.payload, "id")),
        EventKind::UserCreated | EventKind::UserUpdated | EventKind::UserDeleted => {
            ("user", payload_string(&event.payload, "id"))
        }
        EventKind::MediaUploaded | EventKind::MediaDeleted => {
            ("media", payload_string(&event.payload, "id"))
        }
        EventKind::PluginInstalled
        | EventKind::PluginEnabled
        | EventKind::PluginDisabled
        | EventKind::PluginUninstalled => (
            "plugin",
            payload_string(&event.payload, "name").or_else(|| payload_string(&event.payload, "id")),
        ),
        EventKind::Custom(_) => ("custom", payload_string(&event.payload, "id")),
    }
}

fn payload_string(payload: &Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn audit_event_kinds() -> [EventKind; 14] {
    [
        EventKind::ContentCreated,
        EventKind::ContentUpdated,
        EventKind::ContentDeleted,
        EventKind::ContentPublished,
        EventKind::ContentUnpublished,
        EventKind::UserCreated,
        EventKind::UserUpdated,
        EventKind::UserDeleted,
        EventKind::MediaUploaded,
        EventKind::MediaDeleted,
        EventKind::PluginInstalled,
        EventKind::PluginEnabled,
        EventKind::PluginDisabled,
        EventKind::PluginUninstalled,
    ]
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use cycms_config::{DatabaseConfig, DatabaseDriver};
    use cycms_db::DatabasePool;
    use cycms_events::{Event, EventBus, EventKind};
    use cycms_migrate::MigrationEngine;
    use serde_json::{Value, json};
    use sqlx::Row;
    use uuid::Uuid;

    use super::{AuditLogger, SYSTEM_ACTOR_ID};

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
            .unwrap(),
        );
        MigrationEngine::new(Arc::clone(&pool))
            .run_system_migrations(&system_migrations_root())
            .await
            .unwrap();
        pool
    }

    async fn wait_for_audit_rows(pool: &Arc<DatabasePool>, expected: i64) {
        let DatabasePool::Sqlite(inner) = pool.as_ref() else {
            panic!("expected sqlite pool");
        };

        for _ in 0..100 {
            let row = sqlx::query("SELECT COUNT(*) AS count FROM audit_logs")
                .fetch_one(inner)
                .await
                .unwrap();
            let count: i64 = row.try_get("count").unwrap();
            if count >= expected {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        panic!("audit log row count did not reach {expected}");
    }

    #[tokio::test]
    async fn writes_content_events_to_audit_logs() {
        let pool = fresh_sqlite_pool().await;
        let bus = EventBus::new();
        let logger = Arc::new(AuditLogger::new(Arc::clone(&pool)));
        let _handles = logger.subscribe_all(&bus);
        let actor_id = Uuid::new_v4().to_string();

        bus.publish(
            Event::new(EventKind::ContentCreated)
                .with_actor(&actor_id)
                .with_payload(json!({
                    "id": "entry-1",
                    "content_type_api_id": "page",
                    "result": "success"
                })),
        );

        wait_for_audit_rows(&pool, 1).await;

        let DatabasePool::Sqlite(inner) = pool.as_ref() else {
            panic!("expected sqlite pool");
        };
        let row = sqlx::query(
            "SELECT actor_id, action, resource_type, resource_id, result, details FROM audit_logs LIMIT 1",
        )
        .fetch_one(inner)
        .await
        .unwrap();

        let details: String = row.try_get("details").unwrap();
        assert_eq!(row.try_get::<String, _>("actor_id").unwrap(), actor_id);
        assert_eq!(
            row.try_get::<String, _>("action").unwrap(),
            "content.created"
        );
        assert_eq!(
            row.try_get::<String, _>("resource_type").unwrap(),
            "content"
        );
        assert_eq!(row.try_get::<String, _>("resource_id").unwrap(), "entry-1");
        assert_eq!(row.try_get::<String, _>("result").unwrap(), "success");
        assert_eq!(
            serde_json::from_str::<Value>(&details).unwrap()["content_type_api_id"],
            "page"
        );
    }

    #[tokio::test]
    async fn actorless_events_fallback_to_system_actor() {
        let pool = fresh_sqlite_pool().await;
        let bus = EventBus::new();
        let logger = Arc::new(AuditLogger::new(Arc::clone(&pool)));
        let _handles = logger.subscribe_all(&bus);

        bus.publish(Event::new(EventKind::PluginEnabled).with_payload(json!({
            "name": "blog",
            "result": "success"
        })));

        wait_for_audit_rows(&pool, 1).await;

        let DatabasePool::Sqlite(inner) = pool.as_ref() else {
            panic!("expected sqlite pool");
        };
        let row =
            sqlx::query("SELECT actor_id, resource_type, resource_id FROM audit_logs LIMIT 1")
                .fetch_one(inner)
                .await
                .unwrap();

        assert_eq!(
            row.try_get::<String, _>("actor_id").unwrap(),
            SYSTEM_ACTOR_ID
        );
        assert_eq!(row.try_get::<String, _>("resource_type").unwrap(), "plugin");
        assert_eq!(row.try_get::<String, _>("resource_id").unwrap(), "blog");
    }
}
