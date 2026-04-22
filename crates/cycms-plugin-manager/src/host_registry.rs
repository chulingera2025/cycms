use std::cmp::Ordering;
use std::collections::BTreeSet;

use cycms_host_types::{
    AdminPageRegistration, CompiledExtensionRegistry, EditorRegistration, EditorTarget,
    HostRegistryDiagnosticsSnapshot, HostRequestTarget, OwnershipCandidate, OwnershipDecision,
    OwnershipDiagnostics, OwnershipMode, ParserRegistration, ParseTarget,
    PublicPageRegistration, RegistrationSource,
};

pub trait RegistryLookup {
    fn resolve_public_page(
        &self,
        request: &HostRequestTarget,
    ) -> OwnershipDecision<PublicPageRegistration>;

    fn resolve_admin_page(
        &self,
        request: &HostRequestTarget,
    ) -> OwnershipDecision<AdminPageRegistration>;

    fn resolve_parser(&self, target: &ParseTarget) -> OwnershipDecision<ParserRegistration>;

    fn resolve_editor(&self, target: &EditorTarget) -> OwnershipDecision<EditorRegistration>;
}

#[derive(Debug, Clone)]
pub struct HostRegistry {
    compiled: CompiledExtensionRegistry,
}

impl HostRegistry {
    #[must_use]
    pub fn new(compiled: CompiledExtensionRegistry) -> Self {
        Self { compiled }
    }

    #[must_use]
    pub fn compiled(&self) -> &CompiledExtensionRegistry {
        &self.compiled
    }

    #[must_use]
    pub fn diagnostics_snapshot(&self) -> HostRegistryDiagnosticsSnapshot {
        let public_pages = self
            .compiled
            .public_pages
            .iter()
            .map(|page| page.path.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|path| self.resolve_public_page(&HostRequestTarget { path }).diagnostics)
            .collect();
        let admin_pages = self
            .compiled
            .admin_pages
            .iter()
            .map(|page| page.path.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|path| self.resolve_admin_page(&HostRequestTarget { path }).diagnostics)
            .collect();
        let parsers = self
            .compiled
            .parsers
            .iter()
            .map(parser_target_key)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .filter_map(|key| parser_target_from_key(&key))
            .map(|target| self.resolve_parser(&target).diagnostics)
            .collect();
        let editors = self
            .compiled
            .editors
            .iter()
            .map(editor_target_key)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .filter_map(|key| editor_target_from_key(&key))
            .map(|target| self.resolve_editor(&target).diagnostics)
            .collect();

        HostRegistryDiagnosticsSnapshot {
            public_pages,
            admin_pages,
            parsers,
            editors,
            asset_bundles: self.compiled.assets.clone(),
            hooks: self.compiled.hooks.clone(),
            compatibility: self.compiled.compatibility.clone(),
        }
    }

    fn resolve_owned<T>(
        surface: &str,
        target: String,
        mut candidates: Vec<T>,
    ) -> OwnershipDecision<T>
    where
        T: OwnedRegistration,
    {
        candidates.sort_by(compare_candidates::<T>);
        let primary = candidates
            .iter()
            .find(|candidate| candidate.ownership() == OwnershipMode::Replace)
            .cloned();
        let wrappers: Vec<T> = candidates
            .iter()
            .filter(|candidate| candidate.ownership() == OwnershipMode::Wrap)
            .cloned()
            .collect();
        let appenders: Vec<T> = candidates
            .iter()
            .filter(|candidate| candidate.ownership() == OwnershipMode::Append)
            .cloned()
            .collect();

        let diagnostics = OwnershipDiagnostics {
            surface: surface.to_owned(),
            target,
            candidates: candidates.iter().map(ownership_candidate_from_registration).collect(),
            primary: primary.as_ref().map(|candidate| candidate.id().to_owned()),
            wrappers: wrappers.iter().map(|candidate| candidate.id().to_owned()).collect(),
            appenders: appenders
                .iter()
                .map(|candidate| candidate.id().to_owned())
                .collect(),
        };

        OwnershipDecision {
            primary,
            wrappers,
            appenders,
            diagnostics,
        }
    }
}

impl RegistryLookup for HostRegistry {
    fn resolve_public_page(
        &self,
        request: &HostRequestTarget,
    ) -> OwnershipDecision<PublicPageRegistration> {
        let normalized = normalize_path(&request.path);
        let candidates = self
            .compiled
            .public_pages
            .iter()
            .filter(|page| normalize_path(&page.path) == normalized)
            .cloned()
            .collect();
        Self::resolve_owned("public_page", normalized, candidates)
    }

