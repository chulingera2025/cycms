use chrono::{DateTime, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use uuid::Uuid;

use crate::claims::{AuthClaims, TokenType};
use crate::error::AuthError;

/// 一对颁发成功的 access/refresh Token。`expires_in` 遵循 `OAuth2` 惯例，
/// 表示 access token 的剩余秒数。
#[derive(Debug, Clone)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

/// `issue_pair` 的完整结果，额外暴露 refresh 的 jti 与过期时刻，
/// 便于 refresh 旋转时写入 `revoked_tokens` 表。
#[derive(Debug, Clone)]
pub struct IssuedTokens {
    pub pair: TokenPair,
    pub refresh_jti: String,
    pub refresh_expires_at: DateTime<Utc>,
}

/// HS256 JWT 编解码器；构造时固化 secret 与 TTL，线程安全可克隆。
pub struct JwtCodec {
    encoding: EncodingKey,
    decoding: DecodingKey,
    access_ttl_secs: i64,
    refresh_ttl_secs: i64,
    validation: Validation,
}

impl JwtCodec {
    /// 使用对称密钥构造 codec；secret 字面量即 `AuthConfig.jwt_secret`。
    #[must_use]
    pub fn new(secret: &str, access_ttl_secs: u64, refresh_ttl_secs: u64) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret.as_bytes()),
            decoding: DecodingKey::from_secret(secret.as_bytes()),
            access_ttl_secs: i64::try_from(access_ttl_secs).unwrap_or(i64::MAX),
            refresh_ttl_secs: i64::try_from(refresh_ttl_secs).unwrap_or(i64::MAX),
            validation: Validation::new(Algorithm::HS256),
        }
    }

    /// 颁发 access + refresh token 对；jti 使用 UUID v4。
    ///
    /// # Errors
    /// 签名失败返回 [`AuthError::Jwt`]。
    pub fn issue_pair(
        &self,
        user_id: &str,
        roles: Vec<String>,
    ) -> Result<IssuedTokens, AuthError> {
        let now = Utc::now();
        let now_ts = now.timestamp();
        let access_exp_ts = now_ts.saturating_add(self.access_ttl_secs);
        let refresh_exp_ts = now_ts.saturating_add(self.refresh_ttl_secs);

        let access_claims = AuthClaims {
            sub: user_id.to_owned(),
            exp: access_exp_ts,
            iat: now_ts,
            jti: Uuid::new_v4().to_string(),
            token_type: TokenType::Access,
            roles: roles.clone(),
        };
        let refresh_jti = Uuid::new_v4().to_string();
        let refresh_claims = AuthClaims {
            sub: user_id.to_owned(),
            exp: refresh_exp_ts,
            iat: now_ts,
            jti: refresh_jti.clone(),
            token_type: TokenType::Refresh,
            roles,
        };

        let header = Header::new(Algorithm::HS256);
        let access_token = encode(&header, &access_claims, &self.encoding)?;
        let refresh_token = encode(&header, &refresh_claims, &self.encoding)?;

        let refresh_expires_at =
            DateTime::<Utc>::from_timestamp(refresh_exp_ts, 0).unwrap_or(now);
        Ok(IssuedTokens {
            pair: TokenPair {
                access_token,
                refresh_token,
                expires_in: u64::try_from(self.access_ttl_secs).unwrap_or(0),
            },
            refresh_jti,
            refresh_expires_at,
        })
    }

    /// 解码并校验 token，期望的 `token_type` 不匹配返回 [`AuthError::TokenTypeMismatch`]。
    ///
    /// # Errors
    /// - 签名非法 / base64 损坏 → [`AuthError::TokenInvalid`]
    /// - 过期 → [`AuthError::TokenExpired`]
    /// - 类型不符 → [`AuthError::TokenTypeMismatch`]
    pub fn decode(&self, token: &str, expected: TokenType) -> Result<AuthClaims, AuthError> {
        let data = decode::<AuthClaims>(token, &self.decoding, &self.validation).map_err(
            |err| match err.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
                _ => AuthError::TokenInvalid,
            },
        )?;
        if data.claims.token_type != expected {
            return Err(AuthError::TokenTypeMismatch);
        }
        Ok(data.claims)
    }
}

#[cfg(test)]
mod tests {
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

    use super::{JwtCodec, TokenType};
    use crate::claims::AuthClaims;
    use crate::error::AuthError;

    fn codec() -> JwtCodec {
        JwtCodec::new("test-secret", 900, 1_209_600)
    }

    #[test]
    fn issue_then_decode_access_round_trip() {
        let codec = codec();
        let issued = codec
            .issue_pair("user-1", vec!["admin".to_owned(), "editor".to_owned()])
            .unwrap();
        assert_eq!(issued.pair.expires_in, 900);

        let claims = codec.decode(&issued.pair.access_token, TokenType::Access).unwrap();
        assert_eq!(claims.sub, "user-1");
        assert_eq!(claims.token_type, TokenType::Access);
        assert_eq!(claims.roles, vec!["admin".to_owned(), "editor".to_owned()]);
    }

    #[test]
    fn issue_then_decode_refresh_round_trip() {
        let codec = codec();
        let issued = codec.issue_pair("user-1", vec![]).unwrap();
        let claims = codec.decode(&issued.pair.refresh_token, TokenType::Refresh).unwrap();
        assert_eq!(claims.jti, issued.refresh_jti);
        assert_eq!(claims.token_type, TokenType::Refresh);
    }

    #[test]
    fn access_token_rejected_when_expecting_refresh() {
        let codec = codec();
        let issued = codec.issue_pair("user-1", vec![]).unwrap();
        let err = codec
            .decode(&issued.pair.access_token, TokenType::Refresh)
            .unwrap_err();
        assert!(matches!(err, AuthError::TokenTypeMismatch));
    }

    #[test]
    fn tampered_token_rejected() {
        let codec = codec();
        let issued = codec.issue_pair("user-1", vec![]).unwrap();
        // 把最后一个 base64 字符替换为另一个合法字符，破坏签名
        let mut tampered = issued.pair.access_token;
        let last = tampered.pop().unwrap();
        tampered.push(if last == 'A' { 'B' } else { 'A' });
        let err = codec.decode(&tampered, TokenType::Access).unwrap_err();
        assert!(matches!(err, AuthError::TokenInvalid));
    }

    #[test]
    fn expired_token_rejected_as_expired() {
        let codec = codec();
        let past_ts = chrono::Utc::now().timestamp() - 3_600;
        let expired = AuthClaims {
            sub: "u".to_owned(),
            exp: past_ts,
            iat: past_ts - 10,
            jti: "j".to_owned(),
            token_type: TokenType::Access,
            roles: vec![],
        };
        let secret = "test-secret";
        let token = encode(
            &Header::new(Algorithm::HS256),
            &expired,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap();
        let err = codec.decode(&token, TokenType::Access).unwrap_err();
        assert!(matches!(err, AuthError::TokenExpired));
    }

    #[test]
    fn wrong_secret_produces_invalid_error() {
        let codec_a = JwtCodec::new("secret-a", 900, 1_209_600);
        let codec_b = JwtCodec::new("secret-b", 900, 1_209_600);
        let issued = codec_a.issue_pair("user-1", vec![]).unwrap();
        let err = codec_b
            .decode(&issued.pair.access_token, TokenType::Access)
            .unwrap_err();
        assert!(matches!(err, AuthError::TokenInvalid));
    }
}
