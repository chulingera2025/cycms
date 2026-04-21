//! 从 [`ContentTypeDefinition`] 生成 `OpenAPI` / JSON Schema 片段。
//!
//! 输出结构：
//! ```json
//! {
//!   "type": "object",
//!   "properties": { "<field.api_id>": { ... } },
//!   "required": ["<required field.api_id>", ...]
//! }
//! ```
//!
//! 各字段 schema 由 [`field_to_schema`] 根据 [`FieldType`] 生成基础形状，再叠加
//! [`ValidationRule`] 对应的 JSON Schema 关键字（`minLength` / `maxLength` / `minimum` /
//! `maximum` / `pattern` / `enum`）。`Custom` 字段类型通过 [`FieldTypeRegistry`] 委托
//! 到插件 handler；若未注册，仅标记 `x-cycms-custom-type` 供后端日志排查。
//!
//! 该模块生成的片段会直接注入 `/api/docs` 的动态内容类型 CRUD 请求 / 响应 schema。

use serde_json::{Map, Value, json};

use crate::field_type::FieldTypeRegistry;
use crate::model::{
    ContentTypeDefinition, FieldDefinition, FieldType, RelationKind, ValidationRule,
};

/// 由 [`ContentTypeDefinition`] 生成完整的 `OpenAPI` / JSON Schema 片段。
#[must_use]
pub fn to_json_schema(ct: &ContentTypeDefinition, registry: &FieldTypeRegistry) -> Value {
    let mut properties = Map::<String, Value>::new();
    let mut required = Vec::<Value>::new();

    for field in &ct.fields {
        properties.insert(field.api_id.clone(), field_to_schema(field, registry));
        if field.required {
            required.push(Value::String(field.api_id.clone()));
        }
    }

    let mut root = Map::<String, Value>::new();
    root.insert("type".into(), json!("object"));
    root.insert("properties".into(), Value::Object(properties));
    if !required.is_empty() {
        root.insert("required".into(), Value::Array(required));
    }
    root.insert("additionalProperties".into(), Value::Bool(false));
    Value::Object(root)
}

/// 单字段 schema：基础类型形状 + 所有 [`ValidationRule`] 的 JSON Schema 关键字。
#[must_use]
pub fn field_to_schema(field: &FieldDefinition, registry: &FieldTypeRegistry) -> Value {
    let mut schema = base_field_schema(&field.field_type, registry);
    if let Value::Object(ref mut map) = schema {
        for rule in &field.validations {
            apply_rule_to_schema(map, rule);
        }
    }
    schema
}

fn base_field_schema(field_type: &FieldType, registry: &FieldTypeRegistry) -> Value {
    match field_type {
        FieldType::Text | FieldType::RichText => json!({ "type": "string" }),
        FieldType::Number { decimal } => {
            if *decimal {
                json!({ "type": "number" })
            } else {
                json!({ "type": "integer" })
            }
        }
        FieldType::Boolean => json!({ "type": "boolean" }),
        FieldType::DateTime => json!({ "type": "string", "format": "date-time" }),
        FieldType::Json => json!({}),
        FieldType::Media { allowed_types } => {
            let mut m = json!({ "type": "string", "format": "uuid" });
            if !allowed_types.is_empty()
                && let Value::Object(ref mut obj) = m
            {
                obj.insert(
                    "x-cycms-media-allowed-types".into(),
                    Value::Array(
                        allowed_types
                            .iter()
                            .map(|t| Value::String(t.clone()))
                            .collect(),
                    ),
                );
            }
            m
        }
        FieldType::Relation {
            target_type,
            relation_kind,
        } => match relation_kind {
            RelationKind::OneToOne => json!({
                "type": "string",
                "format": "uuid",
                "x-cycms-relation-target": target_type,
                "x-cycms-relation-kind": "one_to_one",
            }),
            RelationKind::OneToMany => json!({
                "type": "array",
                "items": { "type": "string", "format": "uuid" },
                "x-cycms-relation-target": target_type,
                "x-cycms-relation-kind": "one_to_many",
            }),
            RelationKind::ManyToMany => json!({
                "type": "array",
                "items": { "type": "string", "format": "uuid" },
                "x-cycms-relation-target": target_type,
                "x-cycms-relation-kind": "many_to_many",
            }),
        },
        FieldType::Custom { type_name } => match registry.get(type_name) {
            Some(handler) => {
                let mut s = handler.to_openapi_schema();
                if let Value::Object(ref mut obj) = s {
                    obj.insert("x-cycms-custom-type".into(), json!(type_name));
                }
                s
            }
            None => json!({ "x-cycms-custom-type": type_name }),
        },
    }
}

