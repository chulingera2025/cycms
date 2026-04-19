//! 插件运行时抽象。
//!
//! 本 trait 由 `cycms-plugin-manager` 自己持有，`cycms-plugin-native`（任务 16）
//! 与 `cycms-plugin-wasm`（任务 17）反向依赖本 crate 实现该 trait，使 Manager 得以
//! 在不引入具体运行时类型的前提下完成生命周期编排。

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use cycms_core::Result;
use cycms_plugin_api::PluginContext;

use crate::manifest::{PluginKind, PluginManifest};

/// 插件运行时 trait：承担「加载 / 卸载 + 宿主能力接入」的职责。
///
/// 契约约定：
/// - [`PluginRuntime::load`] 成功返回意味着：插件 `on_enable` 已被调用，
///   `event_handlers` 已接入 `EventBus`，`services` 已注册到 `ServiceRegistry`，
///   `routes` 已收集待合并到主路由表。
/// - [`PluginRuntime::unload`] 成功返回意味着：`on_disable` 已被调用，
///   事件订阅 / 服务注册 / 路由绑定均已清理。
/// - 多次 `load` 同一个插件 name 需保持幂等或返回错误；实现方自行约定。
#[async_trait]
pub trait PluginRuntime: Send + Sync {
    /// 本运行时负责的插件类型，用作 `PluginManager` 按 `kind` 路由的 key。
    fn kind(&self) -> PluginKind;

    /// 加载并启用插件：读取 `entry_path` 所指实现、注入 `ctx`、调用 `on_enable`。
    ///
    /// # Errors
    /// 编译 / 实例化 / `on_enable` 返回错误时向上抛出。
    async fn load(
        &self,
        manifest: &PluginManifest,
        entry_path: &Path,
        ctx: Arc<PluginContext>,
    ) -> Result<()>;

    /// 卸载插件：调用 `on_disable`、释放运行时资源。
    ///
    /// # Errors
    /// `on_disable` 返回错误时向上抛出，但 Manager 仍会继续清理数据库侧状态。
    async fn unload(&self, plugin_name: &str) -> Result<()>;

    /// 返回运行时当前持有的插件 name 列表，诊断 / 幂等检查用。
    fn loaded_plugins(&self) -> Vec<String>;
}
