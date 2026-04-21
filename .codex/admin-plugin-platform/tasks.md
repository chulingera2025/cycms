# Implementation Plan

## Phase 1: 后端控制面与同源资产面

状态：Completed

- [x] 1. 扩展插件 manifest 与持久化模型
  - [x] 1.1 将插件前端契约升级为 `frontend.manifest` 与 `frontend.required`。
  - [x] 1.2 将规范化 frontend runtime state 写回已安装插件的 manifest JSON。
  - [x] 1.3 区分 required 与 optional 前端贡献的兼容性状态。
  - _Requirements: 1.1, 1.3, 2.1, 11.3_

- [x] 2. 实现 frontend manifest 解析与校验
  - [x] 2.1 建立 Rust 结构体，表达 admin frontend manifest、贡献与资产描述。
  - [x] 2.2 校验 schema version、SDK range、路径局部性、文件存在性与 digest。
  - [x] 2.3 检测单插件内重复贡献标识与跨插件 field renderer 绑定冲突。
  - _Requirements: 1.2, 2.3, 9.3_

- [x] 3. 构建规范化快照与 revision 能力
  - [x] 3.1 将插件 frontend manifest 规范化为 host-facing snapshot。
  - [x] 3.2 在 install、enable、bootstrap 读取路径上使用持久化 runtime state。
  - [x] 3.3 生成 monotonic revision token，并输出 suppression diagnostics。
  - _Requirements: 2.1, 2.2, 4.2, 4.3, 10.2_

- [x] 4. 提供 plugin asset gateway
  - [x] 4.1 仅从声明资产生成同源 immutable URL。
  - [x] 4.2 返回 `Cache-Control`、`ETag`、`Content-Type` 与 `X-Content-Type-Options`。
  - [x] 4.3 拒绝未声明文件、错误 hash、错误版本和禁用插件资产请求。
  - _Requirements: 3.1, 3.2, 3.3, 9.1_

- [x] 5. 提供 admin extension bootstrap 与 diagnostics API
  - [x] 5.1 返回按当前用户权限过滤后的 bootstrap 文档。
  - [x] 5.2 返回 suppressed / incompatible frontend diagnostics。
  - [x] 5.3 从 bootstrap 中剔除无权限访问的菜单、页面与 settings 贡献。
  - _Requirements: 4.1, 4.3, 8.3, 10.2_

- [x] 6. 增加后端验证覆盖
  - [x] 6.1 增加插件生命周期测试，覆盖 frontend runtime state 写入与启用校验。
  - [x] 6.2 增加 bootstrap 权限过滤、资产解析、兼容性与 digest 校验测试。
  - [x] 6.3 通过 `cargo test -p cycms-plugin-manager --test lifecycle` 与 `cargo check -p cycms-api` 验证。
  - _Requirements: 1.1, 1.2, 2.2, 3.3, 4.1, 4.3, 9.3, 10.2, 11.3_

## Phase 2: 官方后台前端消费与动态装配

状态：Completed

- [x] 7. 实现 frontend bootstrap client 与 registry store
  - [x] 7.1 增加 web API client、query keys 与 registry store。
  - [x] 7.2 在 bootstrap 失败时提供 degraded-mode fallback。
  - _Requirements: 5.1, 11.1, 11.2_

- [x] 8. 重构 admin shell 组合方式
  - [x] 8.1 将硬编码菜单改为 core + plugin menu zones 合成。
  - [x] 8.2 引入固定的 `/admin/x/:plugin/*` 命名空间路由。
  - [x] 8.3 在不阻塞核心路由的前提下解析插件页面贡献。
  - _Requirements: 5.2, 5.3, 11.1_

- [x] 9. 实现插件 settings 集成
  - [x] 9.1 消费动态 settings namespace。
  - [x] 9.2 无自定义页面时回退到 schema-driven settings form。
  - [x] 9.3 有自定义页面时在 namespace 范围内挂载宿主页面。
  - _Requirements: 8.1, 8.2, 8.3_

- [x] 10. 打通 registry 失效与 active-session 退场
  - [x] 10.1 前端感知 revision 变化并刷新 registry，并通过 storage event 同步跨标签页变更。
  - [x] 10.2 插件禁用、卸载后优雅回收陈旧视图，命名空间页会阻止继续挂载失效贡献。
  - _Requirements: 4.2, 11.2_

## Phase 3: 模块宿主、安全强化与可观测性

状态：Completed

- [x] 11. 发布稳定的 admin plugin SDK 与 module host contract
  - [x] 11.1 建立 versioned mount/unmount contract 与 typed host context。
  - [x] 11.2 提供页面、widget、field renderer 的模块加载工具（namespace page / custom settings page / field renderer / editor sidebar slot 已统一接入 loader、样式预加载、same-origin 校验与生命周期遥测）。
  - _Requirements: 6.1, 6.2_

- [x] 12. 实现 extension module host 边界与编辑器扩展点
  - [x] 12.1 为插件页面、slot、field renderer 提供错误隔离与生命周期清理（page host、field renderer host、sidebar slot host 与 ModuleHostBoundary 均具备清理、降级回退和重置能力）。
  - [x] 12.2 支持 CSS 先于 JS 挂载（page host、field renderer host 和 slot host 已统一实现样式预加载与引用计数回收）。
  - [x] 12.3 实现 slot、field renderer、值/校验/dirty-state 桥接（field renderer 和 slot 均已打通 value、validation、dirty-state、contentType 与 entry mode 桥接）。
  - _Requirements: 6.3, 7.1, 7.2, 7.3, 10.1_

- [x] 13. 强化 CSP、integrity 与遥测
  - [x] 13.1 增加 strict same-origin CSP 与 report-only rollout。
  - [x] 13.2 发出 load、mount、unmount、route resolution 的结构化遥测。
  - [x] 13.3 暴露运维可见性、诊断面板与失败原因。
  - _Requirements: 9.2, 9.3, 10.1, 10.2, 10.3_

- [x] 14. 补齐前端集成与端到端生命周期测试
  - [x] 14.1 覆盖菜单组合、命名空间路由、settings 集成与 module host 故障隔离。
  - [x] 14.2 覆盖 install、enable、disable、uninstall 与 active-session invalidation；升级场景沿用相同的 revision invalidation 路径与 diagnostics/telemetry 验证。
  - _Requirements: 5.3, 6.3, 8.2, 11.2_
