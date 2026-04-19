use std::sync::Arc;

use chrono::Utc;
use cycms_config::MediaConfig;
use cycms_db::DatabasePool;
use cycms_events::{Event, EventBus, EventKind};
use serde_json::json;
use tracing::warn;
use uuid::Uuid;

use crate::error::MediaError;
use crate::model::{MediaAsset, MediaDeletePolicy, MediaQuery, PaginatedMedia, UploadInput};
use crate::repository::MediaAssetRepository;
use crate::storage::{LocalStorageBackend, StorageBackend};

/// 默认分页大小。
const DEFAULT_PAGE_SIZE: u64 = 20;
/// 最大允许分页大小。
const MAX_PAGE_SIZE: u64 = 100;

/// 媒体资产管理门面：上传、查询、删除，集成存储后端与事件总线。
pub struct MediaManager {
    repo: MediaAssetRepository,
    storage: Arc<dyn StorageBackend>,
    event_bus: Arc<EventBus>,
    delete_policy: MediaDeletePolicy,
    max_file_size: u64,
    allowed_mime_types: Vec<String>,
}

impl MediaManager {
    /// 以 `LocalStorageBackend` 构造 `MediaManager`。
    pub fn new(db: &Arc<DatabasePool>, event_bus: Arc<EventBus>, config: &MediaConfig) -> Self {
        let storage = Arc::new(LocalStorageBackend::new(
            config.upload_dir.clone(),
            "/uploads",
        ));
        Self {
            repo: MediaAssetRepository::new(Arc::clone(db)),
            storage,
            event_bus,
            delete_policy: MediaDeletePolicy::from_config_str(&config.on_referenced_delete),
            max_file_size: config.max_file_size,
            allowed_mime_types: config.allowed_mime_types.clone(),
        }
    }

    /// 上传文件：校验大小与 MIME 类型 → 生成存储路径 → 落盘 → 写 DB → 发布事件。
    ///
    /// # Errors
    /// 文件过大、MIME 类型不允许、存储或 DB 错误时返回相应错误。
    pub async fn upload(&self, input: UploadInput) -> Result<MediaAsset, MediaError> {
        let data_len = input.data.len() as u64;
        if data_len > self.max_file_size {
            return Err(MediaError::FileTooLarge {
                size: data_len,
                limit: self.max_file_size,
            });
        }

        let mime_type = input.mime_type.unwrap_or_else(|| detect_mime(&input.data));

        if !self.allowed_mime_types.is_empty() && !self.allowed_mime_types.contains(&mime_type) {
            return Err(MediaError::DisallowedMimeType(mime_type));
        }

        let id = Uuid::new_v4().to_string();
        let ext = mime_to_ext(&mime_type);
        let now = Utc::now();
        let storage_key = format!("{}/{}/{id}.{ext}", now.format("%Y"), now.format("%m"));
        let filename = sanitize_filename(&input.original_filename);

        self.storage.store(&storage_key, &input.data).await?;

        let asset = MediaAsset {
            id: id.clone(),
            filename,
            original_filename: input.original_filename,
            mime_type: mime_type.clone(),
            size: i64::try_from(input.data.len()).unwrap_or(i64::MAX),
            storage_path: storage_key,
            metadata: input.metadata,
            uploaded_by: input.uploaded_by.clone(),
            created_at: now,
        };

        self.repo.insert(&asset).await?;

        // 读回以获取 DB 生成的 created_at（SQLite DEFAULT 可能与 Rust 时间略有差异）
        let stored = self.repo.find_by_id(&id).await?.unwrap_or(asset);

        self.event_bus.publish(
            Event::new(EventKind::MediaUploaded)
                .with_actor(&input.uploaded_by)
                .with_payload(json!({
                    "id": id,
                    "filename": stored.filename,
                    "mime_type": mime_type,
                    "size": stored.size,
                })),
        );

        Ok(stored)
    }

    /// 按 ID 查询媒体资产。
    ///
    /// # Errors
    /// DB 错误时返回。
    pub async fn get(&self, id: &str) -> Result<Option<MediaAsset>, MediaError> {
        self.repo.find_by_id(id).await
    }

    /// 带过滤与分页的媒体资产列表查询。
    ///
    /// # Errors
    /// DB 错误时返回。
    pub async fn list(&self, query: &MediaQuery) -> Result<PaginatedMedia, MediaError> {
        self.repo
            .list(query, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)
            .await
    }

    /// 删除媒体资产：先检查引用 → 根据删除策略决定是否继续 → 删除存储文件 → 删除 DB 记录 → 发布事件。
    ///
    /// # Errors
    /// 资产不存在、引用检查失败（Block 策略）或存储/DB 错误时返回。
    pub async fn delete(&self, id: &str) -> Result<(), MediaError> {
        let asset = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| MediaError::AssetNotFound(id.to_owned()))?;

        let ref_count = self.repo.count_references(id).await?;
        if ref_count > 0 {
            match self.delete_policy {
                MediaDeletePolicy::Block => {
                    return Err(MediaError::ReferencedAsset(id.to_owned(), ref_count));
                }
                MediaDeletePolicy::Warn => {
                    warn!(
                        asset_id = id,
                        ref_count, "deleting referenced media asset (warn policy)"
                    );
                }
            }
        }

        self.storage.delete(&asset.storage_path).await?;
        self.repo.delete(id).await?;

        self.event_bus.publish(
            Event::new(EventKind::MediaDeleted)
                .with_actor(&asset.uploaded_by)
                .with_payload(json!({
                    "id": id,
                    "storage_path": asset.storage_path,
                })),
        );

        Ok(())
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────

/// 通过 magic bytes 检测 MIME 类型；失败时回退到 `application/octet-stream`。
fn detect_mime(data: &[u8]) -> String {
    infer::get(data).map_or_else(
        || "application/octet-stream".to_owned(),
        |t| t.mime_type().to_owned(),
    )
}

/// 将 MIME 类型映射到常用文件扩展名。
fn mime_to_ext(mime: &str) -> &'static str {
    match mime {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/svg+xml" => "svg",
        "application/pdf" => "pdf",
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        "audio/mpeg" => "mp3",
        "audio/ogg" => "ogg",
        "text/plain" => "txt",
        "application/zip" => "zip",
        _ => "bin",
    }
}

/// 从原始文件名中提取安全文件名（去除路径分隔符，保留字母数字、连字符、下划线、点）。
fn sanitize_filename(name: &str) -> String {
    let base = std::path::Path::new(name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("upload");
    base.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
