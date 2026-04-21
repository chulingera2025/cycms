# Design Document

## 1. Overview

本设计把 CyCMS 的“插件能力”从当前的后端扩展，提升为“后端插件 + 官方后台前端可注入扩展平台”。设计目标不是让插件随意篡改宿主前端，而是在宿主掌控下为插件提供稳定、可版本化、可观测、可治理的注入面。

本方案采用以下生产设计原则：

1. **同源信任模型**：插件前端资产只从 CyCMS 自身域名加载，不依赖跨域远程脚本。
2. **宿主控制注册**：前端永远先拿到宿主生成的 bootstrap，再决定加载哪些插件资产。
3. **框架无关挂载边界**：插件页面/挂件/字段渲染器通过 `mount/unmount` 合约挂载，而不是共享 React 组件 ABI。
4. **命名空间路由**：插件后台页面统一挂在 `/admin/x/:plugin/*` 下，避免与官方路由和其他插件冲突。
5. **声明式 + 嵌入式双层贡献模型**：菜单、设置、权限、字段类型可以声明式扩展；复杂后台页面和挂件通过模块挂载扩展。
6. **后端强授权，前端弱可见性**：bootstrap 只做可见性过滤，所有真实读写授权仍由后端 API 判定。

## 2. Current Baseline and Constraints

当前仓库的基线约束必须被正视：

- `crates/cycms-plugin-manager/src/manifest.rs` 已有 `frontend` 段，但仅包含 `entry` 字面量，且未被运行时代码消费。
- `crates/cycms-api/src/handlers/plugins.rs` 当前插件 API 不返回任何前端贡献信息。
- `apps/web/src/components/admin/AdminLayout.tsx` 的后台菜单是硬编码的。
- `apps/web/src/routes/index.tsx` 的后台路由是静态定义的。
- `apps/web/src/pages/admin/SettingsPage.tsx` 没有消费后端已有的 settings schema 列表接口。
- `apps/web/src/features/content/FieldRenderer.tsx` 只识别官方内建字段类型，没有插件字段渲染器注册表。
- `crates/cycms-plugin-native/src/runtime.rs` 已注明跨 dylib 目前只稳定保证生命周期钩子；因此前端注入设计不能依赖“动态库路径天然能提供所有后端行为”。

这些约束意味着新设计必须把“前端扩展信息”显式变成宿主可验证、可缓存、可通过 API 暴露的第一等对象，而不是寄希望于前端自己扫插件目录。

## 3. Proposed On-Disk Package Contract

### 3.1 plugin.toml 扩展

```toml
[plugin]
name = "blog"
version = "1.4.0"
kind = "wasm"
entry = "target/plugin.wasm"

[compatibility]
cycms = ">=0.4.0 <0.5.0"

[frontend]
manifest = "admin/manifest.json"
required = true
```

### 3.2 admin/manifest.json

前端 manifest 不再只是“一个入口 JS 路径”，而是插件前端贡献的完整描述文件：

```json
{
  "schemaVersion": 1,
  "sdkVersion": "^1.0.0",
  "pluginName": "blog",
  "pluginVersion": "1.4.0",
  "assets": [
    {
      "id": "route.blog.write",
      "path": "routes/blog-write.8f1a0c8a.js",
      "sha384": "sha384-...",
      "contentType": "text/javascript",
      "styles": ["styles/blog-write.9928e4f1.css"]
    }
  ],
  "menus": [
    {
      "id": "menu.blog.write",
      "label": "博客写作",
      "zone": "content",
      "icon": "book-open",
      "order": 220,
      "to": "/write",
      "requiredPermissions": ["content.entry.create"]
    }
  ],
  "routes": [
    {
      "id": "route.blog.write",
      "path": "/write",
      "moduleAssetId": "route.blog.write",
      "kind": "page",
      "title": "写博客",
      "requiredPermissions": ["content.entry.create"],
      "match": { "contentTypeApiIds": ["blog_post"] }
    }
  ],
  "slots": [
    {
      "id": "slot.blog.seo-sidebar",
      "slot": "content.editor.sidebar",
      "moduleAssetId": "slot.blog.seo-sidebar",
      "order": 120,
      "match": { "contentTypeApiIds": ["blog_post"] }
    }
  ],
  "fieldRenderers": [
    {
      "id": "field.blog.seo-meta",
      "typeName": "blog.seo_meta",
      "moduleAssetId": "field.blog.seo-meta"
    }
  ],
  "settings": {
    "namespace": "blog",
    "customPage": {
      "path": "/settings",
      "moduleAssetId": "settings.blog.page"
    }
  }
}
```

