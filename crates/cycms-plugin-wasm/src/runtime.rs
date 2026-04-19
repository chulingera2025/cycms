//! [`WasmPluginRuntime`] 主体：engine / linker 装配 + 生命周期管理（17.4）。
//!
//! # 装配（17.2）
//!
//! - `Engine`：wasmtime 43 在 `component-model` + `async` feature 下默认打开 Component
//!   Model 与 async support，`Config::default()` 即可
//! - `Linker<HostState>`：`wasmtime_wasi::p2::add_to_linker_async` 透传 WASI preview 2；
//!   `Plugin::add_to_linker::<_, HostStateData>` 绑定 10 组 cycms host function
//!
//! # 生命周期（17.4）
//!
//! - `load`：读取 `.wasm` 字节 → `Component::from_binary` → 构造 `HostState`（聚合
//!   [`PluginContext`] 中的全部核心服务引用 + `WasiCtxBuilder` 继承 stdio/env/network
//!   并 preopen 当前工作目录）→ `Plugin::instantiate_async` → `call_on_enable`。
//!   成功后读取 `HostState.subscribed_event_kinds` 把 `on-event` 代理 handler 订阅到
//!   `EventBus`；`pending_routes` 在 17.4b 由 `compose_router` 合成 axum Router。
//! - `unload`：从表中移除 → abort 事件订阅 → 加锁 Store 调 `call_on_disable`（错误仅
//!   记录，不阻断卸载）→ drop Store 释放内存。
//!
//! # 信任模型
//!
//! cycms 对 Wasm 插件完全信任，不做 fuel / epoch 资源限制；仅依靠 wasmtime 对 trap
//! 的天然进程隔离保证单插件崩溃不影响主进程。Host functions 与 WASI 均无白名单。

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, PoisonError, RwLock};

use async_trait::async_trait;
use axum::Router;
use cycms_core::{Error, Result};
use cycms_events::{Event, EventBus, EventHandler, EventKind, SubscriptionHandle};
use cycms_plugin_api::PluginContext;
use cycms_plugin_manager::{PluginKind, PluginManifest, PluginRuntime};
use tokio::sync::Mutex as AsyncMutex;
use tracing::{info, warn};
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{DirPerms, FilePerms, WasiCtxBuilder};

use crate::bindings::Plugin;
use crate::host::{HostState, HostStateData};

/// Wasm 插件运行时：由 [`PluginManager`](cycms_plugin_manager::PluginManager) 通过
/// [`PluginRuntime`] 调度。
pub struct WasmPluginRuntime {
    engine: Engine,
    linker: Linker<HostState>,
    loaded: RwLock<HashMap<String, LoadedWasmPlugin>>,
}

struct LoadedWasmPlugin {
    store: Arc<AsyncMutex<Store<HostState>>>,
    bindings: Arc<Plugin>,
    subscriptions: Vec<SubscriptionHandle>,
    /// TODO!!!: 17.4b 将 `HostState.pending_routes` 合成 axum Router 存入此处。
    routes: Option<Router>,
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

        Ok(Self {
            engine,
            linker,
            loaded: RwLock::new(HashMap::new()),
        })
    }

    /// 列出当前已加载的 wasm 插件 `(name, Router)`，供 17.4b `compose_router` 落地后由
    /// `ApiGateway`（任务 18）合并到主路由表。
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
}

#[async_trait]
impl PluginRuntime for WasmPluginRuntime {
    fn kind(&self) -> PluginKind {
        PluginKind::Wasm
    }

    async fn load(
        &self,
        manifest: &PluginManifest,
        entry_path: &Path,
        ctx: Arc<PluginContext>,
    ) -> Result<()> {
        let plugin_name = manifest.plugin.name.clone();

        {
            let loaded = self.loaded.read().unwrap_or_else(PoisonError::into_inner);
            if loaded.contains_key(&plugin_name) {
                return Err(Error::Conflict {
                    message: format!("wasm plugin {plugin_name} is already loaded"),
                });
            }
        }

        let wasm_bytes = tokio::fs::read(entry_path).await.map_err(|e| Error::Internal {
            message: format!("read wasm {}: {e}", entry_path.display()),
            source: None,
        })?;
        let component =
            Component::from_binary(&self.engine, &wasm_bytes).map_err(|e| Error::PluginError {
                message: format!("wasm component compile: {e}"),
                source: None,
            })?;

        let host_state = build_host_state(&plugin_name, &ctx)?;
        let mut store = Store::new(&self.engine, host_state);

        let bindings = Plugin::instantiate_async(&mut store, &component, &self.linker)
            .await
            .map_err(|e| Error::PluginError {
                message: format!("wasm instantiate: {e}"),
                source: None,
            })?;
        let bindings = Arc::new(bindings);

        match bindings.call_on_enable(&mut store).await {
            Ok(Ok(())) => {}
            Ok(Err(msg)) => {
                return Err(Error::PluginError {
                    message: format!("wasm on-enable returned error: {msg}"),
                    source: None,
                });
            }
            Err(e) => {
                return Err(Error::PluginError {
                    message: format!("wasm on-enable trap: {e}"),
                    source: None,
                });
            }
        }

        let subscribed = store.data().subscribed_event_kinds.clone();
        let pending_routes = store.data().pending_routes.clone();
        let store_shared = Arc::new(AsyncMutex::new(store));

        let mut subscriptions = Vec::with_capacity(subscribed.len());
        for kind_str in &subscribed {
            let handler: Arc<dyn EventHandler> = Arc::new(WasmEventHandler {
                name: format!("{plugin_name}.on-event/{kind_str}"),
                plugin_name: plugin_name.clone(),
                store: Arc::clone(&store_shared),
                bindings: Arc::clone(&bindings),
            });
            let kind = EventKind::from(kind_str.as_str());
            subscriptions.push(ctx.event_bus.subscribe(kind, handler));
        }

        if !pending_routes.is_empty() {
            // TODO!!!: 17.4b 在 compose_router 中把 pending_routes 合成 axum Router。
            warn!(
                plugin = %plugin_name,
                count = pending_routes.len(),
                "wasm route registration recorded but router composition deferred to 17.4b"
            );
        }

        let mut loaded = self.loaded.write().unwrap_or_else(PoisonError::into_inner);
        loaded.insert(
            plugin_name.clone(),
            LoadedWasmPlugin {
                store: store_shared,
                bindings,
                subscriptions,
                routes: None,
            },
        );
        info!(plugin = %plugin_name, "wasm plugin loaded");
        Ok(())
    }

    async fn unload(&self, plugin_name: &str) -> Result<()> {
        let entry = {
            let mut loaded = self.loaded.write().unwrap_or_else(PoisonError::into_inner);
            loaded.remove(plugin_name)
        };
        let Some(LoadedWasmPlugin {
            store,
            bindings,
            subscriptions,
            routes: _,
        }) = entry
        else {
            return Ok(());
        };

        for handle in subscriptions {
            handle.unsubscribe();
        }

        let disable_result = {
            let mut guard = store.lock().await;
            bindings.call_on_disable(&mut *guard).await
        };
        match disable_result {
            Ok(Ok(())) => {
                info!(plugin = %plugin_name, "wasm plugin unloaded");
            }
            Ok(Err(msg)) => {
                warn!(plugin = %plugin_name, error = %msg, "wasm on-disable returned error");
            }
            Err(e) => {
                warn!(plugin = %plugin_name, error = %e, "wasm on-disable trap");
            }
        }
        // 持有 Arc<Mutex<Store>> 的 EventHandler / Router 已在 subscriptions / loaded 移除
        // 时一起释放，Store 随之 drop 释放线性内存。
        Ok(())
    }

    fn loaded_plugins(&self) -> Vec<String> {
        let loaded = self.loaded.read().unwrap_or_else(PoisonError::into_inner);
        let mut names: Vec<String> = loaded.keys().cloned().collect();
        names.sort();
        names
    }
}

/// 构造 per-plugin `HostState`。WASI 上下文继承宿主 stdio / 环境变量 / 网络，并
/// preopen 当前工作目录为 `.`（完全信任模型下允许 guest 读写本地文件）。
fn build_host_state(plugin_name: &str, ctx: &PluginContext) -> Result<HostState> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_env()
        .inherit_network()
        .allow_ip_name_lookup(true)
        .preopened_dir(&cwd, ".", DirPerms::all(), FilePerms::all())
        .map_err(|e| Error::Internal {
            message: format!("wasi preopened_dir {}: {e}", cwd.display()),
            source: None,
        })?
        .build();

    Ok(HostState {
        plugin_name: plugin_name.to_owned(),
        db: Arc::clone(&ctx.db_pool),
        auth: Arc::clone(&ctx.auth_engine),
        content: Arc::clone(&ctx.content_engine),
        event_bus: Arc::clone(&ctx.event_bus),
        permissions: Arc::clone(&ctx.permission_engine),
        service_registry: Arc::clone(&ctx.service_registry),
        settings: Arc::clone(&ctx.settings_manager),
        subscribed_event_kinds: Vec::new(),
        pending_routes: Vec::new(),
        wasi,
        table: ResourceTable::new(),
    })
}

/// [`EventHandler`] 适配器：EventBus 发布事件时拿住 `Arc<Mutex<Store>>` 串行化
/// 调用 guest 导出的 `on-event`。guest 内部 host function 依赖 `StoreContextMut<HostState>`
/// 独占 Store，Mutex 保障同一时刻只有一条 guest 调用在跑。
struct WasmEventHandler {
    name: String,
    plugin_name: String,
    store: Arc<AsyncMutex<Store<HostState>>>,
    bindings: Arc<Plugin>,
}

#[async_trait]
impl EventHandler for WasmEventHandler {
    fn name(&self) -> &str {
        &self.name
    }

    async fn handle(&self, event: Arc<Event>) -> Result<()> {
        let payload = serde_json::to_string(&event.payload).unwrap_or_default();
        let kind_str = event.kind.as_str().to_owned();
        let result = {
            let mut guard = self.store.lock().await;
            self.bindings
                .call_on_event(&mut *guard, &kind_str, &payload)
                .await
        };
        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(msg)) => {
                warn!(
                    plugin = %self.plugin_name,
                    kind = %kind_str,
                    error = %msg,
                    "wasm on-event returned inner error"
                );
                // 按 EventBus 语义：单插件 handler 失败不阻断其他订阅者；这里吞掉
                // inner Err，仅日志。
                Ok(())
            }
            Err(e) => Err(Error::PluginError {
                message: format!("wasm on-event trap: {e}"),
                source: None,
            }),
        }
    }
}

#[allow(dead_code)]
fn _event_bus_marker(_b: &EventBus) {
    // 仅用于静态确认 EventBus 在 runtime 生命周期内可用；实际订阅走 ctx.event_bus。
}
