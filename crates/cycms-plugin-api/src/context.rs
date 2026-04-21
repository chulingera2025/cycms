use std::sync::Arc;

use cycms_auth::AuthEngine;
use cycms_content_engine::ContentEngine;
use cycms_content_model::ContentModelRegistry;
use cycms_db::DatabasePool;
use cycms_events::EventBus;
use cycms_media::MediaManager;
use cycms_permission::PermissionEngine;
use cycms_publish::PublishManager;
use cycms_revision::RevisionManager;
use cycms_settings::SettingsManager;

use crate::registry::ServiceRegistry;

/// 插件执行时由宿主注入的只读能力集合。
///
/// 当前仅包含插件运行期真正需要的核心子系统；插件生命周期管理继续由宿主统一编排，
/// 不直接向插件开放 `PluginManager` 以避免越过宿主侧状态机与权限边界。
///
/// 该结构体应按引用 (`&PluginContext`) 传给插件生命周期钩子与请求处理路径，
/// 禁止在插件侧持久持有内部字段的所有权，防止绕过 `ServiceRegistry` 的可用性控制。
#[non_exhaustive]
pub struct PluginContext {
    /// 多方言数据库连接池（用于插件 migration、直接 SQL 场景）。
    pub db_pool: Arc<DatabasePool>,
    /// 认证引擎：登录/Token/初始管理员。
    pub auth_engine: Arc<AuthEngine>,
    /// 权限引擎：角色与 `check_permission` 判定。
    pub permission_engine: Arc<PermissionEngine>,
    /// 事件总线：发布/订阅进程内异步事件。
    pub event_bus: Arc<EventBus>,
    /// 系统与插件设置的统一访问门面。
    pub settings_manager: Arc<SettingsManager>,
    /// 内容类型注册表：插件可读取现有类型、注册自定义字段类型（Req 3.6）。
    pub content_model: Arc<ContentModelRegistry>,
    /// 内容实例引擎：插件可执行 CRUD / 查询 / 删除。
    pub content_engine: Arc<ContentEngine>,
    /// 内容版本快照与回滚门面。
    pub revision_manager: Arc<RevisionManager>,
    /// 发布状态机门面。
    pub publish_manager: Arc<PublishManager>,
    /// 媒体资产管理门面。
    pub media_manager: Arc<MediaManager>,
    /// 插件间服务发现与调用。
    pub service_registry: Arc<ServiceRegistry>,
}

impl PluginContext {
    /// 组合现有核心子系统构造 `PluginContext`。
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        db_pool: Arc<DatabasePool>,
        auth_engine: Arc<AuthEngine>,
        permission_engine: Arc<PermissionEngine>,
        event_bus: Arc<EventBus>,
        settings_manager: Arc<SettingsManager>,
        content_model: Arc<ContentModelRegistry>,
        content_engine: Arc<ContentEngine>,
        revision_manager: Arc<RevisionManager>,
        publish_manager: Arc<PublishManager>,
        media_manager: Arc<MediaManager>,
        service_registry: Arc<ServiceRegistry>,
    ) -> Self {
        Self {
            db_pool,
            auth_engine,
            permission_engine,
            event_bus,
            settings_manager,
            content_model,
            content_engine,
            revision_manager,
            publish_manager,
            media_manager,
            service_registry,
        }
    }
}
