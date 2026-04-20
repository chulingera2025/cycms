//! cycms-plugin-native —— Native 插件运行时实现。
//!
//! 实现 [`cycms_plugin_manager::PluginRuntime`] 的 Native 变体：通过静态注册的
//! [`cycms_plugin_api::Plugin`] trait 对象或按约定导出工厂函数的 `.so` 动态库完成
//! 插件 `on_enable / on_disable` 调度，
//! 并负责：
//! - 将 [`cycms_plugin_api::Plugin::event_handlers`] 订阅到 [`cycms_events::EventBus`]
//! - 将 [`cycms_plugin_api::Plugin::services`] 注册到 [`cycms_plugin_api::ServiceRegistry`]
//! - 收集 [`cycms_plugin_api::Plugin::routes`] 供 `ApiGateway`（任务 18）合并到主路由表
//!
//! 运行时优先使用宿主预注册的静态插件实例；若未命中，则回退到按 manifest `entry`
//! 指向的 `.so` 文件进行动态加载。
//! 当前动态库路径保证生命周期钩子可用；复杂路由、服务和事件处理器仍建议走静态注册。

mod runtime;

pub use runtime::NativePluginRuntime;
