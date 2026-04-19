use std::path::{Path, PathBuf};
use std::sync::Arc;

use cycms_auth::AuthEngine;
use cycms_config::AppConfig;
use cycms_content_engine::ContentEngine;
use cycms_content_model::{ContentModelRegistry, FieldTypeRegistry, seed_default_types};
use cycms_core::Result;
use cycms_db::DatabasePool;
use cycms_events::EventBus;
use cycms_media::MediaManager;
use cycms_migrate::MigrationEngine;
use cycms_permission::PermissionEngine;
use cycms_plugin_api::ServiceRegistry;
use cycms_publish::PublishManager;
use cycms_revision::RevisionManager;
use cycms_settings::SettingsManager;

// TODO!!!: 任务 15 余下占位字段替换为真实子系统类型（`PluginManager`）

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
    /// 任务 10：内容类型管理与字段校验 / Schema 输出门面。
    pub content_model: Arc<ContentModelRegistry>,
    /// 任务 11：内容实例 CRUD + 查询引擎 + `EventBus` 集成门面。
    pub content_engine: Arc<ContentEngine>,
    /// 任务 12：内容版本快照与回滚门面。
    pub revision_manager: Arc<RevisionManager>,
    /// 任务 13：发布状态机门面（Draft → Published / Published → Draft）。
    pub publish_manager: Arc<PublishManager>,
    /// 任务 14：媒体资产管理门面（上传/查询/删除）。
    pub media_manager: Arc<MediaManager>,
    /// 占位：任务 15 替换为 `Arc<PluginManager>`
    pub plugin_manager: Arc<PlaceholderService>,
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
    /// `ServiceRegistry` → `ContentModel` → `RevisionManager` → `ContentEngine` → API
    ///
    /// 当 `system_migrations_dir` 为 `Some` 时会执行系统迁移并注入默认 `page` / `post`
    /// 内容类型；传 `None` 跳过迁移与 seed，适合只想构造上下文做诊断的调用方。
    ///
    /// # Errors
    /// 任意子系统初始化失败时返回错误。
    pub async fn bootstrap(&self, system_migrations_dir: Option<&Path>) -> Result<AppContext> {
        let db = Arc::new(DatabasePool::connect(&self.config.database).await?);

        let migrations_applied = system_migrations_dir.is_some();
        if let Some(dir) = system_migrations_dir {
            MigrationEngine::new(Arc::clone(&db))
                .run_system_migrations(dir)
                .await?;
        }

        let auth_engine = Arc::new(AuthEngine::new(Arc::clone(&db), self.config.auth.clone())?);
        let permission_engine = Arc::new(PermissionEngine::new(Arc::clone(&db)));
        let event_bus = Arc::new(EventBus::new());
        let settings_manager = Arc::new(SettingsManager::new(Arc::clone(&db)));
        let field_type_registry = Arc::new(FieldTypeRegistry::new());
        let content_model = Arc::new(ContentModelRegistry::new(
            Arc::clone(&db),
            Arc::clone(&field_type_registry),
        ));
        if migrations_applied {
            seed_default_types(&content_model).await?;
        }
        let service_registry = Arc::new(ServiceRegistry::new());
        let revision_manager = Arc::new(RevisionManager::new(Arc::clone(&db)));
        let publish_manager = Arc::new(PublishManager::new(&db, Arc::clone(&event_bus)));
        let media_manager = Arc::new(MediaManager::new(
            &db,
            Arc::clone(&event_bus),
            &self.config.media,
        ));
        let content_engine = Arc::new(ContentEngine::new(
            Arc::clone(&db),
            Arc::clone(&content_model),
            Arc::clone(&event_bus),
            self.config.content.clone(),
            Arc::clone(&revision_manager),
        ));
        register_core_services(
            &service_registry,
            &db,
            &auth_engine,
            &permission_engine,
            &event_bus,
            &settings_manager,
            &content_model,
            &content_engine,
            &revision_manager,
            &publish_manager,
            &media_manager,
        )?;

        Ok(AppContext {
            config: Arc::new(self.config.clone()),
            db,
            auth_engine,
            permission_engine,
            event_bus,
            settings_manager,
            service_registry,
            content_model,
            content_engine,
            revision_manager,
            publish_manager,
            media_manager,
            plugin_manager: Arc::new(PlaceholderService),
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
#[allow(clippy::too_many_arguments)]
fn register_core_services(
    registry: &ServiceRegistry,
    db: &Arc<DatabasePool>,
    auth_engine: &Arc<AuthEngine>,
    permission_engine: &Arc<PermissionEngine>,
    event_bus: &Arc<EventBus>,
    settings_manager: &Arc<SettingsManager>,
    content_model: &Arc<ContentModelRegistry>,
    content_engine: &Arc<ContentEngine>,
    revision_manager: &Arc<RevisionManager>,
    publish_manager: &Arc<PublishManager>,
    media_manager: &Arc<MediaManager>,
) -> Result<()> {
    registry.register("system.db", Arc::clone(db))?;
    registry.register("system.auth", Arc::clone(auth_engine))?;
    registry.register("system.permission", Arc::clone(permission_engine))?;
    registry.register("system.events", Arc::clone(event_bus))?;
    registry.register("system.settings", Arc::clone(settings_manager))?;
    registry.register("system.content_model", Arc::clone(content_model))?;
    registry.register("system.content_engine", Arc::clone(content_engine))?;
    registry.register("system.media", Arc::clone(media_manager))?;
    registry.register("system.revision", Arc::clone(revision_manager))?;
    registry.register("system.publish", Arc::clone(publish_manager))?;
    Ok(())
}
