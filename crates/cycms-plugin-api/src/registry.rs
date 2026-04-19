use std::any::Any;
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, PoisonError, RwLock};

use tracing::warn;

use crate::error::RegistryError;

/// 服务分隔符，对应 Req 13.1 的 `{plugin_name}.{service_name}` 键格式。
const KEY_SEPARATOR: char = '.';

/// 进程内服务注册表 / 发现门面。
///
/// 存储结构：
/// - `services`：完整 key → 已擦除类型的 `Arc<dyn Any + Send + Sync>`
/// - `availability`：完整 key → 是否可用（默认注册即 `true`，插件禁用时可批量置 `false`）
///
/// 两张表用独立的 `RwLock` 保护；运行时插件切换 availability 不需要重新注册 service。
#[derive(Default)]
pub struct ServiceRegistry {
    services: RwLock<HashMap<String, Arc<dyn Any + Send + Sync>>>,
    availability: RwLock<HashMap<String, bool>>,
}

impl ServiceRegistry {
    /// 构造一个空的注册表。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册服务实例（Req 13.1）。
    ///
    /// - 键必须形如 `{plugin_name}.{service_name}`，两段均非空。
    /// - 允许覆盖已有同名 key，重复注册记录 warning；新实例生效、availability 重置为 `true`。
    ///
    /// # Errors
    /// key 不符合两段式格式时返回 [`RegistryError::InvalidKey`]。
    pub fn register<T>(&self, key: &str, service: Arc<T>) -> Result<(), RegistryError>
    where
        T: Send + Sync + 'static,
    {
        let erased: Arc<dyn Any + Send + Sync> = service;
        self.register_erased(key, erased)
    }

    /// `register` 的类型已擦除版本：直接接收 `Arc<dyn Any + Send + Sync>`。
    ///
    /// 为 `NativePluginRuntime` / `WasmPluginRuntime` 批量注册 [`crate::Plugin::services`]
    /// 时使用。插件侧已把服务实例装进 `Arc<dyn Any + Send + Sync>`，此处直接透传即可，
    /// 无需二次类型还原。
    ///
    /// # Errors
    /// key 不符合两段式格式时返回 [`RegistryError::InvalidKey`]。
    pub fn register_erased(
        &self,
        key: &str,
        service: Arc<dyn Any + Send + Sync>,
    ) -> Result<(), RegistryError> {
        validate_full_key(key)?;

        let mut services = self
            .services
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        if services.contains_key(key) {
            warn!(service_key = %key, "service re-registered, previous instance replaced");
        }
        services.insert(key.to_owned(), service);
        drop(services);

        let mut availability = self
            .availability
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        availability.insert(key.to_owned(), true);
        Ok(())
    }

    /// 按 key 查询类型化服务（Req 13.2 / 13.3）。
    ///
    /// # Errors
    /// - [`RegistryError::ServiceNotFound`]：key 未注册。
    /// - [`RegistryError::ServiceUnavailable`]：已注册但被标记为不可用（例如所属插件禁用）。
    /// - [`RegistryError::TypeMismatch`]：请求类型与注册时不一致。
    pub fn get<T>(&self, key: &str) -> Result<Arc<T>, RegistryError>
    where
        T: Send + Sync + 'static,
    {
        let availability = self
            .availability
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        // availability 表与 services 表一起维护：缺失 == 未注册
        let Some(&available) = availability.get(key) else {
            return Err(RegistryError::ServiceNotFound {
                key: key.to_owned(),
            });
        };
        if !available {
            return Err(RegistryError::ServiceUnavailable {
                key: key.to_owned(),
            });
        }
        drop(availability);

        let services = self.services.read().unwrap_or_else(PoisonError::into_inner);
        let Some(any_arc) = services.get(key).cloned() else {
            return Err(RegistryError::ServiceNotFound {
                key: key.to_owned(),
            });
        };
        drop(services);

        any_arc
            .downcast::<T>()
            .map_err(|_| RegistryError::TypeMismatch {
                key: key.to_owned(),
                expected: std::any::type_name::<T>(),
            })
    }

    /// 按插件前缀批量标记不可用（插件禁用时调用）。
    ///
    /// `prefix` 参数是插件名（不含 `.`）；内部匹配所有以 `"{prefix}."` 开头的 key。
    /// 返回受影响的 key 数量。
    pub fn set_unavailable(&self, prefix: &str) -> usize {
        self.set_prefix_availability(prefix, false)
    }

    /// 按插件前缀批量标记可用（插件启用时调用）。返回受影响的 key 数量。
    pub fn set_available(&self, prefix: &str) -> usize {
        self.set_prefix_availability(prefix, true)
    }

    /// 注销单个服务：同时移除 services / availability 中对应条目。
    ///
    /// 返回 `true` 表示原先存在并已移除，`false` 表示 key 未注册。
    pub fn unregister(&self, key: &str) -> bool {
        let mut services = self
            .services
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        let removed = services.remove(key).is_some();
        drop(services);

        let mut availability = self
            .availability
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        availability.remove(key);
        removed
    }

    /// 按插件前缀枚举所有 key（字典序），承载 tasks.md 中 `resolve_all` 的语义。
    ///
    /// `prefix` 为插件名（不含 `.`）；返回所有以 `"{prefix}."` 开头的完整 key。
    #[must_use]
    pub fn list_by_prefix(&self, prefix: &str) -> Vec<String> {
        let needle = format!("{prefix}{KEY_SEPARATOR}");
        let services = self.services.read().unwrap_or_else(PoisonError::into_inner);
        let mut sorted: BTreeSet<String> = BTreeSet::new();
        for key in services.keys() {
            if key.starts_with(&needle) {
                sorted.insert(key.clone());
            }
        }
        sorted.into_iter().collect()
    }

