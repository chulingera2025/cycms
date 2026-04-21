---
name: cycms-project-development
description: '在 CyCMS workspace 中开发或重构后端、前端和 CLI。用于 crates 分层改动、API wiring、content/auth/permission/media/revision/settings 逻辑、React 管理后台、配置与验证收口。'
argument-hint: '描述这次项目开发目标，例如：给内容版本接口加能力，或调整后台设置页流程'
user-invocable: true
disable-model-invocation: false
---

# CyCMS Project Development

## When to Use

- 在 CyCMS workspace 中实现非插件专属的后端、前端或 CLI 功能。
- 调整内容模型、内容引擎、鉴权、权限、媒体、设置、发布、版本管理等领域逻辑。
- 处理 `cycms-api`、`cycms-kernel`、`cycms-cli` 之间的装配、配置和 HTTP 契约改动。
- 修改 React 管理后台或公开端页面、路由、数据请求与交互逻辑。

## What This Skill Produces

- 先定位功能归属的 crate 或前端 feature，再在正确边界内实现，而不是把逻辑堆到 gateway 或页面层。
- 优先补根因修复和相邻测试，避免只做表面补丁。
- 对跨栈改动保留配置、类型契约、API、前端消费和验证的一致性。

## Procedure

1. 先定位归属层。
   - 核心类型与 trait：`crates/cycms-core/`
   - 应用装配：`crates/cycms-kernel/`
   - HTTP gateway：`crates/cycms-api/`
   - 领域逻辑：content/auth/permission/media/revision/publish/settings 对应 crate
   - CLI：`crates/cycms-cli/`
   - Web：`apps/web/src/features/`、`apps/web/src/pages/`、`apps/web/src/routes/`

2. 先读上下文，再落代码。
   - 先看 [workspace 关键表面与常用验证](./references/workspace-surfaces.md)
   - 同时看目标 crate 的 `Cargo.toml`、入口模块、相邻测试和实际调用方。
   - 跨栈任务先确认后端契约，再调整前端消费。

3. 在拥有者边界内实现。
   - 配置默认值放 `crates/cycms-config/` 与 `cycms.toml`
   - 领域规则放对应 domain crate，不要无故堆到 `cycms-api`
   - 运行时装配和中间件放 `crates/cycms-kernel/`
   - HTTP 适配、鉴权入口、响应模型放 `crates/cycms-api/`
   - 页面与组件逻辑放 `apps/web/src/features/` 或 `apps/web/src/pages/`

4. 只补需要的测试与文档。
   - Rust 侧优先补目标 crate 测试；HTTP 契约变化补 gateway 测试。
   - Web 侧补 Vitest 测试、查询层测试或页面集成测试。
   - 只有当 README、默认配置或 CLI 行为变化时再同步文档。

5. 按改动面选择验证命令。
   - 单 crate：`cargo test -p <crate>`
   - 改了装配或跨 crate wiring：`cargo check -p cycms-kernel`
   - 改了 API 合同：`cargo test -p cycms-api --test gateway`
   - 改了 Web：`cd apps/web && npm run lint && npm run test && npm run build`
   - 改了默认配置：至少补 `cargo test -p cycms-config`

6. 收口时做一次自检。
   - 代码是否落在正确 crate / feature
   - 配置、类型、API、前端消费是否同步
   - 是否引入了不必要的跨层耦合

## Decision Points

- 只是领域规则变化：优先改对应 domain crate，不要先动 API 层。
- 只是页面交互变化：优先改 feature/page，保持 API client 稳定。
- 需要跨后端和前端：先固定接口和类型，再做 UI。
- 涉及 CLI、配置和服务启动行为：同时检查 `cycms-cli`、`cycms-config`、`cycms-kernel`。

## Completion Checks

- 逻辑位于正确层级，没有明显跨层泄漏。
- 改动对应的 Rust / Web 验证已执行。
- 配置、默认值、类型和调用方保持一致。
- 工作区内没有因本次改动产生的新错误。

## References

- [Workspace 关键表面与常用验证](./references/workspace-surfaces.md)