### 3.3 Why `manifest`, not `entry`

生产环境里插件前端往往包含多个入口、代码分块、CSS 资产、不同贡献类型和权限声明。单个 `entry` 无法表达：

- 多个页面与多个挂件。
- JS 与 CSS 资产之间的依赖关系。
- 每个贡献的权限、顺序和匹配规则。
- 后端安装时需要验证的文件摘要集合。

因此 `frontend.manifest` 是宿主与插件前端之间的正式契约，`frontend.entry` 不再满足需求。

## 4. Component Specifications

### Component: Backend Plugin Runtime

**Purpose**: 负责插件安装、升级、启停和前端扩展生命周期联动。

**Location**:

- `crates/cycms-plugin-manager/src/service.rs`
- `crates/cycms-plugin-manager/src/manifest.rs`
- `crates/cycms-plugin-manager/src/discovery.rs`
- `crates/cycms-plugin-manager/src/frontend_manifest.rs`（新增）
- `crates/cycms-plugin-manager/src/frontend_snapshot.rs`（新增）

**Interface**:

```rust
pub struct FrontendSpec {
    pub manifest: String,
    pub required: bool,
}

pub struct NormalizedFrontendSnapshot {
    pub schema_version: u32,
    pub sdk_version: String,
    pub plugin_name: String,
    pub plugin_version: String,
    pub routes: Vec<NormalizedRouteContribution>,
    pub menus: Vec<NormalizedMenuContribution>,
    pub slots: Vec<NormalizedSlotContribution>,
    pub field_renderers: Vec<NormalizedFieldRendererContribution>,
    pub settings: Option<NormalizedSettingsContribution>,
    pub assets: Vec<NormalizedAssetDescriptor>,
}

impl PluginManager {
    pub async fn install_as(&self, source: &DiscoveredPlugin, actor_id: Option<&str>) -> Result<PluginInfo>;
    pub async fn enable_as(&self, name: &str, actor_id: Option<&str>) -> Result<()>;
    pub async fn rebuild_frontend_snapshot_cache(&self) -> Result<()>;
}
```

**Behavior**:

- 插件安装时读取 `plugin.toml`；若存在 `[frontend]`，则读取并校验 `frontend.manifest`。
- 规范化后的 snapshot 作为插件 manifest JSON 的一部分持久化，避免前端 bootstrap 每次重新扫磁盘。
- `frontend.required = true` 且兼容性失败时，`enable_as` 必须失败；`required = false` 时，后端插件可启用，但前端贡献进入 suppressed 状态并出现在 diagnostics 中。
- 启用、禁用、卸载、升级任何一个插件时，都要刷新 frontend snapshot cache 和 revision token。

**Implements**: Req 1.1, 1.3, 2.1, 2.2, 11.3.

### Component: Frontend Manifest Validator

**Purpose**: 负责插件前端 manifest 的 schema 校验、摘要校验、冲突校验和规范化。

**Location**:

- `crates/cycms-plugin-manager/src/frontend_manifest.rs`（新增）
- `crates/cycms-plugin-manager/src/frontend_snapshot.rs`（新增）
- `crates/cycms-plugin-manager/src/error.rs`（扩展错误类型）

**Interface**:

