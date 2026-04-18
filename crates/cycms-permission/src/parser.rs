use crate::error::PermissionError;

/// 解析后的三段式权限代码：`domain.resource.action`。使用借用以零分配返回。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedCode<'a> {
    pub domain: &'a str,
    pub resource: &'a str,
    pub action: &'a str,
}

/// 每段的最大长度，与 migrations 中 `VARCHAR(100)` 对齐。
const SEGMENT_MAX: usize = 100;

/// 解析 `domain.resource.action` 形式的权限代码。
///
/// 约束：
/// - 必须由恰好 2 个 `.` 分隔为 3 段，每段长度 1–100；
/// - 允许字符仅为 `[a-z0-9_]`；大写、空格、通配符、Unicode 均被拒绝；
/// - 不做 trim，调用方在写入存储前应自行归一，防止隐性空格。
///
/// v0.1 故意不支持 `*` 通配符，以保持匹配行为简单明确。
///
/// # Errors
/// 任何违反上述约束的输入都会返回 [`PermissionError::InputValidation`]。
pub fn parse_permission_code(code: &str) -> Result<ParsedCode<'_>, PermissionError> {
    let mut parts = code.split('.');
    let domain = parts
        .next()
        .ok_or_else(|| invalid(code, "missing domain segment"))?;
    let resource = parts
        .next()
        .ok_or_else(|| invalid(code, "missing resource segment"))?;
    let action = parts
        .next()
        .ok_or_else(|| invalid(code, "missing action segment"))?;
    if parts.next().is_some() {
        return Err(invalid(code, "too many segments, expected exactly 3"));
    }

    validate_segment("domain", domain)?;
    validate_segment("resource", resource)?;
    validate_segment("action", action)?;

    Ok(ParsedCode {
        domain,
        resource,
        action,
    })
}

fn validate_segment(label: &str, segment: &str) -> Result<(), PermissionError> {
    if segment.is_empty() {
        return Err(PermissionError::InputValidation(format!(
            "{label} segment must not be empty"
        )));
    }
    if segment.len() > SEGMENT_MAX {
        return Err(PermissionError::InputValidation(format!(
            "{label} segment exceeds {SEGMENT_MAX} chars"
        )));
    }
    for c in segment.chars() {
        if !matches!(c, 'a'..='z' | '0'..='9' | '_') {
            return Err(PermissionError::InputValidation(format!(
                "{label} segment contains invalid character {c:?}; allowed: [a-z0-9_]"
            )));
        }
    }
    Ok(())
}

fn invalid(code: &str, reason: &str) -> PermissionError {
    PermissionError::InputValidation(format!("invalid permission code {code:?}: {reason}"))
}

#[cfg(test)]
mod tests {
    use super::parse_permission_code;

    #[test]
    fn accepts_basic_three_segments() {
        let parsed = parse_permission_code("system.post.read").unwrap();
        assert_eq!(parsed.domain, "system");
        assert_eq!(parsed.resource, "post");
        assert_eq!(parsed.action, "read");
    }

    #[test]
    fn accepts_underscore_and_digits() {
        let parsed = parse_permission_code("plugin_blog.post_v2.read_all").unwrap();
        assert_eq!(parsed.domain, "plugin_blog");
        assert_eq!(parsed.resource, "post_v2");
        assert_eq!(parsed.action, "read_all");
    }

    #[test]
    fn rejects_wildcard() {
        assert!(parse_permission_code("system.*.read").is_err());
        assert!(parse_permission_code("system.post.*").is_err());
    }

    #[test]
    fn rejects_uppercase() {
        assert!(parse_permission_code("System.post.read").is_err());
    }

    #[test]
    fn rejects_empty_segments() {
        assert!(parse_permission_code("").is_err());
        assert!(parse_permission_code("system..read").is_err());
        assert!(parse_permission_code(".post.read").is_err());
        assert!(parse_permission_code("system.post.").is_err());
    }

    #[test]
    fn rejects_too_many_or_too_few_segments() {
        assert!(parse_permission_code("system.post").is_err());
        assert!(parse_permission_code("system.post.read.extra").is_err());
    }

    #[test]
    fn rejects_illegal_characters() {
        assert!(parse_permission_code("system.post.re ad").is_err());
        assert!(parse_permission_code("system.pos-t.read").is_err());
        assert!(parse_permission_code("系统.post.read").is_err());
    }
}
