//! 字段值与字段定义的校验引擎。
//!
//! 语义：
//! - [`validate_field`]：对单个字段的 `value` 做 required / 类型形状 / 规则链校验，
//!   返回 `Vec<FieldViolation>`（可能为空）。
//! - [`validate_fields`]：以 `entry`（JSON 对象）为单位执行所有字段校验，聚合成
//!   [`ContentModelError::SchemaViolation`]。
//! - [`validate_field_definitions`]：检查 [`FieldDefinition`] 列表本身的一致性
//!   （`api_id` 唯一、Relation 目标非空、Custom 类型已注册、规则与字段类型匹配）。
//!
//! `Regex` 规则运行时通过进程级 `LazyLock<RwLock<HashMap>>` 缓存已编译模式，避免
//! 每次 validate 重复编译同一模式。

use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, PoisonError, RwLock};

use regex::Regex;
use serde_json::Value;

use crate::error::{ContentModelError, FieldViolation};
use crate::field_type::FieldTypeRegistry;
use crate::model::{FieldDefinition, FieldType, ValidationRule};

/// 对单字段执行校验，返回 0..n 条违规；空表示通过。
#[must_use]
pub fn validate_field(
    field: &FieldDefinition,
    value: &Value,
    registry: &FieldTypeRegistry,
) -> Vec<FieldViolation> {
    let mut violations = Vec::new();

    if value.is_null() {
        if field.required {
            violations.push(FieldViolation {
                field: field.api_id.clone(),
                rule: "required",
                message: "field is required".to_owned(),
            });
        }
        return violations;
    }

    if let Some(v) = validate_type_shape(&field.api_id, &field.field_type, value, registry) {
        violations.extend(v);
        return violations;
    }

    for rule in &field.validations {
        if let Some(v) = apply_rule(&field.api_id, &field.field_type, rule, value, registry) {
            violations.push(v);
        }
    }

    violations
}

/// 以 entry（期望 JSON Object）为单位对所有字段执行校验。
///
/// # Errors
/// - `entry` 非 JSON object → [`ContentModelError::InputValidation`]
/// - 任一字段或规则失败 → [`ContentModelError::SchemaViolation`]（聚合所有违规）
pub fn validate_fields(
    fields: &[FieldDefinition],
    entry: &Value,
    registry: &FieldTypeRegistry,
) -> std::result::Result<(), ContentModelError> {
    let Some(obj) = entry.as_object() else {
        return Err(ContentModelError::InputValidation(
            "entry payload must be a JSON object".to_owned(),
        ));
    };

    let mut all = Vec::<FieldViolation>::new();
    for field in fields {
        let value = obj.get(&field.api_id).unwrap_or(&Value::Null);
        all.extend(validate_field(field, value, registry));
    }

    if all.is_empty() {
        Ok(())
    } else {
        Err(ContentModelError::SchemaViolation { errors: all })
    }
}

