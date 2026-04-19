//! 内容查询引擎（任务 11 Step 4）。
//!
//! 支持分页 / 多字段排序 / 13 种筛选算子：
//! - 顶层列走固定 [`ColumnField`] 白名单，不允许任意字符串列名；
//! - JSON 字段走点号路径并在编译期做字符合法性校验，避免 SQL 注入；
//! - 方言分歧在 [`compile_list_query`] 内部落地，对 repository 透明。

use std::fmt::Write as _;

use cycms_db::DatabaseType;
use serde_json::Value;

use crate::error::ContentEngineError;
use crate::model::ContentStatus;

/// 对外暴露的内容查询规范。
#[derive(Debug, Clone, Default)]
pub struct ContentQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub sort: Vec<SortSpec>,
    pub filters: Vec<FilterSpec>,
    pub status: Option<ContentStatus>,
    pub populate: Vec<String>,
}

/// 单个排序项。
#[derive(Debug, Clone)]
pub struct SortSpec {
    pub field: FieldRef,
    pub direction: SortDir,
}

/// 排序方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDir {
    Asc,
    Desc,
}

/// 字段引用：要么是顶层列，要么是 `fields.*` 下的 JSON 路径。
#[derive(Debug, Clone)]
pub enum FieldRef {
    Column(ColumnField),
    Json(String),
}

/// `content_entries` 表白名单列。排序 / 过滤仅允许这些列，规避 SQL 注入。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnField {
    Slug,
    Status,
    CreatedBy,
    UpdatedBy,
    CreatedAt,
    UpdatedAt,
    PublishedAt,
}

/// 单个筛选项。
#[derive(Debug, Clone)]
pub struct FilterSpec {
    pub field: FieldRef,
    pub op: FilterOperator,
    pub value: Value,
}

/// 13 种筛选算子（Req 4.3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterOperator {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    Contains,
    StartsWith,
    EndsWith,
    In,
    NotIn,
    Null,
    NotNull,
}

/// 绑定到 `sqlx::Query` 的查询参数（跨方言中间态）。
#[derive(Debug, Clone)]
pub enum QueryParam {
    Text(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

/// [`compile_list_query`] 的产物：供 repository 直接拼接并绑定。
#[derive(Debug, Clone)]
pub struct ListQueryPlan {
    pub where_sql: String,
    pub order_sql: String,
    pub limit_placeholder: String,
    pub offset_placeholder: String,
    /// 含 WHERE 子句的全部参数 + 末尾 `limit` / `offset`。
    pub params: Vec<QueryParam>,
    /// 不含 `limit` / `offset` 的参数个数（供 `COUNT(*)` 语句截取前缀）。
    pub content_param_count: usize,
    pub page: u64,
    pub page_size: u64,
}

impl ColumnField {
    /// 对应的 SQL 列名（三方言一致）。
    #[must_use]
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Slug => "slug",
            Self::Status => "status",
            Self::CreatedBy => "created_by",
            Self::UpdatedBy => "updated_by",
            Self::CreatedAt => "created_at",
            Self::UpdatedAt => "updated_at",
            Self::PublishedAt => "published_at",
        }
    }

    /// 从字符串解析列枚举；未知列返回 `None`。
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "slug" => Some(Self::Slug),
            "status" => Some(Self::Status),
            "created_by" => Some(Self::CreatedBy),
            "updated_by" => Some(Self::UpdatedBy),
            "created_at" => Some(Self::CreatedAt),
            "updated_at" => Some(Self::UpdatedAt),
            "published_at" => Some(Self::PublishedAt),
            _ => None,
        }
    }
}

impl FieldRef {
    /// 解析字段引用：`fields.x.y` → [`Self::Json`]，否则尝试 [`ColumnField`]。
    ///
    /// # Errors
    /// 列名不在白名单或 JSON path 非法时返回 [`ContentEngineError::InvalidQuery`]。
    pub fn parse(s: &str) -> Result<Self, ContentEngineError> {
        if let Some(path) = s.strip_prefix("fields.") {
            validate_json_path(path)?;
            Ok(Self::Json(path.to_owned()))
        } else if let Some(col) = ColumnField::parse(s) {
            Ok(Self::Column(col))
        } else {
            Err(ContentEngineError::InvalidQuery(format!(
                "unknown query field: {s}"
            )))
        }
    }
}

