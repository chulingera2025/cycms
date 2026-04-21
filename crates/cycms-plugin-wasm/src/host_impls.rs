//! 10 组 host function 的 [`Host`](crate::bindings) trait 实现。
//!
//! 每个子模块对 `HostState` 实现对应 interface 的 `Host` trait。完全信任模型下：
//! - `log` 组直接转发到 `tracing`
//! - `settings` / `kv` 代理到 `SettingsManager`（namespace 绑定当前 plugin）
//! - `permission` / `auth` 代理到对应 engine
//! - `content` 代理到 `ContentEngine`
//! - `event` 发布到 `EventBus`
//! - `route` 记录到 `HostState`，由运行时合成 Router
//! - `http` 直出 reqwest，无白名单
//! - `db` 直接访问 `DatabasePool`，无只读约束

mod auth;
mod content;
mod db;
mod event;
mod http;
mod kv;
mod log;
mod permission;
mod route;
mod settings;
