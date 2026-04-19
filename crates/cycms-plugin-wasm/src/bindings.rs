//! `bindgen!` 生成的 host-side 绑定。
//!
//! WIT 文件位于本 crate 的 `wit/` 目录，`plugin` world 聚合 10 个 host interface
//! 与 4 个 guest export。此处仅负责声明宏调用以触发编译期 WIT 校验与类型生成，
//! 具体 Host trait 实现（content / auth / permission / kv / http / event / route /
//! log / settings / db）在 17.3 的 `host_impls/` 模块里分组完成。
//!
//! 约定：`imports: { default: async | trappable }` 让每个 host function 签名为
//! `async fn foo(...) -> wasmtime::Result<...>`，单次调用 trap 不影响其他实例；
//! `exports: { default: async }` 让 guest 调用 `call_on_enable` / `call_handle_http`
//! 等也是 `async`，可以在 host tokio 运行时内直接 `.await`。

wasmtime::component::bindgen!({
    path: "wit",
    world: "plugin",
    imports: { default: async | trappable },
    exports: { default: async },
});
