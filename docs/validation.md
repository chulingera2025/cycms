# Validation Matrix — cycms v0.1

本文档机械对照 `requirements.md` 与 `tasks.md`，逐条验证每个验收标准均有对应实现任务。每条"描述"列为 requirements.md 原文要点简述；"任务"列引用 tasks.md 中真实的子任务编号。

## 需求 → 任务追踪矩阵

| 验收标准 | 描述（requirements.md 原文要点） | 实现任务 | 覆盖 |
|----------|---------------------------------|----------|------|
| **R1 用户认证** | | | |
| 1.1 | 有效凭证返回含 access_token(JWT) + refresh_token 的 JSON | 5.3, 5.4 | ✅ |
| 1.2 | 无效凭证返回 401 且不泄露具体失败原因 | 5.4, 5.7 | ✅ |
| 1.3 | refresh_token 换新 access_token，可选轮换 refresh_token | 5.5 | ✅ |
| 1.4 | 过期/无效 access_token 由认证中间件拦截返回 401 | 5.7 | ✅ |
| 1.5 | 系统无用户时支持创建初始超级管理员 | 5.6 | ✅ |
| 1.6 | 密码使用 Argon2id 哈希，禁止明文存储 | 5.2 | ✅ |
| **R2 角色与权限管理** | | | |
| 2.1 | 创建角色并分配权限（`{domain}.{resource}.{action}` 格式） | 6.1, 6.2 | ✅ |
| 2.2 | 请求受保护资源时按角色检查权限，无权限返回 403 | 6.3, 6.6 | ✅ |
| 2.3 | 支持 `own` 范围修饰符，仅限操作本人资源 | 6.3 | ✅ |
| 2.4 | 插件声明的自定义权限点注册到系统权限表 | 6.4 | ✅ |
| 2.5 | 系统初始化时创建 super_admin / editor / author 默认角色 | 6.5 | ✅ |
| **R3 内容类型管理** | | | |
| 3.1 | 定义新 Content Type 时验证 Schema 合法性并持久化 | 10.1, 10.2 | ✅ |
| 3.2 | 内建字段类型提供验证规则配置（minLength/maxLength/required/unique/regex/min/max） | 10.3 | ✅ |
| 3.3 | 修改已有 Content Type 做变更影响分析并更新 Schema，已有数据不被破坏 | 10.2, 10.6 | ✅ |
| 3.4 | Relation 字段支持 one-to-one / one-to-many / many-to-many | 10.1 | ✅ |
| 3.5 | 内容类型创建后同步更新 REST API 路由（`{type_api_id}`）与 OpenAPI 文档 | 10.4, 18.4, 18.9 | ✅ |
| 3.6 | 插件可注册新字段类型并纳入字段类型列表 | 10.1 | ✅ |
| **R4 内容 CRUD 引擎** | | | |
| 4.1 | POST 创建内容后校验字段并返回 201 + 完整内容对象 | 11.2, 11.3 | ✅ |
| 4.2 | GET 列表支持 page/pageSize、sort、filter[field][operator] 参数 | 11.4 | ✅ |
| 4.3 | 筛选至少支持 eq/ne/gt/gte/lt/lte/contains/startsWith/endsWith/in/notIn/null/notNull | 11.4 | ✅ |
| 4.4 | PUT 更新字段验证后更新并自动创建新版本快照 | 11.2, 11.3, 12.2 | ✅ |
| 4.5 | DELETE 执行软删除或硬删除，删除前检查关联引用 | 11.2, 11.6 | ✅ |
| 4.6 | 创建/更新/删除/发布内容后通过 EventBus 发布对应事件 | 11.5 | ✅ |
| 4.7 | Relation 字段支持通过 populate 控制关联加载深度 | 11.4 | ✅ |
| **R5 版本管理** | | | |
| 5.1 | 内容创建/更新时自动创建不可变版本快照（版本号/字段/操作者/时间戳） | 12.1, 12.2 | ✅ |
| 5.2 | 查询版本历史按时间倒序 | 12.3 | ✅ |
| 5.3 | 查看特定版本返回完整快照 | 12.3 | ✅ |
| 5.4 | 回滚到指定版本：基于目标快照创建新版本并更新当前版本指针 | 12.4 | ✅ |
| **R6 发布管理** | | | |
| 6.1 | draft 状态发布后标记 published_version、状态变更 published、记录 published_at | 13.1, 13.2 | ✅ |
| 6.2 | 已发布内容被编辑：published_version 不变，current_version 更新（草稿与发布并存） | 13.2 | ✅ |
| 6.3 | 撤回将状态从 published 改为 draft 并清除 published_version 绑定 | 13.3 | ✅ |
| 6.4 | 外部查询默认只返 published，管理端 API 可查所有状态 | 13.4 | ✅ |
| **R7 媒体管理** | | | |
| 7.1 | multipart 上传后存储文件并记录 filename/MIME/size/path/uploader 元数据 | 14.1, 14.3 | ✅ |
| 7.2 | 媒体列表支持按 MIME、文件名、上传时间筛选与分页 | 14.1 | ✅ |
| 7.3 | 删除媒体前检查引用；有引用时警告或阻止删除 | 14.4 | ✅ |
| 7.4 | 支持本地 FS 存储（v0.1 默认）并预留 StorageBackend 抽象 | 14.2 | ✅ |
| **R8 REST API 与 OpenAPI** | | | |
| 8.1 | `/api/docs` 提供完整 OpenAPI 3.1 JSON（含插件） | 18.9 | ✅ |
| 8.2 | 新 Content Type 创建后动态更新 OpenAPI 文档 | 18.9 | ✅ |
| 8.3 | 统一错误响应：`{ error: { status, name, message, details? } }` | 18.10 | ✅ |
| 8.4 | 请求链：日志 → 认证 → 权限 → 业务 → 响应格式化 | 18.1, 19.2 | ✅ |
| 8.5 | 插件路由统一挂载在 `/api/v1/x/{plugin_name}/*` | 18.11 | ✅ |
| **R9 事件系统** | | | |
| 9.1 | EventBus 初始化时注册 14 个系统内建事件类型 | 7.1 | ✅ |
| 9.2 | 事件异步分发，单处理器失败不影响其他处理器 | 7.4 | ✅ |
| 9.3 | 事件订阅接受 Native + Wasm 插件的处理器注册 | 7.3, 16.4, 17.3 | ✅ |
| 9.4 | 事件负载 JSON：type/timestamp/triggered_by/data | 7.1, 7.2 | ✅ |
| **R10 插件管理** | | | |
| 10.1 | 安装时解析 Manifest、校验兼容性/依赖、执行插件迁移 | 15.1, 15.3, 15.4, 15.5 | ✅ |
| 10.2 | 依赖未安装/版本不兼容时拒绝安装并返回明确错误 | 15.4 | ✅ |
| 10.3 | 启用按依赖拓扑顺序初始化并调用 on_enable 钩子 | 15.5, 15.6, 16.2 | ✅ |
| 10.4 | 禁用级联禁用依赖方、调用 on_disable、注销路由/事件/服务 | 15.5, 15.6, 16.4 | ✅ |
| 10.5 | 卸载先禁用、执行 down migration、移除注册数据 | 15.5 | ✅ |
| 10.6 | 查询列表返回 name/version/status/dependencies/permissions | 15.2 | ✅ |
| **R11 Native 插件运行时** | | | |
| 11.1 | 通过 `Arc<dyn Plugin>` trait 对象加载并注入宿主能力 | 16.1, 16.2 | ✅ |
| 11.2 | 插件 axum Router 合并到主路由表 | 16.3 | ✅ |
| 11.3 | 插件事件处理闭包注册到 EventBus | 16.4 | ✅ |
| 11.4 | 插件服务 trait 对象注册到 ServiceRegistry | 16.5 | ✅ |
| **R12 Wasm 插件运行时** | | | |
| 12.1 | 使用 wasmtime 编译并实例化 `.wasm` 组件，绑定 Host Functions | 17.1, 17.2 | ✅ |
| 12.2 | Wasm 通过 Host Function 请求内容操作时委托 ContentEngine 并返回 JSON | 17.3 | ✅ |
| 12.3 | Wasm 通过 Host Function 注册路由：宿主创建代理 handler | 17.3, 17.4 | ✅ |
| 12.4 | Wasm 超时或 panic 时捕获异常、记录日志、不影响主进程 | 17.5 | ✅ |
| 12.5 | 至少提供 content/auth/permission/kv/http/event/route/log/settings 共 9 组 Host Function | 17.3 | ✅ |
| **R13 插件服务注册与发现** | | | |
| 13.1 | 以 `{plugin_name}.{service_name}` 为键注册服务实例 | 9.1, 9.2 | ✅ |
| 13.2 | 按键查找服务并返回引用，未找到时明确错误 | 9.2 | ✅ |
| 13.3 | 被依赖插件未启用时对其服务查询返回不可用状态 | 9.2 | ✅ |
| **R14 数据库迁移** | | | |
| 14.1 | 首次启动自动执行所有未应用的系统迁移（按版本顺序） | 4.2 | ✅ |
| 14.2 | 插件安装时按顺序执行其 up migration | 4.3 | ✅ |
| 14.3 | 迁移失败时回滚当前批次并报告错误 | 4.4 | ✅ |
| 14.4 | 每次迁移记录执行时间、来源（system/plugin 名）、结果状态 | 4.1, 4.2 | ✅ |
| **R15 配置与设置** | | | |
| 15.1 | 从 cycms.toml 与环境变量加载配置（环境变量优先） | 2.2, 2.3 | ✅ |
| 15.2 | 系统设置修改持久化到数据库，支持命名空间隔离 | 8.2 | ✅ |
| 15.3 | 插件声明 settings schema，WebApp 管理域动态渲染设置表单 | 8.3, 21.10 | ✅ |
| **R16 可观测性** | | | |
| 16.1 | 每个 HTTP 请求创建 tracing span 记录 method/path/status/latency | 19.2 | ✅ |
| 16.2 | 关键业务操作写入 audit_logs（who/what/when/result） | 19.3 | ✅ |
| **R17 CLI 工具** | | | |
| 17.1 | `cycms new <project-name>` 生成项目骨架 | 20.2 | ✅ |
| 17.2 | `cycms plugin new <plugin-name>` 生成插件脚手架 | 20.6 | ✅ |
| 17.3 | `cycms migrate` 执行待应用迁移并输出摘要 | 20.4 | ✅ |
| 17.4 | `cycms serve` 启动开发服务器（加载配置+插件+监听端口） | 20.3 | ✅ |
| **R18 Web 应用层** | | | |
| 18.1 | 单一 React 应用同时承载 `/admin` 与 `/`，共享前端基础设施 | 21.1, 21.2 | ✅ |
| 18.2 | 后台登录、认证恢复、受保护路由与仪表盘 | 21.3, 21.4 | ✅ |
| 18.3 | 内容类型管理页：列表/新建/编辑/删除 + 字段拖拽 | 21.5 | ✅ |
| 18.4 | 内容管理页：列表/搜索/筛选/分页/Schema 表单/发布/版本历史 | 21.6 | ✅ |
| 18.5 | 媒体库网格/列表视图，上传/预览/删除/筛选 | 21.7 | ✅ |
| 18.6 | 插件管理页：列表/状态/依赖权限信息/启停/设置入口 | 21.8 | ✅ |
| 18.7 | 用户与权限管理：用户列表、角色列表、权限矩阵与 CRUD | 21.9 | ✅ |
| 18.8 | 系统设置页与插件设置页：按 settings schema 动态渲染 | 21.10 | ✅ |
| 18.9 | 访客首页 + 全局导航/页脚 | 21.11 | ✅ |
| 18.10 | 任意 Content Type 驱动的列表页/详情页/栏目页 | 21.12 | ✅ |
| 18.11 | 搜索结果页与 404 / 错误页 | 21.11, 21.13 | ✅ |
| 18.12 | 会员登录/注册/登出与个人资料页 | 21.14 | ✅ |
| 18.13 | 公开路由解析通过插件扩展提供，不绑定单一路由配置中心 | 21.15 | ✅ |
| 18.14 | 同一 Rust 服务同源承载；管理域 SPA，访客域混合渲染 | 21.2, 21.16 | ✅ |
| **R19 多数据库支持** | | | |
| 19.1 | PostgreSQL 使用 JSONB 存储字段数据并支持 GIN 索引与 JSONB 操作符 | 3.3 | ✅ |
| 19.2 | MySQL/SQLite 使用 JSON/TEXT 存储字段数据（功能可能受限） | 3.3 | ✅ |
| 19.3 | 使用 sqlx 连接池管理，支持 max_connections / connect_timeout / idle_timeout 配置 | 3.2 | ✅ |
| **R20 插件 Manifest 规范** | | | |
| 20.1 | 解析并验证必填字段：name / version / kind / entry | 15.1 | ✅ |
| 20.2 | compatibility 字段验证当前 cycms 版本是否在声明范围内 | 15.1, 15.4 | ✅ |
| 20.3 | dependencies 字段遍历校验版本与 optional 标注 | 15.1, 15.4 | ✅ |
| 20.4 | permissions 字段安装时注册权限点到 PermissionEngine | 15.1 | ✅ |
| 20.5 | frontend 字段记录前端入口路径供 WebApp 动态加载插件前端扩展或公开路由扩展 | 15.1, 21.8, 21.15 | ✅ |

