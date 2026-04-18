use argon2::{Algorithm, Argon2, Params, Version};
use cycms_config::Argon2Config;
use password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use rand::rngs::OsRng;

use crate::error::AuthError;

/// 通过 [`Argon2Config`] 配置的 Argon2id 对明文密码哈希，返回 PHC 字符串。
///
/// # Errors
/// - Argon2 参数非法 → [`AuthError::PasswordHash`]
/// - 哈希过程失败 → [`AuthError::PasswordHash`]
pub fn hash_password(plain: &str, cfg: &Argon2Config) -> Result<String, AuthError> {
    let params = Params::new(cfg.m_cost, cfg.t_cost, cfg.p_cost, None)
        .map_err(|err| AuthError::PasswordHash(format!("invalid argon2 params: {err}")))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let salt = SaltString::generate(&mut OsRng);
    argon2
        .hash_password(plain.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|err| AuthError::PasswordHash(format!("argon2 hash failed: {err}")))
}

/// 校验明文密码是否与 PHC 字符串匹配。
///
/// 密码不匹配返回 `Ok(false)`；PHC 解析失败返回 [`AuthError::PasswordHash`]。
///
/// # Errors
/// PHC 格式非法或 Argon2 参数解析失败时返回 [`AuthError::PasswordHash`]。
pub fn verify_password(plain: &str, phc: &str) -> Result<bool, AuthError> {
    let parsed = PasswordHash::new(phc)
        .map_err(|err| AuthError::PasswordHash(format!("invalid phc string: {err}")))?;
    match Argon2::default().verify_password(plain.as_bytes(), &parsed) {
        Ok(()) => Ok(true),
        Err(password_hash::Error::Password) => Ok(false),
        Err(other) => Err(AuthError::PasswordHash(format!(
            "argon2 verify failed: {other}"
        ))),
    }
}

/// 最小密码策略：长度 ≥ 10 且至少包含三类字符（小写 / 大写 / 数字 / 符号）。
pub struct PasswordPolicy;

impl PasswordPolicy {
    /// 校验明文密码，不满足要求时返回 [`AuthError::PasswordPolicy`]。
    ///
    /// # Errors
    /// 长度不足或字符类别少于三类时返回对应的可读错误。
    pub fn validate(plain: &str) -> Result<(), AuthError> {
        if plain.chars().count() < 10 {
            return Err(AuthError::PasswordPolicy(
                "password must be at least 10 characters".to_owned(),
            ));
        }

        let mut has_lower = false;
        let mut has_upper = false;
        let mut has_digit = false;
        let mut has_symbol = false;
        for ch in plain.chars() {
            if ch.is_ascii_lowercase() {
                has_lower = true;
            } else if ch.is_ascii_uppercase() {
                has_upper = true;
            } else if ch.is_ascii_digit() {
                has_digit = true;
            } else {
                has_symbol = true;
            }
        }

        let classes = usize::from(has_lower)
            + usize::from(has_upper)
            + usize::from(has_digit)
            + usize::from(has_symbol);
        if classes < 3 {
            return Err(AuthError::PasswordPolicy(
                "password must include at least 3 of [lowercase, uppercase, digit, symbol]"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{PasswordPolicy, hash_password, verify_password};
    use cycms_config::Argon2Config;

    fn fast_cfg() -> Argon2Config {
        // 测试用小参数加速；不要在生产使用这些值
        Argon2Config {
            m_cost: 16,
            t_cost: 2,
            p_cost: 1,
        }
    }

    #[test]
    fn hash_then_verify_round_trip() {
        let cfg = fast_cfg();
        let phc = hash_password("CorrectHorse42!", &cfg).unwrap();
        assert!(phc.starts_with("$argon2id$"));
        assert!(verify_password("CorrectHorse42!", &phc).unwrap());
    }

    #[test]
    fn wrong_password_returns_false() {
        let cfg = fast_cfg();
        let phc = hash_password("CorrectHorse42!", &cfg).unwrap();
        assert!(!verify_password("WrongHorse42!", &phc).unwrap());
    }

    #[test]
    fn corrupted_phc_returns_error() {
        let err = verify_password("whatever", "$invalid$phc$string").unwrap_err();
        assert!(matches!(err, crate::AuthError::PasswordHash(_)));
    }

    #[test]
    fn invalid_argon2_params_return_hash_error() {
        let bad = Argon2Config {
            m_cost: 0,
            t_cost: 0,
            p_cost: 0,
        };
        let err = hash_password("AnyValid123!", &bad).unwrap_err();
        assert!(
            matches!(err, crate::AuthError::PasswordHash(_)),
            "got: {err:?}"
        );
    }

    #[test]
    fn policy_accepts_strong_password() {
        PasswordPolicy::validate("StrongPass1!").unwrap();
        PasswordPolicy::validate("Abc123!!def").unwrap();
    }

    #[test]
    fn policy_rejects_short_password() {
        let err = PasswordPolicy::validate("Short1!").unwrap_err();
        assert!(matches!(err, crate::AuthError::PasswordPolicy(_)));
    }

    #[test]
    fn policy_rejects_two_classes_only() {
        // 仅小写 + 数字，缺少第三类
        let err = PasswordPolicy::validate("onlylowercase123").unwrap_err();
        assert!(matches!(err, crate::AuthError::PasswordPolicy(_)));
    }
}
