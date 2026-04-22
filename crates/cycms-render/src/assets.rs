use std::collections::BTreeSet;

use cycms_core::{Error, Result};
use cycms_host_types::{
    AdminPageMode, AdminPageRegistration, AssetBundleRegistration, AssetGraph, AssetReference,
    IslandBootSpec, PageDocument, PublicPageRegistration,
};
use cycms_plugin_manager::HostRegistry;

pub trait AssetGraphBuilder {
    fn build_public_page(
        &self,
        page: &PageDocument,
        registration: &PublicPageRegistration,
        registry: &HostRegistry,
    ) -> Result<AssetGraph>;

    fn build_admin_page(
        &self,
        page: &PageDocument,
        registration: &AdminPageRegistration,
        registry: &HostRegistry,
    ) -> Result<AssetGraph>;
}

pub struct DefaultAssetGraphBuilder;

impl AssetGraphBuilder for DefaultAssetGraphBuilder {
    fn build_public_page(
        &self,
        page: &PageDocument,
        registration: &PublicPageRegistration,
        registry: &HostRegistry,
    ) -> Result<AssetGraph> {
        build_asset_graph(
            page,
            &registration.asset_bundle_ids,
            registry,
            true,
            |bundle| bundle_applies_to_public_page(bundle, registration),
        )
    }

    fn build_admin_page(
        &self,
        page: &PageDocument,
        registration: &AdminPageRegistration,
        registry: &HostRegistry,
    ) -> Result<AssetGraph> {
        let include_island_boot = !matches!(registration.mode, AdminPageMode::Html);
        build_asset_graph(
            page,
            &registration.asset_bundle_ids,
            registry,
            include_island_boot,
            |bundle| bundle_applies_to_admin_page(bundle, registration),
        )
    }
}

fn build_asset_graph<F>(
    page: &PageDocument,
    asset_bundle_ids: &[String],
    registry: &HostRegistry,
    include_island_boot: bool,
    applies_to_page: F,
) -> Result<AssetGraph>
where
    F: Fn(&AssetBundleRegistration) -> bool,
{
    let bundle_ids = asset_bundle_ids.iter().cloned().collect::<BTreeSet<_>>();
    let styles = collect_asset_refs(
        registry,
        &bundle_ids,
        "style",
        |bundle| &bundle.styles,
        &applies_to_page,
    );
    let scripts = collect_asset_refs(
        registry,
        &bundle_ids,
        "script",
        |bundle| &bundle.scripts,
        &applies_to_page,
    );
    let modules = collect_asset_refs(
        registry,
        &bundle_ids,
        "module",
        |bundle| &bundle.modules,
        &applies_to_page,
    );
    let island_boot = if include_island_boot {
        build_island_boot(page, &modules)?
    } else {
        Vec::new()
    };

    Ok(AssetGraph {
        styles,
        scripts,
        modules,
        island_boot,
        ..AssetGraph::default()
    })
}

fn build_island_boot(
    page: &PageDocument,
    modules: &[AssetReference],
) -> Result<Vec<IslandBootSpec>> {
    page.islands
        .iter()
        .map(|island| {
            let module = island
                .module_url
                .as_deref()
                .map(str::to_owned)
                .or_else(|| modules.first().map(|m| m.href.clone()))
                .ok_or_else(|| Error::ValidationError {
                    message: format!(
                        "page {} island {} must declare a module_url or the page must register at least one module asset",
                        page.route_id, island.id
                    ),
                    details: None,
                })?;
            Ok(IslandBootSpec {
                island_id: island.id.clone(),
                module,
                props: island.props.clone(),
            })
        })
        .collect()
}

fn collect_asset_refs<F, G>(
    registry: &HostRegistry,
    bundle_ids: &BTreeSet<String>,
    kind: &str,
    select: F,
    applies_to_page: &G,
) -> Vec<AssetReference>
where
    F: Fn(&AssetBundleRegistration) -> &[String],
    G: Fn(&AssetBundleRegistration) -> bool,
{
    let mut items = Vec::new();
    let mut seen_hrefs = BTreeSet::new();

    for bundle in registry
        .compiled()
        .assets
        .iter()
        .filter(|bundle| bundle_ids.contains(&bundle.id))
        .filter(|bundle| applies_to_page(bundle))
    {
        for (index, href) in select(bundle).iter().enumerate() {
            if seen_hrefs.insert(href.clone()) {
                items.push((href.clone(), format!("{}:{kind}:{index}", bundle.id)));
            }
        }
    }

    items.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
    items
        .into_iter()
        .map(|(href, id)| AssetReference { id, href })
        .collect()
}