---

## 覆盖率分析

| 指标 | 数值 |
|------|------|
| 总需求数 | 20 |
| 总验收标准数 | 98 |
| 已覆盖验收标准 | 98 |
| 覆盖率 | 100% |

验收标准行数按 R1:6 + R2:5 + R3:6 + R4:7 + R5:4 + R6:4 + R7:4 + R8:5 + R9:4 + R10:6 + R11:4 + R12:5 + R13:3 + R14:4 + R15:3 + R16:2 + R17:4 + R18:14 + R19:3 + R20:5 = 98。

### 关键路径验证

- **认证 → 权限 → API**：R1 → R2 → R8 完整链路由 Task 5 → 6 → 18 覆盖。
- **内容建模 → CRUD → 修订 → 发布**：R3 → R4 → R5 → R6 由 Task 10 → 11 → 12 → 13 覆盖。
- **插件 manifest → 管理 → Native/Wasm 运行时**：R20 → R10 → R11 / R12 由 Task 15 → 16 / 17 覆盖。
- **配置 → 数据库 → 迁移**：R15 → R19 → R14 由 Task 2 → 3 → 4 覆盖。
- **可观测性**：R16 由 Task 19 覆盖（tracing span + AuditLogger）。
- **CLI → Web 应用层**：R17 → R18 由 Task 20 → 21 覆盖。

