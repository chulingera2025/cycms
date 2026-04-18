use serde::{Deserialize, Serialize};

/// Token 分类；access 用于普通 API 调用，refresh 用于续签。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    Access,
    Refresh,
}

/// JWT Claims 载荷，字段严格按 design.md `§AuthClaims` 约定扩展 jti 与 `token_type`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthClaims {
    pub sub: String,
    pub exp: i64,
    pub iat: i64,
    pub jti: String,
    pub token_type: TokenType,
    pub roles: Vec<String>,
}
