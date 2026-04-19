//! wasmtime Store 内持有的宿主状态：聚合所有核心子系统引用 + 当前插件名，并实现
//! `wasmtime_wasi::WasiView` 以完成 WASI preview 2 的完整透传。
//!
//! 完全信任模型下所有字段对 guest 均可访问（`bindings::cycms::plugin::*::Host`
//! 的实现直接读写这些引用）。

use std::sync::Arc;

use wasmtime::component::{HasData, ResourceTable};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

use cycms_auth::AuthEngine;
use cycms_content_engine::ContentEngine;
use cycms_db::DatabasePool;
use cycms_events::EventBus;
use cycms_permission::PermissionEngine;
use cycms_plugin_api::ServiceRegistry;
use cycms_settings::SettingsManager;

/// 每个 Wasm 插件实例对应一份 `HostState`，随 [`wasmtime::Store`] 生命周期一起存亡。
///
/// 字段在 17.2 搭骨架时未被 host 实现读取（目前仅 `log` 真正工作，其余 9 组为 stub）；
/// 17.3 起会被各组 host function 逐一使用，此处临时 `#[allow(dead_code)]`。
#[allow(dead_code)]
pub(crate) struct HostState {
    /// 当前插件的 manifest name，用作 `settings` namespace / `tracing` 字段。
    pub plugin_name: String,
    pub db: Arc<DatabasePool>,
    pub auth: Arc<AuthEngine>,
    pub content: Arc<ContentEngine>,
    pub event_bus: Arc<EventBus>,
    pub permissions: Arc<PermissionEngine>,
    pub service_registry: Arc<ServiceRegistry>,
    pub settings: Arc<SettingsManager>,
    /// WASI preview 2 上下文：`WasiCtxBuilder::inherit_network/stdio + preopened_dir
    /// ("/")` 构造，完整透传给 guest。
    pub wasi: WasiCtx,
    pub table: ResourceTable,
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

/// `HasData` 适配器：让 bindgen 生成的 `add_to_linker::<T, D>` 能以
/// `&mut HostState` 为每次 host 调用的 self。T = `HostState`，
/// `D::Data<'a> = &'a mut HostState`。
pub(crate) struct HostStateData;

impl HasData for HostStateData {
    type Data<'a> = &'a mut HostState;
}
