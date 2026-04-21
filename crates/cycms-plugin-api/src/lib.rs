//! cycms-plugin-api —— 插件 API 边界。
//!
//! 对外暴露：
//! - [`ServiceRegistry`]：进程内服务发现门面
//! - [`PluginContext`]：宿主注入给插件的能力集合
//! - [`Plugin`] trait：Native 插件接口
//!
//! 下游依赖：
//! - `PluginManager`：负责生命周期编排
//! - `NativePluginRuntime`：负责 trait 对象调度与 services 批量注册
//! - API Gateway：基于 `Plugin::routes` 动态挂载

mod context;
mod error;
mod native;
mod plugin;
mod registry;

pub use context::PluginContext;
pub use error::RegistryError;
pub use native::{NATIVE_PLUGIN_CREATE_SYMBOL, NATIVE_PLUGIN_CREATE_SYMBOL_NAME};
#[doc(hidden)]
pub use native::{into_exported_dynamic_plugin, into_ffi_plugin_ptr};
pub use plugin::{Plugin, PluginRouteDoc};
pub use registry::ServiceRegistry;
