use std::sync::Arc;

use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use cycms_admin_shell::{AdminShellRenderer, DefaultAdminShellRenderer};
use cycms_host_types::{
    AdminPageRegistration, AssetGraph, AssetReference, HeadNode, HookRegistration,
    HostRequestTarget, HtmlNode, OwnershipDecision, PageDocument, PageNode, ParseTarget,
    PublicPageRegistration, TextNode,
};
use cycms_plugin_manager::{HostRegistry, RegistryLookup};
use cycms_render::{
    AssetGraphBuilder, ContentInput, DefaultAssetGraphBuilder, DefaultHtmlRenderer,
    DefaultPageBuilder, HtmlRenderer, PageBuildContext, PageBuildInput, PageBuilder, ParseContext,
    parse_with_defaults,
};
use tracing::error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecyclePhase {
    RequestReceived,
    RouteMatched,
    LoadData,
    ResolveContent,
    ParseContent,
    BuildPage,
    InjectAssets,
    CompatSpaFallback,
    CompatAdminFallback,
    BeforeSend,
}

impl LifecyclePhase {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RequestReceived => "request_received",
            Self::RouteMatched => "route_matched",
            Self::LoadData => "load_data",
            Self::ResolveContent => "resolve_content",
            Self::ParseContent => "parse_content",
            Self::BuildPage => "build_page",
            Self::InjectAssets => "inject_assets",
            Self::CompatSpaFallback => "compat_spa_fallback",
            Self::CompatAdminFallback => "compat_admin_fallback",
            Self::BeforeSend => "before_send",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleTrace {
    pub target: HostRequestTarget,
    pub phases: Vec<LifecyclePhase>,
    pub effective_chain: Vec<String>,
}

impl LifecycleTrace {
    #[must_use]
    pub fn new(path: String) -> Self {
        Self {
            target: HostRequestTarget { path },
            phases: vec![LifecyclePhase::RequestReceived],
            effective_chain: Vec::new(),
        }
    }

    pub fn push(&mut self, phase: LifecyclePhase) {
        self.phases.push(phase);
    }

    #[must_use]
    pub fn header_value(&self) -> String {
        self.phases
            .iter()
            .map(|phase| phase.as_str())
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn push_effective_chain(&mut self, item: String) {
        self.effective_chain.push(item);
    }

    #[must_use]
    pub fn effective_chain_header_value(&self) -> String {
        self.effective_chain.join(",")
    }
}

#[derive(Debug)]
pub struct PublicLifecycleOutcome {
    pub response: Option<Response>,
    pub trace: LifecycleTrace,
}

#[derive(Debug)]
pub struct AdminLifecycleOutcome {
    pub response: Option<Response>,
    pub trace: LifecycleTrace,
}

#[derive(Debug, Clone)]
pub struct DefaultRequestLifecycleEngine {
    registry: Arc<HostRegistry>,
    host_island_runtime_module: Option<String>,
}

impl DefaultRequestLifecycleEngine {
    #[cfg(test)]
    #[must_use]
    pub fn new(registry: Arc<HostRegistry>) -> Self {
        Self {
            registry,
            host_island_runtime_module: None,
        }
    }

    #[must_use]
    pub fn with_host_island_runtime_module(
        registry: Arc<HostRegistry>,
        host_island_runtime_module: Option<String>,
    ) -> Self {
        Self {
            registry,
            host_island_runtime_module,
        }
    }

    pub fn dispatch_before_send(&self, trace: &mut LifecycleTrace) {
        self.dispatch_phase_hooks(LifecyclePhase::BeforeSend, trace);
    }

    #[must_use]
    pub fn execute_public_request(&self, request: &Request) -> PublicLifecycleOutcome {
        let mut trace = LifecycleTrace::new(request.uri().path().to_owned());
        self.enter_phase(LifecyclePhase::RouteMatched, &mut trace);
        self.enter_phase(LifecyclePhase::LoadData, &mut trace);
        self.enter_phase(LifecyclePhase::ResolveContent, &mut trace);
        self.enter_phase(LifecyclePhase::ParseContent, &mut trace);
        self.enter_phase(LifecyclePhase::BuildPage, &mut trace);

        let decision = self.registry.resolve_public_page(&trace.target);
        let response = decision
            .primary
            .as_ref()
            .map(|page| render_owned_public_page(page, self.registry.as_ref(), &mut trace));
        self.enter_phase(LifecyclePhase::InjectAssets, &mut trace);

        PublicLifecycleOutcome { response, trace }
    }

