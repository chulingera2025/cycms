//! [`SettingsManager`] 门面；C4 / C5 依次挂入各能力。

use std::collections::HashMap;
use std::sync::Arc;

use cycms_core::Result;
use cycms_db::DatabasePool;
use serde_json::Value;

use crate::model::SettingEntry;
use crate::repository::{PluginSchemaRepository, SettingsRepository};

/// 系统与插件设置的统一访问门面。
///
/// 在任务 8 的各子阶段逐步填充：C4 挂入 `get/set/delete/get_all`，C5 挂入 schema
/// 相关方法；最终由 C7 注入到 `AppContext.settings_manager`。
pub struct SettingsManager {
    #[allow(dead_code)]
    db: Arc<DatabasePool>,
    settings: SettingsRepository,
    schemas: PluginSchemaRepository,
}

impl SettingsManager {
    #[must_use]
    pub fn new(db: Arc<DatabasePool>) -> Self {
        let settings = SettingsRepository::new(Arc::clone(&db));
        let schemas = PluginSchemaRepository::new(Arc::clone(&db));
        Self {
            db,
            settings,
            schemas,
        }
    }

    #[must_use]
    pub fn settings(&self) -> &SettingsRepository {
        &self.settings
    }

    #[must_use]
    pub fn schemas(&self) -> &PluginSchemaRepository {
        &self.schemas
    }

    /// 读取单条设置值；设置不存在时返回 `None`。
    ///
    /// # Errors
    /// 见 [`SettingsRepository::find`]。
    pub async fn get(&self, namespace: &str, key: &str) -> Result<Option<Value>> {
        Ok(self
            .settings
            .find(namespace, key)
            .await?
            .map(|entry| entry.value))
    }

    /// 写入 / 覆盖设置值，返回写入后的实体（含 `updated_at`）。
    ///
    /// # Errors
    /// 见 [`SettingsRepository::upsert`]。
    pub async fn set(&self, namespace: &str, key: &str, value: Value) -> Result<SettingEntry> {
        self.settings.upsert(namespace, key, value).await
    }

    /// 删除指定设置；若设置不存在则返回 `false`。
    ///
    /// # Errors
    /// 见 [`SettingsRepository::delete`]。
    pub async fn delete(&self, namespace: &str, key: &str) -> Result<bool> {
        self.settings.delete(namespace, key).await
    }

    /// 读取某 namespace 下所有设置的 `key -> value` 映射。
    ///
    /// # Errors
    /// 见 [`SettingsRepository::list_by_namespace`]。
    pub async fn get_all(&self, namespace: &str) -> Result<HashMap<String, Value>> {
        let entries = self.settings.list_by_namespace(namespace).await?;
        Ok(entries
            .into_iter()
            .map(|entry| (entry.key, entry.value))
            .collect())
    }
}
