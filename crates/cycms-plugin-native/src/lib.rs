//! cycms-plugin-native —— Native 插件运行时实现。
//!
//! 实现 [`cycms_plugin_manager::PluginRuntime`] 的 Native 变体：通过静态注册的
//! [`cycms_plugin_api::Plugin`] trait 对象完成插件 `on_enable / on_disable` 调度，
//! 并负责：
//! - 将 [`cycms_plugin_api::Plugin::event_handlers`] 订阅到 [`cycms_events::EventBus`]
//! - 将 [`cycms_plugin_api::Plugin::services`] 注册到 [`cycms_plugin_api::ServiceRegistry`]
//! - 收集 [`cycms_plugin_api::Plugin::routes`] 供 `ApiGateway`（任务 18）合并到主路由表
//!
//! v0.1 不做真正的 `libloading` 动态加载：Native 插件以普通 Rust crate 形式编译进宿主，
//! 启动前由 Kernel / CLI 通过 [`NativePluginRuntime::register_plugin`] 交付 `Arc<dyn Plugin>`。
//! TODO!!! v0.2 引入基于 `libloading` 的 `.so` 动态加载。

mod runtime;

pub use runtime::NativePluginRuntime;
