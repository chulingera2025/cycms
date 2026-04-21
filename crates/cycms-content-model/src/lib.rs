//! cycms-content-model —— 内容类型定义、字段校验与 Schema 输出。
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
//! - [`repository`]：内容类型元数据持久化与兼容性检查辅助查询；
//! - [`validation`]：字段定义与实例值校验；
//! - [`schema`]：内容类型到 JSON Schema/OpenAPI 片段转换；
//! - [`field_type`]：自定义字段类型注册点；
//! - [`service`]：Req 7/8 的编排入口；
//! - [`seed`]：系统默认类型初始化。

mod error;
mod field_type;
mod model;
mod repository;
mod schema;
mod seed;
mod service;
mod validation;

pub use error::{ContentModelError, FieldViolation};
pub use field_type::{FieldTypeHandler, FieldTypeRegistry};
pub use model::{
    ContentTypeDefinition, ContentTypeKind, CreateContentTypeInput, FieldDefinition, FieldType,
    RelationKind, UpdateContentTypeInput, ValidationRule,
};
pub use repository::{
    ContentTypeRepository, NewContentTypeRow, UpdateContentTypeRow, new_content_type_id,
    normalize_api_id, normalize_name, validate_api_id,
};
pub use schema::{field_to_schema, to_json_schema};
pub use seed::seed_default_types;
pub use service::ContentModelRegistry;
pub use validation::{validate_field, validate_field_definitions, validate_fields};
