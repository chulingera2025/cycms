---
name: cycms-plugin-development
description: '实现、扩展或排查 CyCMS 插件系统与 admin extension。用于 plugin lifecycle、frontend manifest/bootstrap、asset gateway、namespace route、field renderer、slot、CSP、telemetry、diagnostics、native/wasm runtime 相关任务。'
argument-hint: '描述这次插件相关目标，例如：新增 admin extension slot，或排查 native 插件启用失败'
user-invocable: true
disable-model-invocation: false
---

# CyCMS Plugin Development

## When to Use

- 开发或修改 CyCMS 的插件生命周期、安装/启用/禁用/卸载流程。
- 开发或排查 native / wasm 插件运行时、loader 或插件 API 契约。
- 开发或修改 admin extension 的 bootstrap、asset gateway、namespace route、field renderer、slot。
- 调整插件相关的 CSP、同源资产加载、遥测、诊断接口或运维面板。

## What This Skill Produces

- 把插件相关需求先切成 runtime、API、安全、前端宿主、运维可见性几个边界，再做最小闭环实现。
- 保持插件前端资产由宿主控制、同源加载、可诊断、可验证。
- 在代码落地后同步补测试和验证，而不是只改一个表层文件。

## Procedure

1. 先判定改动属于哪一层。
   - 生命周期与持久化状态：`crates/cycms-plugin-manager/`
   - Native/Wasm runtime：`crates/cycms-plugin-native/`、`crates/cycms-plugin-wasm/`、`support/cycms-native-loader/`
   - admin extension 后端：`crates/cycms-config/`、`crates/cycms-api/`、`crates/cycms-kernel/`
   - admin extension 前端：`apps/web/src/features/admin-extensions/`、`apps/web/src/features/content/`、后台页面

2. 先读当前契约和相邻消费方，不要只看定义。
   - 先看 [插件关键表面与验证命令](./references/plugin-surfaces.md)
   - 需要改 bootstrap / diagnostics 时，同时看后端返回结构和前端消费点。
   - 需要改 field renderer / slot 时，同时看 host context 定义和内容编辑器消费方。

3. 选择低风险实现策略。
   - diagnostics 需要增加运维字段时，优先在 API 层包装，除非底层 schema 必须共享给多个消费者。
   - JS/CSS 插件资产必须保持同源、白名单化，不要引入远程运行时代码。
   - field renderer 与 slot 改动必须保住 value、validation、dirty-state 和 entry context bridge。
   - native 插件测试优先验证生命周期和可观察副作用，不要让测试依赖跨 dylib 复杂 host 对象。

4. 按边界分层修改。
   - 配置和默认值放在 `cycms.toml` 与 `crates/cycms-config/`
   - HTTP 契约和 API-side observability 放在 `crates/cycms-api/`
   - 中间件和运行时装配放在 `crates/cycms-kernel/`
   - registry、module host、telemetry、diagnostics UI 放在 `apps/web/src/features/admin-extensions/` 与后台页面

5. 补齐测试，而不是依赖手工验证。
   - Rust 侧优先补 gateway、config、plugin-manager lifecycle 测试。
   - Web 侧优先补 loader、host boundary、registry、namespace page、plugins page、editor helper 的 Vitest 测试。
   - 新增 npm 依赖时同步 lockfile 和 `apps/web/src/test/setup.ts`。

6. 用默认验证序列收口。
   1. `cargo test -p cycms-config`
   2. `cargo test -p cycms-api --test gateway`
   3. `cargo check -p cycms-kernel`
   4. `cd apps/web && npm run lint`
   5. `cd apps/web && npm run test`
   6. `cd apps/web && npm run build`
   - 如果新增了 web 依赖，先执行 `cd apps/web && npm install`。

## Decision Points

- 只改插件生命周期或 runtime：聚焦 `cycms-plugin-manager` / runtime crate，跳过前端宿主。
- 只改 admin extension 宿主：先稳住 bootstrap 契约和 host context，再动页面或编辑器消费方。
- 新安全默认值导致测试失败：修测试 harness，不要为了测试降低生产默认值。
- 用户要求“一次做完”：先盘点生命周期、宿主、安全、遥测、测试、文档几个缺口，再一口气闭环。

## Completion Checks

- 插件资产仍由宿主控制且为同源加载。
- diagnostics 和 telemetry 是结构化的，且运维入口可以看到。
- registry invalidation、跨标签页同步和 stale-session 回收没有被破坏。
- 相关 Rust / Web 验证都跑过，并且没有新增编译或 lint 问题。

## References

- [插件关键表面与验证命令](./references/plugin-surfaces.md)