# CyCMS Web Key Surfaces

## Frontend Structure

- `apps/web/src/routes/`
- `apps/web/src/pages/`
- `apps/web/src/features/`
- `apps/web/src/lib/`
- `apps/web/src/types/`

## Admin Shell and Shared Entry Points

- `apps/web/src/components/admin/`
- `apps/web/src/main.tsx`
- `apps/web/vite.config.ts`
- `apps/web/src/test/setup.ts`

## Common Validation

- `cd apps/web && npm run lint`
- `cd apps/web && npm run test`
- `cd apps/web && npm run build`

## Notes

- 页面逻辑优先放 feature，再由 page 负责组合。
- 改 API 消费时同步 `src/types` 与 `src/lib/api`。
- 当前生产构建可能有 chunk-size warning；它是性能提示，不是功能失败。