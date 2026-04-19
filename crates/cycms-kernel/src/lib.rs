use std::path::{Path, PathBuf};
use std::sync::Arc;

use cycms_auth::AuthEngine;
use cycms_config::AppConfig;
use cycms_core::Result;
use cycms_db::DatabasePool;
use cycms_events::EventBus;
use cycms_migrate::MigrationEngine;
use cycms_permission::PermissionEngine;
use cycms_plugin_api::ServiceRegistry;
use cycms_settings::SettingsManager;

// TODO!!!: 任务 10/15 余下占位字段逐步替换为真实子系统类型（PluginManager、ContentModelRegistry）

/// 全局应用上下文，Kernel bootstrap 后在所有组件间共享。
#[non_exhaustive]
pub struct AppContext {
    /// 任务 2：真实应用配置对象。
    pub config: Arc<AppConfig>,
    /// 任务 3：多方言数据库连接池。
    pub db: Arc<DatabasePool>,
    /// 任务 5：认证引擎，提供登录/刷新/初始管理员/Token 校验等能力。
    pub auth_engine: Arc<AuthEngine>,
    /// 任务 6：权限引擎，提供角色/权限 CRUD 与 `check_permission` 判定。
    pub permission_engine: Arc<PermissionEngine>,
    /// 任务 7：进程内异步事件总线，按 `EventKind` 广播订阅者。
    pub event_bus: Arc<EventBus>,
    /// 任务 8:系统与插件设置的统一访问门面。
    pub settings_manager: Arc<SettingsManager>,
    /// 任务 9：插件间服务发现与调用门面。
    pub service_registry: Arc<ServiceRegistry>,
    /// 占位：任务 15 替换为 `Arc<PluginManager>`
    pub plugin_manager: Arc<PlaceholderService>,
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
    /// 初始化顺序：Config → DB → Migration → Auth → Permission → `EventBus` →
    /// `ServiceRegistry` → `PluginManager` → `ContentModel` → API
    ///
    /// 当 `system_migrations_dir` 为 `Some` 时会执行系统迁移；传 `None` 跳过，适合只
    /// 想构造上下文做诊断（例如 `cycms config show`）的调用方。
    ///
    /// # Errors
    /// 任意子系统初始化失败时返回错误。
    pub async fn bootstrap(&self, system_migrations_dir: Option<&Path>) -> Result<AppContext> {
        let db = Arc::new(DatabasePool::connect(&self.config.database).await?);

        if let Some(dir) = system_migrations_dir {
            MigrationEngine::new(Arc::clone(&db))
                .run_system_migrations(dir)
                .await?;
        }

        let auth_engine = Arc::new(AuthEngine::new(Arc::clone(&db), self.config.auth.clone())?);
        let permission_engine = Arc::new(PermissionEngine::new(Arc::clone(&db)));
        let event_bus = Arc::new(EventBus::new());
        let settings_manager = Arc::new(SettingsManager::new(Arc::clone(&db)));
        let service_registry = Arc::new(ServiceRegistry::new());
        register_core_services(
            &service_registry,
            &db,
            &auth_engine,
            &permission_engine,
            &event_bus,
            &settings_manager,
        )?;

        Ok(AppContext {
            config: Arc::new(self.config.clone()),
            db,
            auth_engine,
            permission_engine,
            event_bus,
            settings_manager,
            service_registry,
            plugin_manager: Arc::new(PlaceholderService),
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

/// 启动期把核心子系统注册到 `ServiceRegistry`，供插件通过
/// `{plugin_name}.{service_name}` 约定查询（对齐 Req 13.1）。
///
/// 核心子系统统一使用 `system` 作为 plugin 段，service 段沿用子系统约定名。
fn register_core_services(
    registry: &ServiceRegistry,
    db: &Arc<DatabasePool>,
    auth_engine: &Arc<AuthEngine>,
    permission_engine: &Arc<PermissionEngine>,
    event_bus: &Arc<EventBus>,
    settings_manager: &Arc<SettingsManager>,
) -> Result<()> {
    registry.register("system.db", Arc::clone(db))?;
    registry.register("system.auth", Arc::clone(auth_engine))?;
    registry.register("system.permission", Arc::clone(permission_engine))?;
    registry.register("system.events", Arc::clone(event_bus))?;
    registry.register("system.settings", Arc::clone(settings_manager))?;
    Ok(())
}
