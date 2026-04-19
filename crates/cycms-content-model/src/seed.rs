//! 默认 Content Type 种子：`page`（Single）+ `post`（Collection）。
//!
//! 幂等：若目标 `api_id` 已存在则跳过，不抛错，保证多次调用安全（例如 `cycms serve`
//! 重启 / `cycms seed` 重跑）。不走 SQL 迁移，以便未来调整默认结构时无需新迁移文件。

use cycms_core::Result;
use tracing::info;

use crate::model::{
    ContentTypeDefinition, ContentTypeKind, CreateContentTypeInput, FieldDefinition, FieldType,
    ValidationRule,
};
use crate::service::ContentModelRegistry;

/// 写入默认 `page` / `post` 类型，返回最终存在的两条定义（无论是本次新建还是历史已存在）。
///
/// # Errors
/// - 校验失败 → [`crate::error::ContentModelError::InvalidField`]
/// - DB 故障 → [`cycms_core::Error::Internal`]
pub async fn seed_default_types(
    registry: &ContentModelRegistry,
) -> Result<Vec<ContentTypeDefinition>> {
    let mut out = Vec::with_capacity(2);
    for input in [page_input(), post_input()] {
        out.push(ensure_type(registry, input).await?);
    }
    Ok(out)
}

async fn ensure_type(
    registry: &ContentModelRegistry,
    input: CreateContentTypeInput,
) -> Result<ContentTypeDefinition> {
    if let Some(existing) = registry.get_type(&input.api_id).await? {
        return Ok(existing);
    }
    let created = registry.create_type(input).await?;
    info!(content_type = %created.api_id, "seeded default content type");
    Ok(created)
}

fn page_input() -> CreateContentTypeInput {
    CreateContentTypeInput {
        name: "Page".into(),
        api_id: "page".into(),
        description: Some("Static page with unique slug".into()),
        kind: ContentTypeKind::Single,
        fields: vec![
            FieldDefinition {
                name: "Title".into(),
                api_id: "title".into(),
                field_type: FieldType::Text,
                required: true,
                unique: false,
                default_value: None,
                validations: vec![ValidationRule::MaxLength { value: 255 }],
                position: 0,
            },
            FieldDefinition {
                name: "Slug".into(),
                api_id: "slug".into(),
                field_type: FieldType::Text,
                required: true,
                unique: true,
                default_value: None,
                validations: vec![ValidationRule::MaxLength { value: 255 }],
                position: 1,
            },
            FieldDefinition {
                name: "Body".into(),
                api_id: "body".into(),
                field_type: FieldType::RichText,
                required: false,
                unique: false,
                default_value: None,
                validations: vec![],
                position: 2,
            },
            FieldDefinition {
                name: "Published At".into(),
                api_id: "published_at".into(),
                field_type: FieldType::DateTime,
                required: false,
                unique: false,
                default_value: None,
                validations: vec![],
                position: 3,
            },
        ],
    }
}

fn post_input() -> CreateContentTypeInput {
    CreateContentTypeInput {
        name: "Post".into(),
        api_id: "post".into(),
        description: Some("Blog post collection".into()),
        kind: ContentTypeKind::Collection,
        fields: vec![
            FieldDefinition {
                name: "Title".into(),
                api_id: "title".into(),
                field_type: FieldType::Text,
                required: true,
                unique: false,
                default_value: None,
                validations: vec![ValidationRule::MaxLength { value: 255 }],
                position: 0,
            },
            FieldDefinition {
                name: "Slug".into(),
                api_id: "slug".into(),
                field_type: FieldType::Text,
                required: true,
                unique: true,
                default_value: None,
                validations: vec![
                    ValidationRule::MaxLength { value: 255 },
                    ValidationRule::Regex {
                        pattern: "^[a-z0-9-]+$".into(),
                    },
                ],
                position: 1,
            },
            FieldDefinition {
                name: "Summary".into(),
                api_id: "summary".into(),
                field_type: FieldType::Text,
                required: false,
                unique: false,
                default_value: None,
                validations: vec![ValidationRule::MaxLength { value: 500 }],
                position: 2,
            },
            FieldDefinition {
                name: "Body".into(),
                api_id: "body".into(),
                field_type: FieldType::RichText,
                required: true,
                unique: false,
                default_value: None,
                validations: vec![],
                position: 3,
            },
            FieldDefinition {
                name: "Published At".into(),
                api_id: "published_at".into(),
                field_type: FieldType::DateTime,
                required: false,
                unique: false,
                default_value: None,
                validations: vec![],
                position: 4,
            },
        ],
    }
}
