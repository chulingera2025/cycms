# Design Document — cycms v0.1

## Overview

本文档基于架构蓝图和需求文档，提供 cycms v0.1 各核心组件的详细技术设计。每个组件规格均引用对应需求编号，确保可追溯性。

## Design Principles

1. **模块化**: 每个组件为独立 crate，通过 trait 定义边界，降低耦合
2. **异步优先**: 所有 I/O 操作使用 async/await，基于 tokio 运行时
3. **类型安全**: 充分利用 Rust 类型系统，编译期消除运行时错误
4. **插件优先**: 核心只提供基础设施，业务能力尽量插件化
5. **约定优于配置**: 提供合理默认值，允许按需覆盖

## Project Structure

```
cycms/
  crates/
    cycms-core/           # 核心类型定义、trait、error
    cycms-kernel/         # 应用启动、服务注册、生命周期
    cycms-config/         # 配置加载与管理
    cycms-db/             # 数据库抽象、连接池、查询辅助
    cycms-migrate/        # 数据库迁移引擎
    cycms-auth/           # 认证引擎
    cycms-permission/     # 权限引擎
    cycms-content-model/  # 内容类型与字段定义
    cycms-content-engine/ # 内容 CRUD 引擎
    cycms-revision/       # 版本管理
    cycms-publish/        # 发布管理
    cycms-media/          # 媒体管理
    cycms-api/            # API 网关与路由
    cycms-openapi/        # OpenAPI 文档生成
    cycms-events/         # 事件总线
    cycms-plugin-api/     # 插件 API trait 定义
    cycms-plugin-manager/ # 插件生命周期管理
    cycms-plugin-native/  # Native 插件运行时
    cycms-plugin-wasm/    # Wasm 插件运行时
    cycms-settings/       # 系统设置
    cycms-observability/  # 可观测性
    cycms-cli/            # CLI 工具
  plugins/
    official-auth/        # 官方认证插件示例
    official-media/       # 官方媒体插件示例
    apps/
        web/                  # 统一 React Web 应用（管理域 + 访客域）
  Cargo.toml              # Workspace 根配置
  cycms.toml              # 默认系统配置
```

## Component Specifications

---

### Component: Kernel

**Purpose**: 应用启动引导、服务注册中心、生命周期管理、依赖图构建、运行时上下文提供
**Location**: `crates/cycms-kernel/src/lib.rs`
**Interface**:

```rust
// Implements Req 15.1 (配置加载触发), Req 14.1 (迁移触发), Req 10.3 (插件启动触发)

/// 全局应用上下文，在 Kernel 启动后可被所有组件共享
pub struct AppContext {
    pub config: Arc<AppConfig>,
    pub db_pool: Arc<DatabasePool>,
    pub event_bus: Arc<EventBus>,
    pub plugin_manager: Arc<PluginManager>,
    pub service_registry: Arc<ServiceRegistry>,
    pub settings_manager: Arc<SettingsManager>,
    pub content_model_registry: Arc<ContentModelRegistry>,
}

/// Kernel 负责构建并初始化 AppContext
pub struct Kernel {
    config: AppConfig,
}

impl Kernel {
    /// 从配置文件和环境变量构建 Kernel
    pub async fn build(config_path: Option<&Path>) -> Result<Self>;

    /// 初始化所有子系统并返回 AppContext
    /// 顺序: Config → DB → Migration → EventBus → ServiceRegistry
    ///       → PluginManager → ContentModel → Auth → Permission → API
    pub async fn bootstrap(&self) -> Result<AppContext>;

    /// 构建完整的 axum Router（含系统路由 + 插件路由）
    pub async fn build_router(&self, ctx: &AppContext) -> Result<Router>;

    /// 启动 HTTP 服务器
    pub async fn serve(self) -> Result<()>;

    /// 优雅关闭
    pub async fn shutdown(&self, ctx: &AppContext) -> Result<()>;
}
```

---

### Component: AuthEngine

**Purpose**: 用户认证（登录/注册/Token 刷新）、密码哈希、会话管理、认证中间件
**Location**: `crates/cycms-auth/src/lib.rs`
**Interface**:

```rust
// Implements Req 1.1, 1.2, 1.3, 1.4, 1.5, 1.6

pub struct AuthEngine {
    db: Arc<DatabasePool>,
    config: AuthConfig,
}

impl AuthEngine {
    /// 用户登录，验证凭证并返回 Token 对
    /// Implements Req 1.1, 1.2
    pub async fn login(&self, credentials: LoginRequest) -> Result<TokenPair>;

    /// 刷新 access_token
    /// Implements Req 1.3
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<TokenPair>;

    /// 创建用户（内部使用 Argon2id 哈希密码）
    /// Implements Req 1.5, 1.6
    pub async fn create_user(&self, input: CreateUserInput) -> Result<User>;

    /// 验证 JWT Token 并返回用户身份
    /// Implements Req 1.4
    pub async fn verify_token(&self, token: &str) -> Result<AuthClaims>;

    /// 创建初始超级管理员（仅系统无用户时可用）
    /// Implements Req 1.5
    pub async fn setup_admin(&self, input: CreateUserInput) -> Result<User>;
}

/// axum 认证中间件
/// Implements Req 1.4
pub async fn auth_middleware(
    State(auth): State<Arc<AuthEngine>>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError>;

pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

pub struct AuthClaims {
    pub user_id: Uuid,
    pub roles: Vec<String>,
    pub exp: u64,
}
```

---

### Component: PermissionEngine

**Purpose**: RBAC 角色/权限定义、资源级权限检查、权限中间件、插件权限注册
**Location**: `crates/cycms-permission/src/lib.rs`
**Interface**:

```rust
// Implements Req 2.1, 2.2, 2.3, 2.4, 2.5

/// 权限格式: domain.resource.action (如 content.article.create)
#[derive(Debug, Clone)]
pub struct Permission {
    pub domain: String,
    pub resource: String,
    pub action: String,
    pub scope: PermissionScope,  // Own | All
}

#[derive(Debug, Clone)]
pub enum PermissionScope {
    All,
    Own,
}

pub struct PermissionEngine {
    db: Arc<DatabasePool>,
}

impl PermissionEngine {
    /// 创建角色并分配权限列表
    /// Implements Req 2.1
    pub async fn create_role(&self, input: CreateRoleInput) -> Result<Role>;

    /// 检查用户是否具有指定权限
    /// Implements Req 2.2, 2.3
    pub async fn check_permission(
        &self,
        user_id: Uuid,
        permission: &str,
        resource_owner_id: Option<Uuid>,
    ) -> Result<bool>;

    /// 注册插件自定义权限点
    /// Implements Req 2.4
    pub async fn register_permissions(
        &self,
        plugin_name: &str,
        permissions: Vec<PermissionDefinition>,
    ) -> Result<()>;

    /// 初始化默认角色
    /// Implements Req 2.5
    pub async fn seed_defaults(&self) -> Result<()>;
}

/// axum 权限中间件工厂
/// Implements Req 2.2
pub fn require_permission(
    permission: &'static str,
) -> axum::middleware::FromFnLayer<
    fn(
        axum::extract::State<Arc<PermissionEngine>>,
        axum::extract::Extension<AuthClaims>,
        axum::extract::Request,
        axum::middleware::Next,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<axum::response::Response, AppError>> + Send>>,
    (),
    Arc<PermissionEngine>,
>;
```