/// 编译内容列表查询为 SQL 片段与参数表。
///
/// 约定：
/// - 第一个参数永远是 `content_type_id`；
/// - 若 `query.status` 设置，紧随其后为第二个参数；
/// - 之后按顺序是每个 filter 的参数；
/// - 最后两个参数是 `limit` / `offset`；
/// - `ORDER BY` 默认 `created_at DESC`。
///
/// # Errors
/// JSON 路径非法 / filter 值形状与 operator 不匹配时返回 [`ContentEngineError::InvalidQuery`]。
pub fn compile_list_query(
    db_type: DatabaseType,
    content_type_id: &str,
    query: &ContentQuery,
    default_page_size: u64,
    max_page_size: u64,
) -> Result<ListQueryPlan, ContentEngineError> {
    let page = query.page.unwrap_or(1).max(1);
    let effective_max = max_page_size.max(1);
    let page_size = query
        .page_size
        .unwrap_or(default_page_size)
        .clamp(1, effective_max);
    let offset = page.saturating_sub(1).saturating_mul(page_size);

    let mut params: Vec<QueryParam> = Vec::new();
    let mut idx: usize = 1;

    let mut where_sql = String::new();
    let ph_content_type = placeholder(db_type, idx);
    match db_type {
        DatabaseType::Postgres => {
            write!(where_sql, "content_type_id = {ph_content_type}::UUID").expect("write");
        }
        DatabaseType::MySql | DatabaseType::Sqlite => {
            write!(where_sql, "content_type_id = {ph_content_type}").expect("write");
        }
    }
    params.push(QueryParam::Text(content_type_id.to_owned()));
    idx += 1;

    if let Some(status) = query.status {
        let ph = placeholder(db_type, idx);
        write!(where_sql, " AND status = {ph}").expect("write");
        params.push(QueryParam::Text(status.as_str().to_owned()));
        idx += 1;
    }

    for filter in &query.filters {
        let (frag, filter_params, next_idx) = compile_filter(db_type, filter, idx)?;
        where_sql.push_str(" AND ");
        where_sql.push_str(&frag);
        params.extend(filter_params);
        idx = next_idx;
    }

    let order_sql = if query.sort.is_empty() {
        "created_at DESC".to_owned()
    } else {
        let mut parts = Vec::with_capacity(query.sort.len());
        for spec in &query.sort {
            let field_sql = match &spec.field {
                FieldRef::Column(c) => c.as_sql().to_owned(),
                FieldRef::Json(p) => {
                    validate_json_path(p)?;
                    json_text_path(db_type, p)
                }
            };
            let dir = match spec.direction {
                SortDir::Asc => "ASC",
                SortDir::Desc => "DESC",
            };
            parts.push(format!("{field_sql} {dir}"));
        }
        parts.join(", ")
    };

    let content_param_count = params.len();
    let limit_placeholder = placeholder(db_type, idx);
    let offset_placeholder = placeholder(db_type, idx + 1);
    params.push(QueryParam::Int(
        i64::try_from(page_size).unwrap_or(i64::MAX),
    ));
    params.push(QueryParam::Int(i64::try_from(offset).unwrap_or(i64::MAX)));

    Ok(ListQueryPlan {
        where_sql,
        order_sql,
        limit_placeholder,
        offset_placeholder,
        params,
        content_param_count,
        page,
        page_size,
    })
}

fn placeholder(db_type: DatabaseType, idx: usize) -> String {
    match db_type {
        DatabaseType::Postgres => format!("${idx}"),
        DatabaseType::MySql | DatabaseType::Sqlite => "?".to_owned(),
    }
}

fn validate_json_path(path: &str) -> Result<(), ContentEngineError> {
    if path.is_empty() {
        return Err(ContentEngineError::InvalidQuery(
            "json field path cannot be empty".to_owned(),
        ));
    }
    for seg in path.split('.') {
        let mut chars = seg.chars();
        let first = chars.next().ok_or_else(|| {
            ContentEngineError::InvalidQuery(format!("empty json path segment in `{path}`"))
        })?;
        if !(first.is_ascii_alphabetic() || first == '_') {
            return Err(ContentEngineError::InvalidQuery(format!(
                "json path segment must start with letter or `_`: `{seg}`"
            )));
        }
        if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(ContentEngineError::InvalidQuery(format!(
                "json path segment contains illegal character: `{seg}`"
            )));
        }
    }
    Ok(())
}

fn json_text_path(db_type: DatabaseType, path: &str) -> String {
    let segments: Vec<&str> = path.split('.').collect();
    match db_type {
        DatabaseType::Postgres => {
            let mut out = String::from("\"fields\"");
            for (i, seg) in segments.iter().enumerate() {
                if i + 1 == segments.len() {
                    let _ = write!(out, "->>'{seg}'");
                } else {
                    let _ = write!(out, "->'{seg}'");
                }
            }
            out
        }
        DatabaseType::MySql => {
            let dollar = build_dollar_path(&segments);
            format!("JSON_UNQUOTE(JSON_EXTRACT(`fields`, '{dollar}'))")
        }
        DatabaseType::Sqlite => {
            let dollar = build_dollar_path(&segments);
            format!("json_extract(\"fields\", '{dollar}')")
        }
    }
}

