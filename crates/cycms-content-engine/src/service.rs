//! `ContentEngine` 服务门面（任务 11 Step 6/7）。
//!
//! v0.1 关注点：
//! - [`ContentEngine::create`]：校验 `Single` 唯一性 → 校验 fields → 持久化 →
//!   发布 `content.created`。
//! - [`ContentEngine::update`]：路由 type → 取现状 → 应用三态 patch → 校验 →
//!   持久化 → 发布 `content.updated`。
//! - [`ContentEngine::get`] / [`ContentEngine::list`]：按 type 过滤 + 单层 populate。
//! - [`ContentEngine::delete`]：软/硬删除切换 + 引用完整性检查 + `content.deleted`。
//!
//! `current_version_id` / `published_version_id` 留给任务 12 (Revision) /
//! 13 (Publish) 通过订阅 `content.*` 事件或独立服务接管，service 不直接维护。
//!
//! `populated` 中嵌套 entries 的 `content_type_api_id` 字段 v0.1 不会被回填
//! （需要按 `content_type_id` 反查 `content_types`），调用方如需该字段可单独
//! 通过 [`ContentEngine::get`] 二次查询。

use std::sync::Arc;

use cycms_config::{ContentConfig, DeleteMode};
use cycms_content_model::{ContentModelRegistry, ContentTypeKind};
use cycms_core::Result;
use cycms_db::DatabasePool;
use cycms_events::{Event, EventBus, EventKind};
use cycms_revision::{CreateRevisionInput, RevisionManager};
use serde_json::json;

use crate::error::{ContentEngineError, ReferenceViolation};
use crate::model::{
    ContentEntry, ContentStatus, CreateEntryInput, PaginatedResponse, PaginationMeta,
    UpdateEntryInput,
};
use crate::populate::populate_entries;
use crate::query::ContentQuery;
use crate::repository::{
    ContentEntryRepository, NewContentEntryRow, UpdateContentEntryRow, new_content_entry_id,
};

/// 内容引擎服务门面。聚合 repository、`ContentModelRegistry`、`EventBus`、
/// `RevisionManager` 与 `ContentConfig`，对外提供高层 CRUD。
pub struct ContentEngine {
    db: Arc<DatabasePool>,
    repo: ContentEntryRepository,
    content_model: Arc<ContentModelRegistry>,
    event_bus: Arc<EventBus>,
    content_config: ContentConfig,
    revision_manager: Arc<RevisionManager>,
}

impl ContentEngine {
    /// 组合现有依赖构造 `ContentEngine`。
    #[must_use]
    pub fn new(
        db: Arc<DatabasePool>,
        content_model: Arc<ContentModelRegistry>,
        event_bus: Arc<EventBus>,
        content_config: ContentConfig,
        revision_manager: Arc<RevisionManager>,
    ) -> Self {
        let repo = ContentEntryRepository::new(Arc::clone(&db));
        Self {
            db,
            repo,
            content_model,
            event_bus,
            content_config,
            revision_manager,
        }
    }

    /// 创建一条内容实例。
    ///
    /// 流程：路由 content type → 校验 `Single` 唯一性 → 校验 fields → repository
    /// 插入 → 通过 `EventBus` 发布 `content.created` 事件。
    ///
    /// # Errors
    /// - content type 不存在 → [`ContentEngineError::ContentTypeNotFound`]
    /// - `Single` 类型已存在条目 → [`ContentEngineError::SingleKindAlreadyExists`]
    /// - data 非 JSON object → [`ContentEngineError::InvalidEntryShape`]
    /// - 字段校验失败 → 透传 `ContentModelRegistry::validate_entry` 的错误
    /// - DB 故障 → [`ContentEngineError::Database`]
    pub async fn create(&self, input: CreateEntryInput) -> Result<ContentEntry> {
        if !input.data.is_object() {
            return Err(ContentEngineError::InvalidEntryShape.into());
        }
        let ct = self
            .content_model
            .get_type(&input.content_type_api_id)
            .await?
            .ok_or_else(|| {
                ContentEngineError::ContentTypeNotFound(input.content_type_api_id.clone())
            })?;
        if matches!(ct.kind, ContentTypeKind::Single) && self.repo.count_by_type(&ct.id).await? > 0
        {
            return Err(ContentEngineError::SingleKindAlreadyExists(
                input.content_type_api_id.clone(),
            )
            .into());
        }
        self.content_model
            .validate_entry(&input.content_type_api_id, &input.data)
            .await?;

        let mut entry = self
            .repo
            .insert(NewContentEntryRow {
                id: new_content_entry_id(),
                content_type_id: ct.id.clone(),
                slug: input.slug.clone(),
                status: ContentStatus::Draft,
                fields: input.data.clone(),
                created_by: input.actor_id.clone(),
            })
            .await?;
        entry
            .content_type_api_id
            .clone_from(&input.content_type_api_id);

        self.event_bus.publish(
            Event::new(EventKind::ContentCreated)
                .with_actor(input.actor_id.clone())
                .with_payload(json!({
                    "id": entry.id,
                    "content_type_api_id": input.content_type_api_id,
                })),
        );

        // 创建初始版本快照（失败传播，保证 current_version_id 一致性）
        self.revision_manager
            .create_revision(CreateRevisionInput {
                content_entry_id: entry.id.clone(),
                snapshot: entry.fields.clone(),
                actor_id: input.actor_id.clone(),
                change_summary: None,
            })
            .await?;

        Ok(entry)
    }