---

### Component: ContentModel

**Purpose**: 内容类型定义与管理、字段类型注册、Schema 验证规则、模型元数据存储
**Location**: `crates/cycms-content-model/src/lib.rs`
**Interface**:

```rust
// Implements Req 3.1, 3.2, 3.3, 3.4, 3.5, 3.6

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentTypeDefinition {
    pub id: Uuid,
    pub name: String,
    pub api_id: String,          // URL 友好标识符
    pub description: Option<String>,
    pub fields: Vec<FieldDefinition>,
    pub kind: ContentTypeKind,   // Collection | Single
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub name: String,
    pub api_id: String,
    pub field_type: FieldType,
    pub required: bool,
    pub unique: bool,
    pub default_value: Option<serde_json::Value>,
    pub validations: Vec<ValidationRule>,
    pub position: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldType {
    Text,
    RichText,
    Number { decimal: bool },
    Boolean,
    DateTime,
    Json,
    Media { allowed_types: Vec<String> },
    Relation { target_type: String, relation_kind: RelationKind },
    Custom { type_name: String },  // 插件注册的自定义类型
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationKind {
    OneToOne,
    OneToMany,
    ManyToMany,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationRule {
    MinLength(usize),
    MaxLength(usize),
    Min(f64),
    Max(f64),
    Regex(String),
    Enum(Vec<serde_json::Value>),
    Custom { validator: String },  // 插件自定义验证器名称
}

pub struct ContentModelRegistry {
    db: Arc<DatabasePool>,
    field_type_registry: RwLock<HashMap<String, Box<dyn FieldTypeHandler>>>,
}

impl ContentModelRegistry {
    /// 创建 Content Type
    /// Implements Req 3.1
    pub async fn create_type(&self, input: CreateContentTypeInput) -> Result<ContentTypeDefinition>;

    /// 更新 Content Type（字段变更）
    /// Implements Req 3.3
    pub async fn update_type(&self, id: Uuid, input: UpdateContentTypeInput) -> Result<ContentTypeDefinition>;

    /// 删除 Content Type
    pub async fn delete_type(&self, id: Uuid) -> Result<()>;

    /// 获取所有 Content Type
    pub async fn list_types(&self) -> Result<Vec<ContentTypeDefinition>>;

    /// 获取单个 Content Type
    pub async fn get_type(&self, api_id: &str) -> Result<ContentTypeDefinition>;

    /// 注册自定义字段类型（插件扩展）
    /// Implements Req 3.6
    pub fn register_field_type(&self, name: &str, handler: Box<dyn FieldTypeHandler>);

    /// 验证字段值是否符合 Schema
    /// Implements Req 3.2
    pub fn validate_field(&self, field: &FieldDefinition, value: &serde_json::Value) -> Result<()>;
}

/// 字段类型处理器 trait（插件可实现）
pub trait FieldTypeHandler: Send + Sync {
    fn validate(&self, value: &serde_json::Value, rules: &[ValidationRule]) -> Result<()>;
    fn to_openapi_schema(&self) -> serde_json::Value;
    fn default_value(&self) -> Option<serde_json::Value>;
}
```

---

### Component: ContentEngine

**Purpose**: 内容实例 CRUD、查询/筛选/排序/分页、字段验证执行、内容状态管理
**Location**: `crates/cycms-content-engine/src/lib.rs`
**Interface**:

```rust
// Implements Req 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7

pub struct ContentEngine {
    db: Arc<DatabasePool>,
    model_registry: Arc<ContentModelRegistry>,
    revision_manager: Arc<RevisionManager>,
    publish_manager: Arc<PublishManager>,
    event_bus: Arc<EventBus>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContentEntry {
    pub id: Uuid,
    pub content_type_id: Uuid,
    pub content_type_api_id: String,
    pub slug: Option<String>,
    pub status: ContentStatus,
    pub current_version_id: Uuid,
    pub published_version_id: Option<Uuid>,
    pub fields: serde_json::Value,  // JSONB 字段数据
    pub created_by: Uuid,
    pub updated_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentStatus {
    Draft,
    Published,
    Archived,
}

#[derive(Debug, Deserialize)]
pub struct ContentQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub sort: Option<String>,       // field:asc 或 field:desc
    pub filters: HashMap<String, FilterValue>,
    pub populate: Option<Vec<String>>,
    pub status: Option<ContentStatus>,
}

#[derive(Debug, Deserialize)]
pub struct FilterValue {
    pub operator: FilterOperator,
    pub value: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub enum FilterOperator {
    Eq, Ne, Gt, Gte, Lt, Lte,
    Contains, StartsWith, EndsWith,
    In, NotIn, Null, NotNull,
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub meta: PaginationMeta,
}

#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    pub page: u64,
    pub page_size: u64,
    pub page_count: u64,
    pub total: u64,
}

impl ContentEngine {
    /// 创建内容实例
    /// Implements Req 4.1, 4.6
    pub async fn create(
        &self, type_api_id: &str, data: serde_json::Value, user_id: Uuid,
    ) -> Result<ContentEntry>;

    /// 查询内容列表
    /// Implements Req 4.2, 4.3, 4.7
    pub async fn list(
        &self, type_api_id: &str, query: ContentQuery,
    ) -> Result<PaginatedResponse<ContentEntry>>;

    /// 获取单个内容
    pub async fn get(
        &self, type_api_id: &str, id: Uuid, populate: Option<Vec<String>>,
    ) -> Result<ContentEntry>;

    /// 更新内容实例
    /// Implements Req 4.4, 4.6
    pub async fn update(
        &self, type_api_id: &str, id: Uuid, data: serde_json::Value, user_id: Uuid,
    ) -> Result<ContentEntry>;

    /// 删除内容实例（软删除/硬删除）
    /// Implements Req 4.5, 4.6
    pub async fn delete(
        &self, type_api_id: &str, id: Uuid, hard: bool,
    ) -> Result<()>;
}
```

---

### Component: RevisionManager

**Purpose**: 版本快照创建与存储、版本历史查询、版本比较、版本回滚
**Location**: `crates/cycms-revision/src/lib.rs`
**Interface**:

```rust
// Implements Req 5.1, 5.2, 5.3, 5.4

#[derive(Debug, Serialize, Deserialize)]
pub struct Revision {
    pub id: Uuid,
    pub content_entry_id: Uuid,
    pub version_number: i32,
    pub snapshot: serde_json::Value,  // 完整字段数据快照
    pub change_summary: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

pub struct RevisionManager {
    db: Arc<DatabasePool>,
}

impl RevisionManager {
    /// 创建版本快照
    /// Implements Req 5.1
    pub async fn create_revision(
        &self, entry_id: Uuid, snapshot: serde_json::Value, user_id: Uuid,
    ) -> Result<Revision>;

    /// 查询版本历史（倒序）
    /// Implements Req 5.2
    pub async fn list_revisions(
        &self, entry_id: Uuid, page: u64, page_size: u64,
    ) -> Result<PaginatedResponse<Revision>>;

    /// 获取特定版本快照
    /// Implements Req 5.3
    pub async fn get_revision(
        &self, entry_id: Uuid, version_number: i32,
    ) -> Result<Revision>;

    /// 回滚到指定版本（创建新版本）
    /// Implements Req 5.4
    pub async fn rollback(
        &self, entry_id: Uuid, target_version: i32, user_id: Uuid,
    ) -> Result<Revision>;
}
```

---

### Component: PublishManager

**Purpose**: 草稿/发布状态机、发布/撤回操作、发布版本绑定
**Location**: `crates/cycms-publish/src/lib.rs`
**Interface**:

```rust
// Implements Req 6.1, 6.2, 6.3, 6.4

pub struct PublishManager {
    db: Arc<DatabasePool>,
    event_bus: Arc<EventBus>,
}

impl PublishManager {
    /// 发布内容（将 current_version 标记为 published_version）
    /// Implements Req 6.1
    pub async fn publish(&self, entry_id: Uuid, user_id: Uuid) -> Result<ContentEntry>;

    /// 撤回已发布内容
    /// Implements Req 6.3
    pub async fn unpublish(&self, entry_id: Uuid, user_id: Uuid) -> Result<ContentEntry>;

    /// 判断查询是否应返回已发布版本
    /// Implements Req 6.4
    pub fn should_use_published_version(&self, query: &ContentQuery, is_admin: bool) -> bool;

    /// 获取已发布版本的字段数据
    /// Implements Req 6.2
    pub async fn get_published_fields(&self, entry_id: Uuid) -> Result<Option<serde_json::Value>>;
}
```

---

### Component: MediaManager

**Purpose**: 文件上传/下载、存储后端抽象、媒体元数据管理
**Location**: `crates/cycms-media/src/lib.rs`
**Interface**:

```rust
// Implements Req 7.1, 7.2, 7.3, 7.4

#[derive(Debug, Serialize, Deserialize)]
pub struct MediaAsset {
    pub id: Uuid,
    pub filename: String,
    pub original_filename: String,
    pub mime_type: String,
    pub size: i64,
    pub storage_path: String,
    pub url: String,
    pub metadata: Option<serde_json::Value>,
    pub uploaded_by: Uuid,
    pub created_at: DateTime<Utc>,
}

/// 存储后端抽象（插件可实现 S3、OSS 等）
#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn store(&self, filename: &str, data: Bytes, mime: &str) -> Result<StoredFile>;
    async fn delete(&self, path: &str) -> Result<()>;
    async fn get_url(&self, path: &str) -> Result<String>;
}

/// 本地文件系统存储（v0.1 默认）
pub struct LocalStorage { base_path: PathBuf }

pub struct MediaManager {
    db: Arc<DatabasePool>,
    storage: Arc<dyn StorageBackend>,
}

impl MediaManager {
    /// 上传文件
    /// Implements Req 7.1
    pub async fn upload(
        &self, file: UploadFile, user_id: Uuid,
    ) -> Result<MediaAsset>;

    /// 查询媒体列表
    /// Implements Req 7.2
    pub async fn list(&self, query: MediaQuery) -> Result<PaginatedResponse<MediaAsset>>;

    /// 删除媒体（检查引用）
    /// Implements Req 7.3
    pub async fn delete(&self, id: Uuid, force: bool) -> Result<()>;

    /// 获取单个媒体详情
    pub async fn get(&self, id: Uuid) -> Result<MediaAsset>;
}
```

---

### Component: ApiGateway

**Purpose**: REST 路由注册与分发、请求/响应处理、中间件链、OpenAPI 文档聚合
**Location**: `crates/cycms-api/src/lib.rs`
**Interface**:

```rust
// Implements Req 8.1, 8.2, 8.3, 8.4, 8.5

/// 统一错误响应格式
#[derive(Debug, Serialize)]
pub struct ApiErrorResponse {
    pub error: ApiErrorBody,
}

#[derive(Debug, Serialize)]
pub struct ApiErrorBody {
    pub status: u16,
    pub name: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

pub struct ApiGateway {
    ctx: Arc<AppContext>,
}

impl ApiGateway {
    /// 构建完整路由表
    /// Implements Req 8.4, 8.5
    pub fn build_router(&self) -> Router {
        Router::new()
            // 系统路由
            .nest("/api/v1/auth", auth_routes())
            .nest("/api/v1/users", user_routes())
            .nest("/api/v1/roles", role_routes())
            .nest("/api/v1/content-types", content_type_routes())
            .nest("/api/v1/content", content_routes())
            .nest("/api/v1/media", media_routes())
            .nest("/api/v1/plugins", plugin_routes())
            .nest("/api/v1/settings", settings_routes())
            // 插件路由
            .nest("/api/v1/x", plugin_registered_routes())
            // OpenAPI 文档
            .route("/api/docs", get(openapi_json))
            // 中间件链: 日志 → 认证 → 权限
            .layer(middleware_stack())
    }

    /// 合并插件路由
    /// Implements Req 8.5
    pub fn merge_plugin_routes(&self, plugin_name: &str, router: Router);

    /// 聚合 OpenAPI 文档（系统 + 插件）
    /// Implements Req 8.1, 8.2
    pub fn openapi_doc(&self) -> utoipa::openapi::OpenApi;
}
```

---

### Component: EventBus

**Purpose**: 事件定义/发布/订阅、同步与异步事件分发
**Location**: `crates/cycms-events/src/lib.rs`
**Interface**:

```rust
// Implements Req 9.1, 9.2, 9.3, 9.4

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event_type: String,           // e.g. "content.created"
    pub timestamp: DateTime<Utc>,
    pub triggered_by: Option<Uuid>,   // 触发者 user_id
    pub payload: serde_json::Value,   // 事件特定数据
}

/// 事件处理器 trait
#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: &Event) -> Result<()>;
    fn event_types(&self) -> Vec<String>;
}

pub struct EventBus {
    handlers: RwLock<HashMap<String, Vec<Arc<dyn EventHandler>>>>,
}

impl EventBus {
    pub fn new() -> Self;

    /// 注册事件处理器
    /// Implements Req 9.3
    pub fn subscribe(&self, handler: Arc<dyn EventHandler>);

    /// 注销事件处理器（按插件名批量注销）
    pub fn unsubscribe_plugin(&self, plugin_name: &str);

    /// 异步发布事件（不阻塞调用者）
    /// Implements Req 9.2, 9.4
    pub async fn publish(&self, event: Event);

    /// 同步发布事件（等待所有处理器完成）
    pub async fn publish_sync(&self, event: Event) -> Vec<Result<()>>;
}

/// 系统内建事件类型常量
/// Implements Req 9.1
pub mod events {
    pub const CONTENT_CREATED: &str = "content.created";
    pub const CONTENT_UPDATED: &str = "content.updated";
    pub const CONTENT_DELETED: &str = "content.deleted";
    pub const CONTENT_PUBLISHED: &str = "content.published";
    pub const CONTENT_UNPUBLISHED: &str = "content.unpublished";
    pub const USER_CREATED: &str = "user.created";
    pub const USER_UPDATED: &str = "user.updated";
    pub const USER_DELETED: &str = "user.deleted";
    pub const MEDIA_UPLOADED: &str = "media.uploaded";
    pub const MEDIA_DELETED: &str = "media.deleted";
    pub const PLUGIN_INSTALLED: &str = "plugin.installed";
    pub const PLUGIN_ENABLED: &str = "plugin.enabled";
    pub const PLUGIN_DISABLED: &str = "plugin.disabled";
    pub const PLUGIN_UNINSTALLED: &str = "plugin.uninstalled";
}
```

---

### Component: PluginManager

**Purpose**: 插件发现/安装/启用/禁用/升级/卸载、Manifest 解析、依赖解析
**Location**: `crates/cycms-plugin-manager/src/lib.rs`
**Interface**:

```rust
// Implements Req 10.1, 10.2, 10.3, 10.4, 10.5, 10.6, 20.1, 20.2, 20.3, 20.4, 20.5

/// 插件 Manifest（对应 plugin.toml）
#[derive(Debug, Clone, Deserialize)]
pub struct PluginManifest {
    pub plugin: PluginMeta,
    pub compatibility: CompatibilitySpec,
    pub dependencies: Option<HashMap<String, DependencySpec>>,
    pub permissions: Option<PermissionsSpec>,
    pub frontend: Option<FrontendSpec>,
    pub migrations: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub kind: PluginKind,         // native | wasm
    pub entry: String,             // 入口文件路径
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub enum PluginKind { Native, Wasm }

#[derive(Debug, Clone, Deserialize)]
pub struct CompatibilitySpec {
    pub cycms: String,  // semver range, e.g. ">=0.1.0,<0.2.0"
}

#[derive(Debug, Clone, Deserialize)]
pub struct DependencySpec {
    pub version: String,
    pub optional: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub kind: PluginKind,
    pub status: PluginStatus,
    pub dependencies: Vec<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub enum PluginStatus { Enabled, Disabled }

pub struct PluginManager {
    db: Arc<DatabasePool>,
    native_runtime: Arc<NativePluginRuntime>,
    wasm_runtime: Arc<WasmPluginRuntime>,
    migration_engine: Arc<MigrationEngine>,
    permission_engine: Arc<PermissionEngine>,
}

impl PluginManager {
    /// 安装插件
    /// Implements Req 10.1, 10.2, 20.1, 20.2, 20.3, 20.4
    pub async fn install(&self, source: PluginSource) -> Result<PluginInfo>;

    /// 启用插件（按依赖拓扑序）
    /// Implements Req 10.3
    pub async fn enable(&self, name: &str) -> Result<()>;

    /// 禁用插件（级联禁用依赖方）
    /// Implements Req 10.4
    pub async fn disable(&self, name: &str) -> Result<()>;

    /// 卸载插件
    /// Implements Req 10.5
    pub async fn uninstall(&self, name: &str) -> Result<()>;

    /// 列出所有已安装插件
    /// Implements Req 10.6
    pub async fn list(&self) -> Result<Vec<PluginInfo>>;

    /// 依赖拓扑排序
    fn resolve_dependency_order(&self, plugins: &[PluginManifest]) -> Result<Vec<String>>;
}
```

---

### Component: NativePluginRuntime

**Purpose**: Native Rust 插件加载与执行、trait 对象调度、宿主能力注入
**Location**: `crates/cycms-plugin-native/src/lib.rs`
**Interface**:

```rust
// Implements Req 11.1, 11.2, 11.3, 11.4

/// 所有 Native 插件必须实现的 trait
#[async_trait]
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;

    /// 插件启用时调用
    async fn on_enable(&self, ctx: &PluginContext) -> Result<()>;

    /// 插件禁用时调用
    async fn on_disable(&self, ctx: &PluginContext) -> Result<()>;

    /// 返回插件路由（可选）
    fn routes(&self) -> Option<Router> { None }

    /// 返回插件事件处理器（可选）
    fn event_handlers(&self) -> Vec<Arc<dyn EventHandler>> { vec![] }

    /// 返回插件暴露的服务（可选）
    fn services(&self) -> Vec<(String, Arc<dyn Any + Send + Sync>)> { vec![] }
}

/// 插件上下文：提供宿主能力的访问入口
pub struct PluginContext {
    pub content_engine: Arc<ContentEngine>,
    pub event_bus: Arc<EventBus>,
    pub permission_engine: Arc<PermissionEngine>,
    pub service_registry: Arc<ServiceRegistry>,
    pub settings_manager: Arc<SettingsManager>,
    pub db_pool: Arc<DatabasePool>,
}

pub struct NativePluginRuntime {
    plugins: RwLock<HashMap<String, Arc<dyn Plugin>>>,
}

impl NativePluginRuntime {
    /// 加载并初始化 Native 插件
    /// Implements Req 11.1
    pub async fn load(&self, plugin: Arc<dyn Plugin>, ctx: &PluginContext) -> Result<()>;

    /// 获取插件注册的路由
    /// Implements Req 11.2
    pub fn get_routes(&self, name: &str) -> Option<Router>;

    /// 获取插件事件处理器
    /// Implements Req 11.3
    pub fn get_event_handlers(&self, name: &str) -> Vec<Arc<dyn EventHandler>>;

    /// 注册插件服务到 ServiceRegistry
    /// Implements Req 11.4
    pub fn register_services(&self, name: &str, registry: &ServiceRegistry);

    /// 卸载插件
    pub async fn unload(&self, name: &str) -> Result<()>;
}
```

