use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Content Type 的结构形态：`Collection` 聚合多条实例（常见的文章/产品），
/// `Single` 仅允许一条实例（全局的页面配置、首页等）。
///
/// 序列化为 `snake_case`，与 SQL 默认值 `'collection'` 对齐。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentTypeKind {
    Collection,
    Single,
}

impl ContentTypeKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Collection => "collection",
            Self::Single => "single",
        }
    }
}

/// `FieldType`：内置类型 + 插件自定义类型（Req 3.2 / 3.6）。
///
/// Tagged enum 序列化：`{ "kind": "<snake_case>", ...payload }`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FieldType {
    Text,
    RichText,
    Number {
        #[serde(default)]
        decimal: bool,
    },
    Boolean,
    DateTime,
    Json,
    Media {
        #[serde(default)]
        allowed_types: Vec<String>,
    },
    Relation {
        target_type: String,
        relation_kind: RelationKind,
    },
    /// 插件注册的自定义类型，`type_name` 由插件自行约定（约定包含 `.` 作为命名空间前缀）。
    Custom {
        type_name: String,
    },
}

/// Relation 字段的关联基数（Req 3.4）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    OneToOne,
    OneToMany,
    ManyToMany,
}

/// 字段级校验规则（Req 3.2）。
///
/// Tagged enum 序列化：`{ "rule": "<snake_case>", ...payload }`。
///
/// `required` 与 `unique` 是 [`FieldDefinition`] 上的布尔标志，不出现在此枚举。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "rule", rename_all = "snake_case")]
pub enum ValidationRule {
    MinLength {
        value: usize,
    },
    MaxLength {
        value: usize,
    },
    Min {
        value: f64,
    },
    Max {
        value: f64,
    },
    /// 正则字符串（等价于 tasks.md 里的 `pattern`）。
    Regex {
        pattern: String,
    },
    Enum {
        values: Vec<Value>,
    },
    /// 插件自定义校验器名称，运行时通过 [`crate::field_type::FieldTypeRegistry`] 派发。
    Custom {
        validator: String,
    },
}

/// 字段定义：结构 + 校验规则。序列化后落盘至 `content_types.fields` JSON 列。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub name: String,
    pub api_id: String,
    pub field_type: FieldType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub unique: bool,
    #[serde(default)]
    pub default_value: Option<Value>,
    #[serde(default)]
    pub validations: Vec<ValidationRule>,
    #[serde(default)]
    pub position: i32,
}

/// Content Type 对外视图。`id` 跨方言以字符串形式持有 UUID v4。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentTypeDefinition {
    pub id: String,
    pub name: String,
    pub api_id: String,
    pub description: Option<String>,
    pub kind: ContentTypeKind,
    pub fields: Vec<FieldDefinition>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建 Content Type 的入参。
#[derive(Debug, Clone)]
pub struct CreateContentTypeInput {
    pub name: String,
    pub api_id: String,
    pub description: Option<String>,
    pub kind: ContentTypeKind,
    pub fields: Vec<FieldDefinition>,
}

/// 更新 Content Type 的入参。字段均可选，仅变更非 `None` 的项。
#[derive(Debug, Clone, Default)]
pub struct UpdateContentTypeInput {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub kind: Option<ContentTypeKind>,
    pub fields: Option<Vec<FieldDefinition>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn field_type_serde_roundtrip_text() {
        let v = FieldType::Text;
        let s = serde_json::to_value(&v).unwrap();
        assert_eq!(s, json!({ "kind": "text" }));
        let back: FieldType = serde_json::from_value(s).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn field_type_serde_roundtrip_number_decimal() {
        let v = FieldType::Number { decimal: true };
        let s = serde_json::to_value(&v).unwrap();
        assert_eq!(s, json!({ "kind": "number", "decimal": true }));
        let back: FieldType = serde_json::from_value(s).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn field_type_serde_roundtrip_relation() {
        let v = FieldType::Relation {
            target_type: "article".into(),
            relation_kind: RelationKind::OneToMany,
        };
        let s = serde_json::to_value(&v).unwrap();
        assert_eq!(
            s,
            json!({
                "kind": "relation",
                "target_type": "article",
                "relation_kind": "one_to_many"
            })
        );
        let back: FieldType = serde_json::from_value(s).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn field_type_serde_roundtrip_media() {
        let v = FieldType::Media {
            allowed_types: vec!["image/png".into(), "image/jpeg".into()],
        };
        let s = serde_json::to_value(&v).unwrap();
        assert_eq!(
            s,
            json!({
                "kind": "media",
                "allowed_types": ["image/png", "image/jpeg"]
            })
        );
        let back: FieldType = serde_json::from_value(s).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn field_type_serde_roundtrip_custom() {
        let v = FieldType::Custom {
            type_name: "geo.latlng".into(),
        };
        let s = serde_json::to_value(&v).unwrap();
        assert_eq!(s, json!({ "kind": "custom", "type_name": "geo.latlng" }));
        let back: FieldType = serde_json::from_value(s).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn validation_rule_serde_roundtrip_covers_all_variants() {
        let cases = vec![
            (
                ValidationRule::MinLength { value: 3 },
                json!({ "rule": "min_length", "value": 3 }),
            ),
            (
                ValidationRule::MaxLength { value: 255 },
                json!({ "rule": "max_length", "value": 255 }),
            ),
            (
                ValidationRule::Min { value: 0.0 },
                json!({ "rule": "min", "value": 0.0 }),
            ),
            (
                ValidationRule::Max { value: 100.0 },
                json!({ "rule": "max", "value": 100.0 }),
            ),
            (
                ValidationRule::Regex {
                    pattern: "^[a-z]+$".into(),
                },
                json!({ "rule": "regex", "pattern": "^[a-z]+$" }),
            ),
            (
                ValidationRule::Enum {
                    values: vec![json!("draft"), json!("published")],
                },
                json!({ "rule": "enum", "values": ["draft", "published"] }),
            ),
            (
                ValidationRule::Custom {
                    validator: "my_plugin.zipcode".into(),
                },
                json!({ "rule": "custom", "validator": "my_plugin.zipcode" }),
            ),
        ];
        for (rule, expected) in cases {
            let s = serde_json::to_value(&rule).unwrap();
            assert_eq!(s, expected, "serialize {rule:?}");
            let back: ValidationRule = serde_json::from_value(expected).unwrap();
            assert_eq!(back, rule);
        }
    }

    #[test]
    fn field_definition_default_optional_fields() {
        let raw = json!({
            "name": "Title",
            "api_id": "title",
            "field_type": { "kind": "text" }
        });
        let fd: FieldDefinition = serde_json::from_value(raw).unwrap();
        assert_eq!(fd.name, "Title");
        assert_eq!(fd.api_id, "title");
        assert_eq!(fd.field_type, FieldType::Text);
        assert!(!fd.required);
        assert!(!fd.unique);
        assert!(fd.default_value.is_none());
        assert!(fd.validations.is_empty());
        assert_eq!(fd.position, 0);
    }

    #[test]
    fn content_type_kind_serde_snake_case() {
        assert_eq!(
            serde_json::to_value(ContentTypeKind::Collection).unwrap(),
            json!("collection")
        );
        assert_eq!(
            serde_json::to_value(ContentTypeKind::Single).unwrap(),
            json!("single")
        );
        let parsed: ContentTypeKind = serde_json::from_value(json!("single")).unwrap();
        assert_eq!(parsed, ContentTypeKind::Single);
    }
}