    #[must_use]
    pub fn execute_admin_request(&self, request: &Request) -> AdminLifecycleOutcome {
        let mut trace = LifecycleTrace::new(request.uri().path().to_owned());
        self.enter_phase(LifecyclePhase::RouteMatched, &mut trace);
        self.enter_phase(LifecyclePhase::LoadData, &mut trace);
        self.enter_phase(LifecyclePhase::ResolveContent, &mut trace);
        self.enter_phase(LifecyclePhase::ParseContent, &mut trace);
        self.enter_phase(LifecyclePhase::BuildPage, &mut trace);

        let decision = self.registry.resolve_admin_page(&trace.target);
        let response = decision.primary.as_ref().map(|page| {
            render_owned_admin_page(
                page,
                self.registry.as_ref(),
                self.host_island_runtime_module.as_deref(),
            )
        });
        self.enter_phase(LifecyclePhase::InjectAssets, &mut trace);

        AdminLifecycleOutcome { response, trace }
    }

    fn enter_phase(&self, phase: LifecyclePhase, trace: &mut LifecycleTrace) {
        trace.push(phase);
        self.dispatch_phase_hooks(phase, trace);
    }

    fn dispatch_phase_hooks(&self, phase: LifecyclePhase, trace: &mut LifecycleTrace) {
        let decision = self.registry.resolve_hook_phase(phase.as_str());
        trace.push_effective_chain(format!("phase:{}", phase.as_str()));
        record_hook_decision(phase, &decision, trace);
    }
}

fn render_owned_public_page(
    page: &PublicPageRegistration,
    registry: &HostRegistry,
    trace: &mut LifecycleTrace,
) -> Response {
    let title = page.title.clone().unwrap_or_else(|| page.path.clone());
    let parser_target = ParseTarget {
        content_type: Some("public_page".to_owned()),
        field_name: Some(page.id.clone()),
        source_format: Some("markdown".to_owned()),
    };
    let parser_decision = registry.resolve_parser(&parser_target);
    if let Some(parser) = parser_decision.primary.as_ref() {
        trace.push_effective_chain(format!("parser:{}:{}", parser.id, parser.parser));
    } else {
        trace.push_effective_chain("parser:default.markdown".to_owned());
    }

    let parser_id = parser_decision
        .primary
        .as_ref()
        .map(|parser| parser.parser.clone())
        .unwrap_or_else(|| "default.markdown".to_owned());
    let content = match parse_with_defaults(
        ContentInput::Text(format!("# {title}\n\nHandled by {}", page.handler)),
        &ParseContext {
            format: "markdown".to_owned(),
            parser_id,
            origin_field: Some(page.id.clone()),
            content_type: Some("public_page".to_owned()),
        },
    ) {
        Ok(content) => content,
        Err(source) => {
            error!(path = %page.path, handler = %page.handler, error = %source, "failed to parse host-owned public content");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                "Internal server error",
            )
                .into_response();
        }
    };

    let document = match DefaultPageBuilder.build(
        PageBuildInput {
            route_id: format!("public:{}", page.path),
            status: StatusCode::OK,
            head: vec![HeadNode::Title {
                text: title.clone(),
            }],
            content: Some(content),
            body: vec![PageNode::Html(HtmlNode {
                tag: "main".to_owned(),
                attributes: Default::default(),
                children: vec![PageNode::Html(HtmlNode {
                    tag: "p".to_owned(),
                    attributes: Default::default(),
                    children: vec![PageNode::Text(TextNode {
                        value: format!("Owned by {}", page.handler),
                    })],
                })],
            })],
            actions: Vec::new(),
            islands: Vec::new(),
            cache_tags: vec![format!("plugin:{}", page.source.plugin_name)],
            layout_name: None,
        },
        &PageBuildContext::default(),
    ) {
        Ok(document) => document,
        Err(source) => {
            error!(path = %page.path, handler = %page.handler, error = %source, "failed to build host-owned public page document");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                "Internal server error",
            )
                .into_response();
        }
    };

    let assets = match DefaultAssetGraphBuilder.build_public_page(&document, page, registry) {
        Ok(assets) => assets,
        Err(source) => {
            error!(path = %page.path, handler = %page.handler, error = %source, "failed to build asset graph for host-owned public page");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                "Internal server error",
            )
                .into_response();
        }
    };

    render_owned_document(&document, &assets, &page.path, &page.handler, "public")
}

