# Requirements Document — cycms v0.1

## Introduction

本文档定义 cycms v0.1 的功能需求与验收标准。每条验收标准均关联至架构蓝图中的具体系统组件，确保需求到实现的完整可追溯性。

## Glossary

| Term | Definition |
|---|---|
| Content Type | 内容类型，定义一类内容的结构（字段集合与验证规则） |
| Content Entry | 内容实例，某个 Content Type 下的一条具体内容记录 |
| Field Type | 字段类型，如 Text、RichText、Number、Boolean、DateTime、JSON、Relation、Media 等 |
| Revision | 内容版本快照，不可变，记录某一时刻的完整内容状态 |
| Plugin Manifest | 插件清单文件（TOML），声明插件元信息、版本兼容性、依赖、权限等 |
| Host Function | 宿主函数，由内核提供给 Wasm 插件调用的能力接口 |
| Service Registry | 服务注册表，管理插件间服务发现与调用的中心组件 |

## Requirements

### Requirement 1: 用户认证

#### Acceptance Criteria

1.1 WHEN 用户提交有效的用户名和密码, THE **AuthEngine** SHALL 返回包含 access_token 和 refresh_token 的 JSON 响应，access_token 使用 JWT 格式且包含用户 ID 和角色信息。

1.2 WHEN 用户提交无效的凭证, THE **AuthEngine** SHALL 返回 401 状态码和标准化错误消息，不泄露具体失败原因（用户不存在 vs 密码错误）。

1.3 WHEN 用户携带有效的 refresh_token 请求刷新, THE **AuthEngine** SHALL 颁发新的 access_token 并可选地轮换 refresh_token。

1.4 WHEN 请求携带过期或无效的 access_token, THE **ApiGateway** SHALL 通过认证中间件拦截并返回 401 状态码。

1.5 WHEN 系统初始化且无任何用户时, THE **AuthEngine** SHALL 支持创建初始超级管理员账户。

1.6 WHEN 用户密码被存储时, THE **AuthEngine** SHALL 使用 Argon2id 算法进行哈希处理，不得明文存储。

### Requirement 2: 角色与权限管理

#### Acceptance Criteria

2.1 WHEN 管理员创建新角色并分配权限列表, THE **PermissionEngine** SHALL 持久化该角色定义，权限格式为 `{domain}.{resource}.{action}` (如 `content.article.create`)。

2.2 WHEN 用户发起受保护资源的请求, THE **PermissionEngine** SHALL 根据用户角色检查是否具有该资源对应操作的权限，无权限时返回 403。

2.3 WHEN 权限规则包含范围修饰符 `own`, THE **PermissionEngine** SHALL 仅允许操作当前用户创建的资源。

2.4 WHEN 插件安装时声明了自定义权限点, THE **PermissionEngine** SHALL 将这些权限点注册到系统权限表，使其可被分配给角色。

2.5 WHEN 系统初始化时, THE **PermissionEngine** SHALL 创建默认的 `super_admin`、`editor`、`author` 角色及其预设权限集合。

### Requirement 3: 内容类型管理

#### Acceptance Criteria

3.1 WHEN 管理员通过 API 定义新 Content Type（包含名称、API 标识符、字段列表）, THE **ContentModel** SHALL 验证 Schema 合法性并持久化类型定义。

3.2 WHEN Content Type 包含的字段类型为系统内建类型（Text、RichText、Number、Boolean、DateTime、JSON、Media、Relation）, THE **ContentModel** SHALL 为每种类型提供验证规则配置（如 minLength、maxLength、required、unique、regex、min、max）。

3.3 WHEN 管理员修改已有 Content Type 的字段定义, THE **ContentModel** SHALL 执行变更影响分析并更新 Schema，已有内容实例不被破坏性删除。

3.4 WHEN Content Type 支持 Relation 类型字段, THE **ContentModel** SHALL 支持 one-to-one、one-to-many、many-to-many 三种关联关系的定义。

3.5 WHEN 内容类型被定义后, THE **ContentModel** SHALL 同步更新 REST API 路由（{type_api_id} 为路径参数）和 OpenAPI 文档。

3.6 WHEN 插件注册新的字段类型时, THE **ContentModel** SHALL 接受该自定义字段类型并将其纳入可用字段类型列表。

### Requirement 4: 内容 CRUD 引擎

#### Acceptance Criteria

