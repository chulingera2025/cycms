//! `cycms-auth` —— 认证引擎 crate（任务 5）。
//!
//! 覆盖 Requirements 1.1–1.6：登录、Token 颁发/刷新、初始管理员、axum 认证中间件。
//! 子系统在后续子任务中逐步填充。

mod claims;
mod error;
mod password;
mod revoked;
mod seed;
mod service;
mod token;
mod user;

pub use claims::{AuthClaims, TokenType};
pub use error::AuthError;
pub use password::{PasswordPolicy, hash_password, verify_password};
pub use revoked::RevokedTokenRepository;
pub use seed::CreateUserInput;
pub use service::{AuthEngine, LoginRequest};
pub use token::{IssuedTokens, JwtCodec, TokenPair};
pub use user::{NewUserRow, User, UserRepository};
