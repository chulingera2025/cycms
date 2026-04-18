//! `cycms-db` 提供统一的数据库连接池抽象与方言适配辅助。
//!
//! - [`DatabasePool`] 封装三种 sqlx 连接池，保证上层代码以同一枚举操作 PG / `MySQL` / `SQLite`。
//! - JSON 辅助函数按方言生成片段，上层服务只需关心业务路径而非 SQL 语法差异。

mod error;
mod json;
mod pool;

pub use error::map_sqlx_error;
pub use json::{JsonPathError, json_field_query, json_field_set};
pub use pool::{DatabasePool, DatabaseType};
