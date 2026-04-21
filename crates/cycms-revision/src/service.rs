//! `RevisionManager` 服务门面。
//!
//! 职责：
//! - [`RevisionManager::create_revision`]：为 content entry 创建不可变快照，
//!   并同步更新 `content_entries.current_version_id`。
//! - [`RevisionManager::list_revisions`]：按版本号倒序分页列出历史。
//! - [`RevisionManager::get_revision`]：精确读取某版本快照。
//! - [`RevisionManager::rollback`]：基于目标快照创建新版本，并回写 entry fields。

use std::sync::Arc;

use cycms_core::Result;
use cycms_db::DatabasePool;

use crate::error::RevisionError;
use crate::model::{CreateRevisionInput, PaginatedRevisions, Revision};
use crate::repository::{RevisionRepository, new_revision_id};

pub struct RevisionManager {
    repo: RevisionRepository,
}

impl RevisionManager {
    #[must_use]
    pub fn new(db: Arc<DatabasePool>) -> Self {
        let repo = RevisionRepository::new(db);
        Self { repo }
    }

    /// 为指定 entry 创建新版本快照，`version_number` 自动递增（`MAX+1`）。
    ///
    /// 成功后同步更新 `content_entries.current_version_id` 为新版本 id。
    ///
    /// # Errors
    /// DB 故障 → [`RevisionError::Database`]
    pub async fn create_revision(&self, input: CreateRevisionInput) -> Result<Revision> {
        let next_version = self.repo.max_version(&input.content_entry_id).await? + 1;
        let id = new_revision_id();
        let revision = self
            .repo
            .insert(
                &id,
                &input.content_entry_id,
                next_version,
                &input.snapshot,
                input.change_summary.as_deref(),
                &input.actor_id,
            )
            .await?;
        self.repo
            .update_entry_current_version(&input.content_entry_id, &revision.id)
            .await?;
        Ok(revision)
    }

    /// 分页列出指定 entry 的版本历史（`version_number DESC`）。
    ///
    /// `page` 从 1 开始；`page_size` 为 0 时使用默认值 20。
    ///
    /// # Errors
    /// DB 故障 → [`RevisionError::Database`]
    pub async fn list_revisions(
        &self,
        entry_id: &str,
        page: u64,
        page_size: u64,
    ) -> Result<PaginatedRevisions> {
        let page = page.max(1);
        let page_size = if page_size == 0 { 20 } else { page_size };
        let (data, total) = self.repo.list_by_entry(entry_id, page, page_size).await?;
        Ok(PaginatedRevisions {
            data,
            total,
            page,
            page_size,
        })
    }

    /// 获取指定 entry 的特定版本快照。
    ///
    /// # Errors
    /// - 版本不存在 → [`RevisionError::RevisionNotFound`]
    /// - DB 故障 → [`RevisionError::Database`]
    pub async fn get_revision(&self, entry_id: &str, version_number: i64) -> Result<Revision> {
        self.repo
            .find_by_entry_and_version(entry_id, version_number)
            .await?
            .ok_or_else(|| {
                RevisionError::RevisionNotFound {
                    entry_id: entry_id.to_owned(),
                    version_number,
                }
                .into()
            })
    }

    /// 基于目标版本的快照创建新版本，并将 entry fields 回写为目标快照。
    ///
    /// # Errors
    /// - 目标版本不存在 → [`RevisionError::RevisionNotFound`]
    /// - DB 故障 → [`RevisionError::Database`]
    pub async fn rollback(
        &self,
        entry_id: &str,
        target_version: i64,
        actor_id: &str,
    ) -> Result<Revision> {
        let target = self
            .repo
            .find_by_entry_and_version(entry_id, target_version)
            .await?
            .ok_or_else(|| RevisionError::RevisionNotFound {
                entry_id: entry_id.to_owned(),
                version_number: target_version,
            })?;

        let snapshot = target.snapshot.clone();

        // 用目标快照创建新版本
        let new_revision = self
            .create_revision(CreateRevisionInput {
                content_entry_id: entry_id.to_owned(),
                snapshot: snapshot.clone(),
                actor_id: actor_id.to_owned(),
                change_summary: Some(format!("Rollback to v{target_version}")),
            })
            .await?;

        // 将 entry fields 回写为目标快照
        self.repo.update_entry_fields(entry_id, &snapshot).await?;

        Ok(new_revision)
    }
}