fn bundle_applies_to_public_page(
    bundle: &AssetBundleRegistration,
    page: &PublicPageRegistration,
) -> bool {
    bundle.apply_to.is_empty()
        || bundle
            .apply_to
            .iter()
            .any(|target| target == "public_page" || target == &page.id || target == &page.path)
}

fn bundle_applies_to_admin_page(
    bundle: &AssetBundleRegistration,
    page: &AdminPageRegistration,
) -> bool {
    bundle.apply_to.is_empty()
        || bundle.apply_to.iter().any(|target| {
            target == "admin_page"
                || target == "admin_extension"
                || target == &page.id
                || target == &page.path
        })
}

#[cfg(test)]
mod tests {
    use http::StatusCode;
    use serde_json::json;

    use cycms_host_types::{
        CompiledExtensionRegistry, HeadNode, HtmlNode, IslandMount, OwnershipMode, PageNode,
        RegistrationOriginKind, RegistrationSource, TextNode,
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

    fn page_document(route_id: &str, islands: Vec<IslandMount>) -> PageDocument {
        PageDocument {
            route_id: route_id.to_owned(),
            status: StatusCode::OK,
            head: vec![HeadNode::Title {
                text: "Blog".to_owned(),
            }],
            body: vec![PageNode::Html(HtmlNode {
                tag: "main".to_owned(),
                attributes: Default::default(),
                children: vec![PageNode::Text(TextNode {
                    value: "Blog".to_owned(),
                })],
            })],
            actions: Vec::new(),
            islands,
            cache_tags: Vec::new(),
        }
    }

    #[test]
    fn public_page_asset_graph_deduplicates_and_sorts_assets() {
        let registry = HostRegistry::new(CompiledExtensionRegistry {
            public_pages: vec![PublicPageRegistration {
                id: "blog-home".to_owned(),
                source: source(0),
                path: "/blog".to_owned(),
                priority: 0,
                ownership: OwnershipMode::Replace,
                handler: "blog::public::home".to_owned(),
                title: Some("Blog".to_owned()),
                asset_bundle_ids: vec![
                    "blog-second".to_owned(),
                    "blog-first".to_owned(),
                    "admin-only".to_owned(),
                ],
            }],
            assets: vec![
                AssetBundleRegistration {
                    id: "blog-second".to_owned(),
                    source: source(2),
                    apply_to: vec!["public_page".to_owned()],
                    modules: vec!["/assets/runtime.js".to_owned()],
                    scripts: vec!["/assets/legacy.js".to_owned()],
                    styles: vec![
                        "/assets/theme-b.css".to_owned(),
                        "/assets/theme-a.css".to_owned(),
                    ],
                    inline_data_keys: Vec::new(),
                },
                AssetBundleRegistration {
                    id: "blog-first".to_owned(),
                    source: source(1),
                    apply_to: vec!["blog-home".to_owned()],
                    modules: vec!["/assets/runtime.js".to_owned()],
                    scripts: Vec::new(),
                    styles: vec!["/assets/theme-a.css".to_owned()],
                    inline_data_keys: Vec::new(),
                },
                AssetBundleRegistration {
                    id: "admin-only".to_owned(),
                    source: source(3),
                    apply_to: vec!["admin_page".to_owned()],
                    modules: vec!["/assets/admin.js".to_owned()],
                    scripts: Vec::new(),
                    styles: vec!["/assets/admin.css".to_owned()],
                    inline_data_keys: Vec::new(),
                },
            ],
            ..CompiledExtensionRegistry::default()
        });

        let page = page_document("public:/blog", Vec::new());
        let registration = registry.compiled().public_pages.first().unwrap();
        let graph = DefaultAssetGraphBuilder
            .build_public_page(&page, registration, &registry)
            .unwrap();

        assert_eq!(
            graph
                .styles
                .iter()
                .map(|asset| asset.href.as_str())
                .collect::<Vec<_>>(),
            vec!["/assets/theme-a.css", "/assets/theme-b.css"]
        );
        assert_eq!(
            graph
                .modules
                .iter()
                .map(|asset| asset.href.as_str())
                .collect::<Vec<_>>(),
            vec!["/assets/runtime.js"]
        );
        assert_eq!(
            graph
                .scripts
                .iter()
                .map(|asset| asset.href.as_str())
                .collect::<Vec<_>>(),
            vec!["/assets/legacy.js"]
        );
        assert!(graph.island_boot.is_empty());
    }

    #[test]
    fn empty_apply_to_matches_public_page_by_default() {
        let registry = HostRegistry::new(CompiledExtensionRegistry {
            public_pages: vec![PublicPageRegistration {
                id: "blog-home".to_owned(),
                source: source(0),
                path: "/blog".to_owned(),
                priority: 0,
                ownership: OwnershipMode::Replace,
                handler: "blog::public::home".to_owned(),
                title: Some("Blog".to_owned()),
                asset_bundle_ids: vec!["shared".to_owned()],
            }],
            assets: vec![AssetBundleRegistration {
                id: "shared".to_owned(),
                source: source(1),
                apply_to: Vec::new(),
                modules: vec!["/assets/shared.js".to_owned()],
                scripts: Vec::new(),
                styles: vec!["/assets/shared.css".to_owned()],
                inline_data_keys: Vec::new(),
            }],
            ..CompiledExtensionRegistry::default()
        });

        let page = page_document("public:/blog", Vec::new());
        let registration = registry.compiled().public_pages.first().unwrap();
        let graph = DefaultAssetGraphBuilder
            .build_public_page(&page, registration, &registry)
            .unwrap();

        assert_eq!(graph.styles.len(), 1);
        assert_eq!(graph.styles[0].href, "/assets/shared.css");
        assert_eq!(graph.modules.len(), 1);
        assert_eq!(graph.modules[0].href, "/assets/shared.js");
        assert!(graph.island_boot.is_empty());
    }

    #[test]
    fn missing_bundle_ids_produce_empty_asset_graph() {
        let registry = HostRegistry::new(CompiledExtensionRegistry {
            public_pages: vec![PublicPageRegistration {
                id: "blog-home".to_owned(),
                source: source(0),
                path: "/blog".to_owned(),
                priority: 0,
                ownership: OwnershipMode::Replace,
                handler: "blog::public::home".to_owned(),
                title: Some("Blog".to_owned()),
                asset_bundle_ids: Vec::new(),
            }],
            ..CompiledExtensionRegistry::default()
        });

        let page = page_document("public:/blog", Vec::new());
        let registration = registry.compiled().public_pages.first().unwrap();
        let graph = DefaultAssetGraphBuilder
            .build_public_page(&page, registration, &registry)
            .unwrap();

        assert!(graph.styles.is_empty());
        assert!(graph.scripts.is_empty());
        assert!(graph.modules.is_empty());
        assert!(graph.inline_data.is_empty());
        assert!(graph.island_boot.is_empty());
    }

    #[test]
    fn interactive_page_generates_island_boot_payloads_from_first_module() {
        let registry = HostRegistry::new(CompiledExtensionRegistry {
            public_pages: vec![PublicPageRegistration {
                id: "blog-home".to_owned(),
                source: source(0),
                path: "/blog".to_owned(),
                priority: 0,
                ownership: OwnershipMode::Replace,
                handler: "blog::public::home".to_owned(),
                title: Some("Blog".to_owned()),
                asset_bundle_ids: vec!["blog-main".to_owned()],
            }],
            assets: vec![AssetBundleRegistration {
                id: "blog-main".to_owned(),
                source: source(1),
                apply_to: vec!["public_page".to_owned()],
                modules: vec!["/assets/blog-entry.js".to_owned()],
                scripts: Vec::new(),
                styles: Vec::new(),
                inline_data_keys: Vec::new(),
            }],
            ..CompiledExtensionRegistry::default()
        });

        let page = page_document(
            "public:/blog",
            vec![IslandMount {
                id: "editor".to_owned(),
                component: "EditorIsland".to_owned(),
                props: json!({"entryId": "post-1"}),
                module_url: None,
            }],
        );
        let registration = registry.compiled().public_pages.first().unwrap();
        let graph = DefaultAssetGraphBuilder
            .build_public_page(&page, registration, &registry)
            .unwrap();

        assert_eq!(graph.island_boot.len(), 1);
        assert_eq!(graph.island_boot[0].island_id, "editor");
        assert_eq!(graph.island_boot[0].module, "/assets/blog-entry.js");
        assert_eq!(graph.island_boot[0].props, json!({"entryId": "post-1"}));
    }

    #[test]
    fn interactive_page_without_module_asset_returns_validation_error() {
        let registry = HostRegistry::new(CompiledExtensionRegistry {
            public_pages: vec![PublicPageRegistration {
                id: "blog-home".to_owned(),
                source: source(0),
                path: "/blog".to_owned(),
                priority: 0,
                ownership: OwnershipMode::Replace,
                handler: "blog::public::home".to_owned(),
                title: Some("Blog".to_owned()),
                asset_bundle_ids: vec!["blog-main".to_owned()],
            }],
            assets: vec![AssetBundleRegistration {
                id: "blog-main".to_owned(),
                source: source(1),
                apply_to: vec!["public_page".to_owned()],
                modules: Vec::new(),
                scripts: Vec::new(),
                styles: Vec::new(),
                inline_data_keys: Vec::new(),
            }],
            ..CompiledExtensionRegistry::default()
        });

        let page = page_document(
            "public:/blog",
            vec![IslandMount {
                id: "editor".to_owned(),
                component: "EditorIsland".to_owned(),
                props: json!({"entryId": "post-1"}),
                module_url: None,
            }],
        );
        let registration = registry.compiled().public_pages.first().unwrap();
        let error = DefaultAssetGraphBuilder
            .build_public_page(&page, registration, &registry)
            .unwrap_err();

        assert!(matches!(error, Error::ValidationError { .. }));
        assert!(error.to_string().contains(
            "must declare a module_url or the page must register at least one module asset"
        ));
    }

    #[test]
    fn admin_html_page_collects_admin_assets_without_island_boot() {
        let registry = HostRegistry::new(CompiledExtensionRegistry {
            admin_pages: vec![AdminPageRegistration {
                id: "blog-dashboard".to_owned(),
                source: source(0),
                path: "/admin/x/blog/dashboard".to_owned(),
                title: "Blog Dashboard".to_owned(),
                mode: AdminPageMode::Html,
                priority: 0,
                ownership: OwnershipMode::Replace,
                handler: "blog::admin::dashboard".to_owned(),
                menu_label: None,
                menu_zone: None,
                asset_bundle_ids: vec!["blog-admin".to_owned()],
            }],
            assets: vec![AssetBundleRegistration {
                id: "blog-admin".to_owned(),
                source: source(1),
                apply_to: vec!["admin_page".to_owned()],
                modules: vec!["/assets/blog-admin.js".to_owned()],
                scripts: Vec::new(),
                styles: vec!["/assets/blog-admin.css".to_owned()],
                inline_data_keys: Vec::new(),
            }],
            ..CompiledExtensionRegistry::default()
        });

        let page = page_document("admin:/admin/x/blog/dashboard", Vec::new());
        let registration = registry.compiled().admin_pages.first().unwrap();
        let graph = DefaultAssetGraphBuilder
            .build_admin_page(&page, registration, &registry)
            .unwrap();

        assert_eq!(graph.styles[0].href, "/assets/blog-admin.css");
        assert_eq!(graph.modules[0].href, "/assets/blog-admin.js");
        assert!(graph.island_boot.is_empty());
    }

    #[test]
    fn admin_interactive_page_uses_admin_extension_targets_for_island_boot() {
        let registry = HostRegistry::new(CompiledExtensionRegistry {
            admin_pages: vec![AdminPageRegistration {
                id: "compat.blog.route.root".to_owned(),
                source: source(0),
                path: "/admin/x/blog/dashboard".to_owned(),
                title: "Blog Dashboard".to_owned(),
                mode: AdminPageMode::Compatibility,
                priority: 0,
                ownership: OwnershipMode::Replace,
                handler: "frontend.route:root".to_owned(),
                menu_label: None,
                menu_zone: None,
                asset_bundle_ids: vec!["compat.blog.asset.admin-main".to_owned()],
            }],
            assets: vec![AssetBundleRegistration {
                id: "compat.blog.asset.admin-main".to_owned(),
                source: source(1),
                apply_to: vec!["admin_extension".to_owned()],
                modules: vec!["/plugins/blog/admin/main.js".to_owned()],
                scripts: Vec::new(),
                styles: vec!["/plugins/blog/admin/main.css".to_owned()],
                inline_data_keys: Vec::new(),
            }],
            ..CompiledExtensionRegistry::default()
        });

        let page = page_document(
            "admin:/admin/x/blog/dashboard",
            vec![IslandMount {
                id: "admin-screen:compat.blog.route.root".to_owned(),
                component: "frontend.route:root".to_owned(),
                props: json!({"path": "/admin/x/blog/dashboard"}),
                module_url: None,
            }],
        );
        let registration = registry.compiled().admin_pages.first().unwrap();
        let graph = DefaultAssetGraphBuilder
            .build_admin_page(&page, registration, &registry)
            .unwrap();

        assert_eq!(graph.styles[0].href, "/plugins/blog/admin/main.css");
        assert_eq!(graph.modules[0].href, "/plugins/blog/admin/main.js");
        assert_eq!(graph.island_boot.len(), 1);
        assert_eq!(graph.island_boot[0].module, "/plugins/blog/admin/main.js");
    }

    #[test]
    fn admin_page_filters_out_public_only_bundles() {
        let registry = HostRegistry::new(CompiledExtensionRegistry {
            admin_pages: vec![AdminPageRegistration {
                id: "blog-dashboard".to_owned(),
                source: source(0),
                path: "/admin/x/blog/dashboard".to_owned(),
                title: "Blog Dashboard".to_owned(),
                mode: AdminPageMode::Compatibility,
                priority: 0,
                ownership: OwnershipMode::Replace,
                handler: "frontend.route:root".to_owned(),
                menu_label: None,
                menu_zone: None,
                asset_bundle_ids: vec!["blog-admin".to_owned(), "blog-public".to_owned()],
            }],
            assets: vec![
                AssetBundleRegistration {
                    id: "blog-admin".to_owned(),
                    source: source(1),
                    apply_to: vec!["admin_extension".to_owned()],
                    modules: vec!["/plugins/blog/admin/main.js".to_owned()],
                    scripts: Vec::new(),
                    styles: vec!["/plugins/blog/admin/main.css".to_owned()],
                    inline_data_keys: Vec::new(),
                },
                AssetBundleRegistration {
                    id: "blog-public".to_owned(),
                    source: source(2),
                    apply_to: vec!["public_page".to_owned()],
                    modules: vec!["/assets/public-only.js".to_owned()],
                    scripts: Vec::new(),
                    styles: vec!["/assets/public-only.css".to_owned()],
                    inline_data_keys: Vec::new(),
                },
            ],
            ..CompiledExtensionRegistry::default()
        });

        let page = page_document(
            "admin:/admin/x/blog/dashboard",
            vec![IslandMount {
                id: "admin-screen:blog-dashboard".to_owned(),
                component: "frontend.route:root".to_owned(),
                props: json!({"path": "/admin/x/blog/dashboard"}),
                module_url: None,
            }],
        );
        let registration = registry.compiled().admin_pages.first().unwrap();
        let graph = DefaultAssetGraphBuilder
            .build_admin_page(&page, registration, &registry)
            .unwrap();

        assert_eq!(graph.styles.len(), 1);
        assert_eq!(graph.styles[0].href, "/plugins/blog/admin/main.css");
        assert_eq!(graph.modules.len(), 1);
        assert_eq!(graph.modules[0].href, "/plugins/blog/admin/main.js");
    }

    #[test]
    fn admin_html_mode_skips_island_boot_even_if_page_contains_islands() {
        let registry = HostRegistry::new(CompiledExtensionRegistry {
            admin_pages: vec![AdminPageRegistration {
                id: "blog-dashboard".to_owned(),
                source: source(0),
                path: "/admin/x/blog/dashboard".to_owned(),
                title: "Blog Dashboard".to_owned(),
                mode: AdminPageMode::Html,
                priority: 0,
                ownership: OwnershipMode::Replace,
                handler: "blog::admin::dashboard".to_owned(),
                menu_label: None,
                menu_zone: None,
                asset_bundle_ids: vec!["blog-admin".to_owned()],
            }],
            assets: vec![AssetBundleRegistration {
                id: "blog-admin".to_owned(),
                source: source(1),
                apply_to: vec!["admin_page".to_owned()],
                modules: vec!["/assets/blog-admin.js".to_owned()],
                scripts: Vec::new(),
                styles: vec!["/assets/blog-admin.css".to_owned()],
                inline_data_keys: Vec::new(),
            }],
            ..CompiledExtensionRegistry::default()
        });

        let page = page_document(
            "admin:/admin/x/blog/dashboard",
            vec![IslandMount {
                id: "admin-screen:blog-dashboard".to_owned(),
                component: "blog::admin::dashboard".to_owned(),
                props: json!({"path": "/admin/x/blog/dashboard"}),
                module_url: None,
            }],
        );
        let registration = registry.compiled().admin_pages.first().unwrap();
        let graph = DefaultAssetGraphBuilder
            .build_admin_page(&page, registration, &registry)
            .unwrap();

        assert!(graph.island_boot.is_empty());
    }
}
