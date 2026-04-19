//! 三方言 CRUD 实现；C3 / C5 分别填充 settings 表与 `plugin_settings_schemas` 表。

use std::sync::Arc;

use cycms_db::DatabasePool;

/// `settings` 表 CRUD 占位；C3 填充实际方法。
pub struct SettingsRepository {
    #[allow(dead_code)]
    db: Arc<DatabasePool>,
}

impl SettingsRepository {
    #[must_use]
    pub fn new(db: Arc<DatabasePool>) -> Self {
        Self { db }
    }
}

/// `plugin_settings_schemas` 表 CRUD 占位；C5 填充实际方法。
pub struct PluginSchemaRepository {
    #[allow(dead_code)]
    db: Arc<DatabasePool>,
}

impl PluginSchemaRepository {
    #[must_use]
    pub fn new(db: Arc<DatabasePool>) -> Self {
        Self { db }
    }
}
