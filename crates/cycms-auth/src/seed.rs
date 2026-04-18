use serde::{Deserialize, Serialize};

use crate::error::AuthError;
use crate::password::PasswordPolicy;

/// 创建用户的标准入参；`create_user` 与 `setup_admin` 均以此为输入。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserInput {
    pub username: String,
    pub email: String,
    pub password: String,
}

impl CreateUserInput {
    /// 基础字段校验 + 密码策略校验。
    ///
    /// # Errors
    /// - username 空 / email 非法 → [`AuthError::InputValidation`]
    /// - 密码策略违规 → [`AuthError::PasswordPolicy`]
    pub fn validate(&self) -> Result<(), AuthError> {
        if self.username.trim().is_empty() {
            return Err(AuthError::InputValidation(
                "username must not be empty".to_owned(),
            ));
        }
        if !looks_like_email(&self.email) {
            return Err(AuthError::InputValidation(
                "email must contain a local part and a domain".to_owned(),
            ));
        }
        PasswordPolicy::validate(&self.password)?;
        Ok(())
    }
}

fn looks_like_email(value: &str) -> bool {
    let Some(at) = value.find('@') else {
        return false;
    };
    let local = &value[..at];
    let domain = &value[at + 1..];
    !local.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
}

#[cfg(test)]
mod tests {
    use super::{CreateUserInput, looks_like_email};

    #[test]
    fn looks_like_email_accepts_basic_addresses() {
        assert!(looks_like_email("alice@example.com"));
        assert!(looks_like_email("a.b@sub.example.com"));
    }

    #[test]
    fn looks_like_email_rejects_malformed() {
        assert!(!looks_like_email("no-at-sign"));
        assert!(!looks_like_email("@example.com"));
        assert!(!looks_like_email("alice@nodot"));
        assert!(!looks_like_email("alice@.com"));
        assert!(!looks_like_email("alice@example."));
    }

    #[test]
    fn validate_requires_non_empty_username() {
        let err = CreateUserInput {
            username: "   ".to_owned(),
            email: "a@b.com".to_owned(),
            password: "StrongPass1!".to_owned(),
        }
        .validate()
        .unwrap_err();
        assert!(matches!(err, crate::AuthError::InputValidation(_)));
    }

    #[test]
    fn validate_requires_valid_email() {
        let err = CreateUserInput {
            username: "alice".to_owned(),
            email: "bogus".to_owned(),
            password: "StrongPass1!".to_owned(),
        }
        .validate()
        .unwrap_err();
        assert!(matches!(err, crate::AuthError::InputValidation(_)));
    }

    #[test]
    fn validate_enforces_password_policy() {
        let err = CreateUserInput {
            username: "alice".to_owned(),
            email: "a@b.com".to_owned(),
            password: "short".to_owned(),
        }
        .validate()
        .unwrap_err();
        assert!(matches!(err, crate::AuthError::PasswordPolicy(_)));
    }

    #[test]
    fn validate_accepts_strong_input() {
        CreateUserInput {
            username: "alice".to_owned(),
            email: "a@b.com".to_owned(),
            password: "StrongPass1!".to_owned(),
        }
        .validate()
        .unwrap();
    }
}