fn build_dollar_path(segments: &[&str]) -> String {
    let mut out = String::from("$");
    for seg in segments {
        out.push('.');
        out.push_str(seg);
    }
    out
}

fn compile_filter(
    db_type: DatabaseType,
    filter: &FilterSpec,
    start_idx: usize,
) -> Result<(String, Vec<QueryParam>, usize), ContentEngineError> {
    let field_sql = match &filter.field {
        FieldRef::Column(c) => c.as_sql().to_owned(),
        FieldRef::Json(p) => {
            validate_json_path(p)?;
            json_text_path(db_type, p)
        }
    };

    let mut params: Vec<QueryParam> = Vec::new();
    let mut idx = start_idx;

    let fragment = match filter.op {
        FilterOperator::Eq | FilterOperator::Ne => {
            let ph = placeholder(db_type, idx);
            params.push(value_to_query_param(&filter.value)?);
            idx += 1;
            let op_sql = if matches!(filter.op, FilterOperator::Eq) {
                "="
            } else {
                "<>"
            };
            format!("{field_sql} {op_sql} {ph}")
        }
        FilterOperator::Gt | FilterOperator::Gte | FilterOperator::Lt | FilterOperator::Lte => {
            let ph = placeholder(db_type, idx);
            params.push(value_to_query_param(&filter.value)?);
            idx += 1;
            let op_sql = match filter.op {
                FilterOperator::Gt => ">",
                FilterOperator::Gte => ">=",
                FilterOperator::Lt => "<",
                FilterOperator::Lte => "<=",
                _ => unreachable!("covered by outer arm"),
            };
            format!("{field_sql} {op_sql} {ph}")
        }
        FilterOperator::Contains | FilterOperator::StartsWith | FilterOperator::EndsWith => {
            let text = filter.value.as_str().ok_or_else(|| {
                ContentEngineError::InvalidQuery(
                    "contains/startsWith/endsWith requires string value".to_owned(),
                )
            })?;
            let pattern = match filter.op {
                FilterOperator::Contains => format!("%{text}%"),
                FilterOperator::StartsWith => format!("{text}%"),
                FilterOperator::EndsWith => format!("%{text}"),
                _ => unreachable!("covered by outer arm"),
            };
            let ph = placeholder(db_type, idx);
            params.push(QueryParam::Text(pattern));
            idx += 1;
            format!("{field_sql} LIKE {ph}")
        }
        FilterOperator::In | FilterOperator::NotIn => {
            let values = filter.value.as_array().ok_or_else(|| {
                ContentEngineError::InvalidQuery("in/notIn requires array value".to_owned())
            })?;
            if values.is_empty() {
                let tautology = match filter.op {
                    FilterOperator::In => "1 = 0",
                    FilterOperator::NotIn => "1 = 1",
                    _ => unreachable!("covered by outer arm"),
                };
                return Ok((tautology.to_owned(), params, idx));
            }
            let mut placeholders = Vec::with_capacity(values.len());
            for v in values {
                placeholders.push(placeholder(db_type, idx));
                params.push(value_to_query_param(v)?);
                idx += 1;
            }
            let op_sql = match filter.op {
                FilterOperator::In => "IN",
                FilterOperator::NotIn => "NOT IN",
                _ => unreachable!("covered by outer arm"),
            };
            format!("{field_sql} {op_sql} ({})", placeholders.join(", "))
        }
        FilterOperator::Null => format!("{field_sql} IS NULL"),
        FilterOperator::NotNull => format!("{field_sql} IS NOT NULL"),
    };

    Ok((fragment, params, idx))
}

