//! 依赖解析与拓扑排序。纯函数，无 I/O，供 service 层在 install / enable 时调用。
//!
//! 前置条件：所有传入的 [`PluginManifest`] 已通过 [`PluginManifest::validate`] 校验，
//! 因此 `plugin.version` / `compatibility.cycms` / `dependencies[*].version` 都是
//! 合法的 `SemVer` 字面量，内部直接 `.expect("validated")` 是安全的。

use std::collections::{BTreeMap, BTreeSet};

use semver::{Version, VersionReq};

use crate::error::PluginManagerError;
use crate::manifest::PluginManifest;

/// 校验 manifest 的 `compatibility.cycms` 是否允许当前宿主版本加载（Req 20.2）。
///
/// # Errors
/// 版本范围不匹配时返回 [`PluginManagerError::IncompatibleHost`]。
pub fn check_host_compatibility(
    manifest: &PluginManifest,
    cycms: &Version,
) -> Result<(), PluginManagerError> {
    let req = manifest.parsed_compatibility();
    if req.matches(cycms) {
        Ok(())
    } else {
        Err(PluginManagerError::IncompatibleHost {
            plugin: manifest.plugin.name.clone(),
            required: manifest.compatibility.cycms.clone(),
            actual: cycms.to_string(),
        })
    }
}

/// 对一组 manifest 做依赖拓扑排序：依赖在前，被依赖在后。
///
/// 同层插件按 `name` 字典序出队，保证结果确定性（便于测试与日志对照）。
///
/// # Errors
/// - 非 optional 依赖缺失 → [`PluginManagerError::MissingDependency`]
/// - 非 optional 依赖版本不匹配 → [`PluginManagerError::IncompatibleDependency`]
/// - 依赖图存在循环 → [`PluginManagerError::CyclicDependency`]
///
/// # Panics
/// 仅当传入的 manifest 未经过 [`PluginManifest::validate`] 时才会 panic
/// （前置条件由调用方保证，内部 API 不可达）。
pub fn topological_order(
    manifests: &[PluginManifest],
) -> Result<Vec<String>, PluginManagerError> {
    let by_name: BTreeMap<&str, &PluginManifest> = manifests
        .iter()
        .map(|m| (m.plugin.name.as_str(), m))
        .collect();

    let mut incoming: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut outgoing: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for m in manifests {
        incoming.entry(m.plugin.name.clone()).or_default();
        outgoing.entry(m.plugin.name.clone()).or_default();
    }

    for m in manifests {
        for (dep_name, spec) in &m.dependencies {
            let req = VersionReq::parse(&spec.version).expect("manifest validated");
            match by_name.get(dep_name.as_str()) {
                None if spec.optional => {}
                None => {
                    return Err(PluginManagerError::MissingDependency {
                        plugin: m.plugin.name.clone(),
                        dependency: dep_name.clone(),
                    });
                }
                Some(dep_manifest) => {
                    let dep_ver = dep_manifest.parsed_version();
                    if !req.matches(&dep_ver) {
                        if spec.optional {
                            continue;
                        }
                        return Err(PluginManagerError::IncompatibleDependency {
                            plugin: m.plugin.name.clone(),
                            dependency: dep_name.clone(),
                            required: spec.version.clone(),
                            actual: dep_ver.to_string(),
                        });
                    }
                    outgoing
                        .get_mut(dep_name)
                        .expect("initialized above")
                        .insert(m.plugin.name.clone());
                    incoming
                        .get_mut(&m.plugin.name)
                        .expect("initialized above")
                        .insert(dep_name.clone());
                }
            }
        }
    }

    let mut result = Vec::with_capacity(manifests.len());
    let mut ready: BTreeSet<String> = incoming
        .iter()
        .filter(|(_, deps)| deps.is_empty())
        .map(|(n, _)| n.clone())
        .collect();

    while let Some(name) = ready.iter().next().cloned() {
        ready.remove(&name);
        result.push(name.clone());
        let downstream = outgoing.remove(&name).unwrap_or_default();
        for ds in downstream {
            let deps = incoming.get_mut(&ds).expect("initialized above");
            deps.remove(&name);
            if deps.is_empty() {
                ready.insert(ds);
            }
        }
    }

    if result.len() != manifests.len() {
        let involved: Vec<String> = incoming
            .into_iter()
            .filter(|(_, d)| !d.is_empty())
            .map(|(n, _)| n)
            .collect();
        return Err(PluginManagerError::CyclicDependency { involved });
    }
    Ok(result)
}

/// 返回 `plugin_name` 的反向依赖：其他 manifest 中非 optional 依赖包含该插件的清单。
///
/// disable 时用于判断是否有依赖方仍 enabled，进而决定是拒绝还是级联禁用。
/// 结果按 `name` 字典序排列。
#[must_use]
pub fn reverse_dependencies(plugin_name: &str, manifests: &[PluginManifest]) -> Vec<String> {
    let mut result: Vec<String> = manifests
        .iter()
        .filter(|m| {
            m.dependencies
                .get(plugin_name)
                .is_some_and(|spec| !spec.optional)
        })
        .map(|m| m.plugin.name.clone())
        .collect();
    result.sort();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::PluginManifest;

    fn manifest_toml(name: &str, version: &str, deps: &[(&str, &str, bool)]) -> String {
        use std::fmt::Write as _;

        let mut text = format!(
            r#"
[plugin]
name = "{name}"
version = "{version}"
kind = "native"
entry = "x"

[compatibility]
cycms = ">=0.1.0"
"#
        );
        if !deps.is_empty() {
            text.push_str("\n[dependencies]\n");
            for (dep, ver, opt) in deps {
                writeln!(
                    text,
                    "{dep} = {{ version = \"{ver}\", optional = {opt} }}"
                )
                .expect("write to String never fails");
            }
        }
        text
    }

    fn parse(name: &str, version: &str, deps: &[(&str, &str, bool)]) -> PluginManifest {
        PluginManifest::from_toml_str(&manifest_toml(name, version, deps)).unwrap()
    }

    #[test]
    fn compatible_host_passes() {
        let m = parse("blog", "0.1.0", &[]);
        check_host_compatibility(&m, &Version::parse("0.1.5").unwrap()).unwrap();
    }

    #[test]
    fn incompatible_host_detected() {
        let text = r#"
[plugin]
name = "blog"
version = "0.1.0"
kind = "native"
entry = "x"

[compatibility]
cycms = ">=0.2.0, <0.3.0"
"#;
        let m = PluginManifest::from_toml_str(text).unwrap();
        let err = check_host_compatibility(&m, &Version::parse("0.1.5").unwrap()).unwrap_err();
        assert!(matches!(err, PluginManagerError::IncompatibleHost { .. }));
    }

    #[test]
    fn standalone_list_is_alphabetical() {
        let ms = vec![
            parse("zeta", "0.1.0", &[]),
            parse("alpha", "0.1.0", &[]),
            parse("mu", "0.1.0", &[]),
        ];
        assert_eq!(topological_order(&ms).unwrap(), vec!["alpha", "mu", "zeta"]);
    }

    #[test]
    fn dependency_order_is_respected() {
        let ms = vec![
            parse("app", "0.1.0", &[("auth", "^0.1", false), ("billing", "^0.2", false)]),
            parse("auth", "0.1.0", &[]),
            parse("billing", "0.2.0", &[("auth", "^0.1", false)]),
        ];
        let order = topological_order(&ms).unwrap();
        let pos = |name: &str| order.iter().position(|n| n == name).unwrap();
        assert!(pos("auth") < pos("billing"));
        assert!(pos("billing") < pos("app"));
    }

    #[test]
    fn missing_dependency_errors() {
        let ms = vec![parse("app", "0.1.0", &[("auth", "^0.1", false)])];
        assert!(matches!(
            topological_order(&ms).unwrap_err(),
            PluginManagerError::MissingDependency { .. }
        ));
    }

    #[test]
    fn optional_missing_dependency_is_skipped() {
        let ms = vec![parse("app", "0.1.0", &[("auth", "^0.1", true)])];
        assert_eq!(topological_order(&ms).unwrap(), vec!["app"]);
    }

    #[test]
    fn version_mismatch_errors() {
        let ms = vec![
            parse("app", "0.1.0", &[("auth", "^0.2", false)]),
            parse("auth", "0.1.0", &[]),
        ];
        assert!(matches!(
            topological_order(&ms).unwrap_err(),
            PluginManagerError::IncompatibleDependency { .. }
        ));
    }

    #[test]
    fn optional_version_mismatch_is_skipped() {
        let ms = vec![
            parse("app", "0.1.0", &[("auth", "^0.2", true)]),
            parse("auth", "0.1.0", &[]),
        ];
        // 依赖被跳过，两者之间无边，按字典序出队：app < auth
        assert_eq!(topological_order(&ms).unwrap(), vec!["app", "auth"]);
    }

    #[test]
    fn cyclic_dependency_errors() {
        let ms = vec![
            parse("a", "0.1.0", &[("b", "^0.1", false)]),
            parse("b", "0.1.0", &[("a", "^0.1", false)]),
        ];
        assert!(matches!(
            topological_order(&ms).unwrap_err(),
            PluginManagerError::CyclicDependency { .. }
        ));
    }

    #[test]
    fn reverse_dependencies_lists_non_optional_dependents() {
        let ms = vec![
            parse("core", "0.1.0", &[]),
            parse("app", "0.1.0", &[("core", "^0.1", false)]),
            parse("optional_user", "0.1.0", &[("core", "^0.1", true)]),
            parse("unrelated", "0.1.0", &[]),
        ];
        assert_eq!(reverse_dependencies("core", &ms), vec!["app"]);
    }
}
