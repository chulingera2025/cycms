//! cycms-content-engine —— 内容实例 CRUD / 查询引擎 / 事件集成（任务 11）。
//!
//! 覆盖 Requirements 4.1 – 4.7：
//! - [`ContentEntry`] 持久化到 `content_entries` 表；
//! - create/update 时委托 `ContentModelRegistry::validate_entry` 做字段级校验；
//! - 查询引擎支持分页 / 排序 / 13 种操作符 / 单层 populate；
//! - CRUD 后通过 `EventBus` 发布 `content.{created,updated,deleted}` 事件；
//! - 删除路径检查 `content_relations` 反向引用并按 `ContentConfig.default_delete_mode`
//!   切换软/硬删除。
//!
//! 模块结构（按 Step 分批落地）：
//! - [`error`]：`ContentEngineError` + 跨 crate 映射；
//! - [`model`]：`ContentEntry` / `ContentStatus` / 分页响应等数据结构；
//! - TODO!!!: Step 3 加入 `repository`（三方言 CRUD）；
//! - TODO!!!: Step 4 加入 `query`（筛选 / 排序 / 分页）；
//! - TODO!!!: Step 5 加入 `populate`（单层关联加载）；
//! - TODO!!!: Step 6/7 加入 `service`（`ContentEngine` 门面 + `EventBus` 集成）。

mod error;
mod model;

pub use error::{ContentEngineError, ReferenceViolation};
pub use model::{
    ContentEntry, ContentStatus, CreateEntryInput, PaginatedResponse, PaginationMeta,
    UpdateEntryInput,
};