fn value_to_query_param(v: &Value) -> Result<QueryParam, ContentEngineError> {
    match v {
        Value::String(s) => Ok(QueryParam::Text(s.clone())),
        Value::Bool(b) => Ok(QueryParam::Bool(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(QueryParam::Int(i))
            } else if let Some(f) = n.as_f64() {
                Ok(QueryParam::Float(f))
            } else {
                Err(ContentEngineError::InvalidQuery(format!(
                    "unsupported number literal: {v}"
                )))
            }
        }
        _ => Err(ContentEngineError::InvalidQuery(format!(
            "filter value must be string/bool/number, got: {v}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ColumnField, ContentQuery, FieldRef, FilterOperator, FilterSpec, SortDir, SortSpec,
        compile_list_query,
    };
    use cycms_db::DatabaseType;
    use serde_json::json;

    fn base_query(filters: Vec<FilterSpec>) -> ContentQuery {
        ContentQuery {
            filters,
            ..ContentQuery::default()
        }
    }

    #[test]
    fn default_query_uses_created_at_desc() {
        let plan = compile_list_query(
            DatabaseType::Sqlite,
            "ct-1",
            &ContentQuery::default(),
            20,
            100,
        )
        .unwrap();
        assert_eq!(plan.order_sql, "created_at DESC");
        assert!(plan.where_sql.starts_with("content_type_id = ?"));
        assert_eq!(plan.content_param_count, 1);
        assert_eq!(plan.page, 1);
        assert_eq!(plan.page_size, 20);
    }

    #[test]
    fn page_size_is_clamped_to_cap() {
        let query = ContentQuery {
            page_size: Some(1_000),
            ..ContentQuery::default()
        };
        let plan = compile_list_query(DatabaseType::Sqlite, "ct-1", &query, 20, 50).unwrap();
        assert_eq!(plan.page_size, 50);
    }

    #[test]
    fn postgres_uses_dollar_placeholders_and_uuid_cast() {
        let plan = compile_list_query(
            DatabaseType::Postgres,
            "ct-1",
            &ContentQuery::default(),
            20,
            100,
        )
        .unwrap();
        assert!(plan.where_sql.starts_with("content_type_id = $1::UUID"));
        assert_eq!(plan.limit_placeholder, "$2");
        assert_eq!(plan.offset_placeholder, "$3");
    }

    #[test]
    fn eq_filter_on_json_field_uses_dialect_extract() {
        let q = base_query(vec![FilterSpec {
            field: FieldRef::Json("title".to_owned()),
            op: FilterOperator::Eq,
            value: json!("Hello"),
        }]);
        let plan_pg = compile_list_query(DatabaseType::Postgres, "ct-1", &q, 20, 100).unwrap();
        assert!(plan_pg.where_sql.contains("\"fields\"->>'title' = $2"));

        let plan_my = compile_list_query(DatabaseType::MySql, "ct-1", &q, 20, 100).unwrap();
        assert!(
            plan_my
                .where_sql
                .contains("JSON_UNQUOTE(JSON_EXTRACT(`fields`, '$.title')) = ?")
        );

        let plan_sq = compile_list_query(DatabaseType::Sqlite, "ct-1", &q, 20, 100).unwrap();
        assert!(
            plan_sq
                .where_sql
                .contains("json_extract(\"fields\", '$.title') = ?")
        );
    }

    #[test]
    fn in_with_empty_array_short_circuits() {
        let q = base_query(vec![FilterSpec {
            field: FieldRef::Column(ColumnField::Slug),
            op: FilterOperator::In,
            value: json!([]),
        }]);
        let plan = compile_list_query(DatabaseType::Sqlite, "ct-1", &q, 20, 100).unwrap();
        assert!(plan.where_sql.ends_with("1 = 0"));
        assert_eq!(plan.content_param_count, 1);
    }

    #[test]
    fn sort_falls_back_to_created_at_desc_when_empty() {
        let plan = compile_list_query(
            DatabaseType::Sqlite,
            "ct-1",
            &ContentQuery::default(),
            20,
            100,
        )
        .unwrap();
        assert_eq!(plan.order_sql, "created_at DESC");
    }

    #[test]
    fn sort_multi_field_column_and_json() {
        let q = ContentQuery {
            sort: vec![
                SortSpec {
                    field: FieldRef::Column(ColumnField::UpdatedAt),
                    direction: SortDir::Asc,
                },
                SortSpec {
                    field: FieldRef::Json("title".to_owned()),
                    direction: SortDir::Desc,
                },
            ],
            ..ContentQuery::default()
        };
        let plan = compile_list_query(DatabaseType::Sqlite, "ct-1", &q, 20, 100).unwrap();
        assert_eq!(
            plan.order_sql,
            "updated_at ASC, json_extract(\"fields\", '$.title') DESC"
        );
    }

    #[test]
    fn field_ref_parse_rejects_unknown_column() {
        assert!(FieldRef::parse("xyz").is_err());
        assert!(FieldRef::parse("fields.").is_err());
        assert!(FieldRef::parse("fields.a.0bad").is_err());
        assert!(matches!(
            FieldRef::parse("status").unwrap(),
            FieldRef::Column(_)
        ));
        assert!(matches!(
            FieldRef::parse("fields.title").unwrap(),
            FieldRef::Json(_)
        ));
    }
}
