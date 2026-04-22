use std::sync::Arc;

use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use cycms_admin_shell::{AdminShellRenderer, DefaultAdminShellRenderer};
use cycms_host_types::{
    AdminPageRegistration, HeadNode, HostRequestTarget, HtmlNode, PageDocument, PageNode,
    PublicPageRegistration, TextNode,
};
use cycms_plugin_manager::{HostRegistry, RegistryLookup};
use cycms_render::{
    AssetGraphBuilder, DefaultAssetGraphBuilder, DefaultHtmlRenderer, HtmlRenderer,
};
use tracing::error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecyclePhase {
    RequestReceived,
    RouteMatched,
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
}

impl LifecycleTrace {
    #[must_use]
    pub fn new(path: String) -> Self {
        Self {
            target: HostRequestTarget { path },
            phases: vec![LifecyclePhase::RequestReceived],
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
}

impl DefaultRequestLifecycleEngine {
    #[must_use]
    pub fn new(registry: Arc<HostRegistry>) -> Self {
        Self { registry }
    }

    #[must_use]
    pub fn execute_public_request(&self, request: &Request) -> PublicLifecycleOutcome {
        let mut trace = LifecycleTrace::new(request.uri().path().to_owned());
        trace.push(LifecyclePhase::RouteMatched);

        let decision = self.registry.resolve_public_page(&trace.target);
        let response = decision
            .primary
            .as_ref()
            .map(|page| render_owned_public_page(page, self.registry.as_ref()));

        PublicLifecycleOutcome { response, trace }
    }

    #[must_use]
    pub fn execute_admin_request(&self, request: &Request) -> AdminLifecycleOutcome {
        let mut trace = LifecycleTrace::new(request.uri().path().to_owned());
        trace.push(LifecyclePhase::RouteMatched);

        let decision = self.registry.resolve_admin_page(&trace.target);
        let response = decision
            .primary
            .as_ref()
            .map(|page| render_owned_admin_page(page, self.registry.as_ref()));

        AdminLifecycleOutcome { response, trace }
    }
}

fn render_owned_public_page(page: &PublicPageRegistration, registry: &HostRegistry) -> Response {
    let title = page.title.clone().unwrap_or_else(|| page.path.clone());
    let document = PageDocument {
        route_id: format!("public:{}", page.path),
        status: StatusCode::OK,
        head: vec![HeadNode::Title {
            text: title.clone(),
        }],
        body: vec![PageNode::Html(HtmlNode {
            tag: "main".to_owned(),
            attributes: Default::default(),
            children: vec![
                PageNode::Html(HtmlNode {
                    tag: "h1".to_owned(),
                    attributes: Default::default(),
                    children: vec![PageNode::Text(TextNode { value: title })],
                }),
                PageNode::Html(HtmlNode {
                    tag: "p".to_owned(),
                    attributes: Default::default(),
                    children: vec![PageNode::Text(TextNode {
                        value: format!("Handled by {}", page.handler),
                    })],
                }),
            ],
        })],
        actions: Vec::new(),
        islands: Vec::new(),
        cache_tags: vec![format!("plugin:{}", page.source.plugin_name)],
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

fn render_owned_admin_page(page: &AdminPageRegistration, registry: &HostRegistry) -> Response {
    let shell = DefaultAdminShellRenderer.render_page(page, registry);
    let document = shell.page;
    let assets = match DefaultAssetGraphBuilder.build_admin_page(&document, page, registry) {
        Ok(mut assets) => {
            assets.inline_data.extend(shell.preload);
            assets
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
        AdminPageMode, AssetBundleRegistration, CompiledExtensionRegistry, OwnershipMode,
        PublicPageRegistration, RegistrationOriginKind, RegistrationSource,
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
                LifecyclePhase::RouteMatched
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
                LifecyclePhase::RouteMatched
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
}
