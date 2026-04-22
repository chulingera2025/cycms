use std::collections::BTreeMap;

use cycms_host_types::{
    AdminPageMode, AdminPageRegistration, HeadNode, HtmlNode, InlineDataAsset, IslandMount,
    PageDocument, PageNode, TextNode,
};
use cycms_plugin_manager::HostRegistry;
use http::StatusCode;
use serde_json::json;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminShellDiagnostic {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct AdminShellOutput {
    pub page: PageDocument,
    pub preload: Vec<InlineDataAsset>,
    pub diagnostics: Vec<AdminShellDiagnostic>,
}

pub trait AdminShellRenderer {
    fn render_page(
        &self,
        page: &AdminPageRegistration,
        registry: &HostRegistry,
    ) -> AdminShellOutput;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultAdminShellRenderer;

impl AdminShellRenderer for DefaultAdminShellRenderer {
    fn render_page(
        &self,
        page: &AdminPageRegistration,
        registry: &HostRegistry,
    ) -> AdminShellOutput {
        let shell_mode = admin_page_mode_name(page.mode);
        let title = format!("{} | Admin", page.title);
        let navigation = render_admin_navigation(registry, &page.path);
        let breadcrumbs = admin_breadcrumbs(page, registry);
        let breadcrumb_node = render_admin_breadcrumbs(&breadcrumbs);
        let island = match page.mode {
            AdminPageMode::Html => None,
            AdminPageMode::Hybrid | AdminPageMode::Island | AdminPageMode::Compatibility => {
                Some(IslandMount {
                    id: format!("admin-screen:{}", page.id),
                    component: page.handler.clone(),
                    props: json!({
                        "path": page.path,
                        "mode": shell_mode,
                    }),
                })
            }
        };

        let mut body_children = vec![
            navigation,
            breadcrumb_node,
            PageNode::Html(HtmlNode {
                tag: "h1".to_owned(),
                attributes: Default::default(),
                children: vec![PageNode::Text(TextNode {
                    value: page.title.clone(),
                })],
            }),
            PageNode::Html(HtmlNode {
                tag: "p".to_owned(),
                attributes: Default::default(),
                children: vec![PageNode::Text(TextNode {
                    value: format!("Handled by {}", page.handler),
                })],
            }),
            PageNode::Html(HtmlNode {
                tag: "p".to_owned(),
                attributes: Default::default(),
                children: vec![PageNode::Text(TextNode {
                    value: format!("Admin mode: {shell_mode}"),
                })],
            }),
        ];
        if let Some(mount) = island.clone() {
            body_children.push(PageNode::Island(mount));
        }

        AdminShellOutput {
            page: PageDocument {
                route_id: format!("admin:{}", page.path),
                status: StatusCode::OK,
                head: vec![
                    HeadNode::Title { text: title },
                    HeadNode::Meta {
                        name: "cycms-admin-page-id".to_owned(),
                        content: page.id.clone(),
                    },
                    HeadNode::Meta {
                        name: "cycms-admin-mode".to_owned(),
                        content: shell_mode.to_owned(),
                    },
                ],
                body: vec![PageNode::Html(HtmlNode {
                    tag: "main".to_owned(),
                    attributes: BTreeMap::from([(
                        "data-admin-mode".to_owned(),
                        shell_mode.to_owned(),
                    )]),
                    children: body_children,
                })],
                actions: Vec::new(),
                islands: island.into_iter().collect(),
                cache_tags: vec![format!("plugin:{}", page.source.plugin_name)],
            },
            preload: vec![InlineDataAsset {
                id: format!("admin-preload:{}", page.id),
                value: json!({
                    "pageId": page.id,
                    "path": page.path,
                    "mode": shell_mode,
                    "plugin": page.source.plugin_name,
                    "breadcrumbs": breadcrumbs
                        .iter()
                        .map(|crumb| json!({
                            "label": crumb.label,
                            "href": crumb.href,
                        }))
                        .collect::<Vec<_>>(),
                }),
            }],
            diagnostics: Vec::new(),
        }
    }
}

fn render_admin_navigation(registry: &HostRegistry, current_path: &str) -> PageNode {
    let normalized_current_path = normalize_navigation_path(current_path);
    let groups = registry.admin_menu_tree();
    let group_nodes = groups
        .into_iter()
        .map(|group| {
            let entry_nodes = group
                .entries
                .into_iter()
                .map(|entry| {
                    let mut attributes = BTreeMap::from([("href".to_owned(), entry.path.clone())]);
                    if normalize_navigation_path(&entry.path) == normalized_current_path {
                        attributes.insert("aria-current".to_owned(), "page".to_owned());
                    }

                    PageNode::Html(HtmlNode {
                        tag: "li".to_owned(),
                        attributes: Default::default(),
                        children: vec![PageNode::Html(HtmlNode {
                            tag: "a".to_owned(),
                            attributes,
                            children: vec![PageNode::Text(TextNode { value: entry.label })],
                        })],
                    })
                })
                .collect::<Vec<_>>();

            PageNode::Html(HtmlNode {
                tag: "section".to_owned(),
                attributes: BTreeMap::from([("data-admin-zone".to_owned(), group.zone.clone())]),
                children: vec![
                    PageNode::Html(HtmlNode {
                        tag: "h2".to_owned(),
                        attributes: Default::default(),
                        children: vec![PageNode::Text(TextNode { value: group.zone })],
                    }),
                    PageNode::Html(HtmlNode {
                        tag: "ul".to_owned(),
                        attributes: Default::default(),
                        children: entry_nodes,
                    }),
                ],
            })
        })
        .collect::<Vec<_>>();

    PageNode::Html(HtmlNode {
        tag: "nav".to_owned(),
        attributes: BTreeMap::from([("data-admin-nav".to_owned(), "primary".to_owned())]),
        children: group_nodes,
    })
}

fn render_admin_breadcrumbs(breadcrumbs: &[AdminBreadcrumb]) -> PageNode {
    let items = breadcrumbs
        .iter()
        .enumerate()
        .map(|(index, breadcrumb)| {
            let is_current = index + 1 == breadcrumbs.len();
            let child = if is_current {
                PageNode::Html(HtmlNode {
                    tag: "span".to_owned(),
                    attributes: BTreeMap::from([("aria-current".to_owned(), "page".to_owned())]),
                    children: vec![PageNode::Text(TextNode {
                        value: breadcrumb.label.clone(),
                    })],
                })
            } else if let Some(href) = &breadcrumb.href {
                PageNode::Html(HtmlNode {
                    tag: "a".to_owned(),
                    attributes: BTreeMap::from([("href".to_owned(), href.clone())]),
                    children: vec![PageNode::Text(TextNode {
                        value: breadcrumb.label.clone(),
                    })],
                })
            } else {
                PageNode::Text(TextNode {
                    value: breadcrumb.label.clone(),
                })
            };

            PageNode::Html(HtmlNode {
                tag: "li".to_owned(),
                attributes: Default::default(),
                children: vec![child],
            })
        })
        .collect::<Vec<_>>();

    PageNode::Html(HtmlNode {
        tag: "nav".to_owned(),
        attributes: BTreeMap::from([
            ("data-admin-breadcrumbs".to_owned(), "true".to_owned()),
            ("aria-label".to_owned(), "breadcrumb".to_owned()),
        ]),
        children: vec![PageNode::Html(HtmlNode {
            tag: "ol".to_owned(),
            attributes: Default::default(),
            children: items,
        })],
    })
}

#[derive(Debug, Clone)]
struct AdminBreadcrumb {
    label: String,
    href: Option<String>,
}

fn admin_breadcrumbs(
    page: &AdminPageRegistration,
    registry: &HostRegistry,
) -> Vec<AdminBreadcrumb> {
    let zone = page
        .menu_zone
        .clone()
        .unwrap_or_else(|| "content".to_owned());
    let zone_href = registry
        .admin_menu_tree()
        .into_iter()
        .find(|group| group.zone == zone)
        .and_then(|group| group.entries.into_iter().next().map(|entry| entry.path));

    vec![
        AdminBreadcrumb {
            label: zone,
            href: zone_href,
        },
        AdminBreadcrumb {
            label: page.title.clone(),
            href: None,
        },
    ]
}

fn normalize_navigation_path(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn admin_page_mode_name(mode: AdminPageMode) -> &'static str {
    match mode {
        AdminPageMode::Html => "html",
        AdminPageMode::Hybrid => "hybrid",
        AdminPageMode::Island => "island",
        AdminPageMode::Compatibility => "compatibility",
    }
}

#[cfg(test)]
mod tests {
    use cycms_host_types::{
        AdminPageRegistration, CompiledExtensionRegistry, OwnershipMode, PageNode,
        RegistrationOriginKind, RegistrationSource,
    };
    use cycms_plugin_manager::HostRegistry;

    use super::*;

    fn source(order: usize) -> RegistrationSource {
        RegistrationSource {
            plugin_name: "blog".to_owned(),
            plugin_version: "0.1.0".to_owned(),
            origin: RegistrationOriginKind::HostManifest,
            declaration_order: order,
        }
    }

    fn page(mode: AdminPageMode) -> AdminPageRegistration {
        AdminPageRegistration {
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
            asset_bundle_ids: Vec::new(),
        }
    }

    fn registry(mode: AdminPageMode) -> HostRegistry {
        HostRegistry::new(CompiledExtensionRegistry {
            admin_pages: vec![page(mode)],
            ..CompiledExtensionRegistry::default()
        })
    }

    #[test]
    fn compatibility_page_renders_frame_breadcrumbs_and_preload() {
        let registry = registry(AdminPageMode::Compatibility);
        let output = DefaultAdminShellRenderer
            .render_page(registry.compiled().admin_pages.first().unwrap(), &registry);

        assert_eq!(output.page.head.len(), 3);
        assert_eq!(output.page.islands.len(), 1);
        assert_eq!(output.preload.len(), 1);
        assert!(output.diagnostics.is_empty());

        let PageNode::Html(main) = &output.page.body[0] else {
            panic!("expected main html node");
        };
        assert_eq!(
            main.attributes.get("data-admin-mode"),
            Some(&"compatibility".to_owned())
        );

        let PageNode::Html(nav) = &main.children[0] else {
            panic!("expected admin nav node");
        };
        assert_eq!(
            nav.attributes.get("data-admin-nav"),
            Some(&"primary".to_owned())
        );

        let PageNode::Html(breadcrumbs) = &main.children[1] else {
            panic!("expected breadcrumb nav node");
        };
        assert_eq!(
            breadcrumbs.attributes.get("data-admin-breadcrumbs"),
            Some(&"true".to_owned())
        );
        assert_eq!(
            breadcrumbs.attributes.get("aria-label"),
            Some(&"breadcrumb".to_owned())
        );

        assert_eq!(output.preload[0].id, "admin-preload:blog-dashboard");
        assert_eq!(output.preload[0].value["mode"], "compatibility");
        assert_eq!(
            output.preload[0].value["breadcrumbs"][0]["href"],
            "/admin/x/blog/dashboard"
        );
    }

    #[test]
    fn html_page_skips_island_mounts() {
        let registry = registry(AdminPageMode::Html);
        let output = DefaultAdminShellRenderer
            .render_page(registry.compiled().admin_pages.first().unwrap(), &registry);

        assert!(output.page.islands.is_empty());
        assert_eq!(output.preload[0].value["mode"], "html");
    }

    #[test]
    fn hybrid_and_island_pages_keep_shell_mounts() {
        for (mode, expected_mode) in [
            (AdminPageMode::Hybrid, "hybrid"),
            (AdminPageMode::Island, "island"),
        ] {
            let registry = registry(mode);
            let output = DefaultAdminShellRenderer
                .render_page(registry.compiled().admin_pages.first().unwrap(), &registry);

            assert_eq!(output.page.islands.len(), 1);
            assert_eq!(output.preload[0].value["mode"], expected_mode);

            let PageNode::Html(main) = &output.page.body[0] else {
                panic!("expected main html node");
            };
            assert_eq!(
                main.attributes.get("data-admin-mode"),
                Some(&expected_mode.to_owned())
            );
        }
    }
}
