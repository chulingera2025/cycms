//! `cycms-plugin-manager` —— 插件生命周期与 manifest 管理。
//!
//! 覆盖 Requirements：
//! - R10 插件管理：安装 / 启用 / 禁用 / 卸载 / 列表
//! - R20 插件 Manifest 规范：`plugin.toml` 解析与校验
//!
//! 本 crate 是连接 **manifest 声明 → 数据库插件记录 → Native/Wasm 运行时** 的编排层。
//! `PluginRuntime` 抽象定义在本 crate 内，`cycms-plugin-native` / `cycms-plugin-wasm`
//! 反向依赖并实现该 trait，避免循环依赖。

mod compiler;
mod discovery;
mod error;
mod frontend_manifest;
mod frontend_snapshot;
mod host_registry;
mod manifest;
mod model;
mod repository;
mod resolver;
mod runtime;
mod service;

pub use compiler::compile_extensions;
pub use discovery::{DiscoveredPlugin, discover_plugin_dir, scan_plugins_dir};
pub use error::PluginManagerError;
pub use frontend_manifest::{
    ADMIN_SHELL_SDK_VERSION, AdminFrontendManifest, ContributionMatchSpec, CustomPageContribution,
    FieldRendererContribution, FrontendAsset, MenuContribution, RouteContribution,
    SettingsContribution, SlotContribution, load_frontend_manifest,
};
pub use frontend_snapshot::{
    AdminExtensionBootstrap, AdminExtensionDiagnostics, BootstrapFieldRendererContribution,
    BootstrapMenuContribution, BootstrapPlugin, BootstrapRouteContribution,
    BootstrapSettingsContribution, BootstrapSettingsPage, BootstrapSlotContribution,
    ExtensionDiagnostic, FrontendCompatibility, FrontendRuntimeState, NormalizedFrontendAsset,
    NormalizedFrontendFile, NormalizedFrontendSnapshot, ResolvedPluginAsset,
    build_frontend_runtime_state, extension_revision_token, frontend_runtime_state,
    insert_frontend_runtime_state, plugin_admin_full_path, resolve_plugin_asset,
    validate_cross_plugin_conflicts,
};
pub use host_registry::{HostRegistry, RegistryLookup};
pub use manifest::{
    AdminPageSpec, AssetBundleSpec, CompatibilityBridgeSpec, CompatibilitySpec, DependencySpec,
    EditorSpec, FrontendSpec, HookSpec, HostManifestSpec, ParserSpec, PermissionEntry,
    PermissionsSpec, PluginKind, PluginManifest, PluginMeta, PublicPageSpec,
};
pub use model::{NewPluginRow, PluginRecord, PluginStatus};
pub use repository::PluginRepository;
pub use resolver::{check_host_compatibility, reverse_dependencies, topological_order};
pub use runtime::PluginRuntime;
pub use service::{PluginInfo, PluginManager, PluginManagerConfig};