```rust
pub struct AdminFrontendManifest {
    pub schema_version: u32,
    pub sdk_version: String,
    pub plugin_name: String,
    pub plugin_version: String,
    pub assets: Vec<FrontendAsset>,
    pub menus: Vec<MenuContribution>,
    pub routes: Vec<RouteContribution>,
    pub slots: Vec<SlotContribution>,
    pub field_renderers: Vec<FieldRendererContribution>,
    pub settings: Option<SettingsContribution>,
}

pub fn load_frontend_manifest(plugin_dir: &Path, spec: &FrontendSpec) -> Result<AdminFrontendManifest>;
pub fn validate_frontend_manifest(manifest: &AdminFrontendManifest, plugin_dir: &Path) -> Result<()>;
pub fn normalize_frontend_manifest(manifest: AdminFrontendManifest) -> Result<NormalizedFrontendSnapshot>;
pub fn validate_cross_plugin_conflicts(snapshots: &[NormalizedFrontendSnapshot]) -> Result<()>;
```

**Validation Rules**:

- `plugin_name` 和 `pluginVersion` 必须与 `plugin.toml` 一致。
- 所有 `id` 必须在单插件内唯一。
- 资产文件必须位于插件目录内且必须存在。
- `sha384` 必须与磁盘文件实际摘要一致。
- `moduleAssetId` 必须引用存在的 JS 资产。
- `styles` 只能引用存在的 CSS 资产。
- 菜单 `to`、路由 `path`、settings custom page `path` 必须是插件 namespace 内的相对路径，而不能自定义根级后台路径。
- 跨插件冲突包括：route ID、menu ID、slot contribution ID、field renderer `typeName`。

**Production Decision**:

插件 UI 不允许直接声明宿主根级路径，例如 `/admin/users`。所有页面必须是插件自身命名空间内的相对路径，最终由宿主解析为 `/admin/x/{plugin}/{relative-path}`。

**Implements**: Req 1.1, 1.2, 2.1, 2.3, 9.3.

### Component: Admin Extension Registry API

**Purpose**: 负责向官方后台前端提供按权限过滤后的插件前端 bootstrap 与诊断信息。

**Location**:

- `crates/cycms-api/src/handlers/admin_extensions.rs`（新增）
- `crates/cycms-api/src/lib.rs`
- `crates/cycms-api/src/state.rs`
- `apps/web/src/lib/api/admin-extensions.ts`（新增）

**Interface**:

```rust
GET /api/v1/admin/extensions/bootstrap
GET /api/v1/admin/extensions/diagnostics
POST /api/v1/admin/extensions/events
```

```json
{
  "revision": "extrev:2026-04-21T10:22:17Z:42",
  "shellSdkVersion": "1.0.0",
  "plugins": [
    {
      "name": "blog",
      "version": "1.4.0",
      "routes": [
        {
          "id": "route.blog.write",
          "fullPath": "/admin/x/blog/write",
          "title": "写博客",
          "moduleUrl": "/api/v1/plugin-assets/blog/1.4.0/8f1a0c8a/routes/blog-write.8f1a0c8a.js",
          "styles": [
            "/api/v1/plugin-assets/blog/1.4.0/9928e4f1/styles/blog-write.9928e4f1.css"
          ]
        }
      ],
      "menus": [
        {
          "id": "menu.blog.write",
          "zone": "content",
          "label": "博客写作",
          "to": "/admin/x/blog/write"
        }
      ],
      "diagnostics": []
    }
  ],
  "diagnostics": []
}
```

**Filtering Rules**:

- 只返回已启用插件的 frontend snapshot。
- 只返回当前用户拥有权限的贡献。
- 只返回当前 shell SDK 能理解的贡献。
- suppressed contribution 不返回到正常数组，但要进入 diagnostics。

**Operational Rules**:

- `bootstrap` 响应必须带 `ETag` 和 revision header。
- 所有后台 API 响应都应附带当前 extension revision header，前端据此后台刷新 registry。
- diagnostics API 只对具有插件管理权限的管理员开放。

