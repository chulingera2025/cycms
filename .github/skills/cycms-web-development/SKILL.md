---
name: cycms-web-development
description: '在 CyCMS 管理后台和公开端中开发 React 前端。用于 routes、features、pages、query client、form 与 page integration、admin shell、构建与验证收口。'
argument-hint: '描述这次前端目标，例如：调整后台设置页、修改内容编辑器流程、重构查询层'
user-invocable: true
disable-model-invocation: false
---

# CyCMS Web Development

## When to Use

- 开发或修改 `apps/web` 下的 React 页面、feature、路由、查询与表单逻辑。
- 调整管理后台 shell、公开端页面、交互流程和前端验证。
- 处理 web 类型定义、API client、Vitest 测试和构建问题。

## Also Load When Needed

- 如果前端任务会改后端返回契约、配置或服务装配，再按需加载 `cycms-backend-development`。
- 如果前端任务涉及 admin extension、plugin menu、module host、field renderer 或 slot，再按需加载 `cycms-plugin-development`。

## Procedure

1. 先定位改动层。
   - 路由：`apps/web/src/routes/`
   - 页面：`apps/web/src/pages/`
   - 复用逻辑：`apps/web/src/features/`
   - API / query / types：`apps/web/src/lib/`、`apps/web/src/types/`

2. 先读消费链路。
   - 先看 [Web 关键表面与验证](./references/web-surfaces.md)
   - 页面变更要同时看路由入口、feature hook、API client 和类型定义。
   - 表单变更要同时看初始值、校验、提交路径和错误展示。

3. 在前端边界内实现。
   - 页面编排放 `pages`
   - 共享状态和流程放 `features`
   - HTTP 调用放 `lib/api`
   - 类型契约放 `src/types`

4. 补前端测试。
   - 优先补 Vitest 页面测试、hook 测试、helper 测试。
   - 改测试环境时同步 `vite.config.ts` 和 `src/test/setup.ts`。

5. 用默认前端验证收口。
   - `cd apps/web && npm run lint`
   - `cd apps/web && npm run test`
   - `cd apps/web && npm run build`
   - 新增依赖先执行 `cd apps/web && npm install`

## Decision Points

- 只是 UI 交互变化：优先改 page / feature，不先动 API client。
- 只是数据契约变化：先和 backend skill 对齐接口，再改消费层。
- 只是 admin extension 宿主变化：交给 plugin skill 主导，web skill 负责 consumer 与页面。

## Completion Checks

- 路由、页面、feature、API client 和类型保持一致。
- lint、test、build 已执行。
- 没有引入新的前端类型错误或构建问题。

## References

- [Web 关键表面与验证](./references/web-surfaces.md)