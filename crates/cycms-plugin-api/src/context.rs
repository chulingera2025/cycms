use std::sync::Arc;

use cycms_auth::AuthEngine;
use cycms_db::DatabasePool;
use cycms_events::EventBus;
use cycms_permission::PermissionEngine;
use cycms_settings::SettingsManager;

use crate::registry::ServiceRegistry;

/// 插件执行时由宿主注入的只读能力集合。
///
/// v0.1 仅包含已实现的核心子系统；content / plugin-manager 等后续任务的字段留作
/// 扩展位，以下 TODO 标注对应任务编号以防被误删或提前实现：
///
/// - TODO!!!: 任务 10 增加 `content_model_registry: Arc<ContentModelRegistry>`
/// - TODO!!!: 任务 11 增加 `content_engine: Arc<ContentEngine>`
/// - TODO!!!: 任务 15 增加 `plugin_manager: Arc<PluginManager>`（仅管理能力暴露子集）
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
    /// 插件间服务发现与调用。
    pub service_registry: Arc<ServiceRegistry>,
}

impl PluginContext {
    /// 组合现有核心子系统构造 `PluginContext`。
    #[must_use]
    pub fn new(
        db_pool: Arc<DatabasePool>,
        auth_engine: Arc<AuthEngine>,
        permission_engine: Arc<PermissionEngine>,
        event_bus: Arc<EventBus>,
        settings_manager: Arc<SettingsManager>,
        service_registry: Arc<ServiceRegistry>,
    ) -> Self {
        Self {
            db_pool,
            auth_engine,
            permission_engine,
            event_bus,
            settings_manager,
            service_registry,
        }
    }
}
