# Requirements Document

## Introduction

本文档定义 CyCMS 生产级“插件驱动后台前端扩展平台”的功能性与运行性要求。目标不是做一个演示级微前端，而是在当前插件体系之上补齐可安装、可升级、可诊断、可审计、可回退的后台注入能力。

## Glossary

- **Plugin Frontend Manifest**：插件自带的前端贡献清单，声明页面、菜单、扩展点、字段渲染器、设置贡献与对应资产摘要。
- **Normalized Frontend Snapshot**：宿主在安装/启用时生成的、可直接提供给后台前端消费的规范化插件前端快照。
- **Bootstrap Document**：后台前端登录后获取的按用户过滤结果，包含可见菜单、路由、扩展点、设置与诊断信息。
- **Extension Slot**：后台前端预留的挂载点，例如内容编辑页侧栏、内容编辑头部、仪表盘挂件区等。
- **Plugin Namespace Route**：宿主保留给插件页面的后台命名空间路由前缀，本文档统一采用 `/admin/x/:plugin/*`。

## Requirements

### Requirement 1: Plugin Frontend Package Contract

#### Acceptance Criteria

1. WHEN a plugin exposes admin UI, THE **Frontend Manifest Validator** SHALL require a frontend manifest referenced from `plugin.toml` and physically located inside the plugin directory.
2. WHEN the frontend manifest is parsed, THE **Frontend Manifest Validator** SHALL validate schema version, SDK compatibility range, contribution identifiers, permission predicates, declared asset files, and declared asset digests before the plugin can be enabled.
3. WHEN a plugin marks its frontend contribution as required and the admin compatibility check fails, THE **Backend Plugin Runtime** SHALL refuse enablement and surface a deterministic error to operators.

### Requirement 2: Normalized Contribution Snapshot Lifecycle

#### Acceptance Criteria

1. WHEN plugin installation or upgrade succeeds, THE **Frontend Manifest Validator** SHALL generate a normalized frontend snapshot containing routes, menus, extension points, field renderers, settings contributions, capability requirements, and asset descriptors.
2. WHEN a plugin is enabled, disabled, upgraded, or uninstalled, THE **Admin Extension Registry API** SHALL invalidate any cached normalized snapshot before serving the next bootstrap response.
3. WHEN duplicate route IDs, menu IDs, slot contribution IDs, or field renderer type bindings are detected across enabled plugins, THE **Frontend Manifest Validator** SHALL reject the conflicting change and report the exact identifiers involved.

### Requirement 3: Plugin Asset Publication

#### Acceptance Criteria

1. WHEN the admin shell requests a plugin asset, THE **Plugin Asset Gateway** SHALL serve only files explicitly declared in the normalized frontend snapshot from immutable, versioned, hashed URLs.
2. WHEN the gateway serves a hashed plugin asset, THE **Plugin Asset Gateway** SHALL emit `Cache-Control`, `ETag`, `Content-Type`, and `X-Content-Type-Options` headers suitable for long-lived caching and safe content handling.
3. WHEN a request path is undeclared, attempts directory traversal, or targets a disabled plugin, THE **Plugin Asset Gateway** SHALL deny access without exposing arbitrary filesystem reads.

### Requirement 4: Per-User Bootstrap Registry

#### Acceptance Criteria

1. WHEN an authenticated admin session initializes, THE **Admin Extension Registry API** SHALL return a bootstrap document containing only the plugin contributions the caller is allowed to access.
2. WHEN plugin state or frontend contribution metadata changes, THE **Admin Extension Registry API** SHALL expose a monotonic revision token that the frontend can use to invalidate stale registries.
3. WHEN a contribution depends on an unavailable shell capability, THE **Admin Extension Registry API** SHALL omit that contribution and include a machine-readable suppression reason in diagnostics.

### Requirement 5: Admin Shell Composition

#### Acceptance Criteria

