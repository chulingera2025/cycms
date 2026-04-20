# Implementation Plan — cycms v0.1

## Task Overview

任务按实现依赖顺序排列，每个顶级任务对应一个可独立交付的模块。

---

- [x] 1. 初始化项目骨架与 Workspace
  - [x] 1.1 创建 Cargo.toml workspace 配置，定义所有 crate 成员路径
  - [x] 1.2 创建所有 crate 目录结构（cycms-core 到 cycms-cli，共 22 个 crate）
  - [x] 1.3 创建 cycms-core crate：定义公共 Error 类型、Result 别名、公共 trait
  - [x] 1.4 创建默认 cycms.toml 配置文件模板
  - [x] 1.5 配置 rustfmt.toml 和 clippy 规则
  - [x] 1.6 创建 cycms-kernel crate 骨架（AppContext 定义、bootstrap/serve/shutdown 签名）
  - _Requirements: (基础设施，所有需求的前置)_

- [x] 2. 实现 ConfigManager（cycms-config）
  - [x] 2.1 定义 AppConfig / ServerConfig / DatabaseConfig / AuthConfig / MediaConfig / PluginsConfig 结构体
  - [x] 2.2 实现 TOML 文件加载（使用 toml crate）
  - [x] 2.3 实现环境变量覆盖机制（CYCMS__SECTION__KEY 格式）
  - [x] 2.4 编写配置加载单元测试
  - _Requirements: 15.1_

- [x] 3. 实现 DatabaseLayer（cycms-db）
  - [x] 3.1 定义 DatabasePool 枚举（Postgres/MySql/Sqlite）
  - [x] 3.2 实现连接池创建（sqlx PgPool/MySqlPool/SqlitePool）
  - [x] 3.3 实现 JSONB 查询辅助函数（PG 原生 vs MySQL/SQLite JSON 函数）
  - [x] 3.4 编写连接池创建和查询辅助的集成测试
  - _Requirements: 19.1, 19.2, 19.3_

- [x] 4. 实现 MigrationEngine（cycms-migrate）
  - [x] 4.1 定义迁移记录表 DDL 和 MigrationRecord 结构体
  - [x] 4.2 实现系统迁移执行器（读取 migrations/ 目录，按版本顺序执行）
  - [x] 4.3 实现插件迁移执行器（按插件名独立追踪）
  - [x] 4.4 实现迁移回滚逻辑
  - [x] 4.5 编写系统核心表的初始迁移文件（users、roles、permissions 等所有基础表）
  - _Requirements: 14.1, 14.2, 14.3, 14.4_

- [x] 5. 实现 AuthEngine（cycms-auth）
  - [x] 5.1 定义 User 模型和数据库 CRUD
  - [x] 5.2 实现 Argon2id 密码哈希/验证
  - [x] 5.3 实现 JWT Token 生成/验证（access_token + refresh_token）
  - [x] 5.4 实现登录接口逻辑（凭证验证 → Token 颁发）
  - [x] 5.5 实现 Token 刷新逻辑
  - [x] 5.6 实现初始管理员创建逻辑（仅系统无用户时）
  - [x] 5.7 实现 axum 认证中间件
  - [x] 5.8 编写认证流程单元测试和集成测试
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6_

- [x] 6. 实现 PermissionEngine（cycms-permission）
  - [x] 6.1 定义 Role / Permission 模型和数据库 CRUD
  - [x] 6.2 实现权限格式解析（domain.resource.action）
  - [x] 6.3 实现权限检查逻辑（含 scope=own 判断）
  - [x] 6.4 实现插件权限点注册接口
  - [x] 6.5 实现默认角色种子数据（super_admin / editor / author）
  - [x] 6.6 实现 axum 权限中间件工厂 require_permission()
  - [x] 6.7 编写权限检查单元测试
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

- [x] 7. 实现 EventBus（cycms-events）
  - [x] 7.1 定义 Event trait 和 EventKind 枚举
  - [x] 7.2 实现 EventBus 结构体（tokio broadcast channel）
  - [x] 7.3 实现 EventHandler trait 和注册/取消注册接口
  - [x] 7.4 实现事件的异步分发逻辑（失败处理：日志记录不阻断）
  - [x] 7.5 编写事件发布/订阅单元测试
  - _Requirements: 9.1, 9.2, 9.3, 9.4_

