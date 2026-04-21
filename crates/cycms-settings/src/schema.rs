//! 插件 settings schema 注册时的格式校验。
//!
//! 当前规则（与 Requirement 15.3 对齐）：
//! - schema 必须是 JSON 对象；
//! - schema 文档本身必须通过 JSON Schema meta-schema 校验；
//! - 插件 namespace 写入时，宿主会把 namespace 下当前全部键合成为一个 JSON object，
//!   再按 schema 校验，防止无效配置落盘。

use jsonschema::Validator;
use serde_json::Value;

use crate::error::SettingsError;

/// 校验插件 settings schema 的最小形状。
///
/// # Errors
/// 返回 [`SettingsError::InputValidation`] 当：
/// - schema 非 JSON object；
/// - schema 不符合对应 draft 的 meta-schema。
pub fn validate_schema_shape(schema: &Value) -> Result<(), SettingsError> {
    if !schema.is_object() {
        return Err(SettingsError::InputValidation(
            "schema must be a json object".to_owned(),
        ));
    }

    jsonschema::meta::validate(schema)
        .map_err(|err| SettingsError::InputValidation(format!("invalid json schema: {err}")))?;

    Ok(())
}

/// 编译 schema 为可复用 validator。
///
/// # Errors
/// 当 schema 无法编译为 validator 时返回 [`SettingsError::InputValidation`]。
pub fn compile_schema_validator(schema: &Value) -> Result<Validator, SettingsError> {
    jsonschema::validator_for(schema)
        .map_err(|err| SettingsError::InputValidation(format!("invalid json schema: {err}")))
}

/// 用已注册 schema 校验某个 namespace 的完整快照。
///
/// `instance` 必须是由 `key -> value` 组成的 JSON object；调用方通常会先把当前
/// namespace 的全部键值聚合后再调用此函数。
///
/// # Errors
/// 当实例不满足 schema 时返回 [`SettingsError::InputValidation`]。
pub fn validate_settings_instance(schema: &Value, instance: &Value) -> Result<(), SettingsError> {
    let validator = compile_schema_validator(schema)?;
    let mut errors = validator
        .iter_errors(instance)
        .map(|err| {
            let path = err.instance_path().to_string();
            if path.is_empty() {
                err.to_string()
            } else {
                format!("{err} at {path}")
            }
        })
        .collect::<Vec<_>>();

    if errors.is_empty() {
        return Ok(());
    }

    errors.sort();
    Err(SettingsError::InputValidation(format!(
        "settings value does not conform to schema: {}",
        errors.join("; ")
    )))
}

#[cfg(test)]
mod tests {
    use super::{validate_schema_shape, validate_settings_instance};
    use serde_json::json;

    #[test]
    fn accepts_valid_json_schema_document() {
        let schema = json!({
            "type": "object",
            "properties": {
                "api_key": { "type": "string" }
            }
        });
        assert!(validate_schema_shape(&schema).is_ok());
    }

    #[test]
    fn rejects_invalid_schema_keyword_payload() {
        let schema = json!({ "minimum": "not-a-number" });
        assert!(validate_schema_shape(&schema).is_err());
    }

    #[test]
    fn rejects_non_object_schema() {
        assert!(validate_schema_shape(&json!("string")).is_err());
        assert!(validate_schema_shape(&json!([1, 2, 3])).is_err());
        assert!(validate_schema_shape(&json!(null)).is_err());
    }

    #[test]
    fn validate_instance_reports_schema_mismatch() {
        let schema = json!({
            "type": "object",
            "properties": {
                "api_key": { "type": "string" }
            },
            "required": ["api_key"],
            "additionalProperties": false
        });

        let err = validate_settings_instance(&schema, &json!({ "api_key": 1 })).unwrap_err();
        assert!(err.to_string().contains("does not conform to schema"));
    }

    #[test]
    fn validate_instance_accepts_matching_snapshot() {
        let schema = json!({
            "type": "object",
            "properties": {
                "api_key": { "type": "string" },
                "enabled": { "type": "boolean" }
            },
            "required": ["api_key"]
        });

        assert!(
            validate_settings_instance(&schema, &json!({ "api_key": "secret", "enabled": true }))
                .is_ok()
        );
    }
}
