use crate::pool::{DatabasePool, DatabaseType};

/// JSON path / column 校验失败的细分错误。
#[derive(Debug, thiserror::Error)]
pub enum JsonPathError {
    #[error("json path cannot be empty")]
    EmptyPath,
    #[error("json column name is empty")]
    EmptyColumn,
    #[error("json path segment contains illegal character: {segment}")]
    IllegalPathSegment { segment: String },
    #[error("json column name contains illegal character: {column}")]
    IllegalColumn { column: String },
}

/// 生成 SELECT 列表中的 JSON 字段取值片段。
///
/// 返回片段保留 JSON / JSONB 原类型，便于上层继续与其他 JSON 值比较；若需要 text，
/// 调用方自行包一层 `::text` / `CAST(... AS CHAR)`。
///
/// # Errors
/// column 或 path 含非法字符时返回 [`JsonPathError`]，防止用户输入拼入 SQL 造成注入。
pub fn json_field_query(
    pool: &DatabasePool,
    column: &str,
    path: &str,
) -> Result<String, JsonPathError> {
    let column = validate_column(column)?;
    let segments = parse_path(path)?;

    Ok(match pool.db_type() {
        DatabaseType::Postgres => {
            let mut joined = String::new();
            for segment in &segments {
                use std::fmt::Write as _;
                // `segment` 已过 `is_valid_identifier` 白名单，字符串写入不会产生 SQL 注入。
                let _ = write!(joined, "->'{segment}'");
            }
            format!("\"{column}\"{joined}")
        }
        DatabaseType::MySql => {
            let json_path = build_dollar_path(&segments);
            format!("JSON_EXTRACT(`{column}`, '{json_path}')")
        }
        DatabaseType::Sqlite => {
            let json_path = build_dollar_path(&segments);
            format!("json_extract(\"{column}\", '{json_path}')")
        }
    })
}

/// 生成 UPDATE 语句中的 JSON 字段赋值片段。
///
/// `value_placeholder` 是调用方自行构造的占位符或表达式（如 `$1`、`?`、`to_jsonb($1)`），
/// 本函数不对其做校验，调用方须确保其本身不来自用户输入。
///
/// # Errors
/// column 或 path 含非法字符时返回 [`JsonPathError`]。
pub fn json_field_set(
    pool: &DatabasePool,
    column: &str,
    path: &str,
    value_placeholder: &str,
) -> Result<String, JsonPathError> {
    let column = validate_column(column)?;
    let segments = parse_path(path)?;

    Ok(match pool.db_type() {
        DatabaseType::Postgres => {
            let pg_path = segments.join(",");
            format!("jsonb_set(\"{column}\", '{{{pg_path}}}', {value_placeholder})")
        }
        DatabaseType::MySql => {
            let json_path = build_dollar_path(&segments);
            format!("JSON_SET(`{column}`, '{json_path}', {value_placeholder})")
        }
        DatabaseType::Sqlite => {
            let json_path = build_dollar_path(&segments);
            format!("json_set(\"{column}\", '{json_path}', {value_placeholder})")
        }
    })
}

fn validate_column(column: &str) -> Result<&str, JsonPathError> {
    if column.is_empty() {
        return Err(JsonPathError::EmptyColumn);
    }
    if !is_valid_identifier(column) {
        return Err(JsonPathError::IllegalColumn {
            column: column.to_owned(),
        });
    }
    Ok(column)
}

fn parse_path(path: &str) -> Result<Vec<&str>, JsonPathError> {
    if path.is_empty() {
        return Err(JsonPathError::EmptyPath);
    }
    let segments: Vec<&str> = path.split('.').collect();
    for segment in &segments {
        if !is_valid_identifier(segment) {
            return Err(JsonPathError::IllegalPathSegment {
                segment: (*segment).to_owned(),
            });
        }
    }
    Ok(segments)
}

fn is_valid_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn build_dollar_path(segments: &[&str]) -> String {
    let mut result = String::from("$");
    for segment in segments {
        result.push('.');
        result.push_str(segment);
    }
    result
}
