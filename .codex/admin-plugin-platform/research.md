# Verifiable Research and Technology Proposal

## 1. Core Problem Analysis

CyCMS 当前已经具备后端插件运行时、插件安装/启停状态机和插件后端路由挂载能力，但官方后台前端仍然是静态菜单、静态路由、静态设置页和静态字段渲染器。要让“博客、SEO、SMTP、商城”等插件在安装后把后台页面直接注入官方前端，系统需要新增一套生产可用的前端扩展契约、插件资产分发机制、按权限过滤的前端 bootstrap 接口，以及故障隔离和安全控制。

## 2. Verifiable Technology Recommendations

| Technology/Pattern | Rationale & Evidence |
|---|---|
| **Same-origin ESM plugin modules loaded with `import()`** | Dynamic import can load ECMAScript modules asynchronously on demand and returns a Promise that fulfills to a module namespace object, while network, HTTP, or CORS failures reject asynchronously instead of throwing synchronously [cite:3]. Vite supports production code-splitting for dynamic imports and rewrites async chunk loading to reduce round trips for shared chunks [cite:2]. Because `import()` only recognizes import attributes in its options parameter, while Subresource Integrity is defined for `script` and qualifying `link` elements, browser-side SRI is not the primary enforcement point for modules loaded through `import()` [cite:3][cite:5]. CyCMS should therefore load plugin admin bundles only from its own origin and rely on install-time digest validation plus server-side asset control instead of cross-origin remote bundles [cite:4][cite:5]. |
| **Static admin shell plus namespaced plugin route space** | `createBrowserRouter` constructs a data router from a `RouteObject[]` route tree [cite:1]. React Router also exposes `patchRoutesOnNavigation` specifically for advanced micro-frontend cases where the full route tree cannot be known up front [cite:1]. For CyCMS, the production-stable choice is to keep the host shell route tree static and reserve a plugin namespace such as `/admin/x/:plugin/*`, which eliminates top-level route collisions while still leaving `patchRoutesOnNavigation` available for future large-scale route discovery needs [cite:1]. |
| **Strict CSP with nonce support and report-only rollout** | CSP restricts which resources the browser may load and is documented by MDN as a defense-in-depth control against XSS, but it does not replace input sanitization [cite:4]. MDN recommends nonce- or hash-based strict CSPs and warns against `unsafe-inline` and `unsafe-eval` because they weaken XSS protection [cite:4]. Vite supports CSP nonce propagation via `html.cspNonce` and requires that the placeholder be replaced with a unique nonce value for each response [cite:2]. CSP can be introduced with `Content-Security-Policy-Report-Only` and violation reporting endpoints before enforcing the final policy [cite:4]. |
| **Hashed plugin assets with server-side digest validation** | Subresource Integrity verifies fetched resources against cryptographic hashes and supports `sha256`, `sha384`, and `sha512` digests [cite:5]. Cross-origin SRI requires CORS and `crossorigin`, and browsers support `Integrity-Policy-Report-Only` before enforcement [cite:5]. Vite emits async chunks, CSS code splitting, and modulepreload hints for production assets [cite:2]. CyCMS should therefore require plugin frontend builds to emit hashed assets and an asset manifest whose digests are verified at install/upgrade time before assets are exposed to the admin shell [cite:2][cite:5]. |
| **Host-controlled bootstrap registry filtered by permissions** | `createBrowserRouter` is designed around a host-owned route tree and router initialization flow [cite:1]. CSP allows script loading to be constrained to same-origin or explicitly trusted origins via fetch directives such as `script-src` and `default-src` [cite:4]. A host-owned bootstrap registry lets CyCMS decide which plugin menus, routes, widgets, and field renderers are visible for the current user before any plugin JavaScript is fetched, which aligns the frontend loading path with the same trust boundary already enforced by the backend shell [cite:1][cite:4]. |

## 3. Browsed Sources

- [1] https://reactrouter.com/api/data-routers/createBrowserRouter
- [2] https://vite.dev/guide/features.html
- [3] https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/import
- [4] https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP
- [5] https://developer.mozilla.org/en-US/docs/Web/Security/Subresource_Integrity