1. WHEN the admin shell boots after authentication, THE **Admin Shell Composer** SHALL fetch the bootstrap document before rendering plugin-driven navigation or extension slots.
2. WHEN composing navigation, THE **Admin Shell Composer** SHALL merge core and plugin menu items deterministically by zone, order, label, and plugin name.
3. WHEN a user navigates to a plugin namespace route, THE **Admin Shell Composer** SHALL resolve the matching plugin page contribution without blocking unrelated core admin routes.

### Requirement 6: Stable Plugin UI Mount Contract

#### Acceptance Criteria

1. WHEN a plugin page, widget, or field renderer is loaded, THE **Extension Module Host** SHALL use a stable mount/unmount contract instead of requiring a shared React component ABI.
2. WHEN the host mounts a plugin module, THE **Extension Module Host** SHALL provide a typed context including auth state, i18n, theme, navigation helpers, API client, plugin metadata, and an `AbortSignal`.
3. WHEN plugin code throws during load, mount, render, update, or unmount, THE **Extension Module Host** SHALL isolate the failure to the contribution boundary and keep the rest of the admin shell usable.

### Requirement 7: Editor and Custom Field Extension Points

#### Acceptance Criteria

1. WHEN a content field type is provided by a plugin, THE **Admin Shell Composer** SHALL resolve the matching plugin field renderer contribution before using any generic fallback.
2. WHEN a supported editor slot is rendered, THE **Extension Module Host** SHALL mount all matching plugin contributions in deterministic order using the host slot contract.
3. WHEN a plugin field renderer participates in editing, THE **Extension Module Host** SHALL exchange value, validation, dirty-state, and disposal events through the typed SDK contract.

### Requirement 8: Plugin Settings Integration

#### Acceptance Criteria

1. WHEN a plugin registers a settings schema but no custom settings page, THE **Admin Shell Composer** SHALL surface that namespace through a schema-driven settings form in the official admin UI.
2. WHEN a plugin provides a custom settings page, THE **Extension Module Host** SHALL allow the page to read and write only its declared settings namespace through host-provided APIs.
3. WHEN the current user lacks permission for a plugin settings namespace, THE **Admin Extension Registry API** SHALL exclude both the namespace and any related custom settings page from the bootstrap document.

### Requirement 9: Security Controls for Plugin UI Loading

#### Acceptance Criteria

1. WHEN plugin admin assets are enabled, THE **Plugin Asset Gateway** SHALL serve them only from the same origin as the admin shell.
2. WHEN the admin shell is delivered, THE **Admin Shell Composer** SHALL operate under a CSP that disallows `unsafe-inline` and `unsafe-eval` for scripts and restricts script loading to trusted origins.
3. WHEN plugin packages are installed or upgraded, THE **Frontend Manifest Validator** SHALL verify declared asset digests against on-disk files before contributions become visible to the admin shell.

### Requirement 10: Observability and Diagnostics

#### Acceptance Criteria

1. WHEN plugin bootstrap, asset load, module mount, module unmount, or route resolution occurs, THE **Extension Module Host** SHALL emit structured telemetry tagged with plugin name, version, contribution ID, and result.
2. WHEN the system suppresses, disables, or degrades a contribution, THE **Admin Extension Registry API** SHALL expose the reason through a diagnostics API and bootstrap diagnostics block.
3. WHEN CSP or integrity-related controls are introduced or tightened, THE **Admin Shell Composer** SHALL support report-only rollout and violation collection before final enforcement.

### Requirement 11: Operational Resilience and Compatibility

#### Acceptance Criteria

1. WHEN the bootstrap registry cannot be loaded, THE **Admin Shell Composer** SHALL fall back to core admin routes and show a non-blocking degraded-mode notice.
2. WHEN a plugin is disabled or uninstalled during an active session, THE **Admin Shell Composer** SHALL evict stale contributions and close affected plugin views gracefully.
3. WHEN the shell SDK major version changes, THE **Backend Plugin Runtime** SHALL block required incompatible frontend contributions from entering service until they are upgraded.