4.1 WHEN 用户通过 REST API 创建内容实例（POST /api/v1/content/{type_api_id}）, THE **ContentEngine** SHALL 根据对应 Content Type 的 Schema 验证所有字段，验证通过后持久化并返回 201 和完整内容对象。

4.2 WHEN 用户查询内容列表（GET /api/v1/content/{type_api_id}）, THE **ContentEngine** SHALL 支持分页（page/pageSize）、排序（sort=field:asc/desc）、筛选（filter[field][operator]=value）参数。

4.3 WHEN 查询筛选参数包含操作符, THE **ContentEngine** SHALL 至少支持 eq、ne、gt、gte、lt、lte、contains、startsWith、endsWith、in、notIn、null、notNull 操作符。

4.4 WHEN 用户更新内容实例（PUT /api/v1/content/{type_api_id}/{id}）, THE **ContentEngine** SHALL 执行字段验证后更新，并自动创建新版本快照。

4.5 WHEN 用户删除内容实例（DELETE /api/v1/content/{type_api_id}/{id}）, THE **ContentEngine** SHALL 执行软删除（标记 archived 状态）或硬删除（根据配置），删除前检查关联引用。

4.6 WHEN 内容实例被创建/更新/删除/发布, THE **ContentEngine** SHALL 通过 **EventBus** 发布对应事件（content.created、content.updated、content.deleted、content.published）。

4.7 WHEN Relation 类型字段被查询, THE **ContentEngine** SHALL 支持通过 populate 参数控制关联数据的加载深度。

### Requirement 5: 版本管理

#### Acceptance Criteria

5.1 WHEN 内容实例被创建或更新时, THE **RevisionManager** SHALL 自动创建不可变的版本快照，包含版本号、完整字段数据快照、操作者 ID 和时间戳。

5.2 WHEN 用户查询某内容实例的版本历史（GET /api/v1/content/{type_api_id}/{id}/revisions）, THE **RevisionManager** SHALL 返回按时间倒序排列的版本列表。

5.3 WHEN 用户请求查看特定版本（GET /api/v1/content/{type_api_id}/{id}/revisions/{version}）, THE **RevisionManager** SHALL 返回该版本的完整内容快照。

5.4 WHEN 用户请求回滚到指定版本, THE **RevisionManager** SHALL 基于目标版本快照创建新版本（不删除中间版本），并更新内容实例的当前版本指针。

### Requirement 6: 发布管理

#### Acceptance Criteria

6.1 WHEN 内容实例处于 draft 状态且用户执行发布操作, THE **PublishManager** SHALL 将当前版本标记为 published_version，内容状态变更为 published，记录 published_at 时间。

6.2 WHEN 已发布的内容被编辑, THE **PublishManager** SHALL 保持 published_version 不变，编辑产生的新版本仅更新 current_version（支持「发布版本」和「最新草稿」并存）。

6.3 WHEN 用户执行撤回操作, THE **PublishManager** SHALL 将内容状态从 published 变更为 draft，清除 published_version 绑定。

6.4 WHEN 外部 API 查询内容时默认仅返回 published 状态的内容, THE **PublishManager** SHALL 提供 status 筛选参数, 管理端 API 可查看所有状态的内容。

### Requirement 7: 媒体管理

#### Acceptance Criteria

7.1 WHEN 用户上传文件（POST /api/v1/media/upload，multipart/form-data）, THE **MediaManager** SHALL 存储文件至配置的存储后端，记录文件名、MIME 类型、文件大小、存储路径、上传者 ID 等元数据，并返回媒体对象。

7.2 WHEN 用户查询媒体列表, THE **MediaManager** SHALL 支持按 MIME 类型、文件名、上传时间进行筛选和分页。

7.3 WHEN 用户删除媒体资源, THE **MediaManager** SHALL 检查是否有内容实例引用该媒体，存在引用时警告或阻止删除。

7.4 WHEN 媒体存储后端被配置时, THE **MediaManager** SHALL 支持本地文件系统存储（v0.1 默认），并预留存储后端抽象接口供插件实现（如 S3、OSS）。

### Requirement 8: REST API 与 OpenAPI

#### Acceptance Criteria

8.1 WHEN 系统启动后, THE **ApiGateway** SHALL 在 `/api/docs` 端点提供完整的 OpenAPI 3.1 JSON 文档，覆盖所有系统 API 和已启用插件注册的 API。