**Implements**: Req 2.2, 4.1, 4.2, 4.3, 8.3, 10.2.

### Component: Plugin Asset Gateway

**Purpose**: 负责把插件前端资产以同源、白名单、不可变资源的形式提供给后台前端。

**Location**:

- `crates/cycms-api/src/handlers/plugin_assets.rs`（新增）
- `crates/cycms-api/src/lib.rs`

**Interface**:

```rust
GET /api/v1/plugin-assets/{plugin}/{version}/{digest}/{*path}
```

**Resolution Rules**:

- 只有 snapshot 中声明过的文件可被访问。
- URL 中的 `{digest}` 既是缓存键，也是服务端快速一致性检查键。
- 网关不接受目录索引，不暴露目录 listing。
- 所有路径都必须在插件目录内 canonicalize 后再次校验是否仍落在插件根目录中。

**Response Rules**:

- `Cache-Control: public, max-age=31536000, immutable`
- `ETag` 使用文件摘要或 snapshot 内摘要
- `X-Content-Type-Options: nosniff`
- 正确的 `Content-Type`
- 对禁用插件或不存在资产返回 404，而不是 403，避免泄漏内部布局

**Security Rules**:

- 只接受同源调用，不为外部站点设计跨域共享。
- 不支持通过 query string 指向任意磁盘文件。
- 不依赖浏览器 SRI 执行动态 import 的校验；摘要在安装/升级阶段完成验证。

**Implements**: Req 3.1, 3.2, 3.3, 9.1.

### Component: Admin Shell Composer

**Purpose**: 负责在官方后台前端中加载 bootstrap、合成菜单与插件 namespace 路由、对接设置页与扩展点。

**Location**:

- `apps/web/src/main.tsx`
- `apps/web/src/routes/index.tsx`
- `apps/web/src/components/admin/AdminLayout.tsx`
- `apps/web/src/features/extensions/bootstrap.ts`（新增）
- `apps/web/src/features/extensions/store.ts`（新增）
- `apps/web/src/features/extensions/PluginRouteOutlet.tsx`（新增）
- `apps/web/src/features/settings/PluginSettingsNamespacePage.tsx`（新增）

**Interface**:

```ts
export interface AdminExtensionsBootstrap {
  revision: string;
  shellSdkVersion: string;
  plugins: BootstrapPlugin[];
  diagnostics: BootstrapDiagnostic[];
}

export async function loadAdminExtensionsBootstrap(): Promise<AdminExtensionsBootstrap>;
export function createAdminExtensionRegistry(doc: AdminExtensionsBootstrap): AdminExtensionRegistry;
export function resolvePluginRoute(registry: AdminExtensionRegistry, plugin: string, subpath: string): BootstrapRoute | null;
```

**Routing Model**:

- React Router 顶层仍保持静态。
- 新增固定路由：`/admin/x/:pluginName/*`。
- `PluginRouteOutlet` 读取 wildcard 子路径并在 registry 中匹配插件路由定义。
- 插件菜单项只能指向该命名空间内的路径。

**Why Namespace Route**:

- 避免插件与核心后台路由冲突。
- 避免前端在 router 初始化阶段必须注入任意根级 route tree。
- 把插件 UI 生命周期和权限边界集中在一个宿主受控入口上。

**Menu Composition**:

- 现有 AdminLayout 的硬编码菜单改为 `core + plugin` 合成。
- zone 统一枚举，例如 `dashboard`, `content`, `media`, `commerce`, `marketing`, `settings`, `tools`。
- 同 zone 内先按 `order`，再按 `label`，最后按 `pluginName` 排序。

**Fallback Behavior**:

- bootstrap 拉取失败时，核心后台照常可用。
- 插件菜单不渲染，顶部显示 degraded mode 提示。
- revision header 变化时后台刷新 registry；若当前正在插件页且目标贡献消失，则优雅跳回插件列表页或核心首页。

