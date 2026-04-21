# CyCMS 后台插件前端扩展平台设计

当前实施已经按三期里程碑重排，并已开始落地第一期。

建议按以下顺序阅读：

1. `research.md`
   - 外部依据与关键技术取舍。
2. `blueprint.md`
   - 系统目标、边界、组件与总览数据流。
3. `requirements.md`
   - 生产级能力要求与验收标准。
4. `design.md`
   - 详细设计、数据契约、路由模型、安全模型与运维策略。
5. `tasks.md`
   - 按一期、二期、三期组织的实施拆解。
6. `validation.md`
   - 需求到任务的覆盖矩阵与完整性校验。

这套文档的核心结论是：CyCMS 需要新增一层“宿主控制的后台前端扩展平台”，而不是让插件直接接管官方前端。插件负责声明贡献和提供同源资产，宿主负责校验、授权、路由命名空间、错误隔离、菜单合成和运维治理。

## 三期里程碑

### 一期：后端控制面与同源资产面

目标：让宿主先具备“认识、校验、治理、分发插件前端贡献”的能力，但暂不要求官方后台前端真正动态挂载插件页面。

已落地范围：

1. `plugin.toml` 前端契约升级为 `frontend.manifest` 与 `frontend.required`。
2. 插件安装时解析并校验 `admin/manifest.json`，验证 schema、SDK 范围、路径局部性、权限声明、摘要和资产存在性。
3. 规范化前端快照写回已持久化的插件 manifest JSON，避免引入额外数据库迁移。
4. 插件启用时执行前端兼容性与跨插件冲突校验。
5. 提供按用户权限过滤的 bootstrap API、diagnostics API，以及白名单化的同源 plugin-assets 网关。
6. 增加后端集成测试，覆盖快照写入、兼容性、权限过滤、资产解析与摘要校验。

### 二期：官方后台前端消费与动态装配

目标：让官方后台前端真正消费一期输出的 bootstrap registry，并把插件菜单、命名空间页面、设置页和扩展点装配进现有 shell。

当前已落地范围：

1. 前端 bootstrap client、query keys、registry provider 与 degraded-mode fallback。
2. `/admin/x/:plugin/*` 命名空间路由、动态插件菜单合成与命名空间占位宿主页。
3. 插件 settings namespace 接入，包含 schema-driven settings UI 与自定义 settings 页入口。
4. 基于 revision 的 registry invalidation、跨标签页同步与 active-session 退场已经接入。
5. 前端构建与 lint 已通过，证明二期消费层能够和现有后台 shell 共存。

二期结论：官方后台前端现在已经能够稳定消费后端 registry，并在插件启用、禁用、卸载后完成菜单、设置和命名空间视图的同步回收。

### 三期：模块宿主、安全强化与可观测性

目标：把插件 UI 挂载合约、字段渲染器、扩展插槽、CSP/Integrity、遥测和运维可见性补齐到生产级闭环。

当前已启动范围：

1. 已建立 page-oriented / field-renderer-oriented / slot-oriented mount/unmount contract、typed host context、CSS 预加载和基础错误边界。
2. `PluginNamespacePage` 已从占位页升级为真实的模块宿主页，会按 `moduleUrl` 加载同源插件 ESM 模块，并在页面卸载或贡献失效时执行清理。
3. 内容编辑器中的 `custom` 字段类型现在会从 bootstrap registry 解析插件 field renderer，并通过宿主桥接把 `field`、`value`、`onChange`、`contentTypeApiId`、`entryId`、`mode` 传入插件模块；找不到渲染器或模块挂载失败时会自动回退到宿主原生编辑器。
4. 内容编辑器右侧现在支持 `content.editor.sidebar` slot 扩展点，宿主会把当前 entry 值集合和 `setFieldValue/getFieldValue` 桥接给插件模块，并在无 slot 时保持原有单栏布局。

三期剩余工作：

1. 继续完善更细粒度的 update/unmount 协议，以及字段校验/dirty-state 桥接。
2. 补齐 CSP / integrity / 遥测 / 诊断面板等生产级治理能力。
3. 增加覆盖插件页面挂载、字段扩展点和生命周期回收的集成测试。
