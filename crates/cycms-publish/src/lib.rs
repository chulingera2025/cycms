//! 内容发布状态机（任务 13）。
//!
//! 提供 [`PublishManager`]，实现 Draft → Published / Published → Draft
//! 状态转换及 `content.published` / `content.unpublished` 事件发布。

mod error;
mod repository;
mod service;

pub use error::PublishError;
pub use service::PublishManager;