**Settings Integration**:

- 核心设置页改为动态 namespace 列表。
- 有 schema 无 custom page 的插件，显示 schema-driven 表单。
- 有 custom page 的插件，可在设置页显示入口，但实际内容由 `Extension Module Host` 挂载。

**Implements**: Req 4.1, 5.1, 5.2, 5.3, 8.1, 11.1, 11.2.

### Component: Extension Module Host

**Purpose**: 负责按统一合约挂载插件页面、挂件和字段渲染器，并在 UI 层隔离错误和生命周期。

**Location**:

- `apps/web/src/features/extensions/ModuleHost.tsx`（新增）
- `apps/web/src/features/extensions/slots.tsx`（新增）
- `apps/web/src/features/extensions/field-renderers.ts`（新增）
- `packages/admin-plugin-sdk/src/contracts.ts`（新增）
- `packages/admin-plugin-sdk/src/index.ts`（新增）

**Interface**:

```ts
export interface CycmsMountContext {
  plugin: { name: string; version: string };
  route?: { id: string; path: string; params: Record<string, string> };
  auth: { userId: string; roles: string[] };
  theme: { mode: 'light' | 'dark'; tokens: Record<string, string> };
  i18n: { locale: string; t(key: string, vars?: Record<string, unknown>): string };
  api: CycmsAdminApiClient;
  navigation: { push(to: string): void; replace(to: string): void; back(): void };
  abortSignal: AbortSignal;
  host: { emit(event: HostEvent): void; on(event: string, handler: HostHandler): () => void };
}

export interface CycmsMountedModule {
  update?(ctx: CycmsMountContext): void | Promise<void>;
  unmount?(): void | Promise<void>;
}

export interface CycmsMountableModule {
  mount(target: HTMLElement, ctx: CycmsMountContext):
    | void
    | CycmsMountedModule
    | Promise<void | CycmsMountedModule>;
}
```

**Key Decisions**:

- 插件不直接向宿主返回 React 组件类型。
- 插件模块内部可以自行使用 React/Vue/Solid 等框架，但宿主只认 `mount/unmount`。
- 字段渲染器本质上也是特殊的 mountable module，只是上下文里多了 `value`, `onChange`, `validate`, `disabled`, `locale` 等字段。

**Field Renderer Contract**:

```ts
export interface CycmsFieldRendererContext extends CycmsMountContext {
  field: {
    apiId: string;
    name: string;
    typeName: string;
    required: boolean;
    config: Record<string, unknown>;
  };
  value: unknown;
  disabled: boolean;
  setValue(next: unknown): void;
  setValidation(result: { valid: boolean; message?: string }): void;
}
```

**Lifecycle Rules**:

- ModuleHost 先确保 CSS 依赖已加载，再执行 JS module import 和 `mount`。
- route/slot/field renderer 都包裹在独立错误边界内。
- 卸载时调用 `unmount`；若 route 变化但仍是同一贡献，可选调用 `update`。
- 任一插件异常只影响当前贡献容器，不得让整个后台崩溃。

**Telemetry Rules**:

- 记录 `load_start`, `load_success`, `load_error`, `mount_success`, `mount_error`, `unmount_error`。
- 事件维度带 `plugin`, `version`, `contributionId`, `kind`, `durationMs`。

**Implements**: Req 6.1, 6.2, 6.3, 7.2, 7.3, 10.1.

## 5. Route Resolution Model

插件路由全部使用相对路径，宿主统一解析为：

```text
/admin/x/{pluginName}/{route.path without leading slash}
```

示例：

- 插件：`blog`
- manifest route path：`/write`
- 最终后台路径：`/admin/x/blog/write`

匹配流程：

1. React Router 命中 `/admin/x/:pluginName/*`。
2. `PluginRouteOutlet` 读取 `pluginName` 和剩余子路径。
3. 在 registry 中找到该插件的 route 列表。
4. 通过宿主的 path matcher 匹配子路径。
5. 找到贡献后交给 `Extension Module Host` 加载页面模块。