fn apply_rule_to_schema(map: &mut Map<String, Value>, rule: &ValidationRule) {
    match rule {
        ValidationRule::MinLength { value } => {
            map.insert("minLength".into(), json!(value));
        }
        ValidationRule::MaxLength { value } => {
            map.insert("maxLength".into(), json!(value));
        }
        ValidationRule::Min { value } => {
            map.insert("minimum".into(), json!(value));
        }
        ValidationRule::Max { value } => {
            map.insert("maximum".into(), json!(value));
        }
        ValidationRule::Regex { pattern } => {
            map.insert("pattern".into(), json!(pattern));
        }
        ValidationRule::Enum { values } => {
            map.insert("enum".into(), Value::Array(values.clone()));
        }
        ValidationRule::Custom { validator } => {
            let slot = map
                .entry("x-cycms-validators".to_owned())
                .or_insert_with(|| Value::Array(Vec::new()));
            if let Value::Array(arr) = slot {
                arr.push(json!(validator));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_type::{FieldTypeHandler, FieldTypeRegistry};
    use crate::model::{
        ContentTypeDefinition, ContentTypeKind, FieldDefinition, FieldType, RelationKind,
        ValidationRule,
    };
    use chrono::Utc;
    use serde_json::{Value, json};
    use std::sync::Arc;

    #[allow(clippy::too_many_lines)]
    fn sample_ct() -> ContentTypeDefinition {
        ContentTypeDefinition {
            id: "00000000-0000-0000-0000-000000000000".into(),
            name: "Article".into(),
            api_id: "article".into(),
            description: None,
            kind: ContentTypeKind::Collection,
            fields: vec![
                FieldDefinition {
                    name: "Title".into(),
                    api_id: "title".into(),
                    field_type: FieldType::Text,
                    required: true,
                    unique: false,
                    default_value: None,
                    validations: vec![
                        ValidationRule::MinLength { value: 1 },
                        ValidationRule::MaxLength { value: 255 },
                    ],
                    position: 0,
                },
                FieldDefinition {
                    name: "Count".into(),
                    api_id: "count".into(),
                    field_type: FieldType::Number { decimal: false },
                    required: false,
                    unique: false,
                    default_value: None,
                    validations: vec![ValidationRule::Min { value: 0.0 }],
                    position: 1,
                },
                FieldDefinition {
                    name: "Status".into(),
                    api_id: "status".into(),
                    field_type: FieldType::Text,
                    required: true,
                    unique: false,
                    default_value: None,
                    validations: vec![ValidationRule::Enum {
                        values: vec![json!("draft"), json!("published")],
                    }],
                    position: 2,
                },
                FieldDefinition {
                    name: "Author".into(),
                    api_id: "author".into(),
                    field_type: FieldType::Relation {
                        target_type: "user".into(),
                        relation_kind: RelationKind::OneToOne,
                    },
                    required: true,
                    unique: false,
                    default_value: None,
                    validations: vec![],
                    position: 3,
                },
                FieldDefinition {
                    name: "Tags".into(),
                    api_id: "tags".into(),
                    field_type: FieldType::Relation {
                        target_type: "tag".into(),
                        relation_kind: RelationKind::OneToMany,
                    },
                    required: false,
                    unique: false,
                    default_value: None,
                    validations: vec![],
                    position: 4,
                },
                FieldDefinition {
                    name: "Cover".into(),
                    api_id: "cover".into(),
                    field_type: FieldType::Media {
                        allowed_types: vec!["image/png".into(), "image/jpeg".into()],
                    },
                    required: false,
                    unique: false,
                    default_value: None,
                    validations: vec![],
                    position: 5,
                },
                FieldDefinition {
                    name: "PublishedAt".into(),
                    api_id: "published_at".into(),
                    field_type: FieldType::DateTime,
                    required: false,
                    unique: false,
                    default_value: None,
                    validations: vec![],
                    position: 6,
                },
                FieldDefinition {
                    name: "Profile".into(),
                    api_id: "profile".into(),
                    field_type: FieldType::Json,
                    required: false,
                    unique: false,
                    default_value: None,
                    validations: vec![ValidationRule::Custom {
                        validator: "plugin.profile".into(),
                    }],
                    position: 7,
                },
                FieldDefinition {
                    name: "Point".into(),
                    api_id: "point".into(),
                    field_type: FieldType::Custom {
                        type_name: "geo.latlng".into(),
                    },
                    required: false,
                    unique: false,
                    default_value: None,
                    validations: vec![],
                    position: 8,
                },
            ],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    struct LatLngHandler;
    impl FieldTypeHandler for LatLngHandler {
        fn validate(
            &self,
            _value: &Value,
            _rules: &[ValidationRule],
        ) -> std::result::Result<(), crate::error::ContentModelError> {
            Ok(())
        }
        fn to_openapi_schema(&self) -> Value {
            json!({
                "type": "object",
                "properties": {
                    "lat": { "type": "number" },
                    "lng": { "type": "number" }
                },
                "required": ["lat", "lng"]
            })
        }
    }

    #[test]
    fn generates_expected_schema_snapshot() {
        let registry = FieldTypeRegistry::new();
        registry.register("geo.latlng", Arc::new(LatLngHandler));
        let schema = to_json_schema(&sample_ct(), &registry);

        let expected = json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "title": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 255
                },
                "count": {
                    "type": "integer",
                    "minimum": 0.0
                },
                "status": {
                    "type": "string",
                    "enum": ["draft", "published"]
                },
                "author": {
                    "type": "string",
                    "format": "uuid",
                    "x-cycms-relation-target": "user",
                    "x-cycms-relation-kind": "one_to_one"
                },
                "tags": {
                    "type": "array",
                    "items": { "type": "string", "format": "uuid" },
                    "x-cycms-relation-target": "tag",
                    "x-cycms-relation-kind": "one_to_many"
                },
                "cover": {
                    "type": "string",
                    "format": "uuid",
                    "x-cycms-media-allowed-types": ["image/png", "image/jpeg"]
                },
                "published_at": {
                    "type": "string",
                    "format": "date-time"
                },
                "profile": {
                    "x-cycms-validators": ["plugin.profile"]
                },
                "point": {
                    "type": "object",
                    "properties": {
                        "lat": { "type": "number" },
                        "lng": { "type": "number" }
                    },
                    "required": ["lat", "lng"],
                    "x-cycms-custom-type": "geo.latlng"
                }
            },
            "required": ["title", "status", "author"]
        });

        assert_eq!(schema, expected);
    }

    #[test]
    fn unregistered_custom_type_falls_back_to_marker() {
        let ct = ContentTypeDefinition {
            id: "x".into(),
            name: "X".into(),
            api_id: "x".into(),
            description: None,
            kind: ContentTypeKind::Collection,
            fields: vec![FieldDefinition {
                name: "Point".into(),
                api_id: "point".into(),
                field_type: FieldType::Custom {
                    type_name: "missing.type".into(),
                },
                required: false,
                unique: false,
                default_value: None,
                validations: vec![],
                position: 0,
            }],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let registry = FieldTypeRegistry::new();
        let s = to_json_schema(&ct, &registry);
        let prop = s.get("properties").and_then(|p| p.get("point")).unwrap();
        assert_eq!(
            prop.get("x-cycms-custom-type"),
            Some(&json!("missing.type"))
        );
    }
}