/// 对字段定义列表本身做结构一致性校验。
///
/// # Errors
/// - `api_id` 重复 → [`ContentModelError::InvalidField`]
/// - Relation 目标为空 → [`ContentModelError::InvalidField`]
/// - Custom 类型未在注册表中找到 → [`ContentModelError::InvalidField`]
/// - 校验规则与字段类型不匹配（如 `Number` 配 `MinLength`） → [`ContentModelError::InvalidField`]
pub fn validate_field_definitions(
    fields: &[FieldDefinition],
    registry: &FieldTypeRegistry,
) -> std::result::Result<(), ContentModelError> {
    let mut seen = HashSet::<&str>::new();
    for f in fields {
        if f.api_id.trim().is_empty() {
            return Err(ContentModelError::InvalidField(
                "field.api_id must not be empty".to_owned(),
            ));
        }
        if !seen.insert(f.api_id.as_str()) {
            return Err(ContentModelError::InvalidField(format!(
                "duplicate field api_id: {}",
                f.api_id
            )));
        }
        if f.name.trim().is_empty() {
            return Err(ContentModelError::InvalidField(format!(
                "field `{}`: name must not be empty",
                f.api_id
            )));
        }

        if let FieldType::Relation { target_type, .. } = &f.field_type
            && target_type.trim().is_empty()
        {
            return Err(ContentModelError::InvalidField(format!(
                "field `{}`: relation target_type must not be empty",
                f.api_id
            )));
        }
        if let FieldType::Custom { type_name } = &f.field_type
            && !registry.contains(type_name)
        {
            return Err(ContentModelError::InvalidField(format!(
                "field `{}`: custom field type `{}` not registered",
                f.api_id, type_name
            )));
        }

        for rule in &f.validations {
            if !rule_supports_type(rule, &f.field_type) {
                return Err(ContentModelError::InvalidField(format!(
                    "field `{}`: rule `{}` is not supported by field type `{}`",
                    f.api_id,
                    rule_name(rule),
                    field_type_name(&f.field_type)
                )));
            }
            if let ValidationRule::Regex { pattern } = rule
                && Regex::new(pattern).is_err()
            {
                return Err(ContentModelError::InvalidField(format!(
                    "field `{}`: invalid regex `{}`",
                    f.api_id, pattern
                )));
            }
            if let ValidationRule::Custom { validator } = rule
                && !registry.contains(validator)
            {
                return Err(ContentModelError::InvalidField(format!(
                    "field `{}`: custom validator `{}` not registered",
                    f.api_id, validator
                )));
            }
        }
    }
    Ok(())
}

fn validate_type_shape(
    api_id: &str,
    field_type: &FieldType,
    value: &Value,
    registry: &FieldTypeRegistry,
) -> Option<Vec<FieldViolation>> {
    let mk = |rule: &'static str, msg: String| FieldViolation {
        field: api_id.to_owned(),
        rule,
        message: msg,
    };

    match field_type {
        FieldType::Text | FieldType::RichText => {
            if !value.is_string() {
                return Some(vec![mk(
                    "type",
                    format!("expected string, got {}", json_kind(value)),
                )]);
            }
        }
        FieldType::Number { decimal } => {
            let Some(num) = value.as_number() else {
                return Some(vec![mk(
                    "type",
                    format!("expected number, got {}", json_kind(value)),
                )]);
            };
            if !*decimal && num.as_i64().is_none() && num.as_u64().is_none() {
                return Some(vec![mk(
                    "type",
                    "expected integer, got non-integer number".to_owned(),
                )]);
            }
        }
        FieldType::Boolean => {
            if !value.is_boolean() {
                return Some(vec![mk(
                    "type",
                    format!("expected boolean, got {}", json_kind(value)),
                )]);
            }
        }
        FieldType::DateTime => {
            let Some(s) = value.as_str() else {
                return Some(vec![mk(
                    "type",
                    format!("expected RFC3339 datetime string, got {}", json_kind(value)),
                )]);
            };
            if chrono::DateTime::parse_from_rfc3339(s).is_err() {
                return Some(vec![mk("type", format!("invalid RFC3339 datetime: {s}"))]);
            }
        }
        FieldType::Json => { /* any non-null value passes shape check */ }
        FieldType::Media { .. } => {
            if !value.is_string() {
                return Some(vec![mk(
                    "type",
                    format!("expected media reference string, got {}", json_kind(value)),
                )]);
            }
        }
        FieldType::Relation {
            relation_kind: crate::model::RelationKind::OneToOne,
            ..
        } => {
            if !value.is_string() {
                return Some(vec![mk(
                    "type",
                    format!("expected relation id string, got {}", json_kind(value)),
                )]);
            }
        }
        FieldType::Relation { .. } => {
            if !value.is_array() {
                return Some(vec![mk(
                    "type",
                    format!("expected array of relation ids, got {}", json_kind(value)),
                )]);
            }
            if let Some(arr) = value.as_array()
                && arr.iter().any(|v| !v.is_string())
            {
                return Some(vec![mk(
                    "type",
                    "relation array must contain only id strings".to_owned(),
                )]);
            }
        }
        FieldType::Custom { type_name } => {
            let Some(handler) = registry.get(type_name) else {
                return Some(vec![mk(
                    "type",
                    format!("custom field type `{type_name}` not registered"),
                )]);
            };
            if let Err(e) = handler.validate(value, &[]) {
                return Some(vec![mk("type", format!("custom validator failed: {e}"))]);
            }
        }
    }
    None
}