8.2 WHEN 新 Content Type 被创建后, THE **ApiGateway** SHALL 动态更新 OpenAPI 文档以包含该类型的 CRUD 端点定义和 Schema。

8.3 WHEN API 返回错误时, THE **ApiGateway** SHALL 使用统一错误响应格式：`{ "error": { "status": number, "name": string, "message": string, "details": optional } }`。

8.4 WHEN 所有 API 请求到达时, THE **ApiGateway** SHALL 按顺序执行：请求日志记录 → 认证（如需要） → 权限检查（如需要） → 业务处理 → 响应格式化。

8.5 WHEN 插件通过 ServiceRegistry 注册自定义路由时, THE **ApiGateway** SHALL 在 `/api/v1/x/{plugin_name}/*` 前缀下挂载这些路由。

### Requirement 9: 事件系统

#### Acceptance Criteria

9.1 WHEN EventBus 初始化时, THE **EventBus** SHALL 注册系统内建事件类型：content.created、content.updated、content.deleted、content.published、content.unpublished、user.created、user.updated、user.deleted、media.uploaded、media.deleted、plugin.installed、plugin.enabled、plugin.disabled、plugin.uninstalled。

9.2 WHEN 事件被发布时, THE **EventBus** SHALL 异步分发事件到所有已注册该事件类型的处理器，单个处理器失败不影响其他处理器执行。

9.3 WHEN 插件订阅事件时, THE **EventBus** SHALL 接受 Native 插件和 Wasm 插件的事件处理器注册。

9.4 WHEN 事件包含负载数据时, THE **EventBus** SHALL 使用 JSON 格式传递事件负载，包含事件类型、时间戳、触发者 ID 和事件特定数据。

### Requirement 10: 插件管理

#### Acceptance Criteria

10.1 WHEN 插件被安装时（通过 CLI 或 API 提供插件包路径/ID）, THE **PluginManager** SHALL 解析 Manifest 文件，验证兼容性（cycms 版本范围）、检查依赖是否满足，验证通过后注册插件并执行插件声明的数据库迁移。

10.2 WHEN 插件 Manifest 声明了依赖其他插件, THE **PluginManager** SHALL 在依赖未安装或版本不兼容时拒绝安装并返回明确错误。

10.3 WHEN 插件被启用时, THE **PluginManager** SHALL 按依赖拓扑顺序初始化插件，调用插件的 `on_enable` 生命周期钩子。

10.4 WHEN 插件被禁用时, THE **PluginManager** SHALL 先禁用依赖该插件的其他插件（级联禁用），调用 `on_disable` 生命周期钩子，注销该插件注册的路由、事件处理器和服务。

10.5 WHEN 插件被卸载时, THE **PluginManager** SHALL 先禁用插件，执行 down migration（如需要），移除插件文件和注册数据。

10.6 WHEN 查询已安装插件列表时, THE **PluginManager** SHALL 返回每个插件的名称、版本、状态（enabled/disabled）、依赖关系和权限声明。

### Requirement 11: Native 插件运行时

#### Acceptance Criteria

11.1 WHEN Native 插件被加载时, THE **NativePluginRuntime** SHALL 通过 Rust trait 对象（`dyn Plugin`）加载并初始化插件实例，注入宿主能力引用。

11.2 WHEN Native 插件注册路由时, THE **NativePluginRuntime** SHALL 将插件提供的 axum Router 合并到主路由表中。

11.3 WHEN Native 插件订阅事件时, THE **NativePluginRuntime** SHALL 将插件的事件处理闭包注册到 EventBus。

11.4 WHEN Native 插件暴露服务时, THE **NativePluginRuntime** SHALL 将服务 trait 对象注册到 ServiceRegistry，使其他插件可通过 typed contract 发现和调用。

### Requirement 12: Wasm 插件运行时

#### Acceptance Criteria

12.1 WHEN Wasm 插件被加载时, THE **WasmPluginRuntime** SHALL 使用 wasmtime 编译并实例化 .wasm Component Model 组件，绑定 Host Functions 作为宿主能力接口。

12.2 WHEN Wasm 插件通过 Host Function 请求内容操作时, THE **WasmPluginRuntime** SHALL 将请求委托到 ContentEngine 执行，并将结果序列化为 JSON 返回给 Wasm 实例。

12.3 WHEN Wasm 插件通过 Host Function 注册路由时, THE **WasmPluginRuntime** SHALL 创建代理 handler，将 HTTP 请求参数序列化后调用 Wasm 导出函数处理，并将结果转换为 HTTP 响应。

12.4 WHEN Wasm 插件触发 trap 或 panic 时, THE **WasmPluginRuntime** SHALL 捕获错误并记录日志，不影响主进程和其他插件的运行；cycms 对 Wasm 插件采用**完全信任模型**（与 Native 同权），不强制 fuel / memory / epoch 资源限制，安全审计由插件分发层（如未来的插件市场）负责。

12.5 WHEN 宿主能力被提供给 Wasm 插件时, THE **WasmPluginRuntime** SHALL 提供以下 Host Function 组，全部为**完整访问，不做白名单或沙箱约束**：content（CRUD）、auth（身份查询）、permission（权限检查）、kv（键值存储）、http（外部请求）、event（事件发布/订阅）、route（路由注册）、log（日志）、settings（配置读写）、db（原始 SQL 执行，对当前 DatabasePool）；并通过 wasmtime-wasi 向 guest 完整透传 WASI preview 2（filesystem / sockets / http / clocks / random / cli / stdio），使 Wasm 插件可执行任意系统操作。

### Requirement 13: 插件服务注册与发现

#### Acceptance Criteria

13.1 WHEN 插件暴露服务时, THE **ServiceRegistry** SHALL 以 `{plugin_name}.{service_name}` 为键注册服务实例。

13.2 WHEN 插件请求调用其他插件的服务时, THE **ServiceRegistry** SHALL 根据键查找并返回服务引用，未找到时返回明确错误。

13.3 WHEN 被依赖的插件未启用时, THE **ServiceRegistry** SHALL 对该插件的服务查询返回不可用状态。

### Requirement 14: 数据库迁移

#### Acceptance Criteria

14.1 WHEN 系统首次启动时, THE **MigrationEngine** SHALL 自动执行所有未应用的系统迁移（按版本顺序）。

14.2 WHEN 插件安装时声明了迁移文件, THE **MigrationEngine** SHALL 在插件安装后按顺序执行插件的 up migration。

14.3 WHEN 迁移执行失败时, THE **MigrationEngine** SHALL 回滚当前批次的所有迁移并报告错误。

14.4 WHEN 执行迁移时, THE **MigrationEngine** SHALL 为每次迁移记录执行时间、来源（系统/插件名）和结果状态。

### Requirement 15: 配置与设置

#### Acceptance Criteria

15.1 WHEN 系统启动时, THE **ConfigManager** SHALL 从配置文件（cycms.toml）和环境变量加载配置，环境变量优先级高于配置文件。

15.2 WHEN 系统设置被修改时, THE **SettingsManager** SHALL 将设置持久化到数据库，支持命名空间隔离（系统设置 vs 插件设置）。

15.3 WHEN 插件声明了设置 Schema, THE **SettingsManager** SHALL 存储 Schema 定义，并允许 **WebApp** 管理域基于该 Schema 动态渲染设置表单。

### Requirement 16: 可观测性

#### Acceptance Criteria

16.1 WHEN 每个 HTTP 请求处理时, THE **Observability** SHALL 创建 tracing span 记录请求方法、路径、状态码、处理时长。

16.2 WHEN 关键业务操作发生时（内容变更、用户操作、插件操作）, THE **Observability** SHALL 记录审计日志条目（who/what/when/result）到审计日志表。

### Requirement 17: CLI 工具

#### Acceptance Criteria

17.1 WHEN 用户执行 `cycms new <project-name>`, THE **CLI** SHALL 生成完整的项目骨架，包含 Cargo.toml workspace 配置、目录结构、默认配置文件和示例插件。

17.2 WHEN 用户执行 `cycms plugin new <plugin-name>`, THE **CLI** SHALL 生成插件项目骨架，包含 Manifest 模板、入口代码模板和基本目录结构。

17.3 WHEN 用户执行 `cycms migrate`, THE **CLI** SHALL 执行所有待应用的数据库迁移并输出结果摘要。

17.4 WHEN 用户执行 `cycms serve`, THE **CLI** SHALL 启动开发服务器，加载配置和插件，监听指定端口。

### Requirement 18: Web 应用层（WebApp）

#### Acceptance Criteria

