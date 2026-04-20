use std::any::Any;
use std::future::Future;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use cycms_core::{Error, Result};
use cycms_events::{EventHandler, EventKind};

use core::ffi::c_void;

use crate::{Plugin, PluginContext, PluginRouteDoc};

/// Native 动态插件导出的构造函数符号名。
pub const NATIVE_PLUGIN_CREATE_SYMBOL_NAME: &str = "cycms_native_plugin_create";

/// 供 `libloading` 查找的带尾随 `NUL` 符号名字节串。
pub const NATIVE_PLUGIN_CREATE_SYMBOL: &[u8] = b"cycms_native_plugin_create\0";

#[doc(hidden)]
#[must_use]
pub fn into_ffi_plugin_ptr(plugin: Box<dyn Plugin>) -> *mut c_void {
    Box::into_raw(Box::new(plugin)).cast::<c_void>()
}

struct ExportedDynamicPlugin<T> {
    inner: T,
}

impl<T> ExportedDynamicPlugin<T> {
    const fn new(inner: T) -> Self {
        Self { inner }
    }

    fn block_on_plugin_runtime<R>(future: impl Future<Output = Result<R>>) -> Result<R> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|source| Error::Internal {
                message: "failed to create dynamic plugin runtime".to_owned(),
                source: Some(Box::new(source)),
            })?;
        runtime.block_on(future)
    }
}

#[async_trait]
impl<T> Plugin for ExportedDynamicPlugin<T>
where
    T: Plugin + Send + Sync,
{
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn version(&self) -> &str {
        self.inner.version()
    }

    async fn on_enable(&self, ctx: &PluginContext) -> Result<()> {
        Self::block_on_plugin_runtime(self.inner.on_enable(ctx))
    }

    async fn on_disable(&self, ctx: &PluginContext) -> Result<()> {
        Self::block_on_plugin_runtime(self.inner.on_disable(ctx))
    }

    fn routes(&self) -> Option<Router> {
        None
    }

    fn route_docs(&self) -> Vec<PluginRouteDoc> {
        Vec::new()
    }

    fn event_handlers(&self) -> Vec<(EventKind, Arc<dyn EventHandler>)> {
        Vec::new()
    }

    fn services(&self) -> Vec<(String, Arc<dyn Any + Send + Sync>)> {
        Vec::new()
    }
}

#[doc(hidden)]
#[must_use]
pub fn into_exported_dynamic_plugin<T>(plugin: T) -> Box<dyn Plugin>
where
    T: Plugin + Send + Sync + 'static,
{
    Box::new(ExportedDynamicPlugin::new(plugin))
}

/// 导出 Native 动态插件的工厂函数。
///
/// 该宏会生成宿主约定的 `cycms_native_plugin_create` 符号，返回一个由宿主接管所有权的
/// 插件实例。当前动态库模式保证生命周期钩子可用；`routes` / `services` /
/// `event_handlers` 这类复杂 Rust 对象仍建议走静态注册路径，避免跨 dylib 直接传递。
/// 最常见用法：
///
/// ```ignore
/// pub struct HelloPlugin;
///
/// cycms_plugin_api::export_plugin!(HelloPlugin);
/// ```
///
/// 也可以传入构造表达式：
///
/// ```ignore
/// cycms_plugin_api::export_plugin!(HelloPlugin::new());
/// ```
#[macro_export]
macro_rules! export_plugin {
    ($plugin:expr $(,)?) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn cycms_native_plugin_create() -> *mut ::core::ffi::c_void {
            let plugin: ::std::boxed::Box<dyn $crate::Plugin> =
                $crate::into_exported_dynamic_plugin($plugin);
            $crate::into_ffi_plugin_ptr(plugin)
        }
    };
}