fn apply_rule(
    api_id: &str,
    field_type: &FieldType,
    rule: &ValidationRule,
    value: &Value,
    registry: &FieldTypeRegistry,
) -> Option<FieldViolation> {
    let violation = |r: &'static str, msg: String| FieldViolation {
        field: api_id.to_owned(),
        rule: r,
        message: msg,
    };

    match rule {
        ValidationRule::MinLength { value: min } => value
            .as_str()
            .filter(|s| s.chars().count() < *min)
            .map(|_| violation("min_length", format!("length must be >= {min}"))),
        ValidationRule::MaxLength { value: max } => value
            .as_str()
            .filter(|s| s.chars().count() > *max)
            .map(|_| violation("max_length", format!("length must be <= {max}"))),
        ValidationRule::Min { value: min } => value
            .as_f64()
            .filter(|n| *n < *min)
            .map(|_| violation("min", format!("value must be >= {min}"))),
        ValidationRule::Max { value: max } => value
            .as_f64()
            .filter(|n| *n > *max)
            .map(|_| violation("max", format!("value must be <= {max}"))),
        ValidationRule::Regex { pattern } => {
            let Some(s) = value.as_str() else {
                return Some(violation("regex", "value must be a string".to_owned()));
            };
            match get_or_compile_regex(pattern) {
                Ok(re) => {
                    if re.is_match(s) {
                        None
                    } else {
                        Some(violation(
                            "regex",
                            format!("value does not match pattern `{pattern}`"),
                        ))
                    }
                }
                Err(_) => Some(violation(
                    "regex",
                    format!("invalid regex pattern `{pattern}`"),
                )),
            }
        }
        ValidationRule::Enum { values } => {
            if values.iter().any(|v| v == value) {
                None
            } else {
                Some(violation(
                    "enum",
                    format!(
                        "value must be one of {}",
                        serde_json::to_string(values).unwrap_or_default()
                    ),
                ))
            }
        }
        ValidationRule::Custom { validator } => {
            let Some(handler) = registry.get(validator) else {
                return Some(violation(
                    "custom",
                    format!("custom validator `{validator}` not registered"),
                ));
            };
            match handler.validate(value, std::slice::from_ref(rule)) {
                Ok(()) => None,
                Err(e) => {
                    let _ = field_type;
                    Some(violation(
                        "custom",
                        format!("validator `{validator}` rejected value: {e}"),
                    ))
                }
            }
        }
    }
}

fn rule_supports_type(rule: &ValidationRule, field_type: &FieldType) -> bool {
    matches!(
        (rule, field_type),
        (
            ValidationRule::MinLength { .. }
                | ValidationRule::MaxLength { .. }
                | ValidationRule::Regex { .. },
            FieldType::Text | FieldType::RichText,
        ) | (
            ValidationRule::Min { .. } | ValidationRule::Max { .. },
            FieldType::Number { .. },
        ) | (
            ValidationRule::Enum { .. } | ValidationRule::Custom { .. },
            _,
        )
    )
}

fn rule_name(rule: &ValidationRule) -> &'static str {
    match rule {
        ValidationRule::MinLength { .. } => "min_length",
        ValidationRule::MaxLength { .. } => "max_length",
        ValidationRule::Min { .. } => "min",
        ValidationRule::Max { .. } => "max",
        ValidationRule::Regex { .. } => "regex",
        ValidationRule::Enum { .. } => "enum",
        ValidationRule::Custom { .. } => "custom",
    }
}

fn field_type_name(field_type: &FieldType) -> &'static str {
    match field_type {
        FieldType::Text => "text",
        FieldType::RichText => "rich_text",
        FieldType::Number { .. } => "number",
        FieldType::Boolean => "boolean",
        FieldType::DateTime => "date_time",
        FieldType::Json => "json",
        FieldType::Media { .. } => "media",
        FieldType::Relation { .. } => "relation",
        FieldType::Custom { .. } => "custom",
    }
}

