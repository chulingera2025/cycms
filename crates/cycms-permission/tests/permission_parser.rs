use cycms_core::Error;
use cycms_permission::parse_permission_code;

#[test]
fn happy_path_round_trip_with_all_character_classes() {
    let parsed = parse_permission_code("content.article_draft.publish_2").unwrap();
    assert_eq!(parsed.domain, "content");
    assert_eq!(parsed.resource, "article_draft");
    assert_eq!(parsed.action, "publish_2");
}

#[test]
fn rejects_leading_or_trailing_whitespace() {
    // 解析器不做 trim，留给调用方归一
    assert!(parse_permission_code(" system.post.read").is_err());
    assert!(parse_permission_code("system.post.read ").is_err());
}

#[test]
fn rejects_empty_and_dot_only() {
    assert!(parse_permission_code("").is_err());
    assert!(parse_permission_code(".").is_err());
    assert!(parse_permission_code("..").is_err());
}

#[test]
fn rejects_segments_over_100_chars() {
    let long_domain: String = "a".repeat(101);
    let code = format!("{long_domain}.post.read");
    let err = parse_permission_code(&code).unwrap_err();
    assert!(matches!(
        err,
        cycms_permission::PermissionError::InputValidation(_)
    ));
}

#[test]
fn parser_error_maps_to_core_validation_error() {
    let err: Error = parse_permission_code("INVALID.x.y").unwrap_err().into();
    assert!(matches!(err, Error::ValidationError { .. }));
}
