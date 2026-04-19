//! `cycms-plugin-wasm` —— Wasm Component Model 插件运行时（任务 17）。
//!
//! 覆盖 Requirements：
//! - R12.1 使用 wasmtime 编译并实例化 `.wasm` 组件，绑定 Host Functions
//! - R12.2 内容 host 调用委托到 `ContentEngine`，结果 JSON 化回传给 guest
//! - R12.3 `route.register` → 合成 axum 代理 handler 回调 guest 导出函数
//! - R12.4 guest trap / panic 被运行时捕获，不影响主进程（wasmtime 天然进程隔离）
//! - R12.5 提供 10 组 host function：`content` / `auth` / `permission` / `kv` /
//!   `http` / `event` / `route` / `log` / `settings` / `db`，并透传 WASI preview 2
//!
//! # 信任模型
//!
//! cycms 对 Wasm 插件采用**完全信任**（与 Native 插件同权）：
//! - 不做 fuel / memory / epoch 等资源限制
//! - host functions 不做白名单，含 `db` 原始 SQL 与 `http` 任意域名出站
//! - WASI preview 2 完整透传（文件系统 / 套接字 / 时钟 / 环境变量 / stdio）
//! - 仅保留 wasmtime 对 wasm trap 的天然进程隔离，单插件崩溃不影响主进程
//!
//! 安全审计由上层分发渠道（未来的插件市场）负责；本 crate 不做任何约束。

mod runtime;

pub use runtime::WasmPluginRuntime;