---

### Component: WasmPluginRuntime

**Purpose**: wasmtime 嵌入、Wasm Component 编译/实例化、Host Function 绑定
**Location**: `crates/cycms-plugin-wasm/src/lib.rs`
**WIT 文件目录**: `crates/cycms-plugin-wasm/wit/`（host 与 guest 接口采用 WebAssembly Component Model 的 WIT 定义）

**信任模型**：cycms 对 Wasm 插件采用**完全信任**（与 Native 同权）。Runtime 不做沙箱约束，不强制 fuel / memory / epoch 资源限制；host functions 可完整访问数据库、认证、权限、设置、事件总线；WASI preview 2 通过 `wasmtime-wasi` 完整透传（filesystem / sockets / http / clocks / random / cli / stdio），使 guest 可执行任意系统操作。隔离只保留 wasmtime 对 trap / panic 的天然进程隔离，确保单插件崩溃不影响主进程。安全审计由上层分发渠道（未来的插件市场）负责。

**Interface**:

```rust
// Implements Req 12.1, 12.2, 12.3, 12.4, 12.5
// 使用 wasmtime::component::* API，所有插件均为 Component Model 组件。

use wasmtime::component::{Component, Linker, ResourceTable};

pub struct WasmPluginRuntime {
    engine: wasmtime::Engine,             // 共享 Engine（JIT 编译结果可复用，async_support + component_model 已开）
    linker: Linker<HostState>,            // 绑定全部 Host Function 组 + wasmtime-wasi::p2::add_to_linker_async
    instances: RwLock<HashMap<String, WasmPluginInstance>>,
}

struct WasmPluginInstance {
    store: Mutex<wasmtime::Store<HostState>>,
    bindings: plugin::Plugin,             // 由 wasmtime `bindgen!` 宏生成的 guest 绑定
}

/// 宿主状态，持有对核心组件的引用与资源表
struct HostState {
    db: Arc<DatabasePool>,                // 原始 SQL 入口，支撑 db host 组
    content_engine: Arc<ContentEngine>,
    auth_engine: Arc<AuthEngine>,
    event_bus: Arc<EventBus>,
    permission_engine: Arc<PermissionEngine>,
    service_registry: Arc<ServiceRegistry>,
    settings_manager: Arc<SettingsManager>,
    plugin_name: String,
    resource_table: ResourceTable,        // WASI / Component 资源句柄表
    wasi_ctx: wasmtime_wasi::p2::WasiCtx, // WASI preview 2 上下文（文件/网络/时钟完整继承）
}

impl WasmPluginRuntime {
    /// 编译并实例化 Wasm 组件，绑定 Host Functions
    /// Implements Req 12.1, 12.5
    pub async fn load(&self, name: &str, wasm_bytes: &[u8], ctx: &PluginContext) -> Result<()>;

    /// 调用 Wasm 插件的导出函数处理 HTTP 请求
    /// Implements Req 12.3
    pub async fn handle_request(
        &self, name: &str, func: &str, request: WasmRequest,
    ) -> Result<WasmResponse>;

    /// 调用 Wasm 插件的事件处理函数
    /// Implements Req 12.2
    pub async fn handle_event(&self, name: &str, event: &Event) -> Result<()>;

    /// 卸载 Wasm 插件实例
    pub async fn unload(&self, name: &str) -> Result<()>;
}

/// Host Function 组定义（通过 WIT 接口在 Linker 中注册）
/// Implements Req 12.5
mod host_functions {
    // content 组: content_get, content_list, content_create, content_update, content_delete
    // auth 组: auth_get_current_user, auth_check_token
    // permission 组: permission_check
    // kv 组: kv_get, kv_set, kv_delete
    // http 组: http_fetch（无白名单，可访问任意域名）
    // event 组: event_publish, event_subscribe
    // route 组: route_register
    // log 组: log_info, log_warn, log_error
    // settings 组: settings_get, settings_set
    // db 组: db_query, db_execute（原始 SQL，直接作用于主 DatabasePool）
    //
    // 完全信任模型：不做 host function 白名单，WASI preview 2 完整透传，Wasm 插件与
    // Native 插件具备同等能力。安全审计在上层分发渠道完成。
}
```

**WIT 文件组织约定**：

```
crates/cycms-plugin-wasm/wit/
  deps/               # WASI preview 2 标准接口（wasi:io / wasi:filesystem / wasi:sockets /
                      # wasi:http / wasi:clocks / wasi:random / wasi:cli）完整透传
  host/               # 宿主向 guest 暴露的 10 组 cycms 扩展能力
    content.wit
    auth.wit
    permission.wit
    kv.wit
    http.wit
    event.wit
    route.wit
    log.wit
    settings.wit
    db.wit            # 原始 SQL 执行（不做白名单）
  plugin.wit          # 顶层 world 定义，import host/* 与 wasi:*、export guest lifecycle
```

---

### Component: ServiceRegistry

**Purpose**: 插件间服务发现与调用、typed contract 注册/查询
**Location**: `crates/cycms-plugin-api/src/registry.rs`
**Interface**:

```rust
// Implements Req 13.1, 13.2, 13.3

pub struct ServiceRegistry {
    services: RwLock<HashMap<String, Arc<dyn Any + Send + Sync>>>,
    availability: RwLock<HashMap<String, bool>>,
}

impl ServiceRegistry {
    /// 注册服务
    /// Implements Req 13.1
    pub fn register<T: Send + Sync + 'static>(
        &self, key: &str, service: Arc<T>,
    );

    /// 查找服务（类型安全）
    /// Implements Req 13.2, 13.3
    pub fn get<T: Send + Sync + 'static>(&self, key: &str) -> Result<Arc<T>>;

    /// 标记服务不可用（插件禁用时）
    pub fn set_unavailable(&self, prefix: &str);

    /// 标记服务可用（插件启用时）
    pub fn set_available(&self, prefix: &str);

    /// 注销服务
    pub fn unregister(&self, key: &str);
}
```

---

### Component: DatabaseLayer

**Purpose**: 数据库连接池管理、多数据库抽象、查询构建辅助
**Location**: `crates/cycms-db/src/lib.rs`
**Interface**:

```rust
// Implements Req 19.1, 19.2, 19.3

pub enum DatabasePool {
    Postgres(sqlx::PgPool),
    MySql(sqlx::MySqlPool),
    Sqlite(sqlx::SqlitePool),
}

pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub connect_timeout_secs: u64,
    pub idle_timeout_secs: u64,
}

impl DatabasePool {
    /// 从配置创建连接池
    /// Implements Req 19.3
    pub async fn connect(config: &DatabaseConfig) -> Result<Self>;

    /// 获取数据库类型标识
    pub fn db_type(&self) -> DatabaseType;

    /// 执行原始 SQL
    pub async fn execute(&self, sql: &str) -> Result<u64>;

    /// JSONB 字段查询辅助（PG 使用原生操作符，其他数据库使用 JSON 函数）
    /// Implements Req 19.1, 19.2
    pub fn json_field_query(&self, column: &str, path: &str) -> String;

    /// JSONB 字段设值辅助
    pub fn json_field_set(&self, column: &str, path: &str, value: &str) -> String;
}

pub enum DatabaseType { Postgres, MySql, Sqlite }
```

---

### Component: MigrationEngine

**Purpose**: Schema 迁移执行、迁移版本追踪、回滚支持
**Location**: `crates/cycms-migrate/src/lib.rs`
**Interface**:

```rust
// Implements Req 14.1, 14.2, 14.3, 14.4

pub struct MigrationEngine {
    db: Arc<DatabasePool>,
}

pub struct MigrationRecord {
    pub id: i64,
    pub name: String,
    pub source: String,       // "system" 或 plugin name
    pub applied_at: DateTime<Utc>,
    pub status: MigrationStatus,
}

pub enum MigrationStatus { Applied, Failed, RolledBack }

impl MigrationEngine {
    /// 执行系统迁移
    /// Implements Req 14.1
    pub async fn run_system_migrations(&self) -> Result<Vec<MigrationRecord>>;

    /// 执行插件迁移
    /// Implements Req 14.2
    pub async fn run_plugin_migrations(
        &self, plugin_name: &str, migrations_dir: &Path,
    ) -> Result<Vec<MigrationRecord>>;

    /// 回滚迁移
    /// Implements Req 14.3
    pub async fn rollback(&self, source: &str, count: usize) -> Result<Vec<MigrationRecord>>;

    /// 查询迁移状态
    /// Implements Req 14.4
    pub async fn status(&self) -> Result<Vec<MigrationRecord>>;
}
```

---

### Component: ConfigManager

**Purpose**: 系统配置加载（文件/环境变量）、运行时配置读写
**Location**: `crates/cycms-config/src/lib.rs`
**Interface**:

```rust
// Implements Req 15.1

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub media: MediaConfig,
    pub plugins: PluginsConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub access_token_ttl_secs: u64,
    pub refresh_token_ttl_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MediaConfig {
    pub upload_dir: String,
    pub max_file_size: u64,
    pub allowed_mime_types: Vec<String>,
}

impl AppConfig {
    /// 从 cycms.toml 和环境变量加载配置
    /// 环境变量格式: CYCMS__SERVER__PORT=8080
    /// Implements Req 15.1
    pub fn load(path: Option<&Path>) -> Result<Self>;
}
```

---

### Component: SettingsManager

**Purpose**: 持久化系统设置存储/读取、插件设置管理
**Location**: `crates/cycms-settings/src/lib.rs`
**Interface**:

```rust
// Implements Req 15.2, 15.3

pub struct SettingsManager {
    db: Arc<DatabasePool>,
}

impl SettingsManager {
    /// 读取设置值
    pub async fn get(&self, namespace: &str, key: &str) -> Result<Option<serde_json::Value>>;

    /// 写入设置值
    /// Implements Req 15.2
    pub async fn set(
        &self, namespace: &str, key: &str, value: serde_json::Value,
    ) -> Result<()>;

    /// 获取命名空间下所有设置
    pub async fn get_all(&self, namespace: &str) -> Result<HashMap<String, serde_json::Value>>;

    /// 注册插件设置 Schema
    /// Implements Req 15.3
    pub async fn register_schema(
        &self, plugin_name: &str, schema: serde_json::Value,
    ) -> Result<()>;

    /// 获取插件设置 Schema
    pub async fn get_schema(&self, plugin_name: &str) -> Result<Option<serde_json::Value>>;
}
```

---

### Component: Observability

**Purpose**: 结构化日志集成、请求追踪、审计日志记录
**Location**: `crates/cycms-observability/src/lib.rs`
**Interface**:

```rust
// Implements Req 16.1, 16.2

pub struct AuditLog {
    pub id: Uuid,
    pub actor_id: Uuid,
    pub action: String,          // e.g. "content.create", "user.login"
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub details: Option<serde_json::Value>,
    pub result: AuditResult,
    pub created_at: DateTime<Utc>,
}

pub enum AuditResult { Success, Failure }

pub struct AuditLogger {
    db: Arc<DatabasePool>,
}

impl AuditLogger {
    /// 记录审计日志
    /// Implements Req 16.2
    pub async fn log(&self, entry: AuditLog) -> Result<()>;

    /// 查询审计日志
    pub async fn query(&self, filter: AuditFilter) -> Result<PaginatedResponse<AuditLog>>;
}

/// 请求追踪中间件
/// Implements Req 16.1
pub fn tracing_layer() -> tower_http::trace::TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
    tower_http::trace::DefaultMakeSpan,
    tower_http::trace::DefaultOnRequest,
    tower_http::trace::DefaultOnResponse,
>;
```

---

### Component: CLI

**Purpose**: 项目初始化、插件脚手架、迁移命令、开发服务器
**Location**: `crates/cycms-cli/src/main.rs`
**Interface**:

```rust
// Implements Req 17.1, 17.2, 17.3, 17.4

#[derive(Parser)]
#[command(name = "cycms", about = "cycms CMS CLI")]
pub enum Cli {
    /// 创建新项目
    /// Implements Req 17.1
    New { name: String },

    /// 插件子命令
    Plugin {
        #[command(subcommand)]
        command: PluginCommand,
    },

    /// 执行数据库迁移
    /// Implements Req 17.3
    Migrate {
        #[arg(long)]
        rollback: Option<usize>,
    },

    /// 启动开发服务器
    /// Implements Req 17.4
    Serve {
        #[arg(long, default_value = "cycms.toml")]
        config: String,
    },
}

#[derive(Subcommand)]
pub enum PluginCommand {
    /// 创建插件脚手架
    /// Implements Req 17.2
    New { name: String },
}
```

---

### Component: WebApp

**Purpose**: 统一 React Web 应用，单一前端工程同时承载 `/admin` 管理域与 `/` 访客域
**Location**: `apps/web/`
**Interface**:

```
技术栈: React 19 + TypeScript + Vite

目录结构:
apps/web/
  src/
        api/             # API client 与数据访问封装
        components/      # 通用 UI 组件
        domains/
            admin/         # /admin 管理域
                auth/        # 后台登录与受保护路由 (Req 18.2)
                dashboard/   # 仪表盘 (Req 18.2)
                content-types/ # 内容类型管理 (Req 18.3)
                content/     # 内容列表/编辑/版本历史 (Req 18.4)
                media/       # 媒体库 (Req 18.5)
                plugins/     # 插件管理 (Req 18.6)
                access/      # 用户/角色/权限 (Req 18.7)
                settings/    # 系统设置 (Req 18.8)
            site/          # / 访客域
                shell/       # 首页、导航、页脚、404 (Req 18.9, 18.11)
                content/     # 列表/详情/栏目页 (Req 18.10)
                search/      # 搜索结果页 (Req 18.11)
                member/      # 登录/注册/个人资料 (Req 18.12)
        extensions/      # 插件前端扩展与路由解析器 (Req 18.13, 20.5)
        hooks/           # 通用 hooks
        layouts/         # 布局组件
        router/          # 管理域与访客域路由配置
        stores/          # 会话、UI、查询状态
        types/           # TypeScript 类型定义
```

Key Design Decisions:
- 单一 React 应用承载 `/admin` 与 `/` 两套路由域，共享设计系统、API client 与状态基础设施（Implements Req 18.1）
- 管理域采用 SPA + 受保护路由；访客域采用同源混合渲染策略，由 `cycms serve` 提供静态资产与公开路由入口（Implements Req 18.14）
- 后台表单与插件设置根据 Content Type / settings schema 动态渲染（Implements Req 18.4, 18.8, 15.3）
- 访客域公开路由不绑定固定 `page` / `post` 模型，而是通过插件扩展提供内容类型到页面的解析 contract（Implements Req 18.10, 18.13）
- 会员登录/注册/个人资料与后台认证共用 Auth API，但会话存储与路由守卫按域分离（Implements Req 18.2, 18.12）
- 插件前端扩展通过动态 import 加载，避免引入额外 federation 基础设施（Implements Req 20.5）

---

## Database Schema (Core Tables)

### 多数据库方言映射

核心 DDL 以 PostgreSQL 为基准编写（一级支持），MySQL 8.0+ 与 SQLite 3.38+ 通过下表映射落地。迁移文件按数据库分目录组织：
`migrations/postgres/*.sql`、`migrations/mysql/*.sql`、`migrations/sqlite/*.sql`，由 `MigrationEngine` 根据 `DatabasePool::db_type()` 选择。

| 概念 | PostgreSQL | MySQL 8.0+ | SQLite 3.38+ |
|---|---|---|---|
| UUID 主键 | `UUID DEFAULT gen_random_uuid()` | `CHAR(36)` + 应用层生成 UUID | `TEXT` + 应用层生成 UUID |
| 时间戳（UTC） | `TIMESTAMPTZ DEFAULT now()` | `DATETIME(6) DEFAULT CURRENT_TIMESTAMP(6)` | `TEXT`（ISO 8601）+ `DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))` |
| 半结构化字段 | `JSONB` | `JSON` | `TEXT` + `CHECK(json_valid(x))` |
| JSON 查询 | `->`、`->>`、`@>` 等原生操作符 | `JSON_EXTRACT(col,'$.path')`、`JSON_CONTAINS` | `json_extract(col,'$.path')`、`json_tree` |
| 自增 BIGINT 主键 | `BIGSERIAL` | `BIGINT AUTO_INCREMENT` | `INTEGER PRIMARY KEY AUTOINCREMENT` |
| JSON 索引 | `CREATE INDEX ... USING GIN (col)` | `CREATE INDEX ... ((CAST(col->>'$.x' AS CHAR(64))))`（函数索引，需按字段按需建） | 无原生 JSON 索引，通过表达式索引 `CREATE INDEX ... (json_extract(col,'$.x'))` |
| 布尔 | `BOOLEAN` | `TINYINT(1)` | `INTEGER`（0/1） |
| 外键 | `REFERENCES ... ON DELETE CASCADE` | 同左（InnoDB） | 同左（需 `PRAGMA foreign_keys=ON`） |

> 注意：MySQL 5.7 不在一级兼容范围内；SQLite 3.38 是首个稳定支持 JSON 内建函数的版本。
> GIN 索引在 MySQL/SQLite 上不可用；命中率敏感的 JSON 字段查询性能在非 PG 环境下会明显下降，属于 R19.2 的"功能受限"范畴。

### DDL（PostgreSQL 基准）

```sql
-- 用户与权限
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(255) NOT NULL UNIQUE,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE roles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    is_system BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE user_roles (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, role_id)
);

CREATE TABLE permissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    domain VARCHAR(100) NOT NULL,
    resource VARCHAR(100) NOT NULL,
    action VARCHAR(100) NOT NULL,
    scope VARCHAR(20) NOT NULL DEFAULT 'all',
    source VARCHAR(255) NOT NULL DEFAULT 'system',
    UNIQUE (domain, resource, action, scope)
);

CREATE TABLE role_permissions (
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_id UUID NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    PRIMARY KEY (role_id, permission_id)
);

-- 内容模型
CREATE TABLE content_types (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    api_id VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    kind VARCHAR(20) NOT NULL DEFAULT 'collection',
    fields JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 内容实例
CREATE TABLE content_entries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    content_type_id UUID NOT NULL REFERENCES content_types(id),
    slug VARCHAR(255),
    status VARCHAR(20) NOT NULL DEFAULT 'draft',
    current_version_id UUID,
    published_version_id UUID,
    fields JSONB NOT NULL DEFAULT '{}',
    created_by UUID NOT NULL REFERENCES users(id),
    updated_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    published_at TIMESTAMPTZ
);

CREATE INDEX idx_content_entries_type ON content_entries(content_type_id);
CREATE INDEX idx_content_entries_status ON content_entries(status);
CREATE INDEX idx_content_entries_slug ON content_entries(slug);
CREATE INDEX idx_content_entries_fields ON content_entries USING GIN (fields);

-- 版本
CREATE TABLE content_revisions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    content_entry_id UUID NOT NULL REFERENCES content_entries(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL,
    snapshot JSONB NOT NULL,
    change_summary TEXT,
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (content_entry_id, version_number)
);

-- 内容关联
CREATE TABLE content_relations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_entry_id UUID NOT NULL REFERENCES content_entries(id) ON DELETE CASCADE,
    target_entry_id UUID NOT NULL REFERENCES content_entries(id) ON DELETE CASCADE,
    field_api_id VARCHAR(255) NOT NULL,
    relation_kind VARCHAR(20) NOT NULL,
    position INTEGER NOT NULL DEFAULT 0
);

-- 媒体
CREATE TABLE media_assets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    filename VARCHAR(255) NOT NULL,
    original_filename VARCHAR(255) NOT NULL,
    mime_type VARCHAR(127) NOT NULL,
    size BIGINT NOT NULL,
    storage_path TEXT NOT NULL,
    metadata JSONB,
    uploaded_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 插件
CREATE TABLE plugins (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL UNIQUE,
    version VARCHAR(50) NOT NULL,
    kind VARCHAR(20) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'disabled',
    manifest JSONB NOT NULL,
    installed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 设置
CREATE TABLE settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    namespace VARCHAR(255) NOT NULL,
    key VARCHAR(255) NOT NULL,
    value JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (namespace, key)
);

-- 插件设置 Schema
CREATE TABLE plugin_settings_schemas (
    plugin_name VARCHAR(255) PRIMARY KEY,
    schema JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 迁移记录
CREATE TABLE migration_records (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    source VARCHAR(255) NOT NULL,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    status VARCHAR(20) NOT NULL DEFAULT 'applied'
);

-- 审计日志
CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    actor_id UUID NOT NULL,
    action VARCHAR(255) NOT NULL,
    resource_type VARCHAR(255) NOT NULL,
    resource_id VARCHAR(255),
    details JSONB,
    result VARCHAR(20) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_audit_logs_actor ON audit_logs(actor_id);
CREATE INDEX idx_audit_logs_action ON audit_logs(action);
CREATE INDEX idx_audit_logs_created ON audit_logs(created_at);

-- 插件 KV 存储
CREATE TABLE plugin_kv (
    plugin_name VARCHAR(255) NOT NULL,
    key VARCHAR(255) NOT NULL,
    value JSONB NOT NULL,
    expires_at TIMESTAMPTZ,
    PRIMARY KEY (plugin_name, key)
);
```

