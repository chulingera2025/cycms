//! 插件 settings schema 注册时的格式校验。
//!
//! v0.1 规则（与 Requirement 15.3 对齐）：
//! - schema 必须是 JSON 对象；
//! - 必须至少包含 `type` 或 `properties` 字段，用于管理后台的最低表单渲染假设。
//!
//! TODO!!!: v0.2 接入 `jsonschema` crate 进行完整 JSON Schema 校验，并在
//! `SettingsManager::set` 时对 value 按已注册 schema 做匹配校验。

use serde_json::Value;

use crate::error::SettingsError;

/// 校验插件 settings schema 的最小形状。
///
/// # Errors
/// 返回 [`SettingsError::InputValidation`] 当：
/// - schema 非 JSON object；
/// - 既无 `type` 也无 `properties` 字段。
pub fn validate_schema_shape(schema: &Value) -> Result<(), SettingsError> {
    let Some(obj) = schema.as_object() else {
        return Err(SettingsError::InputValidation(
            "schema must be a json object".to_owned(),
        ));
    };
    if !obj.contains_key("type") && !obj.contains_key("properties") {
        return Err(SettingsError::InputValidation(
            "schema must contain `type` or `properties` field".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_schema_shape;
    use serde_json::json;

    #[test]
    fn accepts_type_only_schema() {
        let schema = json!({ "type": "object" });
        assert!(validate_schema_shape(&schema).is_ok());
    }

    #[test]
    fn accepts_properties_only_schema() {
        let schema = json!({ "properties": { "foo": { "type": "string" } } });
        assert!(validate_schema_shape(&schema).is_ok());
    }

    #[test]
    fn rejects_non_object_schema() {
        assert!(validate_schema_shape(&json!("string")).is_err());
        assert!(validate_schema_shape(&json!([1, 2, 3])).is_err());
        assert!(validate_schema_shape(&json!(null)).is_err());
    }

    #[test]
    fn rejects_object_without_required_keys() {
        let schema = json!({ "title": "untitled" });
        assert!(validate_schema_shape(&schema).is_err());
    }
}