    fn resolve_admin_page(
        &self,
        request: &HostRequestTarget,
    ) -> OwnershipDecision<AdminPageRegistration> {
        let normalized = normalize_path(&request.path);
        let candidates = self
            .compiled
            .admin_pages
            .iter()
            .filter(|page| normalize_path(&page.path) == normalized)
            .cloned()
            .collect();
        Self::resolve_owned("admin_page", normalized, candidates)
    }

    fn resolve_parser(&self, target: &ParseTarget) -> OwnershipDecision<ParserRegistration> {
        let candidates = self
            .compiled
            .parsers
            .iter()
            .filter(|parser| parser_matches(parser, target))
            .cloned()
            .collect();
        Self::resolve_owned("parser", format_parse_target(target), candidates)
    }

    fn resolve_editor(&self, target: &EditorTarget) -> OwnershipDecision<EditorRegistration> {
        let candidates = self
            .compiled
            .editors
            .iter()
            .filter(|editor| editor_matches(editor, target))
            .cloned()
            .collect();
        Self::resolve_owned("editor", format_editor_target(target), candidates)
    }
}

trait OwnedRegistration: Clone {
    fn id(&self) -> &str;
    fn priority(&self) -> i32;
    fn ownership(&self) -> OwnershipMode;
    fn source(&self) -> &RegistrationSource;
}

impl OwnedRegistration for PublicPageRegistration {
    fn id(&self) -> &str { &self.id }
    fn priority(&self) -> i32 { self.priority }
    fn ownership(&self) -> OwnershipMode { self.ownership }
    fn source(&self) -> &RegistrationSource { &self.source }
}

impl OwnedRegistration for AdminPageRegistration {
    fn id(&self) -> &str { &self.id }
    fn priority(&self) -> i32 { self.priority }
    fn ownership(&self) -> OwnershipMode { self.ownership }
    fn source(&self) -> &RegistrationSource { &self.source }
}

impl OwnedRegistration for ParserRegistration {
    fn id(&self) -> &str { &self.id }
    fn priority(&self) -> i32 { self.priority }
    fn ownership(&self) -> OwnershipMode { self.ownership }
    fn source(&self) -> &RegistrationSource { &self.source }
}

impl OwnedRegistration for EditorRegistration {
    fn id(&self) -> &str { &self.id }
    fn priority(&self) -> i32 { self.priority }
    fn ownership(&self) -> OwnershipMode { self.ownership }
    fn source(&self) -> &RegistrationSource { &self.source }
}

fn ownership_candidate_from_registration<T: OwnedRegistration>(registration: &T) -> OwnershipCandidate {
    OwnershipCandidate {
        registration_id: registration.id().to_owned(),
        plugin_name: registration.source().plugin_name.clone(),
        plugin_version: registration.source().plugin_version.clone(),
        origin: registration.source().origin,
        ownership: registration.ownership(),
        priority: registration.priority(),
        declaration_order: registration.source().declaration_order,
    }
}

fn compare_candidates<T: OwnedRegistration>(left: &T, right: &T) -> Ordering {
    right
        .priority()
        .cmp(&left.priority())
        .then(left.ownership().precedence().cmp(&right.ownership().precedence()))
        .then(left.source().declaration_order.cmp(&right.source().declaration_order))
        .then(left.id().cmp(right.id()))
}

