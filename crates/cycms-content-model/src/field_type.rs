//! 字段类型插件扩展的进程内注册表（Req 3.6）。
//!
//! 内置字段类型（[`crate::model::FieldType`] 的前八个变体）不入注册表，由
//! [`crate::validation`] 直接静态分派；本注册表只承载 [`crate::model::FieldType::Custom`]
//! 对应的 `type_name` 以及 [`crate::model::ValidationRule::Custom`] 对应的
//! `validator`（`FieldTypeHandler` 既负责类型校验也负责规则分派）。
//!
//! 并发模型仿 `ServiceRegistry`：单张 `RwLock<HashMap>`，register 持写锁、get 持读锁。

use std::collections::HashMap;
use std::sync::{Arc, PoisonError, RwLock};

use serde_json::Value;

use crate::error::ContentModelError;
use crate::model::ValidationRule;

/// 插件可实现的字段类型 / 校验器处理器。
///
/// 一个 `FieldTypeHandler` 可以同时承担"自定义字段类型"和"自定义校验器"两种角色：
/// - `validate` 被 [`crate::validation::validate_field`] 在命中 `FieldType::Custom` 或
///   `ValidationRule::Custom` 时调用；`rules` 传入的是当前字段定义上匹配本 handler 的
///   所有规则（类型分派的 handler 收全量 rules，规则分派的 handler 只收自己那一条）。
/// - `to_openapi_schema` 供 [`crate::schema`] 生成 `OpenAPI` 片段，调用方可再叠加通用规则。
/// - `default_value` 用于生成新实例时的默认字段值，可返回 `None` 表示无默认。
pub trait FieldTypeHandler: Send + Sync {
    /// 校验 `value` 是否满足当前类型 + `rules`。
    ///
    /// # Errors
    /// 校验失败时返回 [`ContentModelError::InvalidField`] 或
    /// [`ContentModelError::SchemaViolation`]。
    fn validate(
        &self,
        value: &Value,
        rules: &[ValidationRule],
    ) -> std::result::Result<(), ContentModelError>;

    fn to_openapi_schema(&self) -> Value;

    fn default_value(&self) -> Option<Value> {
        None
    }
}

/// 插件字段类型 / 校验器注册表。线程安全，跨 crate 边界以 `Arc<FieldTypeRegistry>` 传递。
#[derive(Default)]
pub struct FieldTypeRegistry {
    handlers: RwLock<HashMap<String, Arc<dyn FieldTypeHandler>>>,
}

impl FieldTypeRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册一个处理器，key 形如 `"{plugin_name}.{type_name}"` 或 `"{plugin_name}.{validator}"`。
    ///
    /// 覆盖同名 key 时不报错（日志层面视情况 warn 交给调用方）。
    pub fn register(&self, key: &str, handler: Arc<dyn FieldTypeHandler>) {
        let mut map = self
            .handlers
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        map.insert(key.to_owned(), handler);
    }

    /// 查询处理器。
    #[must_use]
    pub fn get(&self, key: &str) -> Option<Arc<dyn FieldTypeHandler>> {
        let map = self.handlers.read().unwrap_or_else(PoisonError::into_inner);
        map.get(key).cloned()
    }

    /// 判断处理器是否存在。
    #[must_use]
    pub fn contains(&self, key: &str) -> bool {
        let map = self.handlers.read().unwrap_or_else(PoisonError::into_inner);
        map.contains_key(key)
    }

    /// 列出所有注册的 key，字典序。
    #[must_use]
    pub fn list_names(&self) -> Vec<String> {
        let map = self.handlers.read().unwrap_or_else(PoisonError::into_inner);
        let mut names: Vec<String> = map.keys().cloned().collect();
        names.sort();
        names
    }

    /// 精确移除一个 key，返回是否成功移除。
    pub fn unregister(&self, key: &str) -> bool {
        let mut map = self
            .handlers
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        map.remove(key).is_some()
    }

    /// 按插件名前缀批量卸载（匹配 `"{prefix}."` 起始的所有 key），返回被移除的数量。
    ///
    /// 供 `PluginManager` 在插件卸载 / 禁用时调用。
    pub fn unregister_by_prefix(&self, prefix: &str) -> usize {
        let needle = format!("{prefix}.");
        let mut map = self
            .handlers
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        let doomed: Vec<String> = map
            .keys()
            .filter(|k| k.starts_with(&needle))
            .cloned()
            .collect();
        for k in &doomed {
            map.remove(k);
        }
        doomed.len()
    }
}

#[cfg(test)]
mod tests {
    use super::{FieldTypeHandler, FieldTypeRegistry};
    use crate::error::ContentModelError;
    use crate::model::ValidationRule;
    use serde_json::{Value, json};
    use std::sync::Arc;

    struct AlwaysOk;
    impl FieldTypeHandler for AlwaysOk {
        fn validate(
            &self,
            _value: &Value,
            _rules: &[ValidationRule],
        ) -> std::result::Result<(), ContentModelError> {
            Ok(())
        }
        fn to_openapi_schema(&self) -> Value {
            json!({ "type": "string" })
        }
    }

    #[test]
    fn register_and_get_roundtrip() {
        let registry = FieldTypeRegistry::new();
        registry.register("blog.markdown", Arc::new(AlwaysOk));
        assert!(registry.contains("blog.markdown"));
        assert!(registry.get("blog.markdown").is_some());
    }

    #[test]
    fn list_names_sorted() {
        let registry = FieldTypeRegistry::new();
        registry.register("zeta.one", Arc::new(AlwaysOk));
        registry.register("alpha.two", Arc::new(AlwaysOk));
        assert_eq!(
            registry.list_names(),
            vec!["alpha.two".to_owned(), "zeta.one".to_owned()]
        );
    }

    #[test]
    fn unregister_removes_entry() {
        let registry = FieldTypeRegistry::new();
        registry.register("p.x", Arc::new(AlwaysOk));
        assert!(registry.unregister("p.x"));
        assert!(!registry.unregister("p.x"));
    }

    #[test]
    fn unregister_by_prefix_is_scoped() {
        let registry = FieldTypeRegistry::new();
        registry.register("blog.md", Arc::new(AlwaysOk));
        registry.register("blog.code", Arc::new(AlwaysOk));
        registry.register("auth.totp", Arc::new(AlwaysOk));

        assert_eq!(registry.unregister_by_prefix("blog"), 2);
        assert!(!registry.contains("blog.md"));
        assert!(registry.contains("auth.totp"));
    }
}