- [x] 8. 实现 SettingsManager（cycms-settings）
  - [x] 8.1 定义 Settings 模型和数据库 CRUD（namespace + key + value JSON）
  - [x] 8.2 实现 get_settings / set_settings / delete_settings 方法
  - [x] 8.3 实现插件 settings schema 注册和校验
  - [x] 8.4 编写设置存取单元测试
  - _Requirements: 15.2, 15.3_

- [x] 9. 实现 ServiceRegistry 与 Plugin API（cycms-plugin-api）
  - [x] 9.1 定义 ServiceRegistry 结构体（Arc<RwLock<HashMap>>）
  - [x] 9.2 实现 register / resolve / resolve_all / set_unavailable 方法
  - [x] 9.3 定义 Plugin trait 和 PluginContext（插件 API 边界）
  - [x] 9.4 实现启动时核心服务批量注册
  - [x] 9.5 编写服务注册/解析单元测试
  - _Requirements: 13.1, 13.2, 13.3_

- [x] 10. 实现 ContentModel（cycms-content-model）
  - [x] 10.1 定义 ContentType / ContentField / FieldKind 模型
  - [x] 10.2 实现 ContentType CRUD（创建、更新字段、删除类型）
  - [x] 10.3 实现字段校验规则引擎（required/min/max/pattern/custom）
  - [x] 10.4 实现 JSON Schema 输出生成（从 ContentType 生成 schema）
  - [x] 10.5 实现默认内置类型（page / post）种子数据
  - [x] 10.6 编写内容类型 CRUD 和校验单元测试
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_

- [x] 11. 实现 ContentEngine（cycms-content-engine）
  - [x] 11.1 定义 ContentEntry / ContentData 模型
  - [x] 11.2 实现内容 CRUD（创建/读取/更新/删除）
  - [x] 11.3 实现 field 校验调用（创建/更新时触发 ContentModel 校验）
  - [x] 11.4 实现内容查询引擎（分页、排序、筛选操作符 eq/ne/gt/gte/lt/lte/contains/startsWith/endsWith/in/notIn/null/notNull、JSONB 字段查询、populate 关联加载）
  - [x] 11.5 集成 EventBus 触发 content.created / content.updated / content.deleted / content.published 事件
  - [x] 11.6 实现删除前关联引用检查与软/硬删除切换（受 MediaConfig/ContentConfig 控制）
  - [x] 11.7 编写内容 CRUD 集成测试
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7_

- [x] 12. 实现 RevisionManager（cycms-revision）
  - [x] 12.1 定义 ContentRevision 模型
  - [x] 12.2 实现创建修订版本逻辑（内容更新时自动触发）
  - [x] 12.3 实现修订历史查询（按 entry 分页列表）
  - [x] 12.4 实现修订回滚（指定版本恢复为当前内容）
  - [x] 12.5 编写修订版本集成测试
  - _Requirements: 5.1, 5.2, 5.3, 5.4_

- [x] 13. 实现 PublishManager（cycms-publish）
  - [x] 13.1 定义发布状态机（draft → published，支持撤回与归档）
  - [x] 13.2 实现发布接口（状态转换 + published_version 绑定 + published_at 设置）
  - [x] 13.3 实现撤回接口（清除 published_version，状态回到 draft）
  - [x] 13.4 实现外部查询默认仅返回 published 版本、管理端可查所有状态的过滤逻辑
  - [x] 13.5 集成 EventBus 触发 content.published / content.unpublished 事件
  - [x] 13.6 编写发布流程集成测试
  - _Requirements: 6.1, 6.2, 6.3, 6.4_

- [x] 14. 实现 MediaManager（cycms-media）
  - [x] 14.1 定义 MediaAsset 模型和数据库 CRUD（含按 MIME/文件名/时间筛选与分页）
  - [x] 14.2 实现本地文件系统存储驱动（v0.1 默认）与 StorageBackend trait 抽象
  - [x] 14.3 实现文件上传接口（multipart 解析 + metadata 提取）
  - [x] 14.4 实现删除前关联引用检查与删除处理（阻止或警告，按配置）
  - [x] 14.5 编写上传和检索集成测试
  - _Requirements: 7.1, 7.2, 7.3, 7.4_

