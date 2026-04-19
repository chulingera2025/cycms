//! cycms-content-model —— 内容类型定义、字段校验与 Schema 输出（任务 10）。
//!
//! 覆盖 Requirements 3.1 / 3.2 / 3.3 / 3.4 / 3.5 / 3.6：
//! - [`ContentTypeDefinition`] 持久化到 `content_types` 表，`fields` JSON 列；
//! - [`validation`] 模块执行 required / 类型形状 / 规则链 的字段级校验；
//! - [`schema`] 模块从 `ContentTypeDefinition` 生成 OpenAPI/JSON Schema 片段；
//! - [`field_type::FieldTypeRegistry`] 提供插件自定义字段类型与校验器的进程内注册能力。
//!
//! 模块结构：
//! - [`error`]：`ContentModelError` 枚举 + 跨 crate 映射到 `cycms_core::Error`；
//! - [`model`]：所有数据结构（`ContentTypeDefinition` / `FieldDefinition` / ...）；
//! - TODO!!!: 后续提交依次加入 `repository` / `validation` / `schema` / `field_type` / `service` / `seed`。

mod error;
mod model;

pub use error::{ContentModelError, FieldViolation};
pub use model::{
    ContentTypeDefinition, ContentTypeKind, CreateContentTypeInput, FieldDefinition, FieldType,
    RelationKind, UpdateContentTypeInput, ValidationRule,
};
