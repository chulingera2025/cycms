//! `PublishManager` 服务门面（任务 13）。
//!
//! 实现内容发布状态机：
//! - [`PublishManager::publish`]：Draft → Published，绑定 `published_version_id`，
//!   设置 `published_at`，发布 `content.published` 事件。
//! - [`PublishManager::unpublish`]：Published → Draft，清空 `published_version_id`，
//!   发布 `content.unpublished` 事件（`published_at` 保留历史记录）。
//!
//! 非法状态转换（如对 Published 再次 publish）返回 [`cycms_core::Error::Conflict`]。

use std::sync::Arc;

use cycms_content_engine::{ContentEntry, ContentEntryRepository, ContentStatus};
use cycms_core::Result;
use cycms_db::DatabasePool;
use cycms_events::{Event, EventBus, EventKind};
use serde_json::json;

use crate::error::PublishError;
use crate::repository::PublishRepository;

/// 发布状态机服务门面。
pub struct PublishManager {
    repo: PublishRepository,
    entry_repo: ContentEntryRepository,
    event_bus: Arc<EventBus>,
}

impl PublishManager {
    #[must_use]
    pub fn new(db: &Arc<DatabasePool>, event_bus: Arc<EventBus>) -> Self {
        Self {
            repo: PublishRepository::new(Arc::clone(db)),
            entry_repo: ContentEntryRepository::new(Arc::clone(db)),
            event_bus,
        }
    }

    /// 将 Draft 实例升级为 Published。
    ///
    /// 成功后：`status = published`，`published_version_id = current_version_id`，
    /// `published_at = now()`，并通过 `EventBus` 发布 `content.published`。
    ///
    /// # Errors
    /// - 实例不存在 → [`cycms_core::Error::NotFound`]
    /// - 实例非 Draft 状态 → [`cycms_core::Error::Conflict`]
    /// - DB 故障 → [`cycms_core::Error::Internal`]
    pub async fn publish(
        &self,
        entry_id: &str,
        content_type_api_id: &str,
        actor_id: &str,
    ) -> Result<ContentEntry> {
        let entry = self
            .entry_repo
            .find_by_id(entry_id)
            .await?
            .ok_or_else(|| PublishError::EntryNotFound(entry_id.to_owned()))?;

        if entry.status != ContentStatus::Draft {
            return Err(PublishError::InvalidTransition {
                entry_id: entry_id.to_owned(),
                from: entry.status,
                to: ContentStatus::Published,
            }
            .into());
        }

        self.repo.publish_entry(entry_id, actor_id).await?;

        let mut updated = self.entry_repo.find_by_id(entry_id).await?.ok_or_else(|| {
            cycms_core::Error::Internal {
                message: "published entry not found on read-back".to_owned(),
                source: None,
            }
        })?;
        content_type_api_id.clone_into(&mut updated.content_type_api_id);

        self.event_bus.publish(
            Event::new(EventKind::ContentPublished)
                .with_actor(actor_id)
                .with_payload(json!({
                    "id": entry_id,
                    "content_type_api_id": content_type_api_id,
                })),
        );

        Ok(updated)
    }

    /// 将 Published 实例撤回为 Draft。
    ///
    /// 成功后：`status = draft`，`published_version_id = NULL`，`published_at` 保留。
    /// 通过 `EventBus` 发布 `content.unpublished`。
    ///
    /// # Errors
    /// - 实例不存在 → [`cycms_core::Error::NotFound`]
    /// - 实例非 Published 状态 → [`cycms_core::Error::Conflict`]
    /// - DB 故障 → [`cycms_core::Error::Internal`]
    pub async fn unpublish(
        &self,
        entry_id: &str,
        content_type_api_id: &str,
        actor_id: &str,
    ) -> Result<ContentEntry> {
        let entry = self
            .entry_repo
            .find_by_id(entry_id)
            .await?
            .ok_or_else(|| PublishError::EntryNotFound(entry_id.to_owned()))?;

        if entry.status != ContentStatus::Published {
            return Err(PublishError::InvalidTransition {
                entry_id: entry_id.to_owned(),
                from: entry.status,
                to: ContentStatus::Draft,
            }
            .into());
        }

        self.repo.unpublish_entry(entry_id, actor_id).await?;

        let mut updated = self.entry_repo.find_by_id(entry_id).await?.ok_or_else(|| {
            cycms_core::Error::Internal {
                message: "unpublished entry not found on read-back".to_owned(),
                source: None,
            }
        })?;
        content_type_api_id.clone_into(&mut updated.content_type_api_id);

        self.event_bus.publish(
            Event::new(EventKind::ContentUnpublished)
                .with_actor(actor_id)
                .with_payload(json!({
                    "id": entry_id,
                    "content_type_api_id": content_type_api_id,
                })),
        );

        Ok(updated)
    }
}
