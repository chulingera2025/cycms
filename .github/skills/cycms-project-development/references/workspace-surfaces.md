# CyCMS Workspace Key Surfaces

## Root Entry Points

- `README.md`
- `Cargo.toml`
- `cycms.toml`

## Backend Assembly

- `crates/cycms-config/`
- `crates/cycms-kernel/`
- `crates/cycms-api/`
- `crates/cycms-cli/`

## Core Domain Crates

- `crates/cycms-auth/`
- `crates/cycms-permission/`
- `crates/cycms-content-model/`
- `crates/cycms-content-engine/`
- `crates/cycms-revision/`
- `crates/cycms-publish/`
- `crates/cycms-media/`
- `crates/cycms-settings/`
- `crates/cycms-events/`
- `crates/cycms-observability/`

## Frontend Surfaces

- `apps/web/src/features/`
- `apps/web/src/pages/`
- `apps/web/src/routes/`
- `apps/web/src/lib/`
- `apps/web/src/types/`

## Common Validation Commands

- `cargo test -p <affected-crate>`
- `cargo test -p cycms-config`
- `cargo test -p cycms-api --test gateway`
- `cargo check -p cycms-kernel`
- `cd apps/web && npm run lint`
- `cd apps/web && npm run test`
- `cd apps/web && npm run build`

## Project Notes

- 这是 Rust workspace + React SPA 的双栈仓库；跨栈需求先稳住接口，再改 UI。
- 默认配置和环境变量覆盖规则在 `crates/cycms-config/` 与 `cycms.toml`，不要把默认值散落到别的 crate。
- 服务装配、middleware 和路由组合统一在 `crates/cycms-kernel/` 与 `crates/cycms-api/` 收口。
- Web 构建当前可能出现 chunk-size warning；它属于性能优化提示，不等于功能失败。