# Plugin Change Template

## Scope

- Goal:
- Plugin surface:
  - lifecycle / runtime
  - admin extension backend
  - admin extension frontend
- Touched areas:

## Contract Changes

- Config changes:
- API / diagnostics changes:
- Host context changes:
- Security changes:

## Validation

- Rust:
  - `cargo test -p cycms-config`
  - `cargo test -p cycms-api --test gateway`
  - `cargo test -p cycms-plugin-manager --test lifecycle`
- Web:
  - `cd apps/web && npm run lint`
  - `cd apps/web && npm run test`
  - `cd apps/web && npm run build`

## Done Criteria

- Same-origin and host-controlled asset loading preserved
- Diagnostics / telemetry visible to operators
- Tests updated for changed contracts