    /// 查询当前已注册的 key 总数（监控 / 测试用途）。
    #[must_use]
    pub fn len(&self) -> usize {
        self.services
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

    /// 注册表是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn set_prefix_availability(&self, prefix: &str, available: bool) -> usize {
        let needle = format!("{prefix}{KEY_SEPARATOR}");
        let mut availability = self
            .availability
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        let mut hits = 0usize;
        for (k, v) in availability.iter_mut() {
            if k.starts_with(&needle) {
                *v = available;
                hits += 1;
            }
        }
        hits
    }
}

/// 校验完整 key：必须形如 `{plugin_name}.{service_name}`，两段均非空且不再包含 `.`。
fn validate_full_key(key: &str) -> Result<(), RegistryError> {
    let mut parts = key.split(KEY_SEPARATOR);
    match (parts.next(), parts.next(), parts.next()) {
        (Some(p), Some(s), None) if !p.is_empty() && !s.is_empty() => Ok(()),
        _ => Err(RegistryError::InvalidKey(format!(
            "service key must be '{{plugin_name}}.{{service_name}}', got {key:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::{RegistryError, ServiceRegistry};
    use std::sync::Arc;

    #[derive(Debug)]
    struct DummyAuth {
        label: &'static str,
    }

    #[derive(Debug)]
    struct DummyLogger;

    #[test]
    fn register_and_get_roundtrip() {
        let registry = ServiceRegistry::new();
        let svc = Arc::new(DummyAuth { label: "a" });
        registry.register("system.auth", Arc::clone(&svc)).unwrap();

        let got: Arc<DummyAuth> = registry.get("system.auth").unwrap();
        assert_eq!(got.label, "a");
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn get_returns_not_found_for_unknown_key() {
        let registry = ServiceRegistry::new();
        let err = registry.get::<DummyLogger>("system.missing").unwrap_err();
        assert!(matches!(err, RegistryError::ServiceNotFound { .. }));
    }

    #[test]
    fn get_returns_type_mismatch_on_wrong_type() {
        let registry = ServiceRegistry::new();
        registry
            .register("system.auth", Arc::new(DummyAuth { label: "a" }))
            .unwrap();
        let err = registry.get::<DummyLogger>("system.auth").unwrap_err();
        assert!(matches!(err, RegistryError::TypeMismatch { .. }));
    }

    #[test]
    fn register_rejects_invalid_key() {
        let registry = ServiceRegistry::new();
        for bad in ["", "single", ".leading", "trailing.", "a.b.c", "a..b"] {
            let err = registry
                .register(bad, Arc::new(DummyLogger))
                .expect_err(&format!("expected InvalidKey for {bad:?}"));
            assert!(matches!(err, RegistryError::InvalidKey(_)));
        }
    }

    #[test]
    fn set_unavailable_and_set_available_are_prefix_scoped() {
        let registry = ServiceRegistry::new();
        registry
            .register("blog.render", Arc::new(DummyLogger))
            .unwrap();
        registry
            .register("blog.feed", Arc::new(DummyLogger))
            .unwrap();
        registry
            .register("auth.session", Arc::new(DummyLogger))
            .unwrap();

        assert_eq!(registry.set_unavailable("blog"), 2);

        let err = registry.get::<DummyLogger>("blog.render").unwrap_err();
        assert!(matches!(err, RegistryError::ServiceUnavailable { .. }));
        // auth 命名空间不受影响
        registry.get::<DummyLogger>("auth.session").unwrap();

        assert_eq!(registry.set_available("blog"), 2);
        registry.get::<DummyLogger>("blog.render").unwrap();
    }

    #[test]
    fn unregister_removes_entry() {
        let registry = ServiceRegistry::new();
        registry
            .register("blog.render", Arc::new(DummyLogger))
            .unwrap();
        assert!(registry.unregister("blog.render"));
        assert!(!registry.unregister("blog.render"));
        assert!(registry.get::<DummyLogger>("blog.render").is_err());
    }

    #[test]
    fn list_by_prefix_returns_sorted_matches() {
        let registry = ServiceRegistry::new();
        registry
            .register("zeta.one", Arc::new(DummyLogger))
            .unwrap();
        registry
            .register("blog.render", Arc::new(DummyLogger))
            .unwrap();
        registry
            .register("blog.feed", Arc::new(DummyLogger))
            .unwrap();

        let keys = registry.list_by_prefix("blog");
        assert_eq!(keys, vec!["blog.feed", "blog.render"]);
        assert!(registry.list_by_prefix("missing").is_empty());
    }

    #[test]
    fn reregister_replaces_instance_and_resets_availability() {
        let registry = ServiceRegistry::new();
        registry
            .register("blog.render", Arc::new(DummyAuth { label: "v1" }))
            .unwrap();
        assert_eq!(registry.set_unavailable("blog"), 1);

        registry
            .register("blog.render", Arc::new(DummyAuth { label: "v2" }))
            .unwrap();
        let got: Arc<DummyAuth> = registry.get("blog.render").unwrap();
        assert_eq!(got.label, "v2");
    }
}