fn normalize_path(path: &str) -> String {
    if path == "/" {
        return "/".to_owned();
    }
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn matches_selector(selectors: &[String], actual: Option<&str>) -> bool {
    selectors.is_empty() || actual.is_some_and(|value| selectors.iter().any(|selector| selector == value))
}

fn parser_matches(parser: &ParserRegistration, target: &ParseTarget) -> bool {
    matches_selector(&parser.content_types, target.content_type.as_deref())
        && matches_selector(&parser.field_names, target.field_name.as_deref())
        && matches_selector(&parser.source_formats, target.source_format.as_deref())
}

fn editor_matches(editor: &EditorRegistration, target: &EditorTarget) -> bool {
    matches_selector(&editor.content_types, target.content_type.as_deref())
        && matches_selector(&editor.field_types, target.field_type.as_deref())
        && matches_selector(&editor.screen_targets, target.screen_target.as_deref())
}

fn parser_target_key(parser: &ParserRegistration) -> String {
    format!(
        "ct={}|fn={}|sf={}",
        parser.content_types.join(","),
        parser.field_names.join(","),
        parser.source_formats.join(",")
    )
}

fn parser_target_from_key(key: &str) -> Option<ParseTarget> {
    let mut target = ParseTarget::default();
    for part in key.split('|') {
        let (name, value) = part.split_once('=')?;
        match name {
            "ct" if !value.is_empty() => target.content_type = value.split(',').next().map(ToOwned::to_owned),
            "fn" if !value.is_empty() => target.field_name = value.split(',').next().map(ToOwned::to_owned),
            "sf" if !value.is_empty() => target.source_format = value.split(',').next().map(ToOwned::to_owned),
            _ => {}
        }
    }
    Some(target)
}

fn editor_target_key(editor: &EditorRegistration) -> String {
    format!(
        "ct={}|ft={}|st={}",
        editor.content_types.join(","),
        editor.field_types.join(","),
        editor.screen_targets.join(",")
    )
}

fn editor_target_from_key(key: &str) -> Option<EditorTarget> {
    let mut target = EditorTarget::default();
    for part in key.split('|') {
        let (name, value) = part.split_once('=')?;
        match name {
            "ct" if !value.is_empty() => target.content_type = value.split(',').next().map(ToOwned::to_owned),
            "ft" if !value.is_empty() => target.field_type = value.split(',').next().map(ToOwned::to_owned),
            "st" if !value.is_empty() => target.screen_target = value.split(',').next().map(ToOwned::to_owned),
            _ => {}
        }
    }
    Some(target)
}

fn format_parse_target(target: &ParseTarget) -> String {
    format!(
        "content_type={:?};field_name={:?};source_format={:?}",
        target.content_type, target.field_name, target.source_format
    )
}

fn format_editor_target(target: &EditorTarget) -> String {
    format!(
        "content_type={:?};field_type={:?};screen_target={:?}",
        target.content_type, target.field_type, target.screen_target
    )
}

#[cfg(test)]
mod tests {
    use cycms_host_types::{
        CompatibilityRegistration, HookRegistration, RegistrationOriginKind, RegistrationSource,
    };

    use super::*;

    fn source(plugin_name: &str, declaration_order: usize) -> RegistrationSource {
        RegistrationSource {
            plugin_name: plugin_name.to_owned(),
            plugin_version: "0.1.0".to_owned(),
            origin: RegistrationOriginKind::HostManifest,
            declaration_order,
        }
    }

    #[test]
    fn resolve_public_page_orders_replace_wrap_append() {
        let registry = HostRegistry::new(CompiledExtensionRegistry {
            public_pages: vec![
                PublicPageRegistration {
                    id: "theme-shell".to_owned(),
                    source: source("theme", 3),
                    path: "/blog".to_owned(),
                    priority: 10,
                    ownership: OwnershipMode::Wrap,
                    handler: "theme::wrap".to_owned(),
                    title: None,
                    asset_bundle_ids: Vec::new(),
                },
                PublicPageRegistration {
                    id: "blog-home".to_owned(),
                    source: source("blog", 1),
                    path: "/blog".to_owned(),
                    priority: 100,
                    ownership: OwnershipMode::Replace,
                    handler: "blog::home".to_owned(),
                    title: None,
                    asset_bundle_ids: Vec::new(),
                },
                PublicPageRegistration {
                    id: "analytics".to_owned(),
                    source: source("analytics", 5),
                    path: "/blog".to_owned(),
                    priority: 0,
                    ownership: OwnershipMode::Append,
                    handler: "analytics::append".to_owned(),
                    title: None,
                    asset_bundle_ids: Vec::new(),
                },
            ],
            ..CompiledExtensionRegistry::default()
        });

        let decision = registry.resolve_public_page(&HostRequestTarget {
            path: "/blog/".to_owned(),
        });

        assert_eq!(decision.primary.unwrap().id, "blog-home");
        assert_eq!(decision.wrappers.len(), 1);
        assert_eq!(decision.appenders.len(), 1);
        assert_eq!(decision.diagnostics.primary.as_deref(), Some("blog-home"));
    }

    #[test]
    fn diagnostics_snapshot_keeps_compatibility_entries() {
        let registry = HostRegistry::new(CompiledExtensionRegistry {
            compatibility: vec![CompatibilityRegistration {
                id: "blog.runtime".to_owned(),
                source: source("blog", 0),
                kind: cycms_host_types::CompatibilityKind::DynamicNativePlugin,
                target: "blog".to_owned(),
                metadata: Default::default(),
            }],
            hooks: vec![HookRegistration {
                id: "blog.before_send".to_owned(),
                source: source("blog", 1),
                priority: 0,
                ownership: OwnershipMode::Append,
                phase: "before_send".to_owned(),
                handler: "blog::before_send".to_owned(),
            }],
            ..CompiledExtensionRegistry::default()
        });

        let snapshot = registry.diagnostics_snapshot();
        assert_eq!(snapshot.compatibility.len(), 1);
        assert_eq!(snapshot.hooks.len(), 1);
    }
}