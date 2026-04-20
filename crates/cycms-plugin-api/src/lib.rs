//! cycms-plugin-api —— 插件 API 边界。
//!
//! 任务 9 产出：
//! - [`ServiceRegistry`]：进程内服务发现门面（Req 13.1 / 13.2 / 13.3）
//! - [`PluginContext`]：宿主注入给插件的能力集合
//! - [`Plugin`] trait：Native 插件接口（Req 11.x 的 API 侧定义）
//!
//! 下游依赖：
//! - 任务 15 PluginManager：负责生命周期编排
//! - 任务 16 NativePluginRuntime：负责 trait 对象调度与 services 批量注册
//! - 任务 18 API Gateway：基于 `Plugin::routes` 动态挂载

mod context;
mod error;
mod native;
mod plugin;
mod registry;

pub use context::PluginContext;
pub use error::RegistryError;
#[doc(hidden)]
pub use native::{into_exported_dynamic_plugin, into_ffi_plugin_ptr};
pub use native::{NATIVE_PLUGIN_CREATE_SYMBOL, NATIVE_PLUGIN_CREATE_SYMBOL_NAME};
pub use plugin::{Plugin, PluginRouteDoc};
pub use registry::ServiceRegistry;
