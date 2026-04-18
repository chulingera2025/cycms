use cycms_db::{DatabaseType, JsonPathError, json_field_query, json_field_set};

#[test]
fn postgres_query_uses_native_operators() {
    let sql = json_field_query(DatabaseType::Postgres, "fields", "author.name").unwrap();
    assert_eq!(sql, "\"fields\"->'author'->'name'");
}

#[test]
fn postgres_set_uses_jsonb_set_with_text_array() {
    let sql = json_field_set(DatabaseType::Postgres, "fields", "author.name", "$1").unwrap();
    assert_eq!(sql, "jsonb_set(\"fields\", '{author,name}', $1)");
}

#[test]
fn mysql_query_uses_json_extract() {
    let sql = json_field_query(DatabaseType::MySql, "fields", "author.name").unwrap();
    assert_eq!(sql, "JSON_EXTRACT(`fields`, '$.author.name')");
}

#[test]
fn mysql_set_uses_json_set() {
    let sql = json_field_set(DatabaseType::MySql, "fields", "author.name", "?").unwrap();
    assert_eq!(sql, "JSON_SET(`fields`, '$.author.name', ?)");
}

#[test]
fn sqlite_query_uses_lowercase_json_extract() {
    let sql = json_field_query(DatabaseType::Sqlite, "fields", "author.name").unwrap();
    assert_eq!(sql, "json_extract(\"fields\", '$.author.name')");
}

#[test]
fn sqlite_set_uses_lowercase_json_set() {
    let sql = json_field_set(DatabaseType::Sqlite, "fields", "author.name", "?").unwrap();
    assert_eq!(sql, "json_set(\"fields\", '$.author.name', ?)");
}

#[test]
fn single_segment_path_emits_one_operator() {
    let sql = json_field_query(DatabaseType::Postgres, "fields", "title").unwrap();
    assert_eq!(sql, "\"fields\"->'title'");
}

#[test]
fn illegal_path_segment_is_rejected() {
    let err = json_field_query(
        DatabaseType::Postgres,
        "fields",
        "author'); DROP TABLE users;--",
    )
    .unwrap_err();
    assert!(matches!(err, JsonPathError::IllegalPathSegment { .. }));
}

#[test]
fn illegal_column_is_rejected() {
    let err = json_field_query(DatabaseType::Postgres, "fields\"; DROP--", "x").unwrap_err();
    assert!(matches!(err, JsonPathError::IllegalColumn { .. }));
}

#[test]
fn empty_path_is_rejected() {
    assert!(matches!(
        json_field_query(DatabaseType::Postgres, "fields", "").unwrap_err(),
        JsonPathError::EmptyPath
    ));
}

#[test]
fn empty_column_is_rejected() {
    assert!(matches!(
        json_field_query(DatabaseType::Postgres, "", "x").unwrap_err(),
        JsonPathError::EmptyColumn
    ));
}