    /// 更新一条内容实例。
    ///
    /// 三态 `slug` 语义：
    /// - `None`：保留原值；
    /// - `Some(None)`：清空 slug；
    /// - `Some(Some(s))`：替换。
    ///
    /// `data = None` 时 fields 不变；`data = Some(v)` 时整体替换并触发校验。
    ///
    /// # Errors
    /// - content type / 实例不存在 → [`ContentEngineError::ContentTypeNotFound`] /
    ///   [`ContentEngineError::EntryNotFound`]
    /// - data 非 JSON object → [`ContentEngineError::InvalidEntryShape`]
    /// - 字段校验失败 → 透传 `ContentModelRegistry::validate_entry` 的错误
    /// - DB 故障 → [`ContentEngineError::Database`]
    pub async fn update(
        &self,
        type_api_id: &str,
        id: &str,
        input: UpdateEntryInput,
    ) -> Result<ContentEntry> {
        let ct = self
            .content_model
            .get_type(type_api_id)
            .await?
            .ok_or_else(|| ContentEngineError::ContentTypeNotFound(type_api_id.to_owned()))?;
        let existing = self
            .repo
            .find_by_id_and_type(id, &ct.id)
            .await?
            .ok_or_else(|| ContentEngineError::EntryNotFound(id.to_owned()))?;

        let new_fields = match input.data.clone() {
            Some(v) => {
                if !v.is_object() {
                    return Err(ContentEngineError::InvalidEntryShape.into());
                }
                self.content_model.validate_entry(type_api_id, &v).await?;
                v
            }
            None => existing.fields.clone(),
        };

        let new_slug = match input.slug {
            Some(opt) => opt,
            None => existing.slug.clone(),
        };

        let mut updated = self
            .repo
            .update(
                id,
                UpdateContentEntryRow {
                    slug: new_slug,
                    status: existing.status,
                    fields: new_fields,
                    updated_by: input.actor_id.clone(),
                },
            )
            .await?;
        type_api_id.clone_into(&mut updated.content_type_api_id);

        self.event_bus.publish(
            Event::new(EventKind::ContentUpdated)
                .with_actor(input.actor_id.clone())
                .with_payload(json!({
                    "id": updated.id,
                    "content_type_api_id": type_api_id,
                })),
        );

        // 追加新版本快照
        self.revision_manager
            .create_revision(CreateRevisionInput {
                content_entry_id: updated.id.clone(),
                snapshot: updated.fields.clone(),
                actor_id: input.actor_id.clone(),
                change_summary: None,
            })
            .await?;

        Ok(updated)
    }

    /// 按 `id` 获取单条内容实例（按 type 路由）。可选传入 `populate` 字段名列表
    /// 触发单层关联加载。
    ///
    /// # Errors
    /// - content type 不存在 → [`ContentEngineError::ContentTypeNotFound`]
    /// - DB 故障 → [`ContentEngineError::Database`]
    pub async fn get(
        &self,
        type_api_id: &str,
        id: &str,
        populate: &[String],
    ) -> Result<Option<ContentEntry>> {
        let ct = self
            .content_model
            .get_type(type_api_id)
            .await?
            .ok_or_else(|| ContentEngineError::ContentTypeNotFound(type_api_id.to_owned()))?;
        let Some(mut entry) = self.repo.find_by_id_and_type(id, &ct.id).await? else {
            return Ok(None);
        };
        entry.content_type_api_id = type_api_id.to_owned();

        if !populate.is_empty() {
            let mut entries = populate_entries(&self.db, &self.repo, vec![entry], populate).await?;
            entry = entries
                .pop()
                .ok_or_else(|| ContentEngineError::EntryNotFound(id.to_owned()))?;
        }
        Ok(Some(entry))
    }

    /// 按 `type_api_id` 列出 entries。`query.populate` 非空时执行单层 populate。
    ///
    /// 分页参数从 `ContentConfig.default_page_size` / `max_page_size` 取默认与上限。
    ///
    /// # Errors
    /// - content type 不存在 → [`ContentEngineError::ContentTypeNotFound`]
    /// - 查询编译失败 → [`ContentEngineError::InvalidQuery`]
    /// - DB 故障 → [`ContentEngineError::Database`]
    pub async fn list(
        &self,
        type_api_id: &str,
        query: &ContentQuery,
    ) -> Result<PaginatedResponse<ContentEntry>> {
        let ct = self
            .content_model
            .get_type(type_api_id)
            .await?
            .ok_or_else(|| ContentEngineError::ContentTypeNotFound(type_api_id.to_owned()))?;

        let result = self
            .repo
            .list(
                &ct.id,
                query,
                self.content_config.default_page_size,
                self.content_config.max_page_size,
            )
            .await?;

        let mut entries = result.entries;
        for e in &mut entries {
            type_api_id.clone_into(&mut e.content_type_api_id);
        }
        if !query.populate.is_empty() {
            entries = populate_entries(&self.db, &self.repo, entries, &query.populate).await?;
        }

        let page_count = if result.page_size == 0 {
            0
        } else {
            result.total.div_ceil(result.page_size)
        };
        Ok(PaginatedResponse {
            data: entries,
            meta: PaginationMeta {
                page: result.page,
                page_size: result.page_size,
                page_count,
                total: result.total,
            },
        })
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
