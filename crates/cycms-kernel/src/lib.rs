use std::path::{Path, PathBuf};
use std::sync::Arc;

use cycms_core::Result;

// TODO!!!: 任务 2+ 各字段替换为真实子系统类型（ConfigManager、DatabasePool 等）

/// 全局应用上下文，Kernel bootstrap 后在所有组件间共享。
#[non_exhaustive]
pub struct AppContext {
    /// 应用配置路径，任务 2 起替换为 `Arc<AppConfig>`
    pub config_path: Option<PathBuf>,
    /// 占位：任务 3 替换为 `Arc<DatabasePool>`
    pub db: Arc<PlaceholderService>,
    /// 占位：任务 7 替换为 `Arc<EventBus>`
    pub event_bus: Arc<PlaceholderService>,
    /// 占位：任务 9 替换为 `Arc<ServiceRegistry>`
    pub service_registry: Arc<PlaceholderService>,
    /// 占位：任务 15 替换为 `Arc<PluginManager>`
    pub plugin_manager: Arc<PlaceholderService>,
    /// 占位：任务 8 替换为 `Arc<SettingsManager>`
    pub settings_manager: Arc<PlaceholderService>,
    /// 占位：任务 10 替换为 `Arc<ContentModelRegistry>`
    pub content_model_registry: Arc<PlaceholderService>,
}

/// 各子系统实现前的临时占位类型，任务 2–21 逐步替换。
pub struct PlaceholderService;

/// 应用生命周期管理入口。
#[allow(dead_code)]
pub struct Kernel {
    config_path: Option<PathBuf>,
}

impl Kernel {
    /// 从配置文件路径构建 [`Kernel`] 实例。
    ///
    /// # Errors
    /// 配置文件读取或解析失败时返回错误。
    #[allow(clippy::unused_async)]
    pub async fn build(_config_path: Option<&Path>) -> Result<Self> {
        // TODO!!!: 任务 2 实现配置加载（AppConfig::load）
        todo!("TODO!!!: 任务 2 实现配置加载")
    }

    /// 初始化所有子系统并返回 [`AppContext`]。
    ///
    /// 初始化顺序：Config → DB → Migration → `EventBus` →
    /// `ServiceRegistry` → `PluginManager` → `ContentModel` → Auth → Permission → API
    ///
    /// # Errors
    /// 任意子系统初始化失败时返回错误。
    #[allow(clippy::unused_async)]
    pub async fn bootstrap(&self) -> Result<AppContext> {
        // TODO!!!: 任务 2–9 依次实现各子系统初始化
        todo!("TODO!!!: 任务 2–9 实现各子系统初始化")
    }

    /// 启动 HTTP 服务器，阻塞直至收到关闭信号。
    ///
    /// # Errors
    /// 端口绑定失败或运行时错误时返回错误。
    #[allow(clippy::unused_async)]
    pub async fn serve(self) -> Result<()> {
        // TODO!!!: 任务 18 实现 axum HTTP 服务器启动
        todo!("TODO!!!: 任务 18 实现 HTTP 服务启动")
    }

    /// 优雅关闭所有子系统。
    ///
    /// # Errors
    /// 关闭过程中出现不可恢复错误时返回错误。
    #[allow(clippy::unused_async)]
    pub async fn shutdown(&self, _ctx: &AppContext) -> Result<()> {
        // TODO!!!: 任务 18 实现优雅关闭逻辑
        todo!("TODO!!!: 任务 18 实现优雅关闭")
    }
}