fn record_hook_decision(
    phase: LifecyclePhase,
    decision: &OwnershipDecision<HookRegistration>,
    trace: &mut LifecycleTrace,
) {
    if let Some(primary) = &decision.primary {
        trace.push_effective_chain(format!(
            "{}:replace:{}=>{}",
            phase.as_str(),
            primary.id,
            primary.handler
        ));
    }
    for wrapper in &decision.wrappers {
        trace.push_effective_chain(format!(
            "{}:wrap:{}=>{}",
            phase.as_str(),
            wrapper.id,
            wrapper.handler
        ));
    }
    for appender in &decision.appenders {
        trace.push_effective_chain(format!(
            "{}:append:{}=>{}",
            phase.as_str(),
            appender.id,
            appender.handler
        ));
    }
}

fn render_owned_admin_page(
    page: &AdminPageRegistration,
    registry: &HostRegistry,
    host_island_runtime_module: Option<&str>,
) -> Response {
    let shell = DefaultAdminShellRenderer.render_page(page, registry);
    let document = shell.page;
    let assets = match DefaultAssetGraphBuilder.build_admin_page(&document, page, registry) {
        Ok(mut assets) => {
            assets.inline_data.extend(shell.preload);
            inject_host_island_runtime_module(assets, &document, host_island_runtime_module)
        }
        Err(source) => {
            error!(path = %page.path, handler = %page.handler, error = %source, "failed to build asset graph for host-owned admin page");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                "Internal server error",
            )
                .into_response();
        }
    };

    render_owned_document(&document, &assets, &page.path, &page.handler, "admin")
}

fn inject_host_island_runtime_module(
    mut assets: AssetGraph,
    document: &PageDocument,
    host_island_runtime_module: Option<&str>,
) -> AssetGraph {
    let Some(host_island_runtime_module) = host_island_runtime_module else {
        return assets;
    };

    if document.islands.is_empty()
        || assets
            .modules
            .iter()
            .any(|asset| asset.href == host_island_runtime_module)
    {
        return assets;
    }

    assets.modules.push(AssetReference {
        id: "cycms:host-island-runtime".to_owned(),
        href: host_island_runtime_module.to_owned(),
    });
    assets
}

