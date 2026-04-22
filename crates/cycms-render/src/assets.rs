use std::collections::BTreeSet;

use cycms_core::Result;
use cycms_host_types::{
    AssetBundleRegistration, AssetGraph, AssetReference, PublicPageRegistration,
};
use cycms_plugin_manager::HostRegistry;

pub trait AssetGraphBuilder {
    fn build_public_page(
        &self,
        page: &PublicPageRegistration,
        registry: &HostRegistry,
    ) -> Result<AssetGraph>;
}

pub struct DefaultAssetGraphBuilder;

impl AssetGraphBuilder for DefaultAssetGraphBuilder {
    fn build_public_page(
        &self,
        page: &PublicPageRegistration,
        registry: &HostRegistry,
    ) -> Result<AssetGraph> {
        let bundle_ids = page
            .asset_bundle_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();

        Ok(AssetGraph {
            styles: collect_asset_refs(registry, &bundle_ids, page, "style", |bundle| {
                &bundle.styles
            }),
            scripts: collect_asset_refs(registry, &bundle_ids, page, "script", |bundle| {
                &bundle.scripts
            }),
            modules: collect_asset_refs(registry, &bundle_ids, page, "module", |bundle| {
                &bundle.modules
            }),
            ..AssetGraph::default()
        })
    }
}

fn collect_asset_refs<F>(
    registry: &HostRegistry,
    bundle_ids: &BTreeSet<String>,
    page: &PublicPageRegistration,
    kind: &str,
    select: F,
) -> Vec<AssetReference>
where
    F: Fn(&AssetBundleRegistration) -> &[String],
{
    let mut items = Vec::new();
    let mut seen_hrefs = BTreeSet::new();

    for bundle in registry
        .compiled()
        .assets
        .iter()
        .filter(|bundle| bundle_ids.contains(&bundle.id))
        .filter(|bundle| bundle_applies_to_public_page(bundle, page))
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

#[cfg(test)]
mod tests {
    use cycms_host_types::{
        CompiledExtensionRegistry, OwnershipMode, PublicPageRegistration, RegistrationOriginKind,
        RegistrationSource,
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

        let page = registry.compiled().public_pages.first().unwrap();
        let graph = DefaultAssetGraphBuilder
            .build_public_page(page, &registry)
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

        let page = registry.compiled().public_pages.first().unwrap();
        let graph = DefaultAssetGraphBuilder
            .build_public_page(page, &registry)
            .unwrap();

        assert_eq!(graph.styles.len(), 1);
        assert_eq!(graph.styles[0].href, "/assets/shared.css");
        assert_eq!(graph.modules.len(), 1);
        assert_eq!(graph.modules[0].href, "/assets/shared.js");
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

        let page = registry.compiled().public_pages.first().unwrap();
        let graph = DefaultAssetGraphBuilder
            .build_public_page(page, &registry)
            .unwrap();

        assert!(graph.styles.is_empty());
        assert!(graph.scripts.is_empty());
        assert!(graph.modules.is_empty());
        assert!(graph.inline_data.is_empty());
        assert!(graph.island_boot.is_empty());
    }
}
