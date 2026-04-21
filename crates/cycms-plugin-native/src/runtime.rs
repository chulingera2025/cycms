//! [`NativePluginRuntime`] 实现：把静态注册的 [`Plugin`] trait 对象或导出工厂函数的
//! 动态库装配成 [`PluginRuntime`] 可调度的运行时单元。
//!
//! - 「静态注册优先」：Kernel / CLI 启动阶段可继续通过
//!   [`NativePluginRuntime::register_plugin`] 交付 `Arc<dyn Plugin>`；若未注册同名插件，
//!   runtime 会尝试按 manifest `entry` 动态加载 `.so` 并调用导出的工厂函数。
//! - 事件：`on_enable` 成功之后按 `(EventKind, handler)` 对逐个订阅到 `EventBus`，
//!   订阅句柄与插件绑定；`unload` 统一 abort。
//! - 服务：`Plugin::services()` 以 `{plugin}.{svc}` 为 key 注册到 `ServiceRegistry`，
//!   `unload` 时成对注销。
//! - 路由：`Plugin::routes()` 收集后缓存，`routes_of` / `all_routes` 暴露给
//!   `ApiGateway` 合并到主路由表。
//! - 动态库模式目前只保证生命周期钩子跨 dylib 可用；若插件需要 routes / services /
//!   event handlers，仍应使用宿主静态注册路径。

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, PoisonError, RwLock};

use async_trait::async_trait;
use axum::Router;
use cycms_core::{Error, Result};
use cycms_events::SubscriptionHandle;
use cycms_native_loader::DynamicPluginLibrary;
use cycms_plugin_api::{Plugin, PluginContext, PluginRouteDoc};
use cycms_plugin_manager::{PluginKind, PluginManifest, PluginRuntime};
use tracing::{info, warn};

/// Native 插件运行时：按 `plugin.name()` 静态注册 `Arc<dyn Plugin>`，
/// 由 [`PluginManager`](cycms_plugin_manager::PluginManager) 通过 [`PluginRuntime`] 调度。
#[derive(Default)]
pub struct NativePluginRuntime {
    factories: RwLock<HashMap<String, Arc<dyn Plugin>>>,
    loaded: RwLock<HashMap<String, LoadedPlugin>>,
}

struct LoadedPlugin {
    plugin: Arc<dyn Plugin>,
    ctx: Arc<PluginContext>,
    subscriptions: Vec<SubscriptionHandle>,
    service_keys: Vec<String>,
    routes: Option<Router>,
    route_docs: Vec<PluginRouteDoc>,
    dynamic_library: Option<DynamicPluginLibrary>,
}

struct PreparedPlugin {
    plugin: Arc<dyn Plugin>,
    dynamic_library: Option<DynamicPluginLibrary>,
}

impl NativePluginRuntime {
    /// 构造空运行时。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 预注册一个 Native 插件实例。
    ///
    /// 以 [`Plugin::name`] 为 key 存入 factory 表；重复注册会覆盖并记录 warning。
    /// 之后 `PluginManager::install / enable` 会通过 manifest 名查找并驱动生命周期。
    pub fn register_plugin(&self, plugin: Arc<dyn Plugin>) {
        let name = plugin.name().to_owned();
        let mut factories = self
            .factories
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        if factories.contains_key(&name) {
            warn!(plugin = %name, "native plugin re-registered, previous factory replaced");
        }
        factories.insert(name, plugin);
    }

    /// 返回当前处于 loaded 状态的插件贡献的 axum Router 克隆。
    #[must_use]
    pub fn routes_of(&self, plugin_name: &str) -> Option<Router> {
        let loaded = self.loaded.read().unwrap_or_else(PoisonError::into_inner);
        loaded.get(plugin_name).and_then(|lp| lp.routes.clone())
    }

    /// 列举所有 loaded 插件提供的 `(plugin_name, Router)`，供 `ApiGateway` 合并到主路由表。
    ///
    /// 返回按插件名字典序排列，保证启动日志 / 路由装配结果可复现。
    #[must_use]
    pub fn all_routes(&self) -> Vec<(String, Router)> {
        let loaded = self.loaded.read().unwrap_or_else(PoisonError::into_inner);
        let mut pairs: Vec<(String, Router)> = loaded
            .iter()
            .filter_map(|(name, lp)| lp.routes.clone().map(|r| (name.clone(), r)))
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        pairs
    }

    /// 列出所有 loaded 插件声明的路由文档元数据。
    #[must_use]
    pub fn all_route_docs(&self) -> Vec<(String, Vec<PluginRouteDoc>)> {
        let loaded = self.loaded.read().unwrap_or_else(PoisonError::into_inner);
        let mut pairs: Vec<(String, Vec<PluginRouteDoc>)> = loaded
            .iter()
            .map(|(name, lp)| (name.clone(), lp.route_docs.clone()))
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        pairs
    }

    fn factory_for(&self, name: &str) -> Option<Arc<dyn Plugin>> {
        let factories = self
            .factories
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        factories.get(name).cloned()
    }

