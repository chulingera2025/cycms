use std::collections::HashMap;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use cycms_content_engine::{
    ColumnField, ContentQuery, ContentStatus, FieldRef, FilterOperator, FilterSpec, SortDir,
    SortSpec,
};
use cycms_core::{Error, Result};
use cycms_media::{MediaOrderDir, MediaQuery};
use serde_json::{Number, Value};

pub fn parse_content_query(params: &HashMap<String, String>) -> Result<ContentQuery> {
    let mut query = ContentQuery::default();

    if let Some(page) = params.get("page") {
        query.page = Some(parse_u64(page, "page")?);
    }
    if let Some(page_size) = params.get("pageSize").or_else(|| params.get("page_size")) {
        query.page_size = Some(parse_u64(page_size, "pageSize")?);
    }
    if let Some(sort) = params.get("sort") {
        query.sort = sort
            .split(',')
            .filter(|part| !part.trim().is_empty())
            .map(parse_sort_spec)
            .collect::<Result<Vec<_>>>()?;
    }
    if let Some(populate) = params.get("populate") {
        query.populate = comma_values(populate);
    }
    if let Some(status) = params.get("status") {
        query.status = Some(ContentStatus::from_str(status).map_err(|err| Error::ValidationError {
            message: err.to_string(),
            details: None,
        })?);
    }

    for (key, value) in params {
        if let Some((field, operator)) = parse_filter_key(key) {
            query.filters.push(FilterSpec {
                field: parse_field_ref(&field)?,
                op: parse_filter_operator(&operator)?,
                value: parse_filter_value(&operator, value),
            });
        }
    }

    Ok(query)
}

pub fn parse_media_query(params: &HashMap<String, String>) -> Result<MediaQuery> {
    let mut query = MediaQuery::default();
    if let Some(page) = params.get("page") {
        query.page = Some(parse_u64(page, "page")?);
    }
    if let Some(page_size) = params.get("pageSize").or_else(|| params.get("page_size")) {
        query.page_size = Some(parse_u64(page_size, "pageSize")?);
    }
    if let Some(mime_type) = params.get("mime_type").or_else(|| params.get("mimeType")) {
        query.mime_type = Some(mime_type.trim().to_owned());
    }
    if let Some(filename) = params.get("filename").or_else(|| params.get("filename_contains")) {
        query.filename_contains = Some(filename.trim().to_owned());
    }
    if let Some(uploaded_by) = params.get("uploaded_by") {
        query.uploaded_by = Some(uploaded_by.trim().to_owned());
    }
    if let Some(created_after) = params.get("created_after") {
        query.created_after = Some(parse_datetime(created_after, "created_after")?);
    }
    if let Some(created_before) = params.get("created_before") {
        query.created_before = Some(parse_datetime(created_before, "created_before")?);
    }
    if let Some(order) = params.get("order").or_else(|| params.get("order_dir")) {
        query.order_dir = match order.trim().to_ascii_lowercase().as_str() {
            "asc" => MediaOrderDir::Asc,
            "desc" => MediaOrderDir::Desc,
            _ => {
                return Err(Error::ValidationError {
                    message: format!("invalid order direction: {order}"),
                    details: None,
                });
            }
        };
    }
    Ok(query)
}

fn parse_filter_key(key: &str) -> Option<(String, String)> {
    let rest = key.strip_prefix("filter[")?;
    let (field, operator) = rest.split_once("][")?;
    let operator = operator.strip_suffix(']')?;
    Some((field.to_owned(), operator.to_owned()))
}

fn parse_sort_spec(raw: &str) -> Result<SortSpec> {
    let (field, direction) = raw.split_once(':').unwrap_or((raw, "asc"));
    let direction = match direction.trim().to_ascii_lowercase().as_str() {
        "asc" => SortDir::Asc,
        "desc" => SortDir::Desc,
        _ => {
            return Err(Error::ValidationError {
                message: format!("invalid sort direction: {direction}"),
                details: None,
            });
        }
    };
    Ok(SortSpec {
        field: parse_field_ref(field.trim())?,
        direction,
    })
}

fn parse_field_ref(raw: &str) -> Result<FieldRef> {
    if let Some(column) = ColumnField::parse(raw) {
        return Ok(FieldRef::Column(column));
    }
    let value = if raw.starts_with("fields.") {
        raw.to_owned()
    } else {
        format!("fields.{raw}")
    };
    FieldRef::parse(&value).map_err(Into::into)
}

fn parse_filter_operator(raw: &str) -> Result<FilterOperator> {
    let normalized = raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "eq" => Ok(FilterOperator::Eq),
        "ne" => Ok(FilterOperator::Ne),
        "gt" => Ok(FilterOperator::Gt),
        "gte" => Ok(FilterOperator::Gte),
        "lt" => Ok(FilterOperator::Lt),
        "lte" => Ok(FilterOperator::Lte),
        "contains" => Ok(FilterOperator::Contains),
        "startswith" => Ok(FilterOperator::StartsWith),
        "endswith" => Ok(FilterOperator::EndsWith),
        "in" => Ok(FilterOperator::In),
        "notin" => Ok(FilterOperator::NotIn),
        "null" => Ok(FilterOperator::Null),
        "notnull" => Ok(FilterOperator::NotNull),
        _ => Err(Error::ValidationError {
            message: format!("invalid filter operator: {raw}"),
            details: None,
        }),
    }
}

fn parse_filter_value(operator: &str, raw: &str) -> Value {
    match operator.trim().to_ascii_lowercase().as_str() {
        "in" | "notin" => Value::Array(
            comma_values(raw)
                .into_iter()
                .map(|value| coerce_scalar(&value))
                .collect(),
        ),
        _ => coerce_scalar(raw),
    }
}

fn coerce_scalar(raw: &str) -> Value {
    let trimmed = raw.trim();
    if trimmed.eq_ignore_ascii_case("null") {
        Value::Null
    } else if trimmed.eq_ignore_ascii_case("true") {
        Value::Bool(true)
    } else if trimmed.eq_ignore_ascii_case("false") {
        Value::Bool(false)
    } else if let Ok(value) = trimmed.parse::<i64>() {
        Value::Number(value.into())
    } else if let Ok(value) = trimmed.parse::<f64>() {
        Number::from_f64(value).map_or_else(|| Value::String(trimmed.to_owned()), Value::Number)
    } else {
        Value::String(trimmed.to_owned())
    }
}

fn comma_values(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_u64(raw: &str, field: &str) -> Result<u64> {
    raw.trim().parse::<u64>().map_err(|_| Error::ValidationError {
        message: format!("{field} must be an unsigned integer"),
        details: None,
    })
}

fn parse_datetime(raw: &str, field: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| Error::ValidationError {
            message: format!("{field} must be a valid RFC3339 datetime"),
            details: None,
        })
}