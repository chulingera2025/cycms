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

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::{Arc, PoisonError, RwLock};

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::http::{HeaderName, HeaderValue, Request, Response, StatusCode};
use axum::routing::{self, MethodFilter};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use cycms_core::{Error, Result};
use cycms_events::{Event, EventBus, EventHandler, EventKind, SubscriptionHandle};
use cycms_plugin_api::PluginContext;
use cycms_plugin_manager::{PluginKind, PluginManifest, PluginRuntime};
use serde::{Deserialize, Serialize};
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
    /// 由 [`compose_router`] 根据 guest 的 `route.register` 合成的 axum `Router`；没有
    /// 路由登记时为 `None`。由 `ApiGateway`（任务 18）合并到主路由表。
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

        let wasm_bytes = tokio::fs::read(entry_path)
            .await
            .map_err(|e| Error::Internal {
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

        let routes = if pending_routes.is_empty() {
            None
        } else {
            Some(compose_router(
                &plugin_name,
                pending_routes,
                &store_shared,
                &bindings,
            ))
        };

        let mut loaded = self.loaded.write().unwrap_or_else(PoisonError::into_inner);
        loaded.insert(
            plugin_name.clone(),
            LoadedWasmPlugin {
                store: store_shared,
                bindings,
                subscriptions,
                routes,
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

/// 路由代理共享状态：每个 (path, method) 组合一份，供 axum handler 闭包持有。
struct ProxyShared {
    plugin_name: String,
    store: Arc<AsyncMutex<Store<HostState>>>,
    bindings: Arc<Plugin>,
}

#[derive(Serialize)]
struct WasmHttpRequestPayload {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    #[serde(rename = "body-base64")]
    body_base64: String,
}

#[derive(Deserialize)]
struct WasmHttpResponsePayload {
    status: u16,
    #[serde(default)]
    headers: Vec<(String, String)>,
    #[serde(default, rename = "body-base64")]
    body_base64: String,
}

fn method_to_filter(m: &str) -> Option<MethodFilter> {
    Some(match m {
        "GET" => MethodFilter::GET,
        "POST" => MethodFilter::POST,
        "PUT" => MethodFilter::PUT,
        "PATCH" => MethodFilter::PATCH,
        "DELETE" => MethodFilter::DELETE,
        "HEAD" => MethodFilter::HEAD,
        "OPTIONS" => MethodFilter::OPTIONS,
        _ => return None,
    })
}

/// 把 guest 登记的 `(path, method)` 列表合成单个 axum `Router`。
///
/// 同一 path 上的多个 method 被合并成一个带 [`MethodFilter`] 位掩码的 handler——
/// 所有方法都走同一个 `proxy_handle` → guest `handle-http`，guest 内部按 `method`
/// 自行分派。
fn compose_router(
    plugin_name: &str,
    pending_routes: Vec<(String, String)>,
    store: &Arc<AsyncMutex<Store<HostState>>>,
    bindings: &Arc<Plugin>,
) -> Router {
    let mut by_path: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (p, m) in pending_routes {
        by_path.entry(p).or_default().push(m);
    }

    let mut router = Router::new();
    for (path, methods) in by_path {
        let mut filter: Option<MethodFilter> = None;
        for m in &methods {
            if let Some(f) = method_to_filter(m) {
                filter = Some(match filter {
                    Some(existing) => existing.or(f),
                    None => f,
                });
            }
        }
        let Some(filter) = filter else {
            continue;
        };

        let shared = Arc::new(ProxyShared {
            plugin_name: plugin_name.to_owned(),
            store: Arc::clone(store),
            bindings: Arc::clone(bindings),
        });
        let handler = move |req: Request<Body>| {
            let shared = Arc::clone(&shared);
            async move { proxy_handle(shared, req).await }
        };
        router = router.route(&path, routing::on(filter, handler));
    }
    router
}

async fn proxy_handle(
    shared: Arc<ProxyShared>,
    req: Request<Body>,
) -> std::result::Result<Response<Body>, StatusCode> {
    let method = req.method().as_str().to_owned();
    let uri = req.uri().clone();
    let url = match uri.query() {
        Some(q) => format!("{}?{}", uri.path(), q),
        None => uri.path().to_owned(),
    };
    let headers: Vec<(String, String)> = req
        .headers()
        .iter()
        .map(|(k, v)| (k.as_str().to_owned(), v.to_str().unwrap_or("").to_owned()))
        .collect();
    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(b) => b.to_vec(),
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    let payload = WasmHttpRequestPayload {
        method,
        url,
        headers,
        body_base64: BASE64.encode(&body_bytes),
    };
    let request_json = match serde_json::to_string(&payload) {
        Ok(s) => s,
        Err(e) => {
            warn!(plugin = %shared.plugin_name, error = %e, "wasm handle-http: serialize request");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let result = {
        let mut guard = shared.store.lock().await;
        shared
            .bindings
            .call_handle_http(&mut *guard, &request_json)
            .await
    };
    match result {
        Ok(Ok(response_json)) => parse_wasm_response(&shared.plugin_name, &response_json),
        Ok(Err(msg)) => {
            warn!(plugin = %shared.plugin_name, error = %msg, "wasm handle-http returned inner error");
            Err(StatusCode::BAD_GATEWAY)
        }
        Err(e) => {
            warn!(plugin = %shared.plugin_name, error = %e, "wasm handle-http trap");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

fn parse_wasm_response(
    plugin_name: &str,
    response_json: &str,
) -> std::result::Result<Response<Body>, StatusCode> {
    let resp: WasmHttpResponsePayload = match serde_json::from_str(response_json) {
        Ok(r) => r,
        Err(e) => {
            warn!(plugin = %plugin_name, error = %e, "wasm handle-http: deserialize response");
            return Err(StatusCode::BAD_GATEWAY);
        }
    };
    let body_bytes = match BASE64.decode(resp.body_base64.as_bytes()) {
        Ok(b) => b,
        Err(e) => {
            warn!(plugin = %plugin_name, error = %e, "wasm handle-http: invalid body-base64");
            return Err(StatusCode::BAD_GATEWAY);
        }
    };
    let mut builder = Response::builder().status(resp.status);
    for (k, v) in resp.headers {
        match (
            HeaderName::from_bytes(k.as_bytes()),
            HeaderValue::from_str(&v),
        ) {
            (Ok(name), Ok(val)) => {
                builder = builder.header(name, val);
            }
            _ => {
                warn!(plugin = %plugin_name, header = %k, "wasm handle-http: skipping invalid header");
            }
        }
    }
    builder.body(Body::from(body_bytes)).map_err(|e| {
        warn!(plugin = %plugin_name, error = %e, "wasm handle-http: build response");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}
