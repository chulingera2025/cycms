//! [`WasmPluginRuntime`] 骨架 + wasmtime Engine / Linker 一次性装配。
//!
//! 本步（17.2）完成：
//! - `Engine`：`wasm_component_model` 与 async 在 wasmtime 43 的 `component-model` +
//!   `async` feature 下默认开启，`Config::default()` 即满足需求
//! - `Linker<HostState>`：`wasmtime_wasi::p2::add_to_linker_async` 透传 WASI preview 2，
//!   再用 bindgen 生成的 `Plugin::add_to_linker::<_, HostStateData>` 绑定 10 组 cycms
//!   host function
//!
//! 17.4 起 `load` / `unload` 会创建 per-plugin Store + `HostState` 并驱动生命周期；
//! 目前 `load` 仍返回「未实现」错误，保持契约清晰。

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use cycms_core::{Error, Result};
use cycms_plugin_api::PluginContext;
use cycms_plugin_manager::{PluginKind, PluginManifest, PluginRuntime};
use wasmtime::component::Linker;
use wasmtime::{Config, Engine};

use crate::bindings::Plugin;
use crate::host::{HostState, HostStateData};

/// Wasm 插件运行时：由 [`PluginManager`](cycms_plugin_manager::PluginManager) 通过
/// [`PluginRuntime`] 调度。
pub struct WasmPluginRuntime {
    engine: Engine,
    linker: Linker<HostState>,
}

impl WasmPluginRuntime {
    /// 构造 wasmtime `Engine` 与完成绑定的 `Linker<HostState>`。
    ///
    /// # Errors
    /// Engine 初始化失败，或 `add_to_linker` 任一组注册失败时返回错误。
    pub fn new() -> Result<Self> {
        let cfg = Config::new();
        let engine = Engine::new(&cfg).map_err(|e| Error::Internal {
            message: format!("wasmtime engine init: {e}"),
            source: None,
        })?;

        let mut linker = Linker::<HostState>::new(&engine);
        wasmtime_wasi::p2::add_to_linker_async(&mut linker).map_err(|e| Error::Internal {
            message: format!("wasmtime-wasi add_to_linker_async: {e}"),
            source: None,
        })?;
        Plugin::add_to_linker::<_, HostStateData>(&mut linker, |state| state).map_err(|e| {
            Error::Internal {
                message: format!("cycms host add_to_linker: {e}"),
                source: None,
            }
        })?;

        Ok(Self { engine, linker })
    }

    /// 共享 `Engine` 访问：17.4 `load` 会用它编译 `Component`。
    pub(crate) fn engine(&self) -> &Engine {
        &self.engine
    }

    /// 共享 `Linker` 访问：17.4 `load` 会用它实例化 Component。
    pub(crate) fn linker(&self) -> &Linker<HostState> {
        &self.linker
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
        // TODO!!!: 任务 17.4 实现 Component 编译、Store 构造、on_enable 调用与登记。
        let _engine_ref = self.engine();
        let _linker_ref = self.linker();
        Err(Error::PluginError {
            message: format!(
                "wasm plugin runtime load not yet implemented (requested plugin: {})",
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