这样做的结果是：

- 核心路由树不需要随着插件数量增长而膨胀。
- 插件内部路由冲突被限制在自身 namespace 下。
- 前端路由注入能力生产可用，但实现复杂度明显低于任意根级动态注入。

## 6. Settings Model

### 6.1 默认模式

插件只注册 settings schema，不提供 custom page。宿主在官方设置页面里自动生成 namespace 对应的表单。

### 6.2 自定义模式

插件既注册 settings schema，又提供 custom page。schema 仍然用于后端校验和通用导出；custom page 只负责更好的运营 UI。

### 6.3 权限模式

settings namespace 在 bootstrap 时按权限过滤。未授权的 namespace 不应在前端可见，即使用户手输 URL，也必须被后端 API 拒绝。

## 7. Security Model

### 7.1 Asset Trust

- 插件前端资产只允许同源加载。
- 插件安装/升级时验证摘要。
- 资产 URL 使用版本 + digest 命名，避免缓存污染。

### 7.2 Browser Policy

- 后台页面启用严格 CSP。
- `script-src` 不允许 `unsafe-inline` 和 `unsafe-eval`。
- 插件资产仍然属于同源 `script-src 'self'` 范畴，不额外放开第三方域名。
- 先启用 report-only，再切 enforce。

### 7.3 API Trust

- bootstrap 只做“是否展示”过滤。
- 真实授权仍在后端内容、设置、插件 API 中执行。
- 插件前端即便被直接访问，其所有读写仍受后端鉴权约束。

## 8. Revision and Cache Invalidation

### 8.1 Revision Source

revision token 由以下因素共同决定：

- 已启用插件集合
- 各插件 version
- 各插件 frontend snapshot hash
- shell SDK major/minor version

### 8.2 Client Strategy

- bootstrap 初次加载后缓存到内存。
- 每次后台 API 响应带回 `X-CyCMS-Extension-Revision`。
- 若发现 revision 变化，前端后台刷新 registry。
- 若当前页面贡献已失效，优雅跳转并提示“插件页面已更新或下线”。

## 9. Failure Modes and Expected Outcomes

| Failure Mode | Expected Outcome |
|---|---|
| 插件 frontend manifest 缺失 | required=true 时拒绝启用；required=false 时后端插件可启用但无 UI 贡献。 |
| 资产摘要不匹配 | 拒绝安装或拒绝升级。 |
| 插件菜单权限不足 | bootstrap 中直接过滤该菜单与关联页面。 |
| 插件页面加载 404 | 当前贡献容器显示错误态，宿主后台不崩溃。 |
| 插件模块运行时异常 | 错误边界拦截，记录 telemetry，保留核心后台可用。 |
| bootstrap API 故障 | 核心后台进入 degraded mode，插件 UI 暂时不可用。 |
| 运行中的插件被禁用 | revision 变化触发 registry 刷新并收回对应页面/挂件。 |

## 10. Rollout Strategy

1. 先上线后端 manifest 校验、asset gateway 和 bootstrap/diagnostics API。
2. 再上线前端 registry、插件 namespace 路由和菜单合成，但只启用 diagnostics，不加载任何插件模块。
3. 然后启用 page mount contract。
4. 最后启用 slot 和 field renderer 扩展。
5. CSP 先 report-only，再强制。

## 11. Summary

这套设计不是“让插件前端自己接管后台”，而是“宿主前端在明确、安全、可诊断的边界内调度插件 UI”。

对博客类插件，这意味着：

- 后端插件负责预置内容类型、权限、设置、后端 API。
- 前端贡献负责提供博客写作页、SEO 侧栏、自定义字段渲染器、博客设置页。
- 官方前端仍然是宿主，负责菜单、鉴权、主题、国际化、错误边界、诊断和资产加载策略。
