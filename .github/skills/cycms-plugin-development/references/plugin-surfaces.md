# CyCMS Plugin Development Key Surfaces

## Backend: Plugin Lifecycle and Runtime

- `crates/cycms-plugin-manager/`
- `crates/cycms-plugin-api/`
- `crates/cycms-plugin-native/`
- `crates/cycms-plugin-wasm/`
- `support/cycms-native-loader/`

## Backend: Admin Extension Host

- `crates/cycms-config/src/lib.rs`
- `crates/cycms-api/src/handlers/admin_extensions.rs`
- `crates/cycms-api/src/state.rs`
- `crates/cycms-api/src/admin_extensions_observability.rs`
- `crates/cycms-api/tests/gateway.rs`
- `crates/cycms-kernel/src/lib.rs`
- `cycms.toml`

## Frontend: Admin Extension Runtime

- `apps/web/src/features/admin-extensions/`
- `apps/web/src/features/content/`
- `apps/web/src/lib/api/admin-extensions.ts`
- `apps/web/src/types/index.ts`
- `apps/web/src/pages/admin/PluginNamespacePage.tsx`
- `apps/web/src/pages/admin/PluginsPage.tsx`
- `apps/web/src/components/admin/AdminLayout.tsx`

## Testing Surfaces

- Rust:
  - `cargo test -p cycms-config`
  - `cargo test -p cycms-api --test gateway`
  - `cargo test -p cycms-plugin-manager --test lifecycle`
- Web:
  - `cd apps/web && npm run lint`
  - `cd apps/web && npm run test`
  - `cd apps/web && npm run build`

## Verified Project Notes

- native 动态插件目前更适合验证生命周期钩子和可观察副作用，不要让测试依赖跨 dylib 复杂 host 对象。
- admin extension 资产必须保持宿主控制和同源 URL；安全默认值优先通过配置和中间件收口。
- 当前 admin extension 宿主已具备 module host、same-origin 校验、CSP/report-only、telemetry、diagnostics drawer、field renderer/slot dirty-state 与 validation bridge。