//! 默认 Content Type 种子：博客预设。
//!
//! 幂等：若目标 `api_id` 已存在则跳过，不抛错，保证多次调用安全（例如 `cycms serve`
//! 重启 / `cycms seed` 重跑）。不走 SQL 迁移，以便未来调整默认结构时无需新迁移文件。

use cycms_core::Result;
use serde_json::json;
use tracing::info;

use crate::model::{
    ContentTypeDefinition, ContentTypeKind, CreateContentTypeInput, FieldDefinition, FieldType,
    RelationKind, ValidationRule,
};
use crate::service::ContentModelRegistry;

const IMAGE_TYPES: &[&str] = &["image/png", "image/jpeg", "image/webp", "image/gif"];

/// 写入默认博客预设，返回最终存在的所有定义（无论是本次新建还是历史已存在）。
///
/// # Errors
/// - 校验失败 → [`crate::error::ContentModelError::InvalidField`]
/// - DB 故障 → [`cycms_core::Error::Internal`]
pub async fn seed_default_types(
    registry: &ContentModelRegistry,
) -> Result<Vec<ContentTypeDefinition>> {
    let mut out = Vec::with_capacity(5);
    for input in [
        category_input(),
        tag_input(),
        page_input(),
        post_input(),
        site_settings_input(),
    ] {
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

fn category_input() -> CreateContentTypeInput {
    CreateContentTypeInput {
        name: "Category".into(),
        api_id: "category".into(),
        description: Some("Blog categories used to group posts".into()),
        kind: ContentTypeKind::Collection,
        fields: vec![
            text_field("Name", "name", true, 0, vec![ValidationRule::MaxLength { value: 120 }]),
            text_field(
                "Description",
                "description",
                false,
                1,
                vec![ValidationRule::MaxLength { value: 500 }],
            ),
            media_field("Cover Image", "cover_image", 2),
            text_field(
                "SEO Title",
                "seo_title",
                false,
                3,
                vec![ValidationRule::MaxLength { value: 255 }],
            ),
            text_field(
                "SEO Description",
                "seo_description",
                false,
                4,
                vec![ValidationRule::MaxLength { value: 320 }],
            ),
        ],
    }
}

fn tag_input() -> CreateContentTypeInput {
    CreateContentTypeInput {
        name: "Tag".into(),
        api_id: "tag".into(),
        description: Some("Blog tags used for lightweight topic grouping".into()),
        kind: ContentTypeKind::Collection,
        fields: vec![
            text_field("Name", "name", true, 0, vec![ValidationRule::MaxLength { value: 120 }]),
            text_field(
                "Description",
                "description",
                false,
                1,
                vec![ValidationRule::MaxLength { value: 255 }],
            ),
        ],
    }
}

fn page_input() -> CreateContentTypeInput {
    CreateContentTypeInput {
        name: "Page".into(),
        api_id: "page".into(),
        description: Some("Static pages for site navigation and evergreen content".into()),
        kind: ContentTypeKind::Collection,
        fields: vec![
            text_field("Title", "title", true, 0, vec![ValidationRule::MaxLength { value: 255 }]),
            media_field("Cover Image", "cover_image", 1),
            richtext_field("Body", "body", true, 2),
            text_field(
                "SEO Title",
                "seo_title",
                false,
                3,
                vec![ValidationRule::MaxLength { value: 255 }],
            ),
            text_field(
                "SEO Description",
                "seo_description",
                false,
                4,
                vec![ValidationRule::MaxLength { value: 320 }],
            ),
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
            text_field("Title", "title", true, 0, vec![ValidationRule::MaxLength { value: 255 }]),
            text_field(
                "Excerpt",
                "excerpt",
                false,
                1,
                vec![ValidationRule::MaxLength { value: 500 }],
            ),
            media_field("Cover Image", "cover_image", 2),
            richtext_field("Body", "body", true, 3),
            relation_field(
                "Categories",
                "categories",
                "category",
                RelationKind::ManyToMany,
                false,
                4,
            ),
            relation_field(
                "Tags",
                "tags",
                "tag",
                RelationKind::ManyToMany,
                false,
                5,
            ),
            boolean_field("Featured", "featured", false, 6, Some(false)),
            text_field(
                "SEO Title",
                "seo_title",
                false,
                7,
                vec![ValidationRule::MaxLength { value: 255 }],
            ),
            text_field(
                "SEO Description",
                "seo_description",
                false,
                8,
                vec![ValidationRule::MaxLength { value: 320 }],
            ),
        ],
    }
}

fn site_settings_input() -> CreateContentTypeInput {
    CreateContentTypeInput {
        name: "Site Settings".into(),
        api_id: "site_settings".into(),
        description: Some("Global blog chrome and homepage settings".into()),
        kind: ContentTypeKind::Single,
        fields: vec![
            text_field(
                "Site Name",
                "site_name",
                true,
                0,
                vec![ValidationRule::MaxLength { value: 120 }],
            ),
            text_field(
                "Tagline",
                "tagline",
                false,
                1,
                vec![ValidationRule::MaxLength { value: 255 }],
            ),
            media_field("Logo", "logo", 2),
            text_field(
                "Hero Title",
                "hero_title",
                false,
                3,
                vec![ValidationRule::MaxLength { value: 255 }],
            ),
            text_field(
                "Hero Subtitle",
                "hero_subtitle",
                false,
                4,
                vec![ValidationRule::MaxLength { value: 500 }],
            ),
            text_field(
                "Footer Text",
                "footer_text",
                false,
                5,
                vec![ValidationRule::MaxLength { value: 255 }],
            ),
            relation_field(
                "Featured Posts",
                "featured_posts",
                "post",
                RelationKind::OneToMany,
                false,
                6,
            ),
        ],
    }
}

fn text_field(
    name: &str,
    api_id: &str,
    required: bool,
    position: i32,
    validations: Vec<ValidationRule>,
) -> FieldDefinition {
    FieldDefinition {
        name: name.into(),
        api_id: api_id.into(),
        field_type: FieldType::Text,
        required,
        unique: false,
        default_value: None,
        validations,
        position,
    }
}

fn richtext_field(name: &str, api_id: &str, required: bool, position: i32) -> FieldDefinition {
    FieldDefinition {
        name: name.into(),
        api_id: api_id.into(),
        field_type: FieldType::RichText,
        required,
        unique: false,
        default_value: None,
        validations: vec![],
        position,
    }
}

fn media_field(name: &str, api_id: &str, position: i32) -> FieldDefinition {
    FieldDefinition {
        name: name.into(),
        api_id: api_id.into(),
        field_type: FieldType::Media {
            allowed_types: IMAGE_TYPES.iter().map(|mime| (*mime).to_owned()).collect(),
        },
        required: false,
        unique: false,
        default_value: None,
        validations: vec![],
        position,
    }
}

fn relation_field(
    name: &str,
    api_id: &str,
    target_type: &str,
    relation_kind: RelationKind,
    required: bool,
    position: i32,
) -> FieldDefinition {
    FieldDefinition {
        name: name.into(),
        api_id: api_id.into(),
        field_type: FieldType::Relation {
            target_type: target_type.into(),
            relation_kind,
        },
        required,
        unique: false,
        default_value: None,
        validations: vec![],
        position,
    }
}

fn boolean_field(
    name: &str,
    api_id: &str,
    required: bool,
    position: i32,
    default_value: Option<bool>,
) -> FieldDefinition {
    FieldDefinition {
        name: name.into(),
        api_id: api_id.into(),
        field_type: FieldType::Boolean,
        required,
        unique: false,
        default_value: default_value.map(|value| json!(value)),
        validations: vec![],
        position,
    }
}