---

## Router 职责分工：Kernel vs ApiGateway

为避免 `build_router` 职责重叠，明确约定如下：

- **ApiGateway (`crates/cycms-api`)** 负责构建 **系统内部** 的顶层路由：`auth / users / roles / content-types / content / media / plugins / settings` 以及 `/api/docs`。不知道任何插件存在。
- **Kernel (`crates/cycms-kernel`)** 负责在 `bootstrap` 期间拉起 `ApiGateway` 与 `PluginManager`，取到系统 Router 与**已启用插件的 Router 集合**后，进行最终合并：
  - 插件 Router 统一挂到 `/api/v1/x/{plugin_name}` 前缀下（由 Kernel 完成 `nest`，不交由 ApiGateway 处理）。
  - Kernel 负责叠加跨系统/插件共用的中间件层：请求追踪 → 认证 → 权限 → 限流 → CORS。
- 热更新插件路由时：PluginManager 触发事件，Kernel 在受控点重建 `Arc<Router>` 并原子替换。ApiGateway 不持有插件路由引用。

## Error Code 规范（Req 8.3）

统一错误响应的 `error.name` 字段取值采用 **小写蛇形命名**，分为以下顶层类别（后接可选的具体错误码）：

| Category | HTTP Status | 场景 |
|---|---|---|
| `bad_request` | 400 | 请求格式不合法、参数缺失 |
| `validation_error` | 422 | 字段 Schema 校验失败（`details` 内含字段级错误数组） |
| `unauthorized` | 401 | 未认证或 token 失效（不泄露具体原因，详见 R1.2） |
| `forbidden` | 403 | 已认证但无权限 |
| `not_found` | 404 | 资源不存在或对当前用户不可见 |
| `conflict` | 409 | 唯一性冲突、并发修改、状态机非法跃迁 |
| `rate_limited` | 429 | 触发限流 |
| `payload_too_large` | 413 | 上传体超限 |
| `unsupported_media_type` | 415 | 上传 MIME 不在允许列表 |
| `plugin_error` | 502 | 插件返回错误或 wasm trap |
| `internal_error` | 500 | 未分类的服务端错误 |

`details` 字段为可选的结构化补充，字段级错误统一形如：`{ "field": "title", "rule": "maxLength", "message": "超过 255 字符" }`。

## Non-Functional Design

### 限流（Rate Limiting）

- 在 Kernel 中间件栈内，`认证` 之后、业务之前，基于 `tower-governor` 按 `user_id`（已认证）或来源 IP（匿名）施加令牌桶限流。
- 默认配额：每用户 60 req/min（可在 `cycms.toml` 的 `[server.rate_limit]` 覆写）；敏感端点（登录/注册/刷新）额外更严格（10 req/min）。
- 触发后返回 `rate_limited / 429`，并带 `Retry-After` 头。

### CORS

- 使用 `tower_http::cors::CorsLayer`，默认 `allow_origin` 仅白名单（配置 `[server.cors.allowed_origins]`），凭据 `allow_credentials = true`；`OPTIONS` 预检缓存 600s。
- WebApp 与后端同源部署时可关闭跨域宽松配置。

### 上传校验

- `MediaConfig.max_file_size`（字节）与 `allowed_mime_types`（MIME 白名单）在 `MediaManager::upload` 入口检查；拒绝时返回 `payload_too_large` 或 `unsupported_media_type`。
- 文件名做 `path traversal` 去除（不允许 `..`/绝对路径），服务端使用内部生成的 `storage_path`。

### 密码策略

- 最低长度 10，至少 3 类字符（小写/大写/数字/符号）；注册与修改密码均校验。
- 哈希使用 Argon2id，默认参数：`m_cost=19MiB, t_cost=2, p_cost=1`（见 OWASP 2024 推荐）；参数可通过 `AuthConfig.argon2_*` 覆写。
- 密码错误计数：同一账号 15 分钟内 ≥ 5 次失败触发账号级锁定 5 分钟（记入审计日志）。

### Token 轮换与撤销

- `access_token` TTL 默认 15 分钟；`refresh_token` TTL 默认 14 天。
- refresh 端点每次 **轮换** refresh_token（旧 token 进入黑名单 `revoked_tokens`，24 小时后清理）；如检测到旧 refresh_token 再次被使用则触发该用户全会话强制下线（疑似泄露）。
- 登出端点将当前 access_token 的 `jti` 与 refresh_token 一并列入 `revoked_tokens`；认证中间件每次校验需查询 Redis 缓存（键 `auth:revoked:{jti}`，TTL 对齐 access_token 剩余寿命）以限制 DB 压力。
- `revoked_tokens` 表结构：`{ jti VARCHAR(64) PRIMARY KEY, expires_at TIMESTAMPTZ, reason VARCHAR(32) }`。

### SQL 注入防御

- 所有持久化查询必须通过 `sqlx::query!` / `query_as!` 宏或参数化 API 构造；`DatabasePool::execute(sql: &str)` 仅限于系统迁移和调试脚本，禁止将用户输入拼入 `sql` 字符串。
- 插件的 `kv / content` Host Function 全部走 `ContentEngine` 和 `KvStore` 的高层 API，不暴露原始 SQL 入口。