---

## 风险与待确认事项

1. **R12 Wasm 插件信任模型**：cycms 对 Wasm 插件采用**完全信任**（与 Native 同权），仅依赖 wasmtime 对 trap/panic 的天然进程隔离；不强制 fuel / epoch / 资源限制，host functions 不做白名单，WASI preview 2 完整透传（filesystem/sockets/http 等），插件可执行任意系统操作。安全审计由未来的插件市场负责。17.5 的集成测试只覆盖「trap → Error::PluginError 传播路径」，不验证资源限额。
2. **R11 Native 插件路线**：已明确采用 `Arc<dyn Plugin>` trait 对象加载（非 `.so`/C ABI），同时 Rust ABI 不稳定意味着插件必须与宿主同一 rustc 版本编译；这点需要在 tasks 1.x 的骨架与分发文档中显式说明。
3. **R15.3 / R18.8 WebApp 动态渲染**：插件 settings schema 的渲染器目前依赖 JSON Schema 子集，需在 21.10 中明确支持哪些 schema 关键字（types/enum/default/required/minimum/maximum/minLength/maxLength）。
4. **R18.2 仪表盘与后台认证**：当前只明确了“统计概览 + 快捷入口”，统计指标清单与认证恢复策略（刷新时机、过期跳转、访客域是否共用刷新逻辑）需要在 21.3 / 21.4 细化。
5. **R18.13 前台路由扩展**：公开内容路由由插件提供解析 contract，需在 21.15 明确静态路由、动态参数、栏目聚合、优先级和回退顺序。
6. **R18.14 同源混合渲染**：管理域 SPA 与访客域混合渲染共存时，需要在 21.16 明确 HTML 入口分发、缓存边界和服务端 fallback 规则。
7. **R19.2 MySQL/SQLite 功能受限**：JSON 函数索引在 MySQL 下需按字段逐个建立，SQLite 仅支持表达式索引；高并发查询场景下性能会明显低于 PG。相关差异已在 design.md "多数据库方言映射" 章节声明。
8. **R8.4 中间件顺序**：Kernel 负责叠加 log → auth → permission → rate_limit → CORS，ApiGateway 仅构建系统路由本身；职责划分见 design.md "Router 职责分工" 章节。
9. **R17.1 `cycms new`**：项目骨架生成包含 workspace Cargo.toml、目录结构、默认 `cycms.toml`、示例插件，需在 20.2 中明确模板位置（`assets/project-template/`）与变量替换方式。