- [x] 15. 实现 PluginManager（cycms-plugin-manager）
  - [x] 15.1 定义 PluginManifest 结构体（plugin.toml 解析）
  - [x] 15.2 定义 Plugin 数据库模型和 CRUD（安装/启用/禁用/卸载状态）
  - [x] 15.3 实现插件目录扫描和 manifest 加载
  - [x] 15.4 实现插件依赖检查（version range 解析、拓扑排序）
  - [x] 15.5 实现插件生命周期管理（install → activate → deactivate → uninstall）
  - [x] 15.6 实现 PluginContext 注入构造
  - [x] 15.7 编写插件生命周期和依赖解析单元测试
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 10.6, 20.1, 20.2, 20.3, 20.4, 20.5_

- [x] 16. 实现 NativePluginRuntime（cycms-plugin-native）
  - [x] 16.1 定义 Plugin trait（name/version/on_enable/on_disable/routes/event_handlers/services）与 PluginContext 注入结构
  - [x] 16.2 实现 NativePluginRuntime 核心：通过 `Arc<dyn Plugin>` 管理插件实例，执行 on_enable/on_disable 生命周期钩子
  - [x] 16.3 实现插件 axum Router 合并到主路由表
  - [x] 16.4 实现插件 EventHandler 注册到 EventBus 与按插件名批量注销
  - [x] 16.5 实现插件 services 注册到 ServiceRegistry（typed contract 键）
  - [x] 16.6 编写 Native 插件加载集成测试（使用测试用子 crate 插件）
  - _Requirements: 11.1, 11.2, 11.3, 11.4_

- [x] 17. 实现 WasmPluginRuntime（cycms-plugin-wasm）
  - [x] 17.1 定义 WIT 接口规范文件（host ↔ guest 双向接口，位于 `wit/` 目录）；host 组包含 `db`（原始 SQL），`deps/` 放 WASI preview 2 标准接口
  - [x] 17.2 基于 `wasmtime::component::*` 初始化 Engine 与 Linker（开 `async_support` + `wasm_component_model`），加载 Component Model 组件；通过 `wasmtime-wasi` 完整透传 WASI preview 2
  - [x] 17.3 实现 Host Functions 10 组：content / auth / permission / kv / http / event / route / log / settings / db（**完全访问，不做白名单或沙箱约束**）
  - [x] 17.4 实现 Wasm 组件实例化和生命周期管理（per-plugin Store + HostState）
  - [x] 17.5 实现 trap / panic 故障隔离：wasmtime Trap 与宿主 panic 映射为 `Error::PluginError` 向上传播，保证单插件故障不影响主进程（**不做 fuel / memory / epoch 资源限制**，cycms 对 Wasm 插件完全信任，审计由插件市场负责）
  - [x] 17.6 编写 Wasm 插件加载和调用集成测试（使用测试用 .wasm 组件）
  - _Requirements: 12.1, 12.2, 12.3, 12.4, 12.5_