fn render_owned_document(
    document: &PageDocument,
    assets: &cycms_host_types::AssetGraph,
    path: &str,
    handler: &str,
    surface: &str,
) -> Response {
    match DefaultHtmlRenderer.render(document, assets) {
        Ok(rendered) => (
            rendered.status,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            rendered.html,
        )
            .into_response(),
        Err(source) => {
            error!(path = %path, handler = %handler, error = %source, surface, "failed to render host-owned page");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                "Internal server error",
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::{self, Body};
    use cycms_host_types::{
        AdminPageMode, AssetBundleRegistration, CompiledExtensionRegistry, HookRegistration,
        OwnershipMode, PublicPageRegistration, RegistrationOriginKind, RegistrationSource,
    };
    use cycms_plugin_manager::HostRegistry;

    use super::*;

    fn empty_registry() -> Arc<HostRegistry> {
        Arc::new(HostRegistry::new(CompiledExtensionRegistry::default()))
    }

    fn source(order: usize) -> RegistrationSource {
        RegistrationSource {
            plugin_name: "blog".to_owned(),
            plugin_version: "0.1.0".to_owned(),
            origin: RegistrationOriginKind::HostManifest,
            declaration_order: order,
        }
    }

    fn blog_page_registry() -> Arc<HostRegistry> {
        Arc::new(HostRegistry::new(CompiledExtensionRegistry {
            public_pages: vec![PublicPageRegistration {
                id: "blog-home".to_owned(),
                source: source(0),
                path: "/blog".to_owned(),
                priority: 0,
                ownership: OwnershipMode::Replace,
                handler: "blog::public::home".to_owned(),
                title: Some("Blog".to_owned()),
                asset_bundle_ids: vec!["blog-assets".to_owned()],
            }],
            assets: vec![AssetBundleRegistration {
                id: "blog-assets".to_owned(),
                source: source(1),
                apply_to: vec!["public_page".to_owned()],
                modules: vec!["/assets/blog.js".to_owned()],
                scripts: Vec::new(),
                styles: vec!["/assets/blog.css".to_owned()],
                inline_data_keys: Vec::new(),
            }],
            ..CompiledExtensionRegistry::default()
        }))
    }

    fn admin_page_registry(mode: AdminPageMode) -> Arc<HostRegistry> {
        Arc::new(HostRegistry::new(CompiledExtensionRegistry {
            admin_pages: vec![AdminPageRegistration {
                id: "blog-dashboard".to_owned(),
                source: source(0),
                path: "/admin/x/blog/dashboard".to_owned(),
                title: "Blog Dashboard".to_owned(),
                mode,
                priority: 0,
                ownership: OwnershipMode::Replace,
                handler: "frontend.route:root".to_owned(),
                menu_label: Some("Dashboard".to_owned()),
                menu_zone: Some("content".to_owned()),
                asset_bundle_ids: vec!["blog-admin".to_owned()],
            }],
            assets: vec![AssetBundleRegistration {
                id: "blog-admin".to_owned(),
                source: source(1),
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
    fn default_public_lifecycle_starts_trace_without_response() {
        let request = Request::builder()
            .uri("/posts/hello-world")
            .body(Body::empty())
            .unwrap();

        let outcome =
            DefaultRequestLifecycleEngine::new(empty_registry()).execute_public_request(&request);

        assert!(outcome.response.is_none());
        assert_eq!(outcome.trace.target.path, "/posts/hello-world");
        assert_eq!(
            outcome.trace.phases,
            vec![
                LifecyclePhase::RequestReceived,
                LifecyclePhase::RouteMatched,
                LifecyclePhase::LoadData,
                LifecyclePhase::ResolveContent,
                LifecyclePhase::ParseContent,
                LifecyclePhase::BuildPage,
                LifecyclePhase::InjectAssets,
            ]
        );
    }

    #[test]
    fn default_admin_lifecycle_starts_trace_without_response() {
        let request = Request::builder()
            .uri("/admin/pages")
            .body(Body::empty())
            .unwrap();

        let outcome =
            DefaultRequestLifecycleEngine::new(empty_registry()).execute_admin_request(&request);

        assert!(outcome.response.is_none());
        assert_eq!(outcome.trace.target.path, "/admin/pages");
        assert_eq!(
            outcome.trace.phases,
            vec![
                LifecyclePhase::RequestReceived,
                LifecyclePhase::RouteMatched,
                LifecyclePhase::LoadData,
                LifecyclePhase::ResolveContent,
                LifecyclePhase::ParseContent,
                LifecyclePhase::BuildPage,
                LifecyclePhase::InjectAssets,
            ]
        );
    }

    #[test]
    fn lifecycle_trace_header_value_is_deterministic() {
        let mut trace = LifecycleTrace::new("/".to_owned());
        trace.push(LifecyclePhase::RouteMatched);
        trace.push(LifecyclePhase::CompatSpaFallback);
        trace.push(LifecyclePhase::BeforeSend);

        assert_eq!(
            trace.header_value(),
            "request_received,route_matched,compat_spa_fallback,before_send"
        );
    }

    #[tokio::test]
    async fn owned_public_page_returns_host_rendered_response() {
        let request = Request::builder().uri("/blog").body(Body::empty()).unwrap();

        let outcome = DefaultRequestLifecycleEngine::new(blog_page_registry())
            .execute_public_request(&request);

        let response = outcome.response.expect("owned public page should render");
        let body = body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = std::str::from_utf8(&body).unwrap();

        assert!(html.contains("<title>Blog</title>"));
        assert!(html.contains("Handled by blog::public::home"));
        assert!(html.contains("/assets/blog.css"));
        assert!(html.contains("/assets/blog.js"));
    }

    #[tokio::test]
    async fn owned_admin_page_returns_host_rendered_response() {
        let request = Request::builder()
            .uri("/admin/x/blog/dashboard")
            .body(Body::empty())
            .unwrap();

        let outcome =
            DefaultRequestLifecycleEngine::new(admin_page_registry(AdminPageMode::Compatibility))
                .execute_admin_request(&request);

        let response = outcome.response.expect("owned admin page should render");
        let body = body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = std::str::from_utf8(&body).unwrap();

        assert!(html.contains("<title>Blog Dashboard | Admin</title>"));
        assert!(html.contains("Handled by frontend.route:root"));
        assert!(html.contains("data-admin-nav=\"primary\""));
        assert!(html.contains("Dashboard"));
        assert!(html.contains("aria-current=\"page\""));
        assert!(html.contains("data-admin-breadcrumbs=\"true\""));
        assert!(html.contains("aria-label=\"breadcrumb\""));
        assert!(html.contains("href=\"/admin/x/blog/dashboard\""));
        assert!(html.contains("Blog Dashboard"));
        assert!(html.contains("cycms-admin-page-id"));
        assert!(html.contains("admin-preload:blog-dashboard"));
        assert!(html.contains("data-admin-mode=\"compatibility\""));
        assert!(html.contains("/plugins/blog/admin/main.css"));
        assert!(html.contains("/plugins/blog/admin/main.js"));
        assert!(html.contains("data-island-boot=\"admin-screen:blog-dashboard\""));
    }

    #[tokio::test]
    async fn owned_admin_page_injects_host_island_runtime_module() {
        let request = Request::builder()
            .uri("/admin/x/blog/dashboard")
            .body(Body::empty())
            .unwrap();

        let outcome = DefaultRequestLifecycleEngine::with_host_island_runtime_module(
            admin_page_registry(AdminPageMode::Compatibility),
            Some("/assets/index-runtime.js".to_owned()),
        )
        .execute_admin_request(&request);

        let response = outcome.response.expect("owned admin page should render");
        let body = body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = std::str::from_utf8(&body).unwrap();

        assert!(html.contains("src=\"/assets/index-runtime.js\""));
        assert!(html.contains("data-module=\"/plugins/blog/admin/main.js\""));
    }

    #[tokio::test]
    async fn html_admin_page_skips_island_boot_assets() {
        let request = Request::builder()
            .uri("/admin/x/blog/dashboard")
            .body(Body::empty())
            .unwrap();

        let outcome = DefaultRequestLifecycleEngine::new(admin_page_registry(AdminPageMode::Html))
            .execute_admin_request(&request);

        let response = outcome.response.expect("owned admin page should render");
        let body = body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = std::str::from_utf8(&body).unwrap();

        assert!(html.contains("data-admin-mode=\"html\""));
        assert!(!html.contains("data-island-boot="));
    }

    #[tokio::test]
    async fn admin_navigation_marks_current_entry_with_trailing_slash_request() {
        let request = Request::builder()
            .uri("/admin/x/blog/dashboard/")
            .body(Body::empty())
            .unwrap();

        let outcome =
            DefaultRequestLifecycleEngine::new(admin_page_registry(AdminPageMode::Compatibility))
                .execute_admin_request(&request);

        let response = outcome.response.expect("owned admin page should render");
        let body = body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = std::str::from_utf8(&body).unwrap();

        assert!(html.contains("href=\"/admin/x/blog/dashboard\""));
        assert!(html.contains("aria-current=\"page\""));
    }

    #[test]
    fn lifecycle_records_deterministic_hook_execution_chain() {
        let registry = Arc::new(HostRegistry::new(CompiledExtensionRegistry {
            hooks: vec![
                HookRegistration {
                    id: "theme.build.wrap".to_owned(),
                    source: source(1),
                    priority: 10,
                    ownership: OwnershipMode::Wrap,
                    phase: "build_page".to_owned(),
                    handler: "theme::hooks::build_wrap".to_owned(),
                },
                HookRegistration {
                    id: "blog.build.replace".to_owned(),
                    source: source(0),
                    priority: 100,
                    ownership: OwnershipMode::Replace,
                    phase: "build_page".to_owned(),
                    handler: "blog::hooks::build_replace".to_owned(),
                },
                HookRegistration {
                    id: "analytics.before-send.append".to_owned(),
                    source: source(2),
                    priority: 0,
                    ownership: OwnershipMode::Append,
                    phase: "before_send".to_owned(),
                    handler: "analytics::hooks::before_send".to_owned(),
                },
            ],
            ..CompiledExtensionRegistry::default()
        }));
        let request = Request::builder().uri("/blog").body(Body::empty()).unwrap();

        let engine = DefaultRequestLifecycleEngine::new(registry);
        let mut trace = engine.execute_public_request(&request).trace;
        engine.dispatch_before_send(&mut trace);

        assert!(trace.effective_chain.contains(
            &"build_page:replace:blog.build.replace=>blog::hooks::build_replace".to_owned()
        ));
        assert!(
            trace
                .effective_chain
                .contains(&"build_page:wrap:theme.build.wrap=>theme::hooks::build_wrap".to_owned())
        );
        assert!(
            trace.effective_chain.contains(
                &"before_send:append:analytics.before-send.append=>analytics::hooks::before_send"
                    .to_owned()
            )
        );
    }
}