    fn prepare_plugin(&self, manifest: &PluginManifest, entry: &Path) -> Result<PreparedPlugin> {
        let name = manifest.plugin.name.as_str();
        if let Some(plugin) = self.factory_for(name) {
            return Ok(PreparedPlugin {
                plugin,
                dynamic_library: None,
            });
        }

        let dynamic_library = DynamicPluginLibrary::open(entry).map_err(map_dynamic_load_error)?;
        let plugin: Arc<dyn Plugin> = dynamic_library
            .instantiate()
            .map(Arc::from)
            .map_err(map_dynamic_load_error)?;

        if plugin.name() != name {
            return Err(Error::PluginError {
                message: format!(
                    "native plugin symbol in {} returned name {:?}, expected {:?}",
                    entry.display(),
                    plugin.name(),
                    name
                ),
                source: None,
            });
        }
        if plugin.version() != manifest.plugin.version {
            return Err(Error::PluginError {
                message: format!(
                    "native plugin {} version mismatch: manifest={}, library={}",
                    name,
                    manifest.plugin.version,
                    plugin.version()
                ),
                source: None,
            });
        }

        Ok(PreparedPlugin {
            plugin,
            dynamic_library: Some(dynamic_library),
        })
    }
}

#[async_trait]
impl PluginRuntime for NativePluginRuntime {
    fn kind(&self) -> PluginKind {
        PluginKind::Native
    }

    async fn load(
        &self,
        manifest: &PluginManifest,
        entry: &Path,
        ctx: Arc<PluginContext>,
    ) -> Result<()> {
        let name = manifest.plugin.name.as_str();
        {
            let loaded = self.loaded.read().unwrap_or_else(PoisonError::into_inner);
            if loaded.contains_key(name) {
                return Err(Error::Conflict {
                    message: format!("native plugin {name} is already loaded"),
                });
            }
        }

        let PreparedPlugin {
            plugin,
            dynamic_library,
        } = self.prepare_plugin(manifest, entry)?;

        if let Err(err) = plugin.on_enable(&ctx).await {
            drop(plugin);
            drop(dynamic_library);
            return Err(err);
        }

        let mut subscriptions: Vec<SubscriptionHandle> = Vec::new();
        for (kind, handler) in plugin.event_handlers() {
            let handle = ctx.event_bus.subscribe(kind, handler);
            subscriptions.push(handle);
        }

        let mut service_keys: Vec<String> = Vec::new();
        for (svc_name, svc) in plugin.services() {
            let key = format!("{name}.{svc_name}");
            if let Err(err) = ctx.service_registry.register_erased(&key, svc) {
                warn!(plugin = %name, error = %err, "rolling back native plugin load");
                rollback_registrations(&ctx, subscriptions, &service_keys);
                // on_enable 已经副作用化执行过，这里无法撤回，调用方自行 uninstall 清理。
                drop(plugin);
                drop(dynamic_library);
                return Err(err.into());
            }
            service_keys.push(key);
        }

        let routes = plugin.routes();
        let route_docs = plugin.route_docs();

        let mut loaded = self.loaded.write().unwrap_or_else(PoisonError::into_inner);
        loaded.insert(
            name.to_owned(),
            LoadedPlugin {
                plugin,
                ctx,
                subscriptions,
                service_keys,
                routes,
                route_docs,
                dynamic_library,
            },
        );
        info!(plugin = %name, "native plugin loaded");
        Ok(())
    }

    async fn unload(&self, plugin_name: &str) -> Result<()> {
        let entry = {
            let mut loaded = self.loaded.write().unwrap_or_else(PoisonError::into_inner);
            loaded.remove(plugin_name)
        };
        let Some(mut loaded_plugin) = entry else {
            return Ok(());
        };

        for handle in loaded_plugin.subscriptions.drain(..) {
            handle.unsubscribe();
        }

        let disable_result = loaded_plugin.plugin.on_disable(&loaded_plugin.ctx).await;

        for key in &loaded_plugin.service_keys {
            loaded_plugin.ctx.service_registry.unregister(key);
        }

        let plugin = loaded_plugin.plugin;
        let dynamic_library = loaded_plugin.dynamic_library.take();
        drop(plugin);
        drop(dynamic_library);

        match disable_result {
            Ok(()) => {
                info!(plugin = %plugin_name, "native plugin unloaded");
                Ok(())
            }
            Err(err) => {
                warn!(plugin = %plugin_name, error = %err, "plugin on_disable returned error");
                Err(err)
            }
        }
    }

    fn loaded_plugins(&self) -> Vec<String> {
        let loaded = self.loaded.read().unwrap_or_else(PoisonError::into_inner);
        let mut names: Vec<String> = loaded.keys().cloned().collect();
        names.sort();
        names
    }
}

fn rollback_registrations(
    ctx: &PluginContext,
    subscriptions: Vec<SubscriptionHandle>,
    service_keys: &[String],
) {
    for handle in subscriptions {
        handle.unsubscribe();
    }
    for key in service_keys {
        ctx.service_registry.unregister(key);
    }
}

fn map_dynamic_load_error(source: cycms_native_loader::DynamicPluginLoadError) -> Error {
    Error::PluginError {
        message: source.to_string(),
        source: Some(Box::new(source)),
    }
}