- [x] 18. 实现 ApiGateway（cycms-api）
  - [x] 18.1 定义 axum Router 顶层路由结构
  - [x] 18.2 实现 Auth API（/api/v1/auth/login, /register, /refresh, /me）
  - [x] 18.3 实现 ContentType API（/api/v1/content-types CRUD）
  - [x] 18.4 实现 Content API（/api/v1/content/:type_api_id CRUD + 查询）
  - [x] 18.5 实现 Media API（/api/v1/media 上传/列表/详情/删除）
  - [x] 18.6 实现 Plugin API（/api/v1/plugins 安装/启用/禁用/卸载/列表）
  - [x] 18.7 实现 Settings API（/api/v1/settings CRUD by namespace）
  - [x] 18.8 实现 User/Role API（/api/v1/users, /api/v1/roles CRUD）
  - [x] 18.9 集成 utoipa 自动生成 OpenAPI 文档（/api/docs）
  - [x] 18.10 实现统一错误响应格式（JSON { error: { status, name, code, message, details? } }）
  - [x] 18.11 实现插件路由动态挂载（/api/v1/x/:plugin_name/*）
  - [x] 18.12 编写 API 端到端测试
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

- [x] 19. 实现 Observability（cycms-observability）
  - [x] 19.1 实现 tracing 初始化（JSON/Pretty 格式切换，按级别过滤）
  - [x] 19.2 实现请求级 span 中间件（request_id, method, path, status, latency）
  - [x] 19.3 实现 AuditLogger：关键业务操作写入 audit_logs 表（who/what/when/result）
  - [x] 19.4 编写日志格式与审计日志写入的验证测试
  - _Requirements: 16.1, 16.2_

- [x] 20. 实现 CLI（cycms-cli）
  - [x] 20.1 实现 clap 命令行解析（new / serve / migrate / seed / plugin 子命令）
  - [x] 20.2 实现 `cycms new <project-name>` 子命令：生成项目骨架（Cargo.toml workspace、目录结构、默认配置文件、示例插件）
  - [x] 20.3 实现 `cycms serve` 子命令（加载配置 → 初始化 DB → 运行迁移 → 启动 HTTP）
  - [x] 20.4 实现 `cycms migrate run` / `cycms migrate rollback` 子命令
  - [x] 20.5 实现 `cycms seed` 子命令（创建初始管理员 + 默认角色 + 默认内容类型）
  - [x] 20.6 实现 `cycms plugin new <name>` 子命令：生成插件脚手架（Manifest 模板 + 入口代码）
  - [x] 20.7 实现 `cycms plugin install/enable/disable/remove` 子命令
  - [x] 20.8 编写 CLI 子命令集成测试
  - _Requirements: 17.1, 17.2, 17.3, 17.4_

- [x] 21. 实现 Web 应用层（apps/web）
  - [x] 21.1 初始化 React 19 + TypeScript + Vite 项目
  - [x] 21.2 实现 API Client 层（封装 fetch + 认证 token 管理 + 自动刷新）
  - [x] 21.3 实现管理后台登录页面和认证状态管理（AuthContext/AuthProvider）
  - [x] 21.4 实现管理后台侧边栏导航布局（AdminLayout）
  - [x] 21.5 实现仪表盘页面（内容统计概览 + 快捷操作入口）
  - [x] 21.6 实现内容类型管理页面（创建/编辑/删除/字段设计器）
  - [x] 21.7 实现内容管理页面（列表/创建/编辑/发布）
  - [x] 21.8 实现媒体管理页面（上传/浏览/删除/网格视图）
  - [x] 21.9 实现插件管理页面（列表/安装/启停/卸载）
  - [x] 21.10 实现用户/角色与权限矩阵管理页面
  - [x] 21.11 实现系统设置页面（按命名空间管理键值对）
  - [x] 21.12 集成 Vite 构建产物到 Rust serve（SPA fallback + 静态文件服务）
  - [x] 21.13 实现公共 API 路由（/v1/public — 已发布内容、会员认证/注册）
  - [x] 21.14 实现公共访客页面（首页/内容列表/内容详情/搜索/404）
  - [x] 21.15 实现会员页面（登录/注册/个人资料）
  - [x] 21.16 实现路由守卫（AdminGuard/MemberGuard）与懒加载路由配置
  - _Requirements: 18.1, 18.2, 18.3, 18.4, 18.5, 18.6, 18.7, 18.8_

---

## 实现优先级与依赖关系

```
1(骨架) → 2(配置) → 3(数据库) → 4(迁移) → 5(认证) → 6(权限)
                                            ↘
                                    7(事件) → 8(设置) → 9(注册表)
                                            ↘
                            10(内容模型) → 11(内容引擎) → 12(修订) → 13(发布)
                                            ↘
                                    14(媒体)
                                            ↘
                            15(插件管理) → 16(Native运行时) → 17(Wasm运行时)
                                            ↘
                            18(API网关) → 19(可观测性) → 20(CLI) → 21(Web应用)
```

## 里程碑

| 里程碑 | 包含任务 | 交付物 |
|--------|---------|--------|
| M1: 基础设施 | 1-4 | 项目骨架、配置加载、DB 连接、迁移系统 |
| M2: 认证授权 | 5-6 | 用户认证、角色权限、中间件 |
| M3: 核心服务 | 7-9 | 事件总线、设置管理、服务注册 |
| M4: 内容体系 | 10-14 | 内容建模、CRUD、修订、发布、媒体 |
| M5: 插件体系 | 15-17 | 插件管理、Native/Wasm 双运行时 |
| M6: 接口与交付 | 18-21 | REST API、可观测性、CLI、Web 前端（管理后台 + 公共站点 + 会员系统） |
