//! `cycms-plugin-manager` —— 插件生命周期与 manifest 管理（任务 15）。
//!
//! 覆盖 Requirements：
//! - R10 插件管理：安装 / 启用 / 禁用 / 卸载 / 列表
//! - R20 插件 Manifest 规范：`plugin.toml` 解析与校验
//!
//! 本 crate 是连接 **manifest 声明 → 数据库插件记录 → Native/Wasm 运行时** 的编排层。
//! 为让本 crate 独立于任务 16 / 17 推进，`PluginRuntime` 抽象定义在本 crate 内，
//! `cycms-plugin-native` / `cycms-plugin-wasm` 反向依赖并实现该 trait，避免循环。

mod discovery;
mod error;
mod manifest;
mod model;
mod repository;
mod resolver;
mod runtime;
mod service;

pub use discovery::{DiscoveredPlugin, discover_plugin_dir, scan_plugins_dir};
pub use error::PluginManagerError;
pub use manifest::{
    CompatibilitySpec, DependencySpec, FrontendSpec, PermissionEntry, PermissionsSpec, PluginKind,
    PluginManifest, PluginMeta,
};
pub use model::{NewPluginRow, PluginRecord, PluginStatus};
pub use repository::PluginRepository;
pub use resolver::{check_host_compatibility, reverse_dependencies, topological_order};
pub use runtime::PluginRuntime;
pub use service::{PluginInfo, PluginManager, PluginManagerConfig};