fn json_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

static REGEX_CACHE: LazyLock<RwLock<HashMap<String, Regex>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

fn get_or_compile_regex(pattern: &str) -> std::result::Result<Regex, regex::Error> {
    {
        let cache = REGEX_CACHE.read().unwrap_or_else(PoisonError::into_inner);
        if let Some(re) = cache.get(pattern) {
            return Ok(re.clone());
        }
    }
    let re = Regex::new(pattern)?;
    let mut cache = REGEX_CACHE.write().unwrap_or_else(PoisonError::into_inner);
    cache
        .entry(pattern.to_owned())
        .or_insert_with(|| re.clone());
    Ok(re)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_type::FieldTypeHandler;
    use crate::model::{FieldDefinition, FieldType, RelationKind, ValidationRule};
    use serde_json::{Value, json};
    use std::sync::Arc;

    fn fd(api_id: &str, field_type: FieldType) -> FieldDefinition {
        FieldDefinition {
            name: api_id.to_owned(),
            api_id: api_id.to_owned(),
            field_type,
            required: false,
            unique: false,
            default_value: None,
            validations: vec![],
            position: 0,
        }
    }

    #[test]
    fn null_value_passes_when_not_required() {
        let f = fd("title", FieldType::Text);
        let r = FieldTypeRegistry::new();
        assert!(validate_field(&f, &Value::Null, &r).is_empty());
    }

    #[test]
    fn null_value_fails_when_required() {
        let mut f = fd("title", FieldType::Text);
        f.required = true;
        let r = FieldTypeRegistry::new();
        let v = validate_field(&f, &Value::Null, &r);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].rule, "required");
    }

    #[test]
    fn text_type_mismatch_fails() {
        let f = fd("title", FieldType::Text);
        let r = FieldTypeRegistry::new();
        let v = validate_field(&f, &json!(123), &r);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].rule, "type");
    }

    #[test]
    fn number_integer_variant_rejects_decimal() {
        let f = fd("count", FieldType::Number { decimal: false });
        let r = FieldTypeRegistry::new();
        assert!(validate_field(&f, &json!(42), &r).is_empty());
        let v = validate_field(&f, &json!(1.5), &r);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].rule, "type");
    }

    #[test]
    fn datetime_requires_rfc3339() {
        let f = fd("published_at", FieldType::DateTime);
        let r = FieldTypeRegistry::new();
        assert!(validate_field(&f, &json!("2026-04-19T12:00:00Z"), &r).is_empty());
        let v = validate_field(&f, &json!("not a date"), &r);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn relation_one_to_many_requires_array_of_strings() {
        let f = fd(
            "tags",
            FieldType::Relation {
                target_type: "tag".into(),
                relation_kind: RelationKind::OneToMany,
            },
        );
        let r = FieldTypeRegistry::new();
        assert!(validate_field(&f, &json!(["a", "b"]), &r).is_empty());
        assert_eq!(validate_field(&f, &json!("x"), &r).len(), 1);
        assert_eq!(validate_field(&f, &json!([1, 2]), &r).len(), 1);
    }

    #[test]
    fn min_max_length_enforced_on_strings() {
        let mut f = fd("title", FieldType::Text);
        f.validations = vec![
            ValidationRule::MinLength { value: 3 },
            ValidationRule::MaxLength { value: 5 },
        ];
        let r = FieldTypeRegistry::new();
        assert!(validate_field(&f, &json!("ok!"), &r).is_empty());

        let short = validate_field(&f, &json!("a"), &r);
        assert_eq!(short.len(), 1);
        assert_eq!(short[0].rule, "min_length");

        let long = validate_field(&f, &json!("abcdef"), &r);
        assert_eq!(long.len(), 1);
        assert_eq!(long[0].rule, "max_length");
    }

    #[test]
    fn regex_rule_uses_cached_pattern() {
        let mut f = fd("slug", FieldType::Text);
        f.validations = vec![ValidationRule::Regex {
            pattern: "^[a-z0-9-]+$".into(),
        }];
        let r = FieldTypeRegistry::new();
        assert!(validate_field(&f, &json!("hello-world"), &r).is_empty());
        // second call hits cache, still works
        assert!(validate_field(&f, &json!("abc-123"), &r).is_empty());
        assert_eq!(validate_field(&f, &json!("Bad!"), &r)[0].rule, "regex");
    }

    #[test]
    fn enum_rule_requires_membership() {
        let mut f = fd("status", FieldType::Text);
        f.validations = vec![ValidationRule::Enum {
            values: vec![json!("draft"), json!("published")],
        }];
        let r = FieldTypeRegistry::new();
        assert!(validate_field(&f, &json!("draft"), &r).is_empty());
        assert_eq!(validate_field(&f, &json!("other"), &r)[0].rule, "enum");
    }

    #[test]
    fn custom_rule_dispatches_to_registry() {
        struct AlwaysFail;
        impl FieldTypeHandler for AlwaysFail {
            fn validate(
                &self,
                _value: &Value,
                _rules: &[ValidationRule],
            ) -> std::result::Result<(), ContentModelError> {
                Err(ContentModelError::InvalidField("nope".to_owned()))
            }
            fn to_openapi_schema(&self) -> Value {
                json!({})
            }
        }

        let registry = FieldTypeRegistry::new();
        registry.register("plugin.zip", Arc::new(AlwaysFail));

        let mut f = fd("postcode", FieldType::Text);
        f.validations = vec![ValidationRule::Custom {
            validator: "plugin.zip".into(),
        }];
        let v = validate_field(&f, &json!("12345"), &registry);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].rule, "custom");
    }

    #[test]
    fn custom_rule_missing_handler_violates() {
        let mut f = fd("x", FieldType::Text);
        f.validations = vec![ValidationRule::Custom {
            validator: "plugin.none".into(),
        }];
        let r = FieldTypeRegistry::new();
        let v = validate_field(&f, &json!("x"), &r);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].rule, "custom");
    }

    #[test]
    fn validate_fields_aggregates_into_schema_violation() {
        let fields = vec![
            {
                let mut f = fd("title", FieldType::Text);
                f.required = true;
                f
            },
            fd("count", FieldType::Number { decimal: false }),
        ];
        let r = FieldTypeRegistry::new();

        let entry = json!({ "count": "not a number" });
        let err = validate_fields(&fields, &entry, &r).unwrap_err();
        match err {
            ContentModelError::SchemaViolation { errors } => {
                assert_eq!(errors.len(), 2);
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn validate_fields_rejects_non_object_entry() {
        let fields: Vec<FieldDefinition> = vec![];
        let r = FieldTypeRegistry::new();
        let err = validate_fields(&fields, &json!([]), &r).unwrap_err();
        assert!(matches!(err, ContentModelError::InputValidation(_)));
    }

    #[test]
    fn validate_field_definitions_rejects_duplicate_api_id() {
        let fields = vec![fd("x", FieldType::Text), fd("x", FieldType::Text)];
        let r = FieldTypeRegistry::new();
        assert!(validate_field_definitions(&fields, &r).is_err());
    }

    #[test]
    fn validate_field_definitions_rejects_invalid_regex() {
        let mut f = fd("slug", FieldType::Text);
        f.validations = vec![ValidationRule::Regex {
            pattern: "[".into(),
        }];
        let r = FieldTypeRegistry::new();
        assert!(validate_field_definitions(&[f], &r).is_err());
    }

    #[test]
    fn validate_field_definitions_rejects_wrong_rule_for_type() {
        let mut f = fd("count", FieldType::Number { decimal: false });
        f.validations = vec![ValidationRule::MinLength { value: 1 }];
        let r = FieldTypeRegistry::new();
        assert!(validate_field_definitions(&[f], &r).is_err());
    }

    #[test]
    fn validate_field_definitions_requires_custom_registered() {
        let f = fd(
            "point",
            FieldType::Custom {
                type_name: "geo.latlng".into(),
            },
        );
        let r = FieldTypeRegistry::new();
        assert!(validate_field_definitions(&[f], &r).is_err());
    }
}
