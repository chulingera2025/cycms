# cycms

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](./LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.95%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![Edition](https://img.shields.io/badge/Edition-2024-orange?logo=rust)](https://doc.rust-lang.org/edition-guide/)
[![Status](https://img.shields.io/badge/status-alpha-yellow)](#项目状态)

> 一个用 Rust 编写、面向开发者的模块化 Headless CMS。

`cycms` 以 Rust Workspace 组织，后端由 20+ 个解耦的 crate 构成,
通过 REST API 对外提供内容服务；前端是独立的 React SPA。它不绑定特定展示层，
支持自定义内容模型、发布管理、版本回滚、权限控制，并提供 Native 与 WebAssembly
双插件运行时。

---

## 目录

- [特性](#特性)
- [技术栈](#技术栈)
- [项目结构](#项目结构)
- [快速开始](#快速开始)
- [配置](#配置)
- [API](#api)
- [插件系统](#插件系统)
- [项目状态](#项目状态)
- [贡献](#贡献)
- [许可证](#许可证)

---

## 特性

- **动态内容建模**  — 通过接口动态定义内容类型与字段，支持校验规则与 `one-to-one` /
  `one-to-many` / `many-to-many` 关系字段。
- **发布与版本管理**  — 「已发布版本」与「最新草稿」并行存在，支持版本历史、
  回滚、发布 / 取消发布的全生命周期。
- **权限与角色**  — 形如 `domain.resource.action` 的细粒度权限，支持 `own`
  修饰符，基于角色分配。
- **媒体资源管理**  — 上传、MIME 校验、大小限制、引用计数与删除保护。
- **双插件运行时**  — 同时支持 Native Rust 插件（高性能，完全信任）与
  WebAssembly 插件（沙箱隔离，基于 `wasmtime`）。
- **后台前端扩展平台**  — 宿主控制插件前端资产、bootstrap registry、diagnostics 与 telemetry，并将页面、settings page、field renderer、slot 挂载到官方后台。
- **事件总线**  — 基于 `tokio::sync::broadcast` 的事件派发，插件可订阅业务事件。
- **多数据库后端**  — 通过 `sqlx` 支持 **PostgreSQL / MySQL / SQLite**，
  内置迁移引擎，无需 `sqlx-cli`。
- **OpenAPI 文档**  — 服务启动后在 `/api/docs` 动态暴露 OpenAPI 3.1 JSON。
- **可观测性**  — 基于 `tracing`，支持 `pretty` / `json` 日志格式及审计日志开关。
- **类型安全 & 内存安全**  — 整个 workspace 设置 `unsafe_code = "forbid"`，
  开启 `clippy::pedantic` 全量检查。

## 技术栈

### 后端

| 领域     | 依赖                                                     |
|----------|----------------------------------------------------------|
| 语言     | Rust 1.95+ / Edition 2024                                |
| 运行时   | `tokio`                                                  |
| Web 框架 | `axum` 0.8 + `tower` / `tower-http`                      |
| 数据库   | `sqlx` 0.8（PostgreSQL / MySQL / SQLite）                |
| 鉴权     | `jsonwebtoken` 9 + `argon2` 0.5                          |
| 插件     | `wasmtime` + `wasmtime-wasi`（Wasm）/ `libloading`（Native） |
| 配置     | `toml` + `clap` 4                                        |
| 校验     | `jsonschema`                                             |
| 可观测性 | `tracing` + `tracing-subscriber`                         |

### 前端

| 领域     | 依赖               |
|----------|--------------------|
| 框架     | React 19           |
| 路由     | React Router 7     |
| 语言     | TypeScript         |
| 构建工具 | Vite               |

## 项目结构

```
cycms/
├── apps/
│   └── web/                       # React SPA（管理端 + 公开端）
├── crates/                        # Rust Workspace，按能力域拆分
│   ├── cycms-core/                # 核心类型与 trait
│   ├── cycms-kernel/              # 应用装配与生命周期
│   ├── cycms-config/              # TOML + env 配置加载
│   ├── cycms-db/                  # 数据库抽象（Postgres/MySQL/SQLite）
│   ├── cycms-migrate/             # 自研迁移引擎
│   ├── cycms-auth/                # JWT + Argon2 认证
│   ├── cycms-permission/          # RBAC 权限
│   ├── cycms-content-model/       # 内容类型与字段定义
│   ├── cycms-content-engine/      # 内容 CRUD
│   ├── cycms-revision/            # 版本控制
│   ├── cycms-publish/             # 发布管理
│   ├── cycms-media/               # 媒体资源
│   ├── cycms-settings/            # 系统设置
│   ├── cycms-events/              # 事件总线
│   ├── cycms-observability/       # 日志 / 审计
│   ├── cycms-api/                 # REST API 网关（axum）
│   ├── cycms-openapi/             # OpenAPI 文档
│   ├── cycms-plugin-api/          # 插件对外 trait
│   ├── cycms-plugin-manager/      # 插件生命周期
│   ├── cycms-plugin-native/       # Native 插件运行时
│   ├── cycms-plugin-wasm/         # Wasm 插件运行时
│   └── cycms-cli/                 # `cycms` 命令行
├── support/
│   └── cycms-native-loader/       # Native 插件加载器辅助工程
├── cycms.toml                     # 默认配置模板
├── rustfmt.toml
└── clippy.toml
```

## 快速开始

### 环境要求

- Rust **1.95+**（Edition 2024）
- Node.js **18+**（用于前端开发）
- 可选：PostgreSQL / MySQL（默认内置 SQLite 即可直接运行）

### 1. 克隆仓库

```bash
git clone https://github.com/chulingera2025/cycms.git
cd cycms
```

### 2. 准备配置

项目根目录已提供默认 `cycms.toml`。**生产环境务必修改 `[auth].jwt_secret`**——
保留默认占位符时，服务启动只允许绑定回环地址，并会输出警告。

生成一个安全密钥：

```bash
openssl rand -hex 32
```

### 3. 启动后端

```bash
# 运行数据库迁移（SQLite 下会自动创建 data/cycms.db）
cargo run -p cycms-cli -- migrate --config cycms.toml run

# 创建初始超级管理员（--password 留空则自动生成并打印）
cargo run -p cycms-cli -- seed --config cycms.toml \
    --username admin --email admin@example.local

# 启动 HTTP 服务（默认 0.0.0.0:8080）
cargo run -p cycms-cli -- serve --config cycms.toml
```

### 4. 启动前端（开发模式）

```bash
cd apps/web
npm install
npm run dev           # 默认 http://localhost:3000
```

前端开发服务器已开启 CORS 白名单指向 `http://localhost:3000`，登录后即可访问管理后台。

### 5. 发布构建

```bash
# 后端
cargo build --release
# 产物：target/release/cycms-cli

# 前端
cd apps/web && npm run build
# 产物：apps/web/dist
```

## 配置

所有字段都可通过环境变量覆盖，格式 `CYCMS__<SECTION>__<KEY>`（双下划线）。

```bash
export CYCMS__SERVER__PORT=9000
export CYCMS__DATABASE__DRIVER=postgres
export CYCMS__DATABASE__URL="postgres://cycms:cycms@localhost:5432/cycms"
export CYCMS__AUTH__JWT_SECRET="$(openssl rand -hex 32)"
```

核心配置节：

| 节             | 说明                                                   |
|----------------|--------------------------------------------------------|
| `[server]`     | 监听地址、端口、限流、CORS                             |
| `[database]`   | `driver` = `postgres` / `mysql` / `sqlite`，连接串     |
| `[auth]`       | `jwt_secret`、access/refresh token TTL、Argon2 参数    |
| `[media]`      | 上传目录、文件大小上限、允许的 MIME                    |
| `[events]`     | 事件通道容量、handler 超时                             |
| `[observability]` | 日志格式 / 级别、是否写入审计日志                   |
| `[plugins]`    | 插件目录、是否启用 Wasm                                |
| `[admin_extensions]` | 后台插件前端的 CSP、report-only 与最近事件容量   |

完整示例见根目录 [`cycms.toml`](./cycms.toml)。

## API

服务启动后可直接访问：

- **OpenAPI 文档（JSON）**：`GET http://localhost:8080/api/docs`

主要端点（均位于 `/api/v1` 前缀下）：

| 模块        | 端点                                                         |
|-------------|--------------------------------------------------------------|
| 认证        | `/auth/login`、`/auth/register`、`/auth/refresh`、`/auth/me` |
| 内容        | `/content/{type_api_id}`（列表 / 创建 / 详情 / 更新 / 删除） |
| 版本控制    | `/content/{type}/{id}/revisions`、`.../rollback`             |
| 发布        | `/content/{type}/{id}/publish`、`.../unpublish`              |
| 内容类型    | `/content-types`                                             |
| 媒体        | `/media`、`/media/upload`                                    |
| 用户 / 角色 | `/users`、`/roles`、`/roles/permissions`                     |
| 设置        | `/settings/{namespace}[/{key}]`                              |
| 插件        | `/plugins`（安装 / 启用 / 禁用 / 卸载）                      |
| 后台扩展    | `/admin/extensions/bootstrap`、`/admin/extensions/diagnostics`、`/admin/extensions/events` |
| 公开访问    | `/public/content/{type}` 等（仅已发布内容）                  |
| 插件路由    | `/x/{plugin_name}/*`（由已启用插件注册）                     |

内容列表支持分页、排序及 `eq/ne/gt/gte/lt/lte/contains/startsWith/endsWith/in/notIn` 等过滤操作符。

## 插件系统

`cycms` 的插件系统包含业务运行时和后台前端扩展两部分。

### 业务插件运行时

业务插件共享同一套 [`cycms-plugin-api`](./crates/cycms-plugin-api) trait：

- **Native 插件** — 动态库形式，性能接近原生；**完全信任**，无沙箱。
  适合可信第一方扩展。
- **Wasm 插件** — 基于 `wasmtime` + WASI，在沙箱内运行，跨平台分发。
  适合第三方或不受信任的扩展。

### 后台前端扩展

- 已启用插件可以声明 admin frontend manifest，由宿主生成同源资产 URL。
- 宿主通过 bootstrap registry 按权限过滤菜单、页面、settings page、field renderer 和 slot。
- 官方后台通过固定 namespace 路由与 module host 挂载插件页面，并提供 diagnostics 与 recent events。
- `admin_extensions` 配置节控制 CSP、report-only rollout 和最近事件保留容量。

生成插件脚手架：

```bash
cargo run -p cycms-cli -- plugin new my-plugin
```

管理插件：

```bash
cycms plugin install ./path/to/plugin
cycms plugin list
cycms plugin enable  my-plugin
cycms plugin disable my-plugin [--force]
cycms plugin remove  my-plugin
```

插件可以：订阅事件、注册 `/api/v1/x/{name}/*` 自定义路由、使用内置宿主能力（日志、HTTP、设置读写等）。

## CLI

```text
cycms new <path>                        # 初始化一个新项目
cycms serve   [--config cycms.toml]     # 启动 HTTP 服务
cycms migrate [--config cycms.toml] run                # 执行迁移
cycms migrate [--config cycms.toml] rollback [--count N]
cycms seed    [--config cycms.toml] [--username ...] [--email ...] [--password ...]
cycms plugin  new|install|list|enable|disable|remove
```

## 项目状态

当前版本 **0.1.0**（alpha）。核心链路（认证、RBAC、内容模型、发布、版本、媒体、
事件、插件、OpenAPI）均已实现并带有单元 / 集成测试，但：

- API 在进入 1.0 之前可能有破坏性变更；
- 尚未提供官方 Docker 镜像与 CI 工作流；
- 生产部署的最佳实践文档仍在完善中。

欢迎在真实项目中试用并反馈。

## 贡献

在提交 PR 之前，请先在本地跑通：

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

前端：

```bash
cd apps/web
npm run lint
npm run build
```

提交规范：保持小而清晰的提交粒度，每个提交需保证项目可正常构建。

## 许可证

本项目基于 [MIT License](./LICENSE) 发布。
