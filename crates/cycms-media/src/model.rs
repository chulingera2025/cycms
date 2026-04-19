use chrono::{DateTime, Utc};
use serde_json::Value;

/// 媒体资产实体，对应 `media_assets` 表。
#[derive(Debug, Clone)]
pub struct MediaAsset {
    pub id: String,
    /// 经过安全处理的存储文件名（去除路径分隔符等特殊字符）。
    pub filename: String,
    /// 用户上传时的原始文件名。
    pub original_filename: String,
    pub mime_type: String,
    /// 文件大小，单位字节。
    pub size: i64,
    /// 在存储后端中的相对路径，如 `2026/04/{uuid}.jpg`。
    pub storage_path: String,
    /// 可选的 JSON 元数据（如 EXIF、alt text 等）。
    pub metadata: Option<Value>,
    pub uploaded_by: String,
    pub created_at: DateTime<Utc>,
}

/// 文件上传输入参数。
pub struct UploadInput {
    /// 用户上传时的原始文件名。
    pub original_filename: String,
    /// 文件二进制内容。
    pub data: Vec<u8>,
    /// 可选的 MIME 类型；若为 None，则通过 magic bytes 自动检测。
    pub mime_type: Option<String>,
    pub uploaded_by: String,
    /// 可选的额外元数据（如 alt text）。
    pub metadata: Option<Value>,
}

/// 媒体资产列表查询参数。
#[derive(Debug, Default)]
pub struct MediaQuery {
    /// 精确匹配 MIME 类型，如 `"image/jpeg"`。
    pub mime_type: Option<String>,
    /// 文件名模糊匹配（在 `filename` 和 `original_filename` 中搜索）。
    pub filename_contains: Option<String>,
    /// 精确匹配上传用户 ID。
    pub uploaded_by: Option<String>,
    /// 上传时间下界（含）。
    pub created_after: Option<DateTime<Utc>>,
    /// 上传时间上界（含）。
    pub created_before: Option<DateTime<Utc>>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub order_dir: MediaOrderDir,
}

/// 列表排序方向。
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub enum MediaOrderDir {
    Asc,
    #[default]
    Desc,
}

/// 媒体资产分页响应。
#[derive(Debug)]
pub struct PaginatedMedia {
    pub data: Vec<MediaAsset>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
    pub page_count: u64,
}

/// 删除策略：被引用资产的处理方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaDeletePolicy {
    /// 有引用时返回错误，拒绝删除。
    Block,
    /// 有引用时仅记录警告，仍然执行删除。
    Warn,
}

impl MediaDeletePolicy {
    pub fn from_config_str(s: &str) -> Self {
        if s.eq_ignore_ascii_case("warn") {
            Self::Warn
        } else {
            Self::Block
        }
    }
}
