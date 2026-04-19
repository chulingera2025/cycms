//! `ContentEngine` 服务门面（任务 11 Step 6/7）。
//!
//! v0.1 关注点：
//! - 删除路径在 [`ContentEngine::delete`] 完成（Step 6 已落地）：
//!   - `Soft` 模式标记 `archived` 不阻止；
//!   - `Hard` 模式先扫 `content_relations.target_entry_id`，存在引用即返回
//!     [`ContentEngineError::ReferentialIntegrity`]，否则物理删除；
//!   - 完成后通过 `EventBus` 发布 `content.deleted` 事件，payload 含 `id`、
//!     `content_type_api_id`、`mode`。
//! - TODO!!!: Step 7 加入 `create` / `update` / `get` / `list` 方法 + 字段校验
//!   委托 + `content.created` / `content.updated` 事件 + `Single` kind 唯一性检查。

use std::sync::Arc;

use cycms_config::{ContentConfig, DeleteMode};
use cycms_content_model::ContentModelRegistry;
use cycms_core::Result;
use cycms_db::DatabasePool;
use cycms_events::{Event, EventBus, EventKind};
use serde_json::json;

use crate::error::{ContentEngineError, ReferenceViolation};
use crate::repository::ContentEntryRepository;

/// 内容引擎服务门面。聚合 repository、`ContentModelRegistry`、`EventBus` 与
/// `ContentConfig`，对外提供高层 CRUD（v0.1 Step 6/7 分批落地）。
pub struct ContentEngine {
    repo: ContentEntryRepository,
    content_model: Arc<ContentModelRegistry>,
    event_bus: Arc<EventBus>,
    content_config: ContentConfig,
}

impl ContentEngine {
    /// 组合现有依赖构造 `ContentEngine`。
    #[must_use]
    pub fn new(
        db: Arc<DatabasePool>,
        content_model: Arc<ContentModelRegistry>,
        event_bus: Arc<EventBus>,
        content_config: ContentConfig,
    ) -> Self {
        let repo = ContentEntryRepository::new(db);
        Self {
            repo,
            content_model,
            event_bus,
            content_config,
        }
    }

    /// 删除指定 content type 下的实例。
    ///
    /// `mode` 为 `None` 时使用 `ContentConfig.default_delete_mode`。
    /// - `Soft`：把实例 status 置为 `archived`，不做引用检查；
    /// - `Hard`：先扫 `content_relations` 反向引用，存在则返回
    ///   [`ContentEngineError::ReferentialIntegrity`]，否则物理删除。
    ///
    /// 删除成功后通过 `EventBus` 发布 `content.deleted`，payload 包含 `id` /
    /// `content_type_api_id` / `mode`，actor 字段填 `actor_id`。
    ///
    /// # Errors
    /// - content type 不存在 → [`ContentEngineError::ContentTypeNotFound`]
    /// - 实例不存在 → [`ContentEngineError::EntryNotFound`]
    /// - 硬删时存在引用 → [`ContentEngineError::ReferentialIntegrity`]
    /// - DB 故障 → [`ContentEngineError::Database`]
    pub async fn delete(
        &self,
        type_api_id: &str,
        id: &str,
        mode: Option<DeleteMode>,
        actor_id: &str,
    ) -> Result<()> {
        let mode = mode.unwrap_or(self.content_config.default_delete_mode);

        let ct = self
            .content_model
            .get_type(type_api_id)
            .await?
            .ok_or_else(|| ContentEngineError::ContentTypeNotFound(type_api_id.to_owned()))?;

        let entry = self
            .repo
            .find_by_id_and_type(id, &ct.id)
            .await?
            .ok_or_else(|| ContentEngineError::EntryNotFound(id.to_owned()))?;

        match mode {
            DeleteMode::Soft => {
                self.repo.mark_archived(id, actor_id).await?;
            }
            DeleteMode::Hard => {
                let refs = self.repo.find_inbound_references(id).await?;
                if !refs.is_empty() {
                    let violations = refs
                        .into_iter()
                        .map(|r| ReferenceViolation {
                            source_entry_id: r.source_entry_id,
                            field_api_id: r.field_api_id,
                            relation_kind: r.relation_kind,
                        })
                        .collect();
                    return Err(ContentEngineError::ReferentialIntegrity {
                        entry_id: id.to_owned(),
                        violations,
                    }
                    .into());
                }
                self.repo.delete_hard(id).await?;
            }
        }

        let mode_str = match mode {
            DeleteMode::Soft => "soft",
            DeleteMode::Hard => "hard",
        };
        self.event_bus.publish(
            Event::new(EventKind::ContentDeleted)
                .with_actor(actor_id)
                .with_payload(json!({
                    "id": entry.id,
                    "content_type_api_id": type_api_id,
                    "mode": mode_str,
                })),
        );
        Ok(())
    }
}
