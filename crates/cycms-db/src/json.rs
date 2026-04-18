use crate::pool::{DatabasePool, DatabaseType};

/// JSON path 校验失败的细分错误。
#[derive(Debug, thiserror::Error)]
pub enum JsonPathError {
    #[error("json path cannot be empty")]
    Empty,
    #[error("json path segment contains illegal character: {segment}")]
    IllegalCharacter { segment: String },
}

/// 生成 SELECT 列表中的 JSON 字段取值片段。
///
/// # Errors
/// TODO!!! DB-3 正式实现路径校验与方言生成。
#[allow(clippy::missing_errors_doc)]
pub fn json_field_query(
    _pool: &DatabasePool,
    _column: &str,
    _path: &str,
) -> Result<String, JsonPathError> {
    todo!("TODO!!!: DB-3 实现 JSON 字段查询片段生成")
}

/// 生成 UPDATE 语句中的 JSON 字段赋值片段。
///
/// # Errors
/// TODO!!! DB-3 正式实现路径校验与方言生成。
#[allow(clippy::missing_errors_doc)]
pub fn json_field_set(
    _pool: &DatabasePool,
    _column: &str,
    _path: &str,
    _value_placeholder: &str,
) -> Result<String, JsonPathError> {
    todo!("TODO!!!: DB-3 实现 JSON 字段赋值片段生成")
}

#[doc(hidden)]
pub(crate) fn _db_type_ref(pool: &DatabasePool) -> DatabaseType {
    pool.db_type()
}
