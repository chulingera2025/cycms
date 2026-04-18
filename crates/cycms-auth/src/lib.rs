//! `cycms-auth` —— 认证引擎 crate（任务 5）。
//!
//! 覆盖 Requirements 1.1–1.6：登录、Token 颁发/刷新、初始管理员、axum 认证中间件。
//! 子系统在后续子任务中逐步填充。

mod error;

pub use error::AuthError;
