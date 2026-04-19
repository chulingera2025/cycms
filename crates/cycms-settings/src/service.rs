//! [`SettingsManager`] 门面；C4 / C5 依次挂入各能力。

use std::sync::Arc;

use cycms_db::DatabasePool;

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
}
