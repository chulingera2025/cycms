# CyCMS Backend Key Surfaces

## Assembly and Entry Points

- `crates/cycms-config/`
- `crates/cycms-kernel/`
- `crates/cycms-api/`
- `crates/cycms-cli/`
- `cycms.toml`

## Domain Crates

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

## Common Validation

- `cargo test -p <affected-crate>`
- `cargo test -p cycms-config`
- `cargo test -p cycms-api --test gateway`
- `cargo check -p cycms-kernel`

## Notes

- 配置默认值、TOML 解析和环境变量覆盖统一收口在 `cycms-config`。
- HTTP 层只做适配和权限入口，不要吞掉 domain 规则。
- 服务级 middleware、header 和路由组合放在 `cycms-kernel`。