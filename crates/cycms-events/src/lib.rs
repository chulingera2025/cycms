//! cycms-events —— 进程内异步事件总线。
//!
//! [`EventBus`] 按 [`EventKind`] 分桶广播，handler 独立后台任务消费，
//! 单 handler 失败/超时不阻断其他订阅者。集成点：
//! - ContentEngine：发布 `content.*` 事件
//! - PublishManager：发布 `content.published/unpublished`
//! - PluginManager：发布 `plugin.*` 事件
//! - 插件运行时：注册 [`EventHandler`] 订阅
//! - AuditLogger：订阅关键事件写审计日志

mod bus;
mod error;
mod event;
mod handler;

pub use bus::{DEFAULT_CHANNEL_CAPACITY, DEFAULT_HANDLER_TIMEOUT, EventBus};
pub use error::EventError;
pub use event::{Event, EventKind};
pub use handler::{EventHandler, SubscriptionHandle, SubscriptionId};
