mod request_lifecycle;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::{Request, State};
use axum::http::{HeaderName, HeaderValue, Method, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use cycms_auth::AuthEngine;
use cycms_config::AdminShellMode;
use cycms_config::AppConfig;
use cycms_config::CorsConfig;
use cycms_config::PublicPagesMode;
use cycms_config::{JWT_SECRET_PLACEHOLDER, MIN_JWT_SECRET_BYTES};
use cycms_content_engine::ContentEngine;
use cycms_content_model::{ContentModelRegistry, FieldTypeRegistry, seed_default_types};
use cycms_core::{Error, Result};
use cycms_db::DatabasePool;
use cycms_events::EventBus;
use cycms_host_types::{
    AdminPageMode, AdminPageRegistration, AssetBundleRegistration, CompiledExtensionRegistry,
    OwnershipMode, RegistrationOriginKind, RegistrationSource,
};
use cycms_media::MediaManager;
use cycms_migrate::MigrationEngine;
use cycms_observability::{AuditLogger, init_tracing, request_span_middleware};
use cycms_permission::PermissionEngine;
use cycms_plugin_api::{PluginContext, ServiceRegistry};
use cycms_plugin_manager::{
    HostRegistry, PluginManager, PluginManagerConfig, PluginRuntime, compile_extensions,
};
use cycms_plugin_native::NativePluginRuntime;
use cycms_plugin_wasm::WasmPluginRuntime;
use cycms_publish::PublishManager;
use cycms_revision::RevisionManager;
use cycms_settings::SettingsManager;
use semver::Version;
use serde_json::Value;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tower::{ServiceBuilder, ServiceExt};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tracing::{info, warn};

use crate::request_lifecycle::{DefaultRequestLifecycleEngine, LifecyclePhase, LifecycleTrace};

const COMPILED_REGISTRY_ARTIFACT_NAME: &str = ".compiled-registry.json";

/// 全局应用上下文，Kernel bootstrap 后在所有组件间共享。
#[non_exhaustive]
pub struct AppContext {
    pub config: Arc<AppConfig>,
    pub db: Arc<DatabasePool>,
    pub auth_engine: Arc<AuthEngine>,
    pub permission_engine: Arc<PermissionEngine>,
    pub event_bus: Arc<EventBus>,
    pub settings_manager: Arc<SettingsManager>,
    pub service_registry: Arc<ServiceRegistry>,
    pub content_model: Arc<ContentModelRegistry>,
    pub content_engine: Arc<ContentEngine>,
    pub revision_manager: Arc<RevisionManager>,
    pub publish_manager: Arc<PublishManager>,
    pub media_manager: Arc<MediaManager>,
    pub plugin_manager: Arc<PluginManager>,
    pub host_registry: Arc<HostRegistry>,
    pub native_runtime: Arc<NativePluginRuntime>,
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
    /// 当 `system_migrations_dir` 为 `Some` 时会执行系统迁移并注入默认博客预设
    /// 内容类型；传 `None` 跳过迁移与 seed，适合只想构造上下文做诊断的调用方。
    ///
    /// # Errors
    /// 任意子系统初始化失败时返回错误。
    pub async fn bootstrap(&self, system_migrations_dir: Option<&Path>) -> Result<AppContext> {
        validate_auth_config(&self.config)?;

        let db = Arc::new(DatabasePool::connect(&self.config.database).await?);

        let migration_engine = Arc::new(MigrationEngine::new(Arc::clone(&db)));
        let migrations_applied = system_migrations_dir.is_some();
        if let Some(dir) = system_migrations_dir {
            migration_engine.run_system_migrations(dir).await?;
        }

        let event_bus = Arc::new(EventBus::with_config(
            self.config.events.channel_capacity,
            Duration::from_secs(self.config.events.handler_timeout_secs),
        ));
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
        let web_dist_for_registry = resolve_web_dist_dir(self.config_path.as_deref());
        let compiled = if self.config.host_rendering.compiled_registry_required {
            let artifact_path = compiled_registry_artifact_path(&plugins_root);
            load_compiled_registry_artifact(&artifact_path)?
        } else {
            compile_extensions(&plugins_root)?
        };
        let compiled = inject_native_admin_island_pages(compiled, &web_dist_for_registry);
        let host_registry = Arc::new(HostRegistry::new(compiled));
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
            host_registry,
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

        let api_router = cycms_api::build_router(api_state);
        let admin_extension_security = Arc::new(cycms_api::build_admin_extension_security_state(
            &ctx.config.admin_extensions,
        ));

        // 静态文件：上传目录
        let uploads_dir = PathBuf::from(&ctx.config.media.upload_dir);
        let uploads_service = ServeDir::new(&uploads_dir);

        // 前端 SPA：构建产物目录（apps/web/dist）
        let web_dist = resolve_web_dist_dir(self.config_path.as_deref());
        let public_fallback_service = build_public_fallback_service(
            ctx.config.host_rendering.public_pages_mode,
            Arc::clone(&ctx.host_registry),
            web_dist.clone(),
        );
        let admin_fallback_router = build_admin_fallback_router(
            ctx.config.host_rendering.admin_shell_mode,
            Arc::clone(&ctx.host_registry),
            web_dist,
        );

        let app = api_router
            .nest_service("/uploads", uploads_service)
            .merge(admin_fallback_router)
            .fallback_service(public_fallback_service)
            .layer(
                ServiceBuilder::new()
                    .layer(middleware::from_fn_with_state(
                        admin_extension_security,
                        admin_extension_security_middleware,
                    ))
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
        if counter.window_started_at.elapsed() >= Duration::from_mins(1) {
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
    let admin_extension_events = Arc::new(cycms_api::AdminExtensionEventStore::new(
        ctx.config.admin_extensions.recent_event_capacity,
    ));
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
        admin_extension_events,
        Arc::clone(&ctx.host_registry),
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

async fn admin_extension_security_middleware(
    State(security): State<Arc<cycms_api::AdminExtensionSecurityState>>,
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;

    if security.csp_enabled && !security.csp_policy.is_empty() {
        let header_name = if security.csp_report_only {
            HeaderName::from_static("content-security-policy-report-only")
        } else {
            HeaderName::from_static("content-security-policy")
        };
        if let Ok(header_value) = HeaderValue::from_str(&security.csp_policy) {
            response.headers_mut().insert(header_name, header_value);
        }
    }

    response
}

fn is_sensitive_path(path: &str) -> bool {
    matches!(
        path,
        "/api/v1/auth/login"
            | "/api/v1/auth/register"
            | "/api/v1/auth/refresh"
            | "/api/v1/public/auth/login"
            | "/api/v1/public/auth/register"
            | "/api/v1/public/auth/refresh"
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
/// `{plugin_name}.{service_name}` 约定查询。
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

/// 解析前端构建产物目录：以配置文件所在目录或 cwd 为基准，拼接 `apps/web/dist`。
fn resolve_web_dist_dir(config_path: Option<&Path>) -> PathBuf {
    let base = config_path
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_default();
    base.join("apps/web/dist")
}

fn compiled_registry_artifact_path(plugins_root: &Path) -> PathBuf {
    plugins_root.join(COMPILED_REGISTRY_ARTIFACT_NAME)
}

fn load_compiled_registry_artifact(path: &Path) -> Result<CompiledExtensionRegistry> {
    let payload = std::fs::read_to_string(path).map_err(|source| Error::BadRequest {
        message: format!(
            "compiled registry artifact is required but not readable: {}",
            path.display()
        ),
        source: Some(Box::new(source)),
    })?;
    serde_json::from_str(&payload).map_err(|source| Error::BadRequest {
        message: format!(
            "invalid compiled registry artifact JSON at {}: {source}",
            path.display()
        ),
        source: None,
    })
}

fn build_public_fallback_service(
    mode: PublicPagesMode,
    host_registry: Arc<HostRegistry>,
    web_dist: PathBuf,
) -> Router {
    let host_island_runtime_module = resolve_host_island_runtime_module(&web_dist);
    Router::new()
        .fallback(public_fallback_handler)
        .with_state(PublicFallbackState {
            mode,
            web_dist,
            lifecycle_engine: DefaultRequestLifecycleEngine::with_host_island_runtime_module(
                host_registry,
                host_island_runtime_module,
            ),
        })
}

fn build_admin_fallback_router(
    mode: AdminShellMode,
    host_registry: Arc<HostRegistry>,
    web_dist: PathBuf,
) -> Router {
    let host_island_runtime_module = resolve_host_island_runtime_module(&web_dist);
    Router::new()
        .route("/admin", any(admin_fallback_handler))
        .route("/admin/{*path}", any(admin_fallback_handler))
        .with_state(AdminFallbackState {
            mode,
            web_dist,
            lifecycle_engine: DefaultRequestLifecycleEngine::with_host_island_runtime_module(
                host_registry,
                host_island_runtime_module,
            ),
        })
}

/// 将 `CyCMS` 内置 island admin 页面（content-workspace、media-workspace）注入
/// 已编译的扩展注册表。仅在 Vite 产物存在时注入，dist 不存在时跳过。
fn inject_native_admin_island_pages(
    mut compiled: CompiledExtensionRegistry,
    web_dist: &Path,
) -> CompiledExtensionRegistry {
    let source = RegistrationSource {
        plugin_name: "cycms".to_owned(),
        plugin_version: env!("CARGO_PKG_VERSION").to_owned(),
        origin: RegistrationOriginKind::HostManifest,
        declaration_order: 0,
    };

    struct NativeIsland {
        bundle_id: &'static str,
        page_id: &'static str,
        path: &'static str,
        title: &'static str,
        menu_label: &'static str,
        menu_zone: &'static str,
        manifest_key: &'static str,
        stable_name: &'static str,
    }

    let natives = [
        NativeIsland {
            bundle_id: "cycms:native:content-workspace",
            page_id: "cycms:content-management",
            path: "/admin/content",
            title: "内容管理",
            menu_label: "内容管理",
            menu_zone: "content",
            manifest_key: "src/islands/content-workspace.tsx",
            stable_name: "admin-content-workspace",
        },
        NativeIsland {
            bundle_id: "cycms:native:media-workspace",
            page_id: "cycms:media-management",
            path: "/admin/media",
            title: "媒体管理",
            menu_label: "媒体管理",
            menu_zone: "media",
            manifest_key: "src/islands/media-workspace.tsx",
            stable_name: "admin-media-workspace",
        },
    ];

    for island in &natives {
        let Some(module_url) =
            resolve_host_island_entry_module(web_dist, island.manifest_key, island.stable_name)
        else {
            continue;
        };

        compiled.assets.push(AssetBundleRegistration {
            id: island.bundle_id.to_owned(),
            source: source.clone(),
            apply_to: vec![island.page_id.to_owned()],
            modules: vec![module_url],
            scripts: Vec::new(),
            styles: Vec::new(),
            inline_data_keys: Vec::new(),
        });
        compiled.admin_pages.push(AdminPageRegistration {
            id: island.page_id.to_owned(),
            source: source.clone(),
            path: island.path.to_owned(),
            title: island.title.to_owned(),
            mode: AdminPageMode::Island,
            priority: 0,
            ownership: OwnershipMode::Replace,
            handler: format!("cycms:native-island:{}", island.stable_name),
            menu_label: Some(island.menu_label.to_owned()),
            menu_zone: Some(island.menu_zone.to_owned()),
            asset_bundle_ids: vec![island.bundle_id.to_owned()],
        });
    }

    compiled
}

fn resolve_host_island_runtime_module(web_dist: &Path) -> Option<String> {
    resolve_host_island_entry_module(web_dist, "index.html", "app").or_else(|| {
        let index_html = std::fs::read_to_string(web_dist.join("index.html")).ok()?;
        extract_first_module_script_src(&index_html)
    })
}

fn resolve_host_island_entry_module(
    web_dist: &Path,
    manifest_key: &str,
    stable_entry_name: &str,
) -> Option<String> {
    resolve_vite_manifest_entry_file(web_dist, manifest_key)
        .or_else(|| resolve_stable_host_asset_entry(web_dist, stable_entry_name))
}

fn resolve_vite_manifest_entry_file(web_dist: &Path, manifest_key: &str) -> Option<String> {
    let manifest_path = web_dist.join(".vite").join("manifest.json");
    let manifest = std::fs::read_to_string(manifest_path).ok()?;
    let value: Value = serde_json::from_str(&manifest).ok()?;
    let file = value.get(manifest_key)?.get("file")?.as_str()?;
    Some(format!("/{}", file.trim_start_matches('/')))
}

fn resolve_stable_host_asset_entry(web_dist: &Path, stable_entry_name: &str) -> Option<String> {
    let relative = format!("assets/{stable_entry_name}.js");
    web_dist
        .join(&relative)
        .is_file()
        .then(|| format!("/{relative}"))
}

fn extract_first_module_script_src(index_html: &str) -> Option<String> {
    index_html
        .split("<script")
        .skip(1)
        .find(|segment| segment.contains("type=\"module\"") && segment.contains("src=\""))
        .and_then(|segment| segment.split("src=\"").nth(1))
        .and_then(|segment| segment.split('"').next())
        .map(str::to_owned)
}

#[derive(Clone)]
struct PublicFallbackState {
    mode: PublicPagesMode,
    web_dist: PathBuf,
    lifecycle_engine: DefaultRequestLifecycleEngine,
}

#[derive(Clone)]
struct AdminFallbackState {
    mode: AdminShellMode,
    web_dist: PathBuf,
    lifecycle_engine: DefaultRequestLifecycleEngine,
}

async fn public_fallback_handler(
    State(state): State<PublicFallbackState>,
    request: Request,
) -> Response {
    match state.mode {
        PublicPagesMode::Compat => serve_spa_fallback(&state.web_dist, request).await,
        PublicPagesMode::HostFirst => {
            let outcome = state.lifecycle_engine.execute_public_request(&request);
            if let Some(response) = outcome.response {
                let mut trace = outcome.trace;
                state.lifecycle_engine.dispatch_before_send(&mut trace);
                finalize_public_response(response, state.mode, trace)
            } else {
                let mut trace = outcome.trace;
                trace.push(LifecyclePhase::CompatSpaFallback);
                state.lifecycle_engine.dispatch_before_send(&mut trace);
                let response = serve_spa_fallback(&state.web_dist, request).await;
                finalize_public_response(response, state.mode, trace)
            }
        }
        PublicPagesMode::HostOnly => {
            let outcome = state.lifecycle_engine.execute_public_request(&request);
            if let Some(response) = outcome.response {
                let mut trace = outcome.trace;
                state.lifecycle_engine.dispatch_before_send(&mut trace);
                finalize_public_response(response, state.mode, trace)
            } else {
                let mut trace = outcome.trace;
                state.lifecycle_engine.dispatch_before_send(&mut trace);
                finalize_public_response(
                    axum::http::StatusCode::NOT_FOUND.into_response(),
                    state.mode,
                    trace,
                )
            }
        }
    }
}

async fn admin_fallback_handler(
    State(state): State<AdminFallbackState>,
    request: Request,
) -> Response {
    match state.mode {
        AdminShellMode::Compat => serve_spa_fallback(&state.web_dist, request).await,
        AdminShellMode::HostFirst => {
            let outcome = state.lifecycle_engine.execute_admin_request(&request);
            if let Some(response) = outcome.response {
                let mut trace = outcome.trace;
                state.lifecycle_engine.dispatch_before_send(&mut trace);
                finalize_admin_response(response, state.mode, trace)
            } else {
                let mut trace = outcome.trace;
                trace.push(LifecyclePhase::CompatAdminFallback);
                state.lifecycle_engine.dispatch_before_send(&mut trace);
                let response = serve_spa_fallback(&state.web_dist, request).await;
                finalize_admin_response(response, state.mode, trace)
            }
        }
        AdminShellMode::HostOnly => {
            let outcome = state.lifecycle_engine.execute_admin_request(&request);
            if let Some(response) = outcome.response {
                let mut trace = outcome.trace;
                state.lifecycle_engine.dispatch_before_send(&mut trace);
                finalize_admin_response(response, state.mode, trace)
            } else {
                let mut trace = outcome.trace;
                state.lifecycle_engine.dispatch_before_send(&mut trace);
                finalize_admin_response(
                    axum::http::StatusCode::NOT_FOUND.into_response(),
                    state.mode,
                    trace,
                )
            }
        }
    }
}

async fn serve_spa_fallback(web_dist: &Path, request: Request) -> Response {
    ServeDir::new(web_dist)
        .fallback(ServeFile::new(web_dist.join("index.html")))
        .oneshot(request)
        .await
        .expect("ServeDir fallback should be infallible")
        .map(axum::body::Body::new)
}

fn finalize_public_response(
    mut response: Response,
    mode: PublicPagesMode,
    mut trace: LifecycleTrace,
) -> Response {
    trace.push(LifecyclePhase::BeforeSend);
    response.headers_mut().insert(
        HeaderName::from_static("x-cycms-public-pages-mode"),
        HeaderValue::from_static(match mode {
            PublicPagesMode::Compat => "compat",
            PublicPagesMode::HostFirst => "host-first",
            PublicPagesMode::HostOnly => "host-only",
        }),
    );
    if let Ok(value) = HeaderValue::from_str(&trace.header_value()) {
        response
            .headers_mut()
            .insert(HeaderName::from_static("x-cycms-lifecycle-trace"), value);
    }
    let chain = trace.effective_chain_header_value();
    if !chain.is_empty()
        && let Ok(value) = HeaderValue::from_str(&chain)
    {
        response
            .headers_mut()
            .insert(HeaderName::from_static("x-cycms-lifecycle-chain"), value);
    }
    response
}

fn finalize_admin_response(
    mut response: Response,
    mode: AdminShellMode,
    mut trace: LifecycleTrace,
) -> Response {
    trace.push(LifecyclePhase::BeforeSend);
    response.headers_mut().insert(
        HeaderName::from_static("x-cycms-admin-shell-mode"),
        HeaderValue::from_static(match mode {
            AdminShellMode::Compat => "compat",
            AdminShellMode::HostFirst => "host-first",
            AdminShellMode::HostOnly => "host-only",
        }),
    );
    if let Ok(value) = HeaderValue::from_str(&trace.header_value()) {
        response
            .headers_mut()
            .insert(HeaderName::from_static("x-cycms-lifecycle-trace"), value);
    }
    let chain = trace.effective_chain_header_value();
    if !chain.is_empty()
        && let Ok(value) = HeaderValue::from_str(&chain)
    {
        response
            .headers_mut()
            .insert(HeaderName::from_static("x-cycms-lifecycle-chain"), value);
    }
    response
}

/// 启动前的认证配置安全校验。
///
/// 回环地址视为本机开发，仅警告；其他地址视为可被远端访问，占位符或过短密钥
/// 直接拒绝启动，避免默认配置在生产环境被误用。
fn validate_auth_config(config: &AppConfig) -> Result<()> {
    let host = config.server.host.as_str();
    let is_loopback = is_loopback_host(host);
    let secret = config.auth.jwt_secret.as_str();

    if secret == JWT_SECRET_PLACEHOLDER {
        if is_loopback {
            warn!(
                host = %host,
                "auth.jwt_secret 仍为默认占位符，仅适用于本机开发；生产请设置 CYCMS__AUTH__JWT_SECRET"
            );
            return Ok(());
        }
        return Err(Error::BadRequest {
            message: format!(
                "auth.jwt_secret 仍为默认占位符，拒绝在 host={host} 启动；\
                 请在 cycms.toml 中替换或设置 CYCMS__AUTH__JWT_SECRET 环境变量"
            ),
            source: None,
        });
    }

    if secret.len() < MIN_JWT_SECRET_BYTES {
        if is_loopback {
            warn!(
                secret_len = secret.len(),
                minimum = MIN_JWT_SECRET_BYTES,
                "auth.jwt_secret 长度不足，HS256 推荐至少 {MIN_JWT_SECRET_BYTES} 字节"
            );
            return Ok(());
        }
        return Err(Error::BadRequest {
            message: format!(
                "auth.jwt_secret 长度 {} 小于推荐的 {MIN_JWT_SECRET_BYTES} 字节，拒绝在 host={host} 启动",
                secret.len()
            ),
            source: None,
        });
    }

    Ok(())
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "127.0.0.1" | "localhost" | "::1" | "[::1]")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;

    use axum::body::{self, Body};
    use axum::extract::Request;
    use axum::http::StatusCode;
    use cycms_config::AdminShellMode;
    use cycms_config::PublicPagesMode;
    use cycms_config::{AppConfig, JWT_SECRET_PLACEHOLDER};
    use cycms_core::Error;
    use cycms_host_types::{
        AdminPageMode, AdminPageRegistration, AssetBundleRegistration, CompiledExtensionRegistry,
        OwnershipMode, RegistrationOriginKind, RegistrationSource,
    };
    use cycms_plugin_manager::HostRegistry;
    use tempfile::tempdir;
    use tower::ServiceExt;

    use super::{
        build_admin_fallback_router, build_public_fallback_service,
        compiled_registry_artifact_path, load_compiled_registry_artifact,
        resolve_host_island_entry_module, validate_auth_config,
    };

    fn config_with(host: &str, secret: &str) -> AppConfig {
        let mut config = AppConfig::default();
        config.server.host = host.to_owned();
        config.auth.jwt_secret = secret.to_owned();
        config
    }

    fn empty_host_registry() -> Arc<HostRegistry> {
        Arc::new(HostRegistry::new(CompiledExtensionRegistry::default()))
    }

    fn admin_host_registry() -> Arc<HostRegistry> {
        Arc::new(HostRegistry::new(CompiledExtensionRegistry {
            admin_pages: vec![AdminPageRegistration {
                id: "blog-dashboard".to_owned(),
                source: RegistrationSource {
                    plugin_name: "blog".to_owned(),
                    plugin_version: "0.1.0".to_owned(),
                    origin: RegistrationOriginKind::HostManifest,
                    declaration_order: 0,
                },
                path: "/admin/x/blog/dashboard".to_owned(),
                title: "Blog Dashboard".to_owned(),
                mode: AdminPageMode::Compatibility,
                priority: 0,
                ownership: OwnershipMode::Replace,
                handler: "frontend.route:root".to_owned(),
                menu_label: Some("Dashboard".to_owned()),
                menu_zone: Some("content".to_owned()),
                asset_bundle_ids: vec!["blog-admin".to_owned()],
            }],
            assets: vec![AssetBundleRegistration {
                id: "blog-admin".to_owned(),
                source: RegistrationSource {
                    plugin_name: "blog".to_owned(),
                    plugin_version: "0.1.0".to_owned(),
                    origin: RegistrationOriginKind::HostManifest,
                    declaration_order: 1,
                },
                apply_to: vec!["admin_extension".to_owned()],
                modules: vec!["/plugins/blog/admin/main.js".to_owned()],
                scripts: Vec::new(),
                styles: vec!["/plugins/blog/admin/main.css".to_owned()],
                inline_data_keys: Vec::new(),
            }],
            ..CompiledExtensionRegistry::default()
        }))
    }

    #[test]
    fn allows_placeholder_on_loopback_with_warning() {
        for host in ["127.0.0.1", "localhost", "::1", "[::1]"] {
            let config = config_with(host, JWT_SECRET_PLACEHOLDER);
            validate_auth_config(&config).expect("loopback 应允许占位符启动");
        }
    }

    #[test]
    fn rejects_placeholder_on_non_loopback() {
        let config = config_with("0.0.0.0", JWT_SECRET_PLACEHOLDER);
        let err = validate_auth_config(&config).unwrap_err();
        assert!(matches!(err, Error::BadRequest { .. }));
    }

    #[test]
    fn allows_short_secret_on_loopback() {
        let config = config_with("127.0.0.1", "too-short");
        validate_auth_config(&config).expect("loopback 允许弱密钥启动");
    }

    #[test]
    fn rejects_short_secret_on_non_loopback() {
        let config = config_with("10.0.0.1", "too-short");
        let err = validate_auth_config(&config).unwrap_err();
        assert!(matches!(err, Error::BadRequest { .. }));
    }

    #[test]
    fn accepts_strong_secret_on_any_host() {
        let strong = "a".repeat(48);
        for host in ["127.0.0.1", "0.0.0.0", "example.com"] {
            let config = config_with(host, &strong);
            validate_auth_config(&config).expect("强密钥在任何 host 都应通过");
        }
    }

    #[test]
    fn resolves_host_island_entry_from_vite_manifest() {
        let temp = tempdir().unwrap();
        fs::create_dir_all(temp.path().join(".vite")).unwrap();
        fs::write(
            temp.path().join(".vite").join("manifest.json"),
            r#"{
  "src/islands/content-workspace.tsx": {
    "file": "assets/admin-content-workspace-abc123.js",
    "name": "admin-content-workspace",
    "src": "src/islands/content-workspace.tsx",
    "isEntry": true
  }
}"#,
        )
        .unwrap();

        let resolved = resolve_host_island_entry_module(
            temp.path(),
            "src/islands/content-workspace.tsx",
            "admin-content-workspace",
        );

        assert_eq!(
            resolved,
            Some("/assets/admin-content-workspace-abc123.js".to_owned())
        );
    }

    #[test]
    fn resolves_host_island_entry_from_stable_asset_name_when_manifest_is_missing() {
        let temp = tempdir().unwrap();
        fs::create_dir_all(temp.path().join("assets")).unwrap();
        fs::write(
            temp.path().join("assets").join("admin-media-workspace.js"),
            "export const mount = () => {};",
        )
        .unwrap();

        let resolved = resolve_host_island_entry_module(
            temp.path(),
            "src/islands/media-workspace.tsx",
            "admin-media-workspace",
        );

        assert_eq!(
            resolved,
            Some("/assets/admin-media-workspace.js".to_owned())
        );
    }

    #[test]
    fn compiled_registry_artifact_path_uses_plugins_root() {
        let path = compiled_registry_artifact_path(PathBuf::from("/tmp/cycms/plugins").as_path());
        assert_eq!(
            path,
            PathBuf::from("/tmp/cycms/plugins/.compiled-registry.json")
        );
    }

    #[test]
    fn load_compiled_registry_artifact_roundtrips_json_payload() {
        let temp = tempdir().unwrap();
        let path = temp.path().join(".compiled-registry.json");
        let payload = serde_json::to_string(&CompiledExtensionRegistry::default()).unwrap();
        fs::write(&path, payload).unwrap();

        let registry = load_compiled_registry_artifact(&path).unwrap();
        assert_eq!(registry, CompiledExtensionRegistry::default());
    }

    #[tokio::test]
    async fn compat_mode_serves_spa_index_for_unknown_public_path() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("index.html"), "<html>compat</html>").unwrap();

        let response = build_public_fallback_service(
            PublicPagesMode::Compat,
            empty_host_registry(),
            temp.path().into(),
        )
        .oneshot(
            Request::builder()
                .uri("/posts/hello-world")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(std::str::from_utf8(&body).unwrap(), "<html>compat</html>");
    }

    #[tokio::test]
    async fn host_first_mode_falls_back_to_spa_with_lifecycle_headers() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("index.html"), "<html>host-first</html>").unwrap();

        let response = build_public_fallback_service(
            PublicPagesMode::HostFirst,
            empty_host_registry(),
            temp.path().into(),
        )
        .oneshot(
            Request::builder()
                .uri("/posts/hello-world")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("x-cycms-public-pages-mode").unwrap(),
            "host-first"
        );
        assert_eq!(
            response.headers().get("x-cycms-lifecycle-trace").unwrap(),
            "request_received,route_matched,load_data,resolve_content,parse_content,build_page,inject_assets,compat_spa_fallback,before_send"
        );
    }

    #[tokio::test]
    async fn host_only_mode_returns_not_found_without_spa_fallback() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("index.html"), "<html>host-only</html>").unwrap();

        let response = build_public_fallback_service(
            PublicPagesMode::HostOnly,
            empty_host_registry(),
            temp.path().into(),
        )
        .oneshot(
            Request::builder()
                .uri("/posts/hello-world")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get("x-cycms-public-pages-mode").unwrap(),
            "host-only"
        );
        assert_eq!(
            response.headers().get("x-cycms-lifecycle-trace").unwrap(),
            "request_received,route_matched,load_data,resolve_content,parse_content,build_page,inject_assets,before_send"
        );
    }

    #[tokio::test]
    async fn admin_compat_mode_serves_spa_index_for_unknown_path() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("index.html"), "<html>admin-compat</html>").unwrap();

        let response = build_admin_fallback_router(
            AdminShellMode::Compat,
            empty_host_registry(),
            temp.path().into(),
        )
        .oneshot(
            Request::builder()
                .uri("/admin/content")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            std::str::from_utf8(&body).unwrap(),
            "<html>admin-compat</html>"
        );
    }

    #[tokio::test]
    async fn admin_host_first_mode_renders_owned_admin_page() {
        let temp = tempdir().unwrap();
        fs::write(
            temp.path().join("index.html"),
            "<html><head><script type=\"module\" src=\"/assets/index-runtime.js\"></script></head><body>admin-host-first</body></html>",
        )
        .unwrap();

        let response = build_admin_fallback_router(
            AdminShellMode::HostFirst,
            admin_host_registry(),
            temp.path().into(),
        )
        .oneshot(
            Request::builder()
                .uri("/admin/x/blog/dashboard")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("x-cycms-admin-shell-mode").unwrap(),
            "host-first"
        );
        assert_eq!(
            response.headers().get("x-cycms-lifecycle-trace").unwrap(),
            "request_received,route_matched,load_data,resolve_content,parse_content,build_page,inject_assets,before_send"
        );
        let body = body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = std::str::from_utf8(&body).unwrap();
        assert!(html.contains("Blog Dashboard | Admin"));
        assert!(html.contains("/plugins/blog/admin/main.css"));
        assert!(html.contains("/plugins/blog/admin/main.js"));
        assert!(html.contains("/assets/index-runtime.js"));
        assert!(html.contains("data-island-boot=\"admin-screen:blog-dashboard\""));
    }

    #[tokio::test]
    async fn admin_host_only_mode_returns_not_found_without_spa_fallback() {
        let temp = tempdir().unwrap();
        fs::write(
            temp.path().join("index.html"),
            "<html>admin-host-only</html>",
        )
        .unwrap();

        let response = build_admin_fallback_router(
            AdminShellMode::HostOnly,
            empty_host_registry(),
            temp.path().into(),
        )
        .oneshot(
            Request::builder()
                .uri("/admin/content")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get("x-cycms-admin-shell-mode").unwrap(),
            "host-only"
        );
        assert_eq!(
            response.headers().get("x-cycms-lifecycle-trace").unwrap(),
            "request_received,route_matched,load_data,resolve_content,parse_content,build_page,inject_assets,before_send"
        );
    }
}
