use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::{Request, State};
use axum::http::{HeaderValue, Method, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use cycms_auth::AuthEngine;
use cycms_config::AppConfig;
use cycms_config::CorsConfig;
use cycms_content_engine::ContentEngine;
use cycms_content_model::{ContentModelRegistry, FieldTypeRegistry, seed_default_types};
use cycms_core::{Error, Result};
use cycms_db::DatabasePool;
use cycms_events::EventBus;
use cycms_media::MediaManager;
use cycms_migrate::MigrationEngine;
use cycms_observability::{AuditLogger, init_tracing, request_span_middleware};
use cycms_permission::PermissionEngine;
use cycms_plugin_api::{PluginContext, ServiceRegistry};
use cycms_plugin_manager::{PluginManager, PluginManagerConfig, PluginRuntime};
use cycms_plugin_native::NativePluginRuntime;
use cycms_plugin_wasm::WasmPluginRuntime;
use cycms_publish::PublishManager;
use cycms_revision::RevisionManager;
use cycms_settings::SettingsManager;
use semver::Version;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tower::ServiceBuilder;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tracing::{info, warn};

/// 全局应用上下文，Kernel bootstrap 后在所有组件间共享。
#[non_exhaustive]
pub struct AppContext {
    /// 任务 2：真实应用配置对象。
    pub config: Arc<AppConfig>,
    /// 任务 3：多方言数据库连接池。
    pub db: Arc<DatabasePool>,
    /// 任务 5：认证引擎，提供登录/刷新/初始管理员/Token 校验等能力。
    pub auth_engine: Arc<AuthEngine>,
    /// 任务 6：权限引擎，提供角色/权限 CRUD 与 `check_permission` 判定。
    pub permission_engine: Arc<PermissionEngine>,
    /// 任务 7：进程内异步事件总线，按 `EventKind` 广播订阅者。
    pub event_bus: Arc<EventBus>,
    /// 任务 8:系统与插件设置的统一访问门面。
    pub settings_manager: Arc<SettingsManager>,
    /// 任务 9：插件间服务发现与调用门面。
    pub service_registry: Arc<ServiceRegistry>,
    /// 任务 10：内容类型管理与字段校验 / Schema 输出门面。
    pub content_model: Arc<ContentModelRegistry>,
    /// 任务 11：内容实例 CRUD + 查询引擎 + `EventBus` 集成门面。
    pub content_engine: Arc<ContentEngine>,
    /// 任务 12：内容版本快照与回滚门面。
    pub revision_manager: Arc<RevisionManager>,
    /// 任务 13：发布状态机门面（Draft → Published / Published → Draft）。
    pub publish_manager: Arc<PublishManager>,
    /// 任务 14：媒体资产管理门面（上传/查询/删除）。
    pub media_manager: Arc<MediaManager>,
    /// 任务 15：插件生命周期管理器，封装 install / enable / disable / uninstall 状态机。
    pub plugin_manager: Arc<PluginManager>,
    /// 任务 16：Native 插件运行时。
    /// 宿主代码 / CLI 在 `serve` 前通过 `native_runtime.register_plugin(...)` 交付
    /// `Arc<dyn Plugin>`，`PluginManager::enable` 时由此 runtime 执行生命周期钩子。
    pub native_runtime: Arc<NativePluginRuntime>,
    /// 任务 17：Wasm Component Model 插件运行时。
    /// `PluginManager::enable` 时由此 runtime 从 `.wasm` 加载 guest 并执行生命周期
    /// 钩子；`all_routes()` 暴露 guest 注册的路由供 API Gateway 合并。
    pub wasm_runtime: Arc<WasmPluginRuntime>,
}

/// 应用生命周期管理入口。
#[allow(dead_code)]
pub struct Kernel {
    config: AppConfig,
    config_path: Option<PathBuf>,
}

impl Kernel {
    /// 从配置文件路径构建 [`Kernel`] 实例。
    ///
    /// # Errors
    /// 配置文件读取或解析失败时返回错误。
    #[allow(clippy::unused_async)]
    pub async fn build(config_path: Option<&Path>) -> Result<Self> {
        let config = AppConfig::load(config_path)?;
        Ok(Self {
            config,
            config_path: config_path.map(Path::to_path_buf),
        })
    }

    /// 初始化所有子系统并返回 [`AppContext`]。
    ///
    /// 初始化顺序：Config → DB → Migration → Auth → Permission → `EventBus` →
    /// `ServiceRegistry` → `ContentModel` → `RevisionManager` → `ContentEngine` →
    /// `PluginContext` → `PluginManager` → API
    ///
    /// 当 `system_migrations_dir` 为 `Some` 时会执行系统迁移并注入默认 `page` / `post`
    /// 内容类型；传 `None` 跳过迁移与 seed，适合只想构造上下文做诊断的调用方。
    ///
    /// # Errors
    /// 任意子系统初始化失败时返回错误。
    pub async fn bootstrap(&self, system_migrations_dir: Option<&Path>) -> Result<AppContext> {
        let db = Arc::new(DatabasePool::connect(&self.config.database).await?);

        let migration_engine = Arc::new(MigrationEngine::new(Arc::clone(&db)));
        let migrations_applied = system_migrations_dir.is_some();
        if let Some(dir) = system_migrations_dir {
            migration_engine.run_system_migrations(dir).await?;
        }

        let event_bus = Arc::new(EventBus::new());
        if self.config.observability.audit_enabled {
            let audit_logger = Arc::new(AuditLogger::new(Arc::clone(&db)));
            let _subscriptions = audit_logger.subscribe_all(&event_bus);
        }

        let auth_engine = Arc::new(AuthEngine::new(Arc::clone(&db), self.config.auth.clone())?);
        let permission_engine = Arc::new(PermissionEngine::new(Arc::clone(&db)));
        let settings_manager = Arc::new(SettingsManager::new(Arc::clone(&db)));
        let field_type_registry = Arc::new(FieldTypeRegistry::new());
        let content_model = Arc::new(ContentModelRegistry::new(
            Arc::clone(&db),
            Arc::clone(&field_type_registry),
        ));
        if migrations_applied {
            seed_default_types(&content_model).await?;
        }
        let service_registry = Arc::new(ServiceRegistry::new());
        let revision_manager = Arc::new(RevisionManager::new(Arc::clone(&db)));
        let publish_manager = Arc::new(PublishManager::new(&db, Arc::clone(&event_bus)));
        let media_manager = Arc::new(MediaManager::new(
            &db,
            Arc::clone(&event_bus),
            &self.config.media,
        ));
        let content_engine = Arc::new(ContentEngine::new(
            Arc::clone(&db),
            Arc::clone(&content_model),
            Arc::clone(&event_bus),
            self.config.content.clone(),
            Arc::clone(&revision_manager),
        ));
        register_core_services(
            &service_registry,
            &db,
            &auth_engine,
            &permission_engine,
            &event_bus,
            &settings_manager,
            &content_model,
            &content_engine,
            &revision_manager,
            &publish_manager,
            &media_manager,
        )?;

        let plugin_context = Arc::new(PluginContext::new(
            Arc::clone(&db),
            Arc::clone(&auth_engine),
            Arc::clone(&permission_engine),
            Arc::clone(&event_bus),
            Arc::clone(&settings_manager),
            Arc::clone(&content_model),
            Arc::clone(&content_engine),
            Arc::clone(&revision_manager),
            Arc::clone(&publish_manager),
            Arc::clone(&media_manager),
            Arc::clone(&service_registry),
        ));

        let cycms_version =
            Version::parse(env!("CARGO_PKG_VERSION")).map_err(|e| Error::Internal {
                message: format!("parse cycms version: {e}"),
                source: None,
            })?;
        let plugins_root =
            resolve_plugins_root(self.config_path.as_deref(), &self.config.plugins.directory);
        let native_runtime = Arc::new(NativePluginRuntime::new());
        let native_as_trait: Arc<dyn PluginRuntime> =
            Arc::clone(&native_runtime) as Arc<dyn PluginRuntime>;
        let wasm_runtime = Arc::new(WasmPluginRuntime::new()?);
        let wasm_as_trait: Arc<dyn PluginRuntime> =
            Arc::clone(&wasm_runtime) as Arc<dyn PluginRuntime>;
        let plugin_manager = Arc::new(PluginManager::new(
            Arc::clone(&db),
            Arc::clone(&migration_engine),
            Arc::clone(&permission_engine),
            Arc::clone(&settings_manager),
            Arc::clone(&service_registry),
            Arc::clone(&event_bus),
            Arc::clone(&plugin_context),
            PluginManagerConfig {
                cycms_version,
                plugins_root,
                runtimes: vec![native_as_trait, wasm_as_trait],
            },
        ));
        service_registry.register("system.plugin_manager", Arc::clone(&plugin_manager))?;
        if migrations_applied {
            plugin_manager.restore_enabled_plugins().await?;
        }

        Ok(AppContext {
            config: Arc::new(self.config.clone()),
            db,
            auth_engine,
            permission_engine,
            event_bus,
            settings_manager,
            service_registry,
            content_model,
            content_engine,
            revision_manager,
            publish_manager,
            media_manager,
            plugin_manager,
            native_runtime,
            wasm_runtime,
        })
    }

    /// 启动 HTTP 服务器，阻塞直至收到关闭信号。
    ///
    /// # Errors
    /// 端口绑定失败或运行时错误时返回错误。
    pub async fn serve(self) -> Result<()> {
        init_tracing(&self.config.observability)?;
        let migrations_dir = default_system_migrations_dir();
        let ctx = self.bootstrap(Some(&migrations_dir)).await?;
        let api_state = build_api_state(&ctx);
        let rate_limit = Arc::new(RateLimitState::new(
            ctx.config.server.rate_limit.requests_per_minute,
            ctx.config.server.rate_limit.sensitive_requests_per_minute,
        ));
        let app = cycms_api::build_router(api_state).layer(
            ServiceBuilder::new()
                .layer(middleware::from_fn(request_span_middleware))
                .layer(middleware::from_fn_with_state(
                    rate_limit,
                    rate_limit_middleware,
                ))
                .layer(build_cors_layer(&ctx.config.server.cors)?),
        );

        let address = format!("{}:{}", ctx.config.server.host, ctx.config.server.port);
        let listener = TcpListener::bind(&address)
            .await
            .map_err(|source| Error::Internal {
                message: format!("failed to bind http listener on {address}: {source}"),
                source: Some(Box::new(source)),
            })?;
        info!(address = %address, "cycms http server listening");

        axum::serve(listener, app.into_make_service())
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(|source| Error::Internal {
                message: format!("http server terminated with error: {source}"),
                source: Some(Box::new(source)),
            })?;

        self.shutdown(&ctx).await
    }

    /// 优雅关闭所有子系统。
    ///
    /// # Errors
    /// 关闭过程中出现不可恢复错误时返回错误。
    pub async fn shutdown(&self, ctx: &AppContext) -> Result<()> {
        let mut native_plugins = PluginRuntime::loaded_plugins(ctx.native_runtime.as_ref());
        native_plugins.sort();
        for plugin_name in native_plugins.into_iter().rev() {
            if let Err(error) =
                PluginRuntime::unload(ctx.native_runtime.as_ref(), &plugin_name).await
            {
                warn!(plugin = %plugin_name, error = %error, "failed to unload native plugin during shutdown");
            }
            ctx.service_registry.set_unavailable(&plugin_name);
        }

        let mut wasm_plugins = PluginRuntime::loaded_plugins(ctx.wasm_runtime.as_ref());
        wasm_plugins.sort();
        for plugin_name in wasm_plugins.into_iter().rev() {
            if let Err(error) = PluginRuntime::unload(ctx.wasm_runtime.as_ref(), &plugin_name).await
            {
                warn!(plugin = %plugin_name, error = %error, "failed to unload wasm plugin during shutdown");
            }
            ctx.service_registry.set_unavailable(&plugin_name);
        }

        info!("cycms kernel shutdown complete");
        Ok(())
    }
}

struct RateLimitState {
    general_limit: u32,
    sensitive_limit: u32,
    general: Mutex<WindowCounter>,
    sensitive: Mutex<WindowCounter>,
}

struct WindowCounter {
    window_started_at: Instant,
    count: u32,
}

impl RateLimitState {
    fn new(general_limit: u32, sensitive_limit: u32) -> Self {
        let now = Instant::now();
        Self {
            general_limit,
            sensitive_limit,
            general: Mutex::new(WindowCounter {
                window_started_at: now,
                count: 0,
            }),
            sensitive: Mutex::new(WindowCounter {
                window_started_at: now,
                count: 0,
            }),
        }
    }

    async fn check(&self, sensitive: bool) -> Result<()> {
        let (limit, state, label) = if sensitive {
            (self.sensitive_limit, &self.sensitive, "sensitive")
        } else {
            (self.general_limit, &self.general, "general")
        };

        if limit == 0 {
            return Ok(());
        }

        let mut counter = state.lock().await;
        if counter.window_started_at.elapsed() >= Duration::from_secs(60) {
            counter.window_started_at = Instant::now();
            counter.count = 0;
        }

        if counter.count >= limit {
            return Err(Error::RateLimited {
                message: format!("{label} request limit exceeded: {limit} per minute"),
            });
        }

        counter.count += 1;
        Ok(())
    }
}

fn build_api_state(ctx: &AppContext) -> Arc<cycms_api::ApiState> {
    Arc::new(cycms_api::ApiState::new(
        Arc::clone(&ctx.config),
        Arc::clone(&ctx.auth_engine),
        Arc::clone(&ctx.permission_engine),
        Arc::clone(&ctx.event_bus),
        Arc::clone(&ctx.content_model),
        Arc::clone(&ctx.content_engine),
        Arc::clone(&ctx.revision_manager),
        Arc::clone(&ctx.publish_manager),
        Arc::clone(&ctx.media_manager),
        Arc::clone(&ctx.plugin_manager),
        Arc::clone(&ctx.settings_manager),
        Arc::clone(&ctx.service_registry),
        Arc::clone(&ctx.native_runtime),
        Arc::clone(&ctx.wasm_runtime),
    ))
}

fn build_cors_layer(config: &CorsConfig) -> Result<CorsLayer> {
    let mut cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .allow_credentials(config.allow_credentials)
        .max_age(Duration::from_secs(config.max_age_secs));

    if config.allowed_origins.iter().any(|origin| origin == "*") {
        cors = cors.allow_origin(Any);
    } else {
        let origins = config
            .allowed_origins
            .iter()
            .map(|origin| {
                HeaderValue::from_str(origin).map_err(|source| Error::BadRequest {
                    message: format!("invalid CORS origin configured: {origin}"),
                    source: Some(Box::new(source)),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        cors = cors.allow_origin(AllowOrigin::list(origins));
    }

    Ok(cors)
}

fn default_system_migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../cycms-migrate/migrations/system")
}

async fn rate_limit_middleware(
    State(rate_limit): State<Arc<RateLimitState>>,
    request: Request,
    next: Next,
) -> Response {
    if request.method() == Method::OPTIONS {
        return next.run(request).await;
    }

    let sensitive = is_sensitive_path(request.uri().path());
    if let Err(error) = rate_limit.check(sensitive).await {
        return error.into_response();
    }

    next.run(request).await
}

fn is_sensitive_path(path: &str) -> bool {
    matches!(
        path,
        "/api/v1/auth/login" | "/api/v1/auth/register" | "/api/v1/auth/refresh"
    )
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut signal) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            signal.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
}

/// 启动期把核心子系统注册到 `ServiceRegistry`，供插件通过
/// `{plugin_name}.{service_name}` 约定查询（对齐 Req 13.1）。
///
/// 核心子系统统一使用 `system` 作为 plugin 段，service 段沿用子系统约定名。
#[allow(clippy::too_many_arguments)]
fn register_core_services(
    registry: &ServiceRegistry,
    db: &Arc<DatabasePool>,
    auth_engine: &Arc<AuthEngine>,
    permission_engine: &Arc<PermissionEngine>,
    event_bus: &Arc<EventBus>,
    settings_manager: &Arc<SettingsManager>,
    content_model: &Arc<ContentModelRegistry>,
    content_engine: &Arc<ContentEngine>,
    revision_manager: &Arc<RevisionManager>,
    publish_manager: &Arc<PublishManager>,
    media_manager: &Arc<MediaManager>,
) -> Result<()> {
    registry.register("system.db", Arc::clone(db))?;
    registry.register("system.auth", Arc::clone(auth_engine))?;
    registry.register("system.permission", Arc::clone(permission_engine))?;
    registry.register("system.events", Arc::clone(event_bus))?;
    registry.register("system.settings", Arc::clone(settings_manager))?;
    registry.register("system.content_model", Arc::clone(content_model))?;
    registry.register("system.content_engine", Arc::clone(content_engine))?;
    registry.register("system.media", Arc::clone(media_manager))?;
    registry.register("system.revision", Arc::clone(revision_manager))?;
    registry.register("system.publish", Arc::clone(publish_manager))?;
    Ok(())
}

/// 解析 `plugins.directory` 到绝对路径：相对路径时以配置文件所在目录为基准，
/// 否则使用当前工作目录；已是绝对路径时直接返回。
fn resolve_plugins_root(config_path: Option<&Path>, directory: &str) -> PathBuf {
    let raw = PathBuf::from(directory);
    if raw.is_absolute() {
        return raw;
    }
    let base = config_path
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_default();
    base.join(raw)
}
