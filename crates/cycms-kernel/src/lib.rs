use std::path::{Path, PathBuf};
use std::sync::Arc;

use cycms_config::AppConfig;
use cycms_core::Result;

// TODO!!!: 任务 2+ 各字段替换为真实子系统类型（ConfigManager、DatabasePool 等）

/// 全局应用上下文，Kernel bootstrap 后在所有组件间共享。
#[non_exhaustive]
pub struct AppContext {
    /// 任务 2：真实应用配置对象。
    pub config: Arc<AppConfig>,
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
    config: AppConfig,
    config_path: Option<PathBuf>,
}

impl Kernel {
    /// 从配置文件路径构建 [`Kernel`] 实例。
    ///
    /// # Errors
    /// 配置文件读取或解析失败时返回错误。
    #[allow(clippy::unused_async)]
    pub async fn build(config_path: Option<&Path>) -> Result<Self> {
        let config = AppConfig::load(config_path)?;
        Ok(Self {
            config,
            config_path: config_path.map(Path::to_path_buf),
        })
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
        Ok(AppContext {
            config: Arc::new(self.config.clone()),
            db: Arc::new(PlaceholderService),
            event_bus: Arc::new(PlaceholderService),
            service_registry: Arc::new(PlaceholderService),
            plugin_manager: Arc::new(PlaceholderService),
            settings_manager: Arc::new(PlaceholderService),
            content_model_registry: Arc::new(PlaceholderService),
        })
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