18.1 WHEN 系统提供官方前端时, THE **WebApp** SHALL 以单一 React 应用同时承载 `/admin` 管理域与 `/` 访客域，并共享设计系统、API Client 与路由基础设施。

18.2 WHEN 管理员登录后台时, THE **WebApp** SHALL 提供登录页、认证状态恢复、受保护路由和仪表盘，仪表盘包含内容统计概览和快捷操作入口。

18.3 WHEN 管理员进入内容类型管理页面时, THE **WebApp** SHALL 展示所有 Content Type 列表，支持新建、编辑、删除类型操作，字段管理使用拖拽排序。

18.4 WHEN 管理员进入内容管理页面时, THE **WebApp** SHALL 提供内容列表、搜索、筛选、排序、批量操作、分页、Schema 驱动编辑表单、草稿保存、发布操作与版本历史入口。

18.5 WHEN 管理员进入媒体库时, THE **WebApp** SHALL 展示已上传媒体的网格/列表视图，支持上传、预览、删除和筛选。

18.6 WHEN 管理员进入插件管理页面时, THE **WebApp** SHALL 展示已安装插件列表及状态，支持启用/禁用、查看依赖/权限声明和进入插件设置入口。

18.7 WHEN 管理员进入用户与权限管理页面时, THE **WebApp** SHALL 展示用户列表、角色列表和权限矩阵，支持用户/角色 CRUD 与权限分配。

18.8 WHEN 管理员进入系统设置页面时, THE **WebApp** SHALL 提供系统设置与插件设置入口，并根据 settings schema 动态渲染表单。

18.9 WHEN 访客访问站点首页时, THE **WebApp** SHALL 提供公开首页以及全局导航和页脚，并消费 CMS 内容构建首页模块。

18.10 WHEN 访客访问公开内容路由时, THE **WebApp** SHALL 支持任意 Content Type 驱动的列表页、详情页和栏目/分类页，而不是仅限内置 `page` / `post`。

18.11 WHEN 访客执行公开站点搜索或访问不存在的路由时, THE **WebApp** SHALL 提供搜索结果页与 404 / 错误页。

18.12 WHEN 站点成员使用访客账户能力时, THE **WebApp** SHALL 提供注册、登录、登出和基本个人资料页。

18.13 WHEN 公开站点需要将内容解析到页面路由时, THE **WebApp** SHALL 通过可扩展接口接入插件提供的前台路由映射/解析能力，而不在核心规格中绑定单一路由配置中心。

18.14 WHEN `cycms serve` 提供官方前端时, THE **WebApp** SHALL 由同一 Rust 服务同源承载；管理域采用 SPA 交互模型，访客域采用面向内容站点的混合渲染模式。

### Requirement 19: 多数据库支持

#### Acceptance Criteria

19.1 WHEN 系统配置使用 PostgreSQL 时, THE **DatabaseLayer** SHALL 利用 JSONB 类型存储内容字段数据，支持 GIN 索引和 JSONB 查询操作符。

19.2 WHEN 系统配置使用 MySQL 或 SQLite 时, THE **DatabaseLayer** SHALL 使用 JSON/TEXT 类型存储内容字段数据，提供兼容的查询能力（可能功能受限）。

19.3 WHEN 数据库连接建立时, THE **DatabaseLayer** SHALL 使用 sqlx 的连接池管理，支持配置最大连接数、连接超时和空闲超时。

### Requirement 20: 插件 Manifest 规范

#### Acceptance Criteria

20.1 WHEN 插件包含 Manifest 文件（plugin.toml）, THE **PluginManager** SHALL 解析并验证以下必填字段：name、version、kind（native/wasm）、entry（入口文件路径）。

20.2 WHEN Manifest 包含 compatibility 字段, THE **PluginManager** SHALL 验证当前 cycms 版本是否在声明的兼容范围内。

20.3 WHEN Manifest 包含 dependencies 字段, THE **PluginManager** SHALL 遍历所有依赖并验证版本满足，支持 optional 依赖标注。

20.4 WHEN Manifest 包含 permissions 字段, THE **PluginManager** SHALL 在安装时将声明的权限点注册到 PermissionEngine。

20.5 WHEN Manifest 包含 frontend 字段, THE **PluginManager** SHALL 记录前端入口路径，供 **WebApp** 动态加载插件前端扩展或公开路由扩展。
