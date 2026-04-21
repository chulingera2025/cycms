---
name: cycms-backend-development
description: '在 CyCMS workspace 中开发或重构 Rust 后端与 CLI。用于 config/kernel/api wiring、content/auth/permission/media/revision/publish/settings 领域逻辑、HTTP 契约、服务启动与验证收口。'
argument-hint: '描述这次后端目标，例如：调整 revision API、修改 auth 配置约束、重构 kernel wiring'
user-invocable: true
disable-model-invocation: false
---

# CyCMS Backend Development

## When to Use

- 开发或修改 Rust 后端 crate、CLI、配置、HTTP gateway 或服务装配。
- 调整内容、鉴权、权限、媒体、设置、版本、发布等领域逻辑。
- 处理 `cycms-config`、`cycms-kernel`、`cycms-api`、`cycms-cli` 之间的 wiring。

## Also Load When Needed

- 如果后端改动涉及插件生命周期、admin extension bootstrap、plugin runtime 或插件安全策略，再按需加载 `cycms-plugin-development`。
- 如果后端契约会改变 React 管理后台的消费或页面交互，再按需加载 `cycms-web-development`。

## Procedure

1. 先定位改动归属。
   - 配置和默认值：`crates/cycms-config/` 与 `cycms.toml`
   - 领域逻辑：对应 domain crate
   - HTTP 适配：`crates/cycms-api/`
   - 服务装配 / middleware：`crates/cycms-kernel/`
   - CLI：`crates/cycms-cli/`

2. 先读上下文，再写代码。
   - 先看 [后端关键表面与常用验证](./references/backend-surfaces.md)
   - 需要改 HTTP 契约时，同时看 handler、response type、gateway test 和实际调用方。
   - 需要改配置时，同时看默认 TOML、env override 和启动约束。

3. 在拥有者边界内实现。
   - 领域规则放对应 crate，不要无故堆到 `cycms-api`。
   - 配置默认值统一放 `cycms-config`。
   - 装配和 middleware 统一收口到 `cycms-kernel`。
   - CLI 行为变化同步 `cycms-cli` 和 README。

4. 补相邻测试。
   - 改 domain crate 就补该 crate 测试。
   - 改 API 合同补 gateway 测试。
   - 改配置默认值至少补 `cycms-config` 测试。

5. 用改动面选择验证。
   - `cargo test -p <affected-crate>`
   - `cargo test -p cycms-config`
   - `cargo test -p cycms-api --test gateway`
   - `cargo check -p cycms-kernel`

## Decision Points

- 只是领域规则变化：优先改 domain crate。
- 只是 HTTP 适配变化：优先改 API 层和网关测试。
- 只是启动/中间件/路由组合变化：优先改 kernel。
- 需要同步前端：先稳定返回契约，再交给 web skill 处理消费层。

## Completion Checks

- 逻辑落在正确 crate，没有明显跨层泄漏。
- 默认值、env override、启动行为和测试保持一致。
- 相关 Rust 验证已执行且没有新增错误。

## References

- [后端关键表面与常用验证](./references/backend-surfaces.md)