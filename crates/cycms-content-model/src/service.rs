//! `ContentModelRegistry` 门面：内容类型 CRUD + 字段校验 + Schema 输出编排。
//!
//! 功能边界（Req 3.1 / 3.2 / 3.3 / 3.4 / 3.5 / 3.6）：
//! - `create_type`：字段定义 + `api_id` 校验 → 重复检测 → 持久化。
//! - `update_type`：字段级 diff 日志（v0.1 仅 warn，不阻断；TODO 任务 11 增加 entries 级检查）。
//! - `delete_type`：直接按 `api_id` 删除；TODO 任务 11 检查反向引用。
//! - `validate_entry` / `to_json_schema`：薄封装 `validation` / `schema` 模块。
//! - `register_field_type` / `unregister_field_types_by_prefix`：插件字段类型生命周期。

use std::collections::HashSet;
use std::sync::Arc;

use cycms_core::Result;
use cycms_db::DatabasePool;
use serde_json::Value;
use tracing::warn;

use crate::error::ContentModelError;
use crate::field_type::{FieldTypeHandler, FieldTypeRegistry};
use crate::model::{
    ContentTypeDefinition, CreateContentTypeInput, FieldDefinition, UpdateContentTypeInput,
};
use crate::repository::{
    ContentTypeRepository, NewContentTypeRow, UpdateContentTypeRow, new_content_type_id,
    normalize_api_id, normalize_name,
};
use crate::schema::to_json_schema;
use crate::validation::{validate_field_definitions, validate_fields};

/// Content Type 管理门面。
pub struct ContentModelRegistry {
    repo: ContentTypeRepository,
    field_types: Arc<FieldTypeRegistry>,
}

impl ContentModelRegistry {
    /// 组合 db 与共享的 `FieldTypeRegistry`。
    #[must_use]
    pub fn new(db: Arc<DatabasePool>, field_types: Arc<FieldTypeRegistry>) -> Self {
        Self {
            repo: ContentTypeRepository::new(db),
            field_types,
        }
    }

    /// 语义糖：内部新建独立的 `FieldTypeRegistry`，适合测试或无插件场景。
    #[must_use]
    pub fn with_fresh_registry(db: Arc<DatabasePool>) -> Self {
        Self::new(db, Arc::new(FieldTypeRegistry::new()))
    }

    /// 引用共享的 `FieldTypeRegistry`，供 kernel / 插件运行时注册字段类型。
    #[must_use]
    pub fn field_types(&self) -> &Arc<FieldTypeRegistry> {
        &self.field_types
    }

    /// 创建 Content Type（Req 3.1）。
    ///
    /// # Errors
    /// - 输入字段非法 → [`ContentModelError::InvalidField`] / [`ContentModelError::InputValidation`]
    /// - `api_id` 已存在 → [`ContentModelError::DuplicateApiId`]
    /// - DB 故障 → [`ContentModelError::Database`]
    pub async fn create_type(
        &self,
        input: CreateContentTypeInput,
    ) -> Result<ContentTypeDefinition> {
        let row = self.prepare_row(&input)?;

        if self.repo.find_by_api_id(&row.api_id).await?.is_some() {
            return Err(ContentModelError::DuplicateApiId(row.api_id).into());
        }

        self.repo.insert(row).await
    }

    /// 按 `api_id` 更新 Content Type（Req 3.3）。
    ///
    /// 字段级 diff（添加 / 删除 / 改类型）会记录 `warn!` 日志，v0.1 不阻断；后续任务 11
    /// 集成 `ContentEngine` 时应在此收紧：含实例的字段不允许破坏性变更。
    ///
    /// # Errors
    /// - `api_id` 不存在 → [`ContentModelError::NotFound`]
    /// - 新字段定义非法 → [`ContentModelError::InvalidField`]
    /// - DB 故障 → [`ContentModelError::Database`]
    pub async fn update_type(
        &self,
        api_id: &str,
        input: UpdateContentTypeInput,
    ) -> Result<ContentTypeDefinition> {
        let normalized_api = normalize_api_id(api_id)?;
        let existing = self
            .repo
            .find_by_api_id(&normalized_api)
            .await?
            .ok_or_else(|| ContentModelError::NotFound(normalized_api.clone()))?;

        let name = match input.name {
            Some(raw) => normalize_name(&raw)?,
            None => existing.name.clone(),
        };
        let description = match input.description {
            Some(opt) => opt.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()),
            None => existing.description.clone(),
        };
        let kind = input.kind.unwrap_or(existing.kind);
        let fields = match input.fields {
            Some(fields) => {
                let normalized = normalize_fields(fields)?;
                validate_field_definitions(&normalized, &self.field_types)?;
                log_field_diff(&normalized_api, &existing.fields, &normalized);
                normalized
            }
            None => existing.fields.clone(),
        };

        self.repo
            .update(
                &existing.id,
                UpdateContentTypeRow {
                    name,
                    description,
                    kind,
                    fields,
                },
            )
            .await
    }

    /// 按 `api_id` 删除。TODO!!!: 任务 11 集成 `ContentEngine` 时增加反向引用检查。
    ///
    /// # Errors
    /// DB 故障 → [`ContentModelError::Database`]。
    pub async fn delete_type(&self, api_id: &str) -> Result<bool> {
        let normalized = normalize_api_id(api_id)?;
        match self.repo.find_by_api_id(&normalized).await? {
            Some(def) => self.repo.delete_by_id(&def.id).await,
            None => Ok(false),
        }
    }

    /// 按 `api_id` 查询。
    ///
    /// # Errors
    /// - `api_id` 非法 → [`ContentModelError::InputValidation`]
    /// - DB 故障 → [`ContentModelError::Database`]
    pub async fn get_type(&self, api_id: &str) -> Result<Option<ContentTypeDefinition>> {
        let normalized = normalize_api_id(api_id)?;
        self.repo.find_by_api_id(&normalized).await
    }

    /// 列出全部 Content Type，按 `api_id` 升序。
    ///
    /// # Errors
    /// DB 故障 → [`ContentModelError::Database`]。
    pub async fn list_types(&self) -> Result<Vec<ContentTypeDefinition>> {
        self.repo.list().await
    }

    /// 按 DB `id` 查询，便于与其他子系统（revision / publish）按主键互通。
    ///
    /// # Errors
    /// DB 故障 → [`ContentModelError::Database`]。
    pub async fn find_by_id(&self, id: &str) -> Result<Option<ContentTypeDefinition>> {
        self.repo.find_by_id(id).await
    }

    /// 注册插件字段类型 / 校验器处理器（Req 3.6）。
    pub fn register_field_type(&self, key: &str, handler: Arc<dyn FieldTypeHandler>) {
        self.field_types.register(key, handler);
    }

    /// 按插件名前缀批量卸载（插件禁用 / 卸载时调用），返回受影响数量。
    pub fn unregister_field_types_by_prefix(&self, prefix: &str) -> usize {
        self.field_types.unregister_by_prefix(prefix)
    }

    /// 对给定 `api_id` 的 Content Type 执行 entry 级校验（Req 3.2）。
    ///
    /// # Errors
    /// - `api_id` 不存在 → [`ContentModelError::NotFound`]
    /// - `entry` 非 JSON object → [`ContentModelError::InputValidation`]
    /// - 任一字段不满足 → [`ContentModelError::SchemaViolation`]
    pub async fn validate_entry(&self, api_id: &str, entry: &Value) -> Result<()> {
        let ct = self.require_type(api_id).await?;
        validate_fields(&ct.fields, entry, &self.field_types).map_err(Into::into)
    }

    /// 为 `api_id` 指定的 Content Type 生成 JSON Schema / `OpenAPI` 片段（Req 3.5）。
    ///
    /// # Errors
    /// - `api_id` 不存在 → [`ContentModelError::NotFound`]
    /// - DB 故障 → [`ContentModelError::Database`]
    pub async fn to_json_schema(&self, api_id: &str) -> Result<Value> {
        let ct = self.require_type(api_id).await?;
        Ok(to_json_schema(&ct, &self.field_types))
    }

    fn prepare_row(
        &self,
        input: &CreateContentTypeInput,
    ) -> std::result::Result<NewContentTypeRow, cycms_core::Error> {
        let api_id = normalize_api_id(&input.api_id)?;
        let name = normalize_name(&input.name)?;
        let description = input
            .description
            .as_ref()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty());
        let fields = normalize_fields(input.fields.clone())?;
        validate_field_definitions(&fields, &self.field_types)?;

        Ok(NewContentTypeRow {
            id: new_content_type_id(),
            name,
            api_id,
            description,
            kind: input.kind,
            fields,
        })
    }

    async fn require_type(&self, api_id: &str) -> Result<ContentTypeDefinition> {
        let normalized = normalize_api_id(api_id)?;
        self.repo
            .find_by_api_id(&normalized)
            .await?
            .ok_or_else(|| ContentModelError::NotFound(normalized).into())
    }
}

fn normalize_fields(
    fields: Vec<FieldDefinition>,
) -> std::result::Result<Vec<FieldDefinition>, ContentModelError> {
    fields
        .into_iter()
        .map(|mut f| {
            f.api_id = normalize_api_id(&f.api_id)?;
            f.name = normalize_name(&f.name)?;
            Ok(f)
        })
        .collect()
}

fn log_field_diff(api_id: &str, old: &[FieldDefinition], new: &[FieldDefinition]) {
    let old_ids: HashSet<&str> = old.iter().map(|f| f.api_id.as_str()).collect();
    let new_ids: HashSet<&str> = new.iter().map(|f| f.api_id.as_str()).collect();

    for removed in old_ids.difference(&new_ids) {
        warn!(
            content_type = %api_id,
            field = %removed,
            "field removed on content type update (TODO: check referencing entries)"
        );
    }
    for added in new_ids.difference(&old_ids) {
        warn!(
            content_type = %api_id,
            field = %added,
            "field added on content type update"
        );
    }
    for f_new in new {
        if let Some(f_old) = old.iter().find(|o| o.api_id == f_new.api_id)
            && f_old.field_type != f_new.field_type
        {
            warn!(
                content_type = %api_id,
                field = %f_new.api_id,
                "field type changed on content type update (TODO: migrate data)"
            );
        }
    }
}
