//! [`WasmPluginRuntime`] 骨架：在后续子任务中逐步填充 engine / linker / 生命周期。
//!
//! 当前仅占位，保证 crate 可编译并向 Kernel 暴露统一类型。后续：
//! - 17.1 在 `wit/` 下定义 10 组 host 接口与 plugin world
//! - 17.2 初始化 wasmtime `Engine` / `Linker<HostState>`，接入 wasmtime-wasi preview 2
//! - 17.3 实现 10 组 host function（完全访问，无白名单）
//! - 17.4 实现 Component 编译 / 实例化 / `on_enable` / `unload`
//! - 17.5 trap / panic 映射为 `Error::PluginError`

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use cycms_core::{Error, Result};
use cycms_plugin_api::PluginContext;
use cycms_plugin_manager::{PluginKind, PluginManifest, PluginRuntime};

/// Wasm 插件运行时：由 [`PluginManager`](cycms_plugin_manager::PluginManager) 通过
/// [`PluginRuntime`] 调度。
///
/// TODO!!!: 任务 17.2 接入 `wasmtime::Engine` / `Linker<HostState>` 与 wasmtime-wasi。
#[derive(Default)]
pub struct WasmPluginRuntime;

impl WasmPluginRuntime {
    /// 构造运行时。
    ///
    /// TODO!!!: 17.2 起该构造函数会做 wasmtime `Config` / `Engine` / `Linker`
    /// 的一次性装配，并返回 `Result`。
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PluginRuntime for WasmPluginRuntime {
    fn kind(&self) -> PluginKind {
        PluginKind::Wasm
    }

    async fn load(
        &self,
        manifest: &PluginManifest,
        _entry_path: &Path,
        _ctx: Arc<PluginContext>,
    ) -> Result<()> {
        // TODO!!!: 任务 17.2/17.4 实现 Component 编译、Store 构造、on_enable 调用与登记。
        Err(Error::PluginError {
            message: format!(
                "wasm plugin runtime not yet implemented (requested plugin: {})",
                manifest.plugin.name
            ),
            source: None,
        })
    }

    async fn unload(&self, _plugin_name: &str) -> Result<()> {
        // TODO!!!: 任务 17.4 清理 Store / 订阅 / 服务 / 路由。
        Ok(())
    }

    fn loaded_plugins(&self) -> Vec<String> {
        Vec::new()
    }
}
