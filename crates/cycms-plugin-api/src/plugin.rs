use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use cycms_core::Result;
use cycms_events::{EventHandler, EventKind};

use crate::context::PluginContext;

/// 插件路由文档元数据，供 `ApiGateway` 聚合进 `/api/docs`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginRouteDoc {
    pub path: String,
    pub methods: Vec<String>,
}

/// Native 插件必须实现的 trait。
///
/// 只在 `cycms-plugin-api` 定义 trait 本身与宿主可感知的返回值形状；具体加载、调度、
/// Router 合并、服务批量注册由 `NativePluginRuntime` 完成。
///
/// 所有 handler 都以 `&self` 接收，要求实现方是内部可变/共享状态安全的。
#[async_trait]
pub trait Plugin: Send + Sync {
    /// 插件唯一标识（同时作为 `ServiceRegistry` 键的 plugin 段）。
    fn name(&self) -> &str;

    /// 插件版本，遵循 SemVer（`PluginManager` 的依赖解析以此为输入）。
    fn version(&self) -> &str;

    /// 插件启用时的一次性初始化入口。
    ///
    /// # Errors
    /// 启用失败时返回错误，`PluginManager` 会记录并跳过该插件。
    async fn on_enable(&self, ctx: &PluginContext) -> Result<()>;

    /// 插件禁用时的清理入口。
    ///
    /// # Errors
    /// 清理失败时返回错误，`PluginManager` 记录但继续卸载流程。
    async fn on_disable(&self, ctx: &PluginContext) -> Result<()>;

    /// 插件贡献的 axum 路由（挂载到 `/api/v1/x/{plugin_name}/*`，由 Native 运行时负责合并）。
    fn routes(&self) -> Option<Router> {
        None
    }

    /// 插件对外暴露的路由文档元数据。
    ///
    /// `path` 为插件内部相对路径（例如 `/hello`），最终由 `ApiGateway` 挂载到
    /// `/api/v1/x/{plugin_name}` 前缀下。未提供时文档层会退化为通配描述。
    fn route_docs(&self) -> Vec<PluginRouteDoc> {
        Vec::new()
    }

    /// 插件提供的事件处理器清单，以 `(EventKind, handler)` 对给出。
    ///
    /// `NativePluginRuntime` 在 `on_enable` 之后逐项调用 [`cycms_events::EventBus::subscribe`],
    /// 把返回的 `SubscriptionHandle` 与插件绑定；禁用时统一 `abort`。同一 handler 想订阅
    /// 多个 `EventKind` 时需要在返回列表中重复出现。
    fn event_handlers(&self) -> Vec<(EventKind, Arc<dyn EventHandler>)> {
        Vec::new()
    }

    /// 插件对外暴露的服务列表：`(service_name, Arc<impl Send+Sync+'static>)`。
    ///
    /// Runtime 会按 `{plugin_name}.{service_name}` 组装完整 key 后写入 `ServiceRegistry`
    /// `service_name` 由实现方自行约定，不得包含 `.`。
    fn services(&self) -> Vec<(String, Arc<dyn Any + Send + Sync>)> {
        Vec::new()
    }